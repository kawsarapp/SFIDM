use crate::model::{
    ChunkInfo, MediaFormatOption, MediaInspectResult, PlaylistItem, PlaylistInspectResult,
};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct MediaEngine;

fn get_ytdlp_bin() -> String {
    // 1. Check relative to current exe
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
                "../../bin/yt-dlp",
            ] {
                let p = parent.join(sub);
                if p.is_file() {
                    return p.to_string_lossy().to_string();
                }
            }
        }
    }

    // 2. Check local relative paths
    let candidates = [
        "bin/yt-dlp.exe",
        "bin/yt-dlp",
        "sidm-core/bin/yt-dlp.exe",
        "sidm-core/bin/yt-dlp",
        "./bin/yt-dlp.exe",
        "./bin/yt-dlp",
    ];
    for c in &candidates {
        if std::path::Path::new(c).is_file() {
            return c.to_string();
        }
    }

    // 3. Search system PATH
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let names = if cfg!(windows) {
                vec!["yt-dlp.exe", "yt-dlp.cmd", "yt-dlp"]
            } else {
                vec!["yt-dlp"]
            };
            for name in names {
                let p = dir.join(name);
                if p.is_file() {
                    return p.to_string_lossy().to_string();
                }
            }
        }
    }

    // 4. Standard platform paths
    let standard = if cfg!(windows) {
        vec!["C:\\Program Files\\yt-dlp\\yt-dlp.exe", "C:\\yt-dlp\\yt-dlp.exe"]
    } else {
        vec!["/usr/bin/yt-dlp", "/usr/local/bin/yt-dlp", "/snap/bin/yt-dlp"]
    };
    for s in standard {
        if std::path::Path::new(s).is_file() {
            return s.to_string();
        }
    }

    if cfg!(target_os = "windows") {
        "yt-dlp.exe".to_string()
    } else {
        "yt-dlp".to_string()
    }
}

pub fn strip_unc_prefix(path_str: &str) -> String {
    if let Some(s) = path_str.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", s)
    } else if let Some(s) = path_str.strip_prefix(r"\\?\") {
        s.to_string()
    } else {
        path_str.to_string()
    }
}

pub fn get_ffmpeg_bin() -> Option<String> {
    // 1. Check relative to current exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for sub in &[
                "bin/ffmpeg.exe",
                "bin/ffmpeg",
                "ffmpeg.exe",
                "ffmpeg",
                "../bin/ffmpeg.exe",
                "../bin/ffmpeg",
                "../../bin/ffmpeg.exe",
                "../../bin/ffmpeg",
            ] {
                let p = parent.join(sub);
                if p.is_file() {
                    let s = strip_unc_prefix(&p.to_string_lossy());
                    return Some(s);
                }
            }
        }
    }

    // 2. Check local relative paths
    let candidates = [
        "bin/ffmpeg.exe",
        "bin/ffmpeg",
        "sidm-core/bin/ffmpeg.exe",
        "sidm-core/bin/ffmpeg",
        "./bin/ffmpeg.exe",
        "./bin/ffmpeg",
    ];
    for c in &candidates {
        let p = std::path::Path::new(c);
        if p.is_file() {
            let s = strip_unc_prefix(&p.to_string_lossy());
            return Some(s);
        }
    }

    // 3. Search system PATH
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            let names = if cfg!(windows) {
                vec!["ffmpeg.exe", "ffmpeg.cmd", "ffmpeg"]
            } else {
                vec!["ffmpeg"]
            };
            for name in names {
                let p = dir.join(name);
                if p.is_file() {
                    let s = strip_unc_prefix(&p.to_string_lossy());
                    return Some(s);
                }
            }
        }
    }

    // 4. Standard platform directories
    let standard = if cfg!(windows) {
        let mut list = vec![
            "C:\\Program Files\\SIDM\\bin\\ffmpeg.exe".to_string(),
            "C:\\Program Files\\SIDM\\ffmpeg.exe".to_string(),
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
            "C:\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
            "C:\\ProgramData\\chocolatey\\bin\\ffmpeg.exe".to_string(),
        ];
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            list.push(format!("{}\\Programs\\SIDM\\bin\\ffmpeg.exe", local_app_data));
            list.push(format!("{}\\Programs\\SIDM\\ffmpeg.exe", local_app_data));
        }
        list
    } else {
        vec![
            "/usr/bin/ffmpeg".to_string(),
            "/usr/local/bin/ffmpeg".to_string(),
            "/snap/bin/ffmpeg".to_string(),
            "/opt/homebrew/bin/ffmpeg".to_string(),
        ]
    };
    for s in standard {
        let p = std::path::Path::new(&s);
        if p.is_file() {
            return Some(s);
        }
    }

    None
}

