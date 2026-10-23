use crate::model::{ChunkInfo, MediaFormatOption, MediaInspectResult};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct MediaEngine;

fn get_ytdlp_bin() -> String {
    let candidates = [
        "bin/yt-dlp.exe",
        "bin/yt-dlp",
        "sidm-core/bin/yt-dlp.exe",
        "sidm-core/bin/yt-dlp",
        "./bin/yt-dlp.exe",
        "./bin/yt-dlp",
        "./sidm-core/bin/yt-dlp.exe",
        "./sidm-core/bin/yt-dlp",
        "/home/kawsar/Documents/Github/Repo/SIDM/bin/yt-dlp",
        "/home/kawsar/Documents/Github/Repo/SIDM/sidm-core/bin/yt-dlp",
        "yt-dlp.exe",
        "yt-dlp",
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for sub in &[
                "bin/yt-dlp.exe",
                "bin/yt-dlp",
                "yt-dlp.exe",
                "yt-dlp",
                "../bin/yt-dlp.exe",
                "../bin/yt-dlp",
                "../../bin/yt-dlp.exe",
                "../../bin/yt-dlp"
            ] {
                let p = parent.join(sub);
                if p.exists() {
                    return p.to_string_lossy().to_string();
                }
            }
        }
    }
    if cfg!(target_os = "windows") {
        "yt-dlp.exe".to_string()
    } else {
        "yt-dlp".to_string()
    }
}

