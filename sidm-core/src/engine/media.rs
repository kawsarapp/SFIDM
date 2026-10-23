use crate::model::{ChunkInfo, MediaFormatOption, MediaInspectResult};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct MediaEngine;

impl MediaEngine {
    /// Inspect media formats using yt-dlp
    pub async fn inspect(url: &str, cookies: Option<&str>) -> Result<MediaInspectResult> {
        let mut cmd = Command::new("yt-dlp");
        cmd.arg("--dump-single-json")
            .arg("--no-warnings")
            .arg("--no-playlist")
            .arg(url);

        if let Some(cookie_str) = cookies {
            if !cookie_str.is_empty() {
                cmd.arg("--add-header").arg(format!("Cookie: {}", cookie_str));
            }
        }

        let output = cmd.output().await.map_err(|e| {
            anyhow!(
                "Failed to run yt-dlp. Make sure yt-dlp is installed on your system. Error: {}",
                e
            )
        })?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Media inspection failed: {}", err_msg));
        }

        let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let title = json_val
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Title")
            .to_string();

        let uploader = json_val
            .get("uploader")
            .or_else(|| json_val.get("channel"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let thumbnail = json_val
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let duration_seconds = json_val
            .get("duration")
            .and_then(|v| v.as_u64());

        let mut formats = Vec::new();

        // Extract available formats and simplify for the user
        if let Some(formats_arr) = json_val.get("formats").and_then(|v| v.as_array()) {
            for f in formats_arr {
                let format_id = f.get("format_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let ext = f.get("ext").and_then(|v| v.as_str()).unwrap_or("mp4").to_string();
                let height = f.get("height").and_then(|v| v.as_u64());
                let fps = f.get("fps").and_then(|v| v.as_f64()).map(|f| f as f32);
                let filesize = f.get("filesize").or_else(|| f.get("filesize_approx")).and_then(|v| v.as_u64());
                let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
                let acodec = f.get("acodec").and_then(|v| v.as_str()).unwrap_or("none");

                let has_video = vcodec != "none";
                let has_audio = acodec != "none";

                let resolution = if let Some(h) = height {
                    Some(format!("{}p", h))
                } else {
                    None
                };

                let label = if has_video {
                    let h_label = match height {
                        Some(4320) => "8K Ultra HD",
                        Some(2160) => "4K UHD",
                        Some(1440) => "1440p 2K QHD",
                        Some(1080) => "1080p Full HD",
                        Some(720) => "720p HD",
                        Some(480) => "480p SD",
                        Some(360) => "360p",
                        Some(h) => &format!("{}p", h),
                        None => "Video",
                    };
                    if let Some(f_rate) = fps {
                        if f_rate >= 50.0 {
                            format!("{} ({} FPS) [{}]", h_label, f_rate as u32, ext)
                        } else {
                            format!("{} [{}]", h_label, ext)
                        }
                    } else {
                        format!("{} [{}]", h_label, ext)
                    }
                } else if has_audio {
                    let abr = f.get("abr").and_then(|v| v.as_f64()).unwrap_or(128.0) as u32;
                    format!("Audio Only ({} kbps) [{}]", abr, ext)
                } else {
                    format!("Format {} [{}]", format_id, ext)
                };

                formats.push(MediaFormatOption {
                    format_id,
                    label,
                    ext,
                    resolution,
                    fps,
                    filesize_approx: filesize,
                    has_video,
                    has_audio,
                    direct_url: f.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
                });
            }
        }

        // Add standard smart presets at the top
        let mut smart_presets = vec![
            MediaFormatOption {
                format_id: "best_8k".to_string(),
                label: "🌟 Best Available Quality (Up to 8K Ultra HD)".to_string(),
                ext: "mp4".to_string(),
                resolution: Some("4320p".to_string()),
                fps: Some(60.0),
                filesize_approx: None,
                has_video: true,
                has_audio: true,
                direct_url: None,
            },
            MediaFormatOption {
                format_id: "best_1080p".to_string(),
                label: "🎬 1080p Full HD (Recommended)".to_string(),
                ext: "mp4".to_string(),
                resolution: Some("1080p".to_string()),
                fps: Some(60.0),
                filesize_approx: None,
                has_video: true,
                has_audio: true,
                direct_url: None,
            },
            MediaFormatOption {
                format_id: "best_720p".to_string(),
                label: "⚡ 720p HD (Fast & Compact)".to_string(),
                ext: "mp4".to_string(),
                resolution: Some("720p".to_string()),
                fps: None,
                filesize_approx: None,
                has_video: true,
                has_audio: true,
                direct_url: None,
            },
            MediaFormatOption {
                format_id: "best_audio".to_string(),
                label: "🎵 Audio Only (MP3 320kbps High Quality)".to_string(),
                ext: "mp3".to_string(),
                resolution: None,
                fps: None,
                filesize_approx: None,
                has_video: false,
                has_audio: true,
                direct_url: None,
            },
        ];

        // Deduplicate or append specific video formats
        smart_presets.extend(formats);

        Ok(MediaInspectResult {
            url: url.to_string(),
            title,
            uploader,
            thumbnail,
            duration_seconds,
            formats: smart_presets,
        })
    }

    /// Download media with yt-dlp + concurrent connections (-N 8) + ffmpeg muxing
    pub async fn download<F>(
        url: &str,
        save_path: &Path,
        format_id: Option<&str>,
        cookies: Option<&str>,
        on_progress: F,
    ) -> Result<()>
    where
        F: Fn(u64, u64, Vec<ChunkInfo>) + Send + Sync + 'static,
    {
        let mut cmd = Command::new("yt-dlp");

        // Format selector mapping
        let format_arg = match format_id.unwrap_or("best_1080p") {
            "best_8k" => "bestvideo+bestaudio/best",
            "best_1080p" => "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best",
            "best_720p" => "bestvideo[height<=720]+bestaudio/best[height<=720]/best",
            "best_audio" => "bestaudio/best",
            custom => custom,
        };

        cmd.arg("-f")
            .arg(format_arg)
            .arg("--merge-output-format")
            .arg("mp4")
            .arg("-N")
            .arg("8") // 8 parallel fragment connections for speed!
            .arg("--newline")
            .arg("--progress")
            .arg("-o")
            .arg(save_path.to_str().unwrap_or("download.mp4"))
            .arg(url);

        if format_id == Some("best_audio") {
            cmd.arg("-x").arg("--audio-format").arg("mp3");
        }

        if let Some(cookie_str) = cookies {
            if !cookie_str.is_empty() {
                cmd.arg("--add-header").arg(format!("Cookie: {}", cookie_str));
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow!("Failed to start media download process: {}", e)
        })?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let mut reader = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            // Parse yt-dlp progress: "[download]  45.2% of ~120.50MiB at 12.35MiB/s ETA 00:05"
            if line.contains("[download]") && line.contains('%') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut percent = 0.0f32;
                let mut speed = 0u64;

                for (i, p) in parts.iter().enumerate() {
                    if p.ends_with('%') {
                        let num = p.trim_end_matches('%');
                        percent = num.parse::<f32>().unwrap_or(0.0);
                    }
                    if p.contains("/s") && i > 0 {
                        let speed_str = parts[i - 1];
                        let val = speed_str.parse::<f64>().unwrap_or(0.0);
                        if p.starts_with("MiB") {
                            speed = (val * 1024.0 * 1024.0) as u64;
                        } else if p.starts_with("KiB") {
                            speed = (val * 1024.0) as u64;
                        } else {
                            speed = val as u64;
                        }
                    }
                }

                let simulated_bytes = (percent * 100_000.0) as u64;
                on_progress(
                    simulated_bytes,
                    speed,
                    vec![ChunkInfo {
                        index: 0,
                        start_byte: 0,
                        end_byte: 100_000_000,
                        downloaded_bytes: simulated_bytes,
                        active: true,
                    }],
                );
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(anyhow!("Media download process failed with status {}", status));
        }

        Ok(())
    }
}