pub fn get_ffmpeg_dir() -> Option<String> {
    if let Some(bin_path) = get_ffmpeg_bin() {
        let p = std::path::Path::new(&bin_path);
        if let Some(parent) = p.parent() {
            let dir_str = strip_unc_prefix(&parent.to_string_lossy());
            return Some(dir_str);
        }
    }
    None
}

fn get_ffmpeg_location() -> Option<String> {
    get_ffmpeg_dir()
}

fn apply_cookies(cmd: &mut Command, url: &str, cookies: Option<&str>) {
    // Never pass browser cookies to YouTube. YouTube web cookies trigger SABR anti-bot 429 rate-limits.
    if url.contains("youtube.com") || url.contains("youtu.be") {
        return;
    }
    if let Some(cookie_str) = cookies {
        let clean = cookie_str.trim().replace('\r', "").replace('\n', "; ");
        if clean.len() > 5 {
            cmd.arg("--add-header").arg(format!("Cookie: {}", clean));
        }
    }
}

impl MediaEngine {
    /// Fast playlist inspection (titles, IDs, thumbnails, durations) without downloading video payloads
    pub async fn inspect_playlist(url: &str, cookies: Option<&str>) -> Result<PlaylistInspectResult> {
        let bin = get_ytdlp_bin();
        let mut cmd = Command::new(&bin);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        cmd.arg("--ignore-config")
            .arg("--flat-playlist")
            .arg("-J")
            .arg("--no-warnings");

        if url.contains("youtube.com") || url.contains("youtu.be") {
            cmd.arg("--extractor-args").arg("youtube:player_client=android,ios,web");
        }

        cmd.arg(url);

        if let Some(ref ffmpeg_loc) = get_ffmpeg_location() {
            cmd.arg("--ffmpeg-location").arg(ffmpeg_loc);
        }

        apply_cookies(&mut cmd, url, cookies);

        let output = cmd.output().await.map_err(|e| {
            anyhow!("Failed to run yt-dlp for playlist inspect ({}): {}", bin, e)
        })?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Playlist inspection failed: {}", err_msg));
        }

        let json_val: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let playlist_title = json_val
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Media Playlist")
            .to_string();

        let mut items = Vec::new();

        if let Some(entries) = json_val.get("entries").and_then(|v| v.as_array()) {
            for entry in entries {
                let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() {
                    continue;
                }

                let title = entry
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled Video")
                    .to_string();

                let item_url = entry
                    .get("url")
                    .and_then(|v| v.as_str())
                    .map(|u| {
                        if u.starts_with("http") {
                            u.to_string()
                        } else {
                            format!("https://www.youtube.com/watch?v={}", id)
                        }
                    })
                    .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", id));

                let duration_seconds = entry.get("duration").and_then(|v| v.as_u64());

                let thumbnail = entry
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        entry
                            .get("thumbnails")
                            .and_then(|t| t.as_array())
                            .and_then(|arr| arr.last())
                            .and_then(|th| th.get("url"))
                            .and_then(|u| u.as_str())
                    })
                    .map(|s| s.to_string());