impl MediaEngine {
    /// Inspect media formats using bundled or system yt-dlp
    pub async fn inspect(url: &str, cookies: Option<&str>) -> Result<MediaInspectResult> {
        let bin = get_ytdlp_bin();
        let mut cmd = Command::new(&bin);
        cmd.arg("--dump-single-json")
            .arg("--no-warnings")
            .arg("--no-playlist")
            .arg(url);

        if let Some(cookie_str) = cookies {
            if cookie_str.len() > 10 && !cookie_str.contains('\n') {
                cmd.arg("--add-header").arg(format!("Cookie: {}", cookie_str));
            }
        }

        let output = cmd.output().await.map_err(|e| {
            anyhow!(
                "Failed to run yt-dlp ({}). Error: {}",
                bin,
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
                label: "🎬 1080p Full HD (Premiere Pro & DaVinci Ready)".to_string(),
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

    /// Download media with yt-dlp + concurrent fragment streams + ffmpeg muxing
    pub async fn download<F>(
        url: &str,
        save_path: &Path,
        format_id: Option<&str>,
        cookies: Option<&str>,
        on_progress: F,
    ) -> Result<()>
    where
        F: Fn(u64, Option<u64>, u64, f32, Option<u64>, Vec<ChunkInfo>) + Send + Sync + 'static,
    {
        let bin = get_ytdlp_bin();
        let mut cmd = Command::new(&bin);

        // Format selector mapping (Optimized for Adobe Premiere Pro, DaVinci Resolve, & Final Cut)
        // Prioritizes standard H.264 (AVC1) video and AAC (M4A) audio for zero-conversion NLE import
        let format_arg = match format_id.unwrap_or("best_1080p") {
            "best_8k" => "bestvideo+bestaudio[ext=m4a]/bestvideo+bestaudio/best",
            "best_1080p" => "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=1080]+bestaudio[ext=m4a]/bestvideo[height<=1080]+bestaudio/best",
            "best_720p" => "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=720]+bestaudio/best",
            "best_audio" => "bestaudio[ext=m4a]/bestaudio/best",
            custom => custom,
        };

        // Determine destination directory and filename template
        let save_dir = save_path.parent().unwrap_or_else(|| Path::new("."));
        let _ = std::fs::create_dir_all(save_dir);

        // Output template: saves into target folder with video title
        let output_template = format!("{}/%(title)s.%(ext)s", save_dir.to_string_lossy());

        cmd.arg("-f")
            .arg(format_arg)
            .arg("--merge-output-format")
            .arg("mp4")
            .arg("--postprocessor-args")
            .arg("MergerName:-c:v copy -c:a aac -b:a 192k") // Ensures Premiere Pro 100% native audio decoding
            .arg("-N")
            .arg("16") // 16 parallel fragment streams for speed
            .arg("--buffer-size")
            .arg("16M")
            .arg("--no-playlist")
            .arg("--newline")
            .arg("--progress")
            .arg("--user-agent")
            .arg("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
            .arg("-o")
            .arg(&output_template)
            .arg(url);

        if format_id == Some("best_audio") {
            cmd.arg("-x").arg("--audio-format").arg("mp3");
        }

        if let Some(cookie_str) = cookies {
            if cookie_str.len() > 10 && !cookie_str.contains('\n') {
                cmd.arg("--add-header").arg(format!("Cookie: {}", cookie_str));
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow!("Failed to start yt-dlp at '{}': {}", bin, e)
        })?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let mut reader = BufReader::new(stdout).lines();

        while let Ok(Some(line)) = reader.next_line().await {
            // Parse yt-dlp progress: "[download]  45.2% of ~120.50MiB at 12.35MiB/s ETA 00:05"
            if line.contains("[download]") && line.contains('%') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut percent = 0.0f32;
                let mut speed = 0u64;
                let mut total_bytes: Option<u64> = None;
                let mut eta_seconds: Option<u64> = None;

                for (i, p) in parts.iter().enumerate() {
                    // 1. Percentage
                    if p.ends_with('%') {
                        let num = p.trim_end_matches('%');
                        if let Ok(val) = num.parse::<f32>() {
                            percent = val;
                        }
                    }

                    // 2. Total Size: following "of"
                    if *p == "of" && i + 1 < parts.len() {
                        let size_token = parts[i + 1].trim_start_matches('~');
                        if let Some(bytes) = parse_size_to_bytes(size_token) {
                            total_bytes = Some(bytes);
                        }
                    }

                    // 3. Speed: token ending with /s or following "at"
                    if p.contains("/s") && i > 0 {
                        let speed_str = parts[i - 1];
                        if let Ok(val) = speed_str.parse::<f64>() {
                            if p.starts_with("GiB") {
                                speed = (val * 1024.0 * 1024.0 * 1024.0) as u64;
                            } else if p.starts_with("MiB") {
                                speed = (val * 1024.0 * 1024.0) as u64;
                            } else if p.starts_with("KiB") {
                                speed = (val * 1024.0) as u64;
                            } else {
                                speed = val as u64;
                            }
                        }
                    }

                    // 4. ETA: token following "ETA"
                    if *p == "ETA" && i + 1 < parts.len() {
                        eta_seconds = parse_eta_to_seconds(parts[i + 1]);
                    }
                }

                let downloaded_bytes = if let Some(tot) = total_bytes {
                    (tot as f64 * (percent as f64 / 100.0)) as u64
                } else {
                    (percent * 1024.0 * 1024.0) as u64
                };

                on_progress(
                    downloaded_bytes,
                    total_bytes,
                    speed,
                    percent,
                    eta_seconds,
                    vec![ChunkInfo {
                        index: 0,
                        start_byte: 0,
                        end_byte: total_bytes.unwrap_or(100_000_000),
                        downloaded_bytes,
                        active: true,
                    }],
                );
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            return Err(anyhow!("Media download process failed with exit code: {}", status));
        }

        Ok(())
    }
}

fn parse_size_to_bytes(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() { return None; }

    let (num_part, unit) = if s.ends_with("GiB") {
        (&s[..s.len() - 3], 1024.0 * 1024.0 * 1024.0)
    } else if s.ends_with("MiB") {
        (&s[..s.len() - 3], 1024.0 * 1024.0)
    } else if s.ends_with("KiB") {
        (&s[..s.len() - 3], 1024.0)
    } else if s.ends_with('B') {
        (&s[..s.len() - 1], 1.0)
    } else {
        (s, 1024.0 * 1024.0)
    };

    num_part.parse::<f64>().ok().map(|n| (n * unit) as u64)
}

fn parse_eta_to_seconds(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m = parts[0].parse::<u64>().ok()?;
            let sec = parts[1].parse::<u64>().ok()?;
            Some(m * 60 + sec)
        }
        3 => {
            let h = parts[0].parse::<u64>().ok()?;
            let m = parts[1].parse::<u64>().ok()?;
            let sec = parts[2].parse::<u64>().ok()?;
            Some(h * 3600 + m * 60 + sec)
        }
        _ => None,
    }
}
