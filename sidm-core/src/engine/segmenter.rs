use crate::model::ChunkInfo;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, RANGE, USER_AGENT};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{watch, RwLock};

const WRITE_BUFFER_SIZE: usize = 512 * 1024; // 512 KB bulk write buffer
const MIN_STEAL_THRESHOLD: u64 = 3 * 1024 * 1024; // 3 MB minimum to warrant dynamic work stealing

pub struct ProbeResult {
    pub total_bytes: Option<u64>,
    pub supports_range: bool,
    pub suggested_filename: Option<String>,
}

#[derive(Clone)]
struct DynamicChunkState {
    pub index: usize,
    pub start_byte: u64,
    pub end_byte: u64,
    pub current_offset: u64,
    pub downloaded_bytes: u64,
    pub active: bool,
}

pub struct Segmenter {
    client: reqwest::Client,
}

impl Segmenter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .tcp_nodelay(true) // Disable Nagle's algorithm for zero packet delay
            .pool_max_idle_per_host(64)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Probe the URL to determine file size, range support, and filename
    pub async fn probe(&self, url: &str, cookies: Option<&str>) -> Result<ProbeResult> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 SIDM/2.0 Turbo"),
        );
        if let Some(cookie_str) = cookies {
            if let Ok(val) = HeaderValue::from_str(cookie_str) {
                headers.insert(reqwest::header::COOKIE, val);
            }
        }

        // Try HEAD first
        let head_resp = self.client.head(url).headers(headers.clone()).send().await;

        let resp = match head_resp {
            Ok(r) if r.status().is_success() => r,
            _ => {
                // If HEAD fails or is rejected, do a Range: bytes=0-0 GET probe
                headers.insert(RANGE, HeaderValue::from_static("bytes=0-0"));
                self.client.get(url).headers(headers).send().await?
            }
        };

        let headers = resp.headers();

        // 1. Content-Length
        let total_bytes = headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .or_else(|| {
                // Check Content-Range (e.g. bytes 0-0/1048576)
                headers
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|cr| cr.split('/').last())
                    .and_then(|v| v.parse::<u64>().ok())
            });

        // 2. Range support
        let supports_range = headers
            .get(ACCEPT_RANGES)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_lowercase().contains("bytes"))
            .unwrap_or(false)
            || headers.contains_key("content-range");

        // 3. Suggested filename from Content-Disposition
        let suggested_filename = headers
            .get(CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .and_then(|cd| {
                if let Some(idx) = cd.find("filename=") {
                    let raw = &cd[idx + 9..];
                    let clean = raw.trim_matches(|c| c == '"' || c == '\'' || c == ' ');
                    Some(clean.split(';').next().unwrap_or(clean).to_string())
                } else {
                    None
                }
            });

        Ok(ProbeResult {
            total_bytes,
            supports_range,
            suggested_filename,
        })
    }

    /// Execute Extreme Turbo segmented download with Zero-Copy Pre-Allocation & Work Stealing
    pub async fn download<F>(
        &self,
        url: &str,
        save_path: &Path,
        total_bytes: Option<u64>,
        supports_range: bool,
        connections: usize,
        cookies: Option<&str>,
        pause_rx: watch::Receiver<bool>,
        on_progress: F,
    ) -> Result<()>
    where
        F: Fn(u64, u64, Vec<ChunkInfo>) + Send + Sync + 'static,
    {
        let conn_count = if supports_range && total_bytes.is_some() && total_bytes.unwrap() > 512 * 1024 {
            connections.clamp(2, 64)
        } else {
            1
        };

        let total_size = total_bytes.unwrap_or(0);

        // Step 1: Zero-Copy Pre-Allocation (0ms wait at 100%)
        {
            let file = std::fs::File::create(save_path)
                .with_context(|| format!("Failed to create destination file {:?}", save_path))?;
            if total_size > 0 {
                let _ = file.set_len(total_size);
            }
        }

        // Single stream fallback if server doesn't support ranges
        if conn_count == 1 || total_size == 0 {
            return self
                .download_single_stream(url, save_path, total_size, cookies, pause_rx, on_progress)
                .await;
        }

        // Initial chunk distribution
        let chunk_size = total_size / (conn_count as u64);
        let mut initial_chunks = Vec::with_capacity(conn_count);

        for i in 0..conn_count {
            let start_byte = i as u64 * chunk_size;
            let end_byte = if i == conn_count - 1 {
                total_size - 1
            } else {
                (i as u64 + 1) * chunk_size - 1
            };

            initial_chunks.push(DynamicChunkState {
                index: i,
                start_byte,
                end_byte,
                current_offset: start_byte,
                downloaded_bytes: 0,
                active: true,
            });
        }

        let downloaded_total = Arc::new(AtomicU64::new(0));
        let chunk_states = Arc::new(RwLock::new(initial_chunks));
        let is_cancelled = Arc::new(AtomicBool::new(false));

        // Background high-frequency progress monitor
        let progress_downloaded = downloaded_total.clone();
        let progress_chunks = chunk_states.clone();
        let progress_cancelled = is_cancelled.clone();
        let progress_handle = tokio::spawn(async move {
            let mut last_bytes = 0u64;
            let mut last_tick = Instant::now();

            loop {
                tokio::time::sleep(Duration::from_millis(300)).await;
                if progress_cancelled.load(Ordering::Relaxed) {
                    break;
                }

                let current_bytes = progress_downloaded.load(Ordering::Relaxed);
                let now = Instant::now();
                let elapsed = now.duration_since(last_tick).as_secs_f64();

                let speed = if elapsed > 0.0 {
                    ((current_bytes.saturating_sub(last_bytes)) as f64 / elapsed) as u64
                } else {
                    0
                };

                last_bytes = current_bytes;
                last_tick = now;

                let states = progress_chunks.read().await;
                let ui_chunks: Vec<ChunkInfo> = states
                    .iter()
                    .map(|c| ChunkInfo {
                        index: c.index,
                        start_byte: c.start_byte,
                        end_byte: c.end_byte,
                        downloaded_bytes: c.downloaded_bytes,
                        active: c.active,
                    })
                    .collect();

                on_progress(current_bytes, speed, ui_chunks);

                if total_size > 0 && current_bytes >= total_size {
                    break;
                }
            }
        });

        // Launch concurrent workers with Dynamic Work Stealing
        let mut worker_handles = Vec::new();

        for worker_id in 0..conn_count {
            let client = self.client.clone();
            let url = url.to_string();
            let cookies = cookies.map(|s| s.to_string());
            let path = save_path.to_path_buf();
            let downloaded_total = downloaded_total.clone();
            let chunk_states = chunk_states.clone();
            let mut pause_rx = pause_rx.clone();

            let handle = tokio::spawn(async move {
                // Outer loop: continuously execute assigned chunk or steal from the slowest chunk
                loop {
                    // Check pause
                    if *pause_rx.borrow_and_update() {
                        return Err(anyhow!("Download paused"));
                    }

                    // Get or steal chunk
                    let assignment = {
                        let mut states = chunk_states.write().await;

                        // First try worker's own initial slot
                        if let Some(c) = states.get_mut(worker_id) {
                            if c.active && c.current_offset <= c.end_byte {
                                Some((c.index, c.current_offset, c.end_byte))
                            } else {
                                // Dynamic Work Stealing: Find slowest/largest chunk with remaining > MIN_STEAL_THRESHOLD
                                let mut best_candidate: Option<(usize, u64)> = None;
                                for item in states.iter() {
                                    if item.active && item.end_byte > item.current_offset {
                                        let remaining = item.end_byte - item.current_offset;
                                        if remaining >= MIN_STEAL_THRESHOLD {
                                            if let Some((_, best_rem)) = best_candidate {
                                                if remaining > best_rem {
                                                    best_candidate = Some((item.index, remaining));
                                                }
                                            } else {
                                                best_candidate = Some((item.index, remaining));
                                            }
                                        }
                                    }
                                }

                                if let Some((target_idx, remaining)) = best_candidate {
                                    let half = remaining / 2;
                                    let old_end = states[target_idx].end_byte;
                                    let split_point = states[target_idx].current_offset + half;

                                    // Shrink the target chunk
                                    states[target_idx].end_byte = split_point;

                                    // Create a new stolen chunk for this worker
                                    let new_idx = states.len();
                                    states.push(DynamicChunkState {
                                        index: new_idx,
                                        start_byte: split_point + 1,
                                        end_byte: old_end,
                                        current_offset: split_point + 1,
                                        downloaded_bytes: 0,
                                        active: true,
                                    });

                                    Some((new_idx, split_point + 1, old_end))
                                } else {
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    };

                    let (chunk_idx, range_start, range_end) = match assignment {
                        Some(a) => a,
                        None => break, // No more work to steal, worker can rest!
                    };

                    if range_start > range_end {
                        let mut states = chunk_states.write().await;
                        if let Some(c) = states.get_mut(chunk_idx) {
                            c.active = false;
                        }
                        continue;
                    }

                    // Open file handle with seek
                    let mut chunk_file = std::fs::OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .with_context(|| format!("Worker failed to open {:?}", path))?;

                    chunk_file.seek(SeekFrom::Start(range_start))?;

                    let range_header = format!("bytes={}-{}", range_start, range_end);
                    let mut req = client.get(&url).header(RANGE, range_header);

                    if let Some(cookie_str) = &cookies {
                        req = req.header(reqwest::header::COOKIE, cookie_str);
                    }

                    let resp = req.send().await?;
                    if !resp.status().is_success() {
                        return Err(anyhow!("Chunk {} HTTP error: {}", chunk_idx, resp.status()));
                    }

                    let mut stream = resp.bytes_stream();
                    let mut write_buf = Vec::with_capacity(WRITE_BUFFER_SIZE);
                    let mut chunk_downloaded = 0u64;

                    while let Some(item) = stream.next().await {
                        if *pause_rx.borrow_and_update() {
                            return Err(anyhow!("Download paused"));
                        }

                        let bytes = item?;
                        let len = bytes.len() as u64;

                        write_buf.extend_from_slice(&bytes);
                        chunk_downloaded += len;
                        downloaded_total.fetch_add(len, Ordering::Relaxed);

                        // 512KB Block-Buffered Disk Flush (Bypasses disk thrashing)
                        if write_buf.len() >= WRITE_BUFFER_SIZE {
                            chunk_file.write_all(&write_buf)?;
                            write_buf.clear();

                            // Update chunk offset
                            if let Ok(mut states) = chunk_states.try_write() {
                                if let Some(c) = states.get_mut(chunk_idx) {
                                    c.current_offset = range_start + chunk_downloaded;
                                    c.downloaded_bytes = chunk_downloaded;
                                }
                            }
                        }
                    }

                    // Flush any remaining buffered bytes
                    if !write_buf.is_empty() {
                        chunk_file.write_all(&write_buf)?;
                        write_buf.clear();
                    }
                    chunk_file.flush()?;

                    // Mark chunk completed
                    {
                        let mut states = chunk_states.write().await;
                        if let Some(c) = states.get_mut(chunk_idx) {
                            c.downloaded_bytes = chunk_downloaded;
                            c.active = false;
                        }
                    }
                }

                Ok::<(), anyhow::Error>(())
            });

            worker_handles.push(handle);
        }

        // Wait for all workers to finish
        let mut first_error = None;
        for handle in worker_handles {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
                Err(join_err) => {
                    if first_error.is_none() {
                        first_error = Some(anyhow!("Worker panicked: {}", join_err));
                    }
                }
            }
        }

        is_cancelled.store(true, Ordering::Relaxed);
        let _ = progress_handle.await;

        if let Some(err) = first_error {
            return Err(err);
        }

        Ok(())
    }

    /// Single stream fallback download with block buffering
    async fn download_single_stream<F>(
        &self,
        url: &str,
        save_path: &Path,
        _total_size: u64,
        cookies: Option<&str>,
        mut pause_rx: watch::Receiver<bool>,
        on_progress: F,
    ) -> Result<()>
    where
        F: Fn(u64, u64, Vec<ChunkInfo>) + Send + Sync + 'static,
    {
        let mut file = std::fs::File::create(save_path)?;
        let mut req = self.client.get(url);
        if let Some(cookie_str) = cookies {
            req = req.header(reqwest::header::COOKIE, cookie_str);
        }

        let resp = req.send().await?;
        let mut stream = resp.bytes_stream();
        let mut downloaded = 0u64;
        let mut last_bytes = 0u64;
        let mut last_tick = Instant::now();
        let mut write_buf = Vec::with_capacity(WRITE_BUFFER_SIZE);

        while let Some(chunk) = stream.next().await {
            if *pause_rx.borrow_and_update() {
                return Err(anyhow!("Download paused"));
            }

            let bytes = chunk?;
            let len = bytes.len() as u64;
            write_buf.extend_from_slice(&bytes);
            downloaded += len;

            if write_buf.len() >= WRITE_BUFFER_SIZE {
                file.write_all(&write_buf)?;
                write_buf.clear();
            }

            let now = Instant::now();
            if now.duration_since(last_tick).as_millis() >= 300 {
                let elapsed = now.duration_since(last_tick).as_secs_f64();
                let speed = if elapsed > 0.0 {
                    ((downloaded.saturating_sub(last_bytes)) as f64 / elapsed) as u64
                } else {
                    0
                };
                last_bytes = downloaded;
                last_tick = now;

                on_progress(
                    downloaded,
                    speed,
                    vec![ChunkInfo {
                        index: 0,
                        start_byte: 0,
                        end_byte: downloaded,
                        downloaded_bytes: downloaded,
                        active: true,
                    }],
                );
            }
        }

        if !write_buf.is_empty() {
            file.write_all(&write_buf)?;
        }
        file.flush()?;
        Ok(())
    }
}