                items.push(PlaylistItem {
                    id,
                    title,
                    url: item_url,
                    duration_seconds,
                    thumbnail,
                });
            }
        }

        let count = items.len();
        Ok(PlaylistInspectResult {
            title: playlist_title,
            count,
            items,
        })
    }

    /// Inspect media formats using bundled or system yt-dlp
    pub async fn inspect(url: &str, cookies: Option<&str>) -> Result<MediaInspectResult> {
        let bin = get_ytdlp_bin();
        let mut cmd = Command::new(&bin);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        cmd.arg("--ignore-config")
            .arg("--dump-single-json")
            .arg("--no-warnings")
            .arg("--no-playlist");

        if url.contains("youtube.com") || url.contains("youtu.be") {
            cmd.arg("--extractor-args").arg("youtube:player_client=android,ios,web");
        }

        cmd.arg(url);

        if let Some(ref ffmpeg_loc) = get_ffmpeg_location() {
            cmd.arg("--ffmpeg-location").arg(ffmpeg_loc);
        }

        apply_cookies(&mut cmd, url, cookies);

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
    ) -> Result<String>
    where
        F: Fn(u64, Option<u64>, u64, f32, Option<u64>, Vec<ChunkInfo>) + Send + Sync + 'static,
    {
        let bin = get_ytdlp_bin();
        let mut cmd = Command::new(&bin);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        // Robust format selector (handles YouTube DASH, Facebook, TikTok, Instagram, and single stream MP4s)
        let format_arg = match format_id.unwrap_or("best_1080p") {
            "best_8k" => "bestvideo+bestaudio/best".to_string(),
            "best_1080p" => "bestvideo[height<=?1080]+bestaudio/best[height<=?1080]/best".to_string(),
            "best_720p" => "bestvideo[height<=?720]+bestaudio/best[height<=?720]/best".to_string(),
            "best_audio" => "bestaudio/best".to_string(),
            custom => {
                if custom.contains('+') || custom.contains("audio") || custom.contains("ba") {
                    custom.to_string()
                } else {
                    format!("{}+bestaudio/best", custom)
                }
            }
        };

        // Determine destination directory
        let save_dir = save_path.parent().unwrap_or_else(|| Path::new("."));
        let _ = std::fs::create_dir_all(save_dir);

        // Output template: saves into target folder with video title
        let output_template = save_dir
            .join("%(title).150s.%(ext)s")
            .to_string_lossy()
            .to_string();

        cmd.arg("-N")
            .arg("16") // 16 parallel fragment streams for speed
            .arg("--buffer-size")
            .arg("16M")
            .arg("--no-playlist")
            .arg("--newline")
            .arg("--progress")
            .arg("--windows-filenames")
            .arg("--ignore-config") // CRITICAL: Never load global config that might force auto-subs or bad parameters
            .arg("--no-write-subs") // Do not fetch or write subtitles
            .arg("--no-write-auto-subs") // Do not fetch auto-generated captions (prevents 429 errors!)
            .arg("--no-embed-subs") // Do not embed subtitles
            .arg("--sub-langs").arg("") // Explicitly empty language list
            .arg("--compat-options").arg("no-youtube-unavailable-subtitles")
            .arg("--no-check-certificates")
            .arg("--print")
            .arg("after_move:filepath")
            .arg("-o")
            .arg(&output_template);

        if url.contains("youtube.com") || url.contains("youtu.be") {
            cmd.arg("--extractor-args").arg("youtube:player_client=android,ios,web");
        } else {
            cmd.arg("--user-agent")
                .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36");
        }

        if let Some(ref ffmpeg_loc) = get_ffmpeg_location() {
            cmd.arg("--ffmpeg-location").arg(ffmpeg_loc);

            // Prepend ffmpeg directory to child process PATH so yt-dlp finds both ffmpeg and ffprobe
            let path_key = if cfg!(windows) { "Path" } else { "PATH" };
            let mut paths = vec![std::path::PathBuf::from(ffmpeg_loc)];
            if let Some(curr) = std::env::var_os(path_key).or_else(|| std::env::var_os("PATH")) {
                paths.extend(std::env::split_paths(&curr));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                cmd.env("PATH", joined);
            }
        }

        apply_cookies(&mut cmd, url, cookies);

        if format_id == Some("best_audio") {
            // Studio-Quality 320kbps MP3
            cmd.arg("-x")
                .arg("--audio-format").arg("mp3")
                .arg("--audio-quality").arg("0");
        } else {
            // Video: Merge into MP4 container
            cmd.arg("-f").arg(&format_arg)
                .arg("--merge-output-format").arg("mp4");
        }

        cmd.arg(url);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow!("Failed to start yt-dlp at '{}': {}", bin, e)
        })?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

        // Drain stderr asynchronously so it doesn't block OS pipes
        let err_task = tokio::spawn(async move {
            let mut err_reader = BufReader::new(stderr).lines();
            let mut lines = Vec::new();
            while let Ok(Some(l)) = err_reader.next_line().await {
                if l.contains("ERROR:") || l.contains("WARNING:") {
                    eprintln!("yt-dlp: {}", l);
                }
                lines.push(l);
            }
            lines
        });

        let mut reader = BufReader::new(stdout).lines();
        let mut final_filepath = String::new();

        while let Ok(Some(line)) = reader.next_line().await {
            let trimmed = line.trim();

            // Detect final output filepath printed by yt-dlp
            if (trimmed.starts_with('/') || (trimmed.len() > 3 && &trimmed[1..3] == ":\\"))
                && (trimmed.ends_with(".mp4") || trimmed.ends_with(".mp3") || trimmed.ends_with(".mkv") || trimmed.ends_with(".webm") || trimmed.ends_with(".m4a"))
            {
                final_filepath = trimmed.to_string();
            }

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
        let err_lines = err_task.await.unwrap_or_default();

        // Safety fallback: if video and audio streams were downloaded separately (e.g. yt-dlp left .f137.mp4 and .f140.m4a),
        // invoke FFmpeg directly to mux them into a single clean MP4 and remove the unmerged fragments!
        if let Ok(Some(muxed)) = postprocess_merge_if_needed(save_dir, if !final_filepath.is_empty() { Some(&final_filepath) } else { None }).await {
            final_filepath = muxed;
        }

        let file_exists = !final_filepath.is_empty() && std::path::Path::new(&final_filepath).exists();

        // If file exists on disk with content, treat download as successful even if yt-dlp returned non-zero for subtitles
        if !status.success() && !file_exists {
            let error_msg = err_lines
                .into_iter()
                .filter(|l| l.contains("ERROR:") || l.contains("Error"))
                .collect::<Vec<_>>()
                .join(" ");

            let final_err = if !error_msg.is_empty() {
                error_msg
            } else {
                format!("Process exited with code: {}", status)
            };
            return Err(anyhow!("Media download failed: {}", final_err));
        }

        Ok(final_filepath)
    }
}

