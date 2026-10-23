use crate::model::ChunkInfo;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, RANGE, USER_AGENT};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

pub struct ProbeResult {
    pub total_bytes: Option<u64>,
    pub supports_range: bool,
    pub suggested_filename: Option<String>,
}

pub struct Segmenter {
    client: reqwest::Client,
}

impl Segmenter {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(32)
            .build()
            .unwrap_or_default();

        Self { client }
    }

    /// Probe the URL to determine file size, range support, and filename
    pub async fn probe(&self, url: &str, cookies: Option<&str>) -> Result<ProbeResult> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (X11; Linux x86_64) SIDM/1.0"),
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

    /// Execute high-speed segmented download with Zero-Copy Pre-Allocation
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
        let conn_count = if supports_range && total_bytes.is_some() && total_bytes.unwrap() > 1024 * 1024 {
            connections.clamp(2, 32)
        } else {
            1
        };

        let total_size = total_bytes.unwrap_or(0);

        // Step 1: Zero-Copy Pre-Allocation (IDM freeze bypass)
        {
            let file = std::fs::File::create(save_path)
                .with_context(|| format!("Failed to create destination file {:?}", save_path))?;
            if total_size > 0 {
                // Pre-allocate the exact file size so chunks can be written anywhere without end-of-download copy
                let _ = file.set_len(total_size);
            }
        }

        // Single connection fallback if server doesn't support ranges or size is unknown
        if conn_count == 1 || total_size == 0 {
            return self
                .download_single_stream(url, save_path, total_size, cookies, pause_rx, on_progress)
                .await;
        }

        // Calculate chunk byte boundaries
        let chunk_size = total_size / (conn_count as u64);
        let mut chunks = Vec::with_capacity(conn_count);

        for i in 0..conn_count {
            let start_byte = i as u64 * chunk_size;
            let end_byte = if i == conn_count - 1 {
                total_size - 1
            } else {
                (i as u64 + 1) * chunk_size - 1
            };

            chunks.push(ChunkInfo {
                index: i,
                start_byte,
                end_byte,
                downloaded_bytes: 0,
                active: true,
            });
        }

        let downloaded_total = Arc::new(AtomicU64::new(0));
        let chunks_state = Arc::new(tokio::sync::RwLock::new(chunks.clone()));
        let is_cancelled = Arc::new(AtomicBool::new(false));

        // Spawn progress reporting loop
        let progress_downloaded = downloaded_total.clone();
        let progress_chunks = chunks_state.clone();
        let progress_cancelled = is_cancelled.clone();
        let progress_handle = tokio::spawn(async move {
            let mut last_bytes = 0u64;
            let mut last_tick = Instant::now();

            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
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

                let current_chunks = progress_chunks.read().await.clone();
                on_progress(current_bytes, speed, current_chunks);

                if total_size > 0 && current_bytes >= total_size {
                    break;
                }
            }
        });

        // Spawn worker tasks for each connection chunk
        let mut worker_handles = Vec::new();

        for chunk in chunks {
            let client = self.client.clone();
            let url = url.to_string();
            let cookies = cookies.map(|s| s.to_string());
            let path = save_path.to_path_buf();
            let downloaded_total = downloaded_total.clone();
            let chunks_state = chunks_state.clone();
            let mut pause_rx = pause_rx.clone();

            let handle = tokio::spawn(async move {
                let mut chunk_file = std::fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .with_context(|| format!("Worker failed to open {:?}", path))?;

                // Seek to this chunk's start byte
                chunk_file.seek(SeekFrom::Start(chunk.start_byte))?;

                let range_header = format!("bytes={}-{}", chunk.start_byte, chunk.end_byte);
                let mut req = client.get(&url).header(RANGE, range_header);

                if let Some(cookie_str) = &cookies {
                    req = req.header(reqwest::header::COOKIE, cookie_str);
                }

                let resp = req.send().await?;
                if !resp.status().is_success() {
                    return Err(anyhow!("Chunk {} HTTP error: {}", chunk.index, resp.status()));
                }

                let mut stream = resp.bytes_stream();
                let mut chunk_downloaded = 0u64;

                while let Some(item) = stream.next().await {
                    // Check pause
                    if *pause_rx.borrow_and_update() {
                        return Err(anyhow!("Download paused"));
                    }

                    let bytes = item?;
                    let len = bytes.len() as u64;

                    chunk_file.write_all(&bytes)?;
                    chunk_downloaded += len;
                    downloaded_total.fetch_add(len, Ordering::Relaxed);

                    // Update chunk progress periodically
                    if chunk_downloaded % (128 * 1024) < len {
                        if let Ok(mut state) = chunks_state.try_write() {
                            if let Some(c) = state.get_mut(chunk.index) {
                                c.downloaded_bytes = chunk_downloaded;
                            }
                        }
                    }
                }

                // Final chunk update
                {
                    let mut state = chunks_state.write().await;
                    if let Some(c) = state.get_mut(chunk.index) {
                        c.downloaded_bytes = chunk_downloaded;
                        c.active = false;
                    }
                }

                chunk_file.flush()?;
                Ok::<(), anyhow::Error>(())
            });

            worker_handles.push(handle);
        }

        // Wait for all chunk workers to complete
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

    /// Single stream fallback download
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

        while let Some(chunk) = stream.next().await {
            if *pause_rx.borrow_and_update() {
                return Err(anyhow!("Download paused"));
            }

            let bytes = chunk?;
            file.write_all(&bytes)?;
            downloaded += bytes.len() as u64;

            let now = Instant::now();
            if now.duration_since(last_tick).as_millis() >= 400 {
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

        file.flush()?;
        Ok(())
    }
}