/// Bulletproof FFmpeg muxer fallback:
/// If yt-dlp failed to merge separate DASH video & audio streams and left separate files on disk
/// (e.g. "Video.f137.mp4" + "Video.f140.m4a", or "Video.mp4" + "Video.m4a"),
/// this scans the destination directory, muxes the streams using FFmpeg directly, and removes the raw fragments.
async fn postprocess_merge_if_needed(save_dir: &Path, expected_file: Option<&str>) -> Result<Option<String>> {
    let mut entries = match std::fs::read_dir(save_dir) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    let mut files = Vec::new();
    while let Some(Ok(entry)) = entries.next() {
        let p = entry.path();
        if p.is_file() {
            files.push(p);
        }
    }

    fn is_audio(p: &Path) -> bool {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        matches!(ext.as_str(), "m4a" | "mp3" | "opus" | "aac" | "ogg" | "flac" | "wav")
            || name.contains(".f140.")
            || name.contains(".f251.")
            || name.contains(".f250.")
            || name.contains(".f249.")
            || name.contains(".f139.")
            || name.contains(".faudio.")
    }

    fn is_video(p: &Path) -> bool {
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        !is_audio(p) && matches!(ext.as_str(), "mp4" | "webm" | "mkv" | "mov" | "avi" | "flv")
    }

    fn extract_base_title(p: &Path) -> String {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if let Some(idx) = stem.rfind(".f") {
            let suffix = &stem[idx + 2..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                return stem[..idx].to_string();
            }
        }
        stem.to_string()
    }

    let mut video_candidates = Vec::new();
    let mut audio_candidates = Vec::new();

    for f in &files {
        if is_video(f) {
            video_candidates.push(f.clone());
        } else if is_audio(f) {
            audio_candidates.push(f.clone());
        }
    }

    for v in &video_candidates {
        let v_base = extract_base_title(v);
        if v_base.is_empty() {
            continue;
        }

        if let Some(exp) = expected_file {
            let exp_stem = Path::new(exp).file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let exp_base = if let Some(idx) = exp_stem.rfind(".f") {
                &exp_stem[..idx]
            } else {
                exp_stem
            };
            if !exp_base.is_empty() && !v_base.contains(exp_base) && !exp_base.contains(&v_base) {
                if video_candidates.len() > 1 {
                    continue;
                }
            }
        }

        for a in &audio_candidates {
            let a_base = extract_base_title(a);
            if a_base.is_empty() {
                continue;
            }

            if v_base == a_base || v.file_name() != a.file_name() && (v_base.starts_with(&a_base) || a_base.starts_with(&v_base)) {
                let merged_file_name = format!("{}.mp4", v_base);
                let merged_path = save_dir.join(&merged_file_name);
                let temp_path = save_dir.join(format!("{}.__sidm_muxing__.mp4", v_base));

                let v_len = std::fs::metadata(v).map(|m| m.len()).unwrap_or(0);
                let a_len = std::fs::metadata(a).map(|m| m.len()).unwrap_or(0);
                if v_len == 0 || a_len == 0 {
                    continue;
                }

                println!("🎬 [SIDM FFmpeg Merger] Muxing separate streams:");
                println!("   Video: {:?}", v);
                println!("   Audio: {:?}", a);
                println!("   Target: {:?}", merged_path);

                let ffmpeg_bin = get_ffmpeg_bin().unwrap_or_else(|| "ffmpeg".to_string());
                let mut mux_cmd = Command::new(&ffmpeg_bin);
                #[cfg(windows)]
                mux_cmd.creation_flags(0x08000000);

                mux_cmd.arg("-y")
                    .arg("-i").arg(v)
                    .arg("-i").arg(a)
                    .arg("-c:v").arg("copy")
                    .arg("-c:a").arg("aac")
                    .arg("-b:a").arg("192k")
                    .arg("-movflags").arg("+faststart")
                    .arg(&temp_path);

                match mux_cmd.status().await {
                    Ok(st) if st.success() && temp_path.is_file() => {
                        let _ = std::fs::remove_file(v);
                        let _ = std::fs::remove_file(a);
                        let _ = std::fs::rename(&temp_path, &merged_path);
                        println!("✅ [SIDM FFmpeg Merger] Successfully merged into: {:?}", merged_path);
                        return Ok(Some(merged_path.to_string_lossy().to_string()));
                    }
                    Ok(st) => {
                        eprintln!("⚠️ [SIDM FFmpeg Merger] FFmpeg mux returned exit code: {}", st);
                        let _ = std::fs::remove_file(&temp_path);
                    }
                    Err(e) => {
                        eprintln!("⚠️ [SIDM FFmpeg Merger] Failed to execute FFmpeg ({}): {}", ffmpeg_bin, e);
                        let _ = std::fs::remove_file(&temp_path);
                    }
                }
            }
        }
    }

    Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_unc_prefix() {
        assert_eq!(
            strip_unc_prefix(r"\\?\C:\Users\User\AppData\Local\Programs\SIDM\bin"),
            r"C:\Users\User\AppData\Local\Programs\SIDM\bin"
        );
        assert_eq!(
            strip_unc_prefix(r"\\?\UNC\server\share\file.mp4"),
            r"\\server\share\file.mp4"
        );
        assert_eq!(
            strip_unc_prefix("/usr/bin/ffmpeg"),
            "/usr/bin/ffmpeg"
        );
    }

    #[test]
    fn test_ffmpeg_discovery() {
        let bin = get_ffmpeg_bin();
        println!("Discovered FFmpeg binary: {:?}", bin);
        if let Some(b) = bin {
            assert!(!b.starts_with(r"\\?\"));
            let dir = get_ffmpeg_dir();
            assert!(dir.is_some());
            assert!(!dir.unwrap().starts_with(r"\\?\"));
        }
    }

    #[test]
    fn test_parse_size_and_eta() {
        assert_eq!(parse_size_to_bytes("100MiB"), Some(104857600));
        assert_eq!(parse_size_to_bytes("1.5GiB"), Some(1610612736));
        assert_eq!(parse_eta_to_seconds("01:30"), Some(90));
        assert_eq!(parse_eta_to_seconds("01:00:00"), Some(3600));
    }
}
