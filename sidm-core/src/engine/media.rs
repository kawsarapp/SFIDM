use crate::model::{
    ChunkInfo, MediaFormatOption, MediaInspectResult, PlaylistItem, PlaylistInspectResult,
};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct MediaEngine;

pub fn clean_path(p: &Path) -> std::path::PathBuf {
    let s = p.to_string_lossy();
    let stripped = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s.to_string()
    };
    if cfg!(windows) {
        std::path::PathBuf::from(stripped.replace('/', "\\"))
    } else {
        std::path::PathBuf::from(stripped)
    }
}

#[allow(dead_code)]
pub fn strip_unc_prefix(path_str: &str) -> String {
    if let Some(s) = path_str.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", s)
    } else if let Some(s) = path_str.strip_prefix(r"\\?\") {
        s.to_string()
    } else {
        path_str.to_string()
    }
}

fn get_ytdlp_bin() -> String {
    let names: &[&str] = if cfg!(windows) {
        &["yt-dlp.exe", "yt-dlp"]
    } else {
        &["yt-dlp", "yt-dlp.exe"]
    };

    // 1. Check relative to current exe
    if let Ok(exe) = std::env::current_exe() {
        let clean_exe = clean_path(&exe);
        if let Some(parent) = clean_exe.parent() {
            let dirs = [
                parent.to_path_buf(),
                parent.join("bin"),
                parent.join("..").join("bin"),
                parent.join("..").join("..").join("bin"),
                parent.join("..").join("..").join("..").join("bin"),
            ];
            for dir in &dirs {
                for name in names {
                    let p = dir.join(name);
                    if p.is_file() {
                        return clean_path(&p).to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    // 2. Check local relative paths from CWD
    for dir_sub in &["", "bin", "sidm-core/bin"] {
        for name in names {
            let p = if dir_sub.is_empty() {
                std::path::PathBuf::from(name)
            } else {
                std::path::Path::new(dir_sub).join(name)
            };
            if p.is_file() {
                return clean_path(&p).to_string_lossy().to_string();
            }
        }
    }

    // 3. Search system PATH
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for name in names {
                let p = dir.join(name);
                if p.is_file() {
                    return clean_path(&p).to_string_lossy().to_string();
                }
            }
        }
    }

    // 4. Standard platform paths
    let standard = if cfg!(windows) {
        let mut list = vec![
            "C:\\Program Files\\SIDM\\bin\\yt-dlp.exe".to_string(),
            "C:\\Program Files\\SIDM\\yt-dlp.exe".to_string(),
            "C:\\Program Files (x86)\\SIDM\\bin\\yt-dlp.exe".to_string(),
            "C:\\Program Files (x86)\\SIDM\\yt-dlp.exe".to_string(),
            "C:\\Program Files\\yt-dlp\\yt-dlp.exe".to_string(),
            "C:\\yt-dlp\\yt-dlp.exe".to_string(),
            "C:\\ProgramData\\chocolatey\\bin\\yt-dlp.exe".to_string(),
        ];
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            list.push(format!("{}\\Programs\\SIDM\\bin\\yt-dlp.exe", local_app_data));
            list.push(format!("{}\\Programs\\SIDM\\yt-dlp.exe", local_app_data));
        }
        if let Ok(app_data) = std::env::var("APPDATA") {
            list.push(format!("{}\\SIDM\\bin\\yt-dlp.exe", app_data));
            list.push(format!("{}\\SIDM\\yt-dlp.exe", app_data));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            list.push(format!("{}\\scoop\\shims\\yt-dlp.exe", user_profile));
            list.push(format!("{}\\Downloads\\SIDM\\bin\\yt-dlp.exe", user_profile));
            list.push(format!("{}\\Downloads\\SIDM\\yt-dlp.exe", user_profile));
        }
        list
    } else {
        vec![
            "/usr/bin/yt-dlp".to_string(),
            "/usr/local/bin/yt-dlp".to_string(),
            "/snap/bin/yt-dlp".to_string(),
            "/opt/homebrew/bin/yt-dlp".to_string(),
        ]
    };
    for s in standard {
        let p = std::path::Path::new(&s);
        if p.is_file() {
            return s;
        }
    }

    if cfg!(target_os = "windows") {
        "yt-dlp.exe".to_string()
    } else {
        "yt-dlp".to_string()
    }
}

pub fn get_ffmpeg_bin() -> Option<String> {
    let names: &[&str] = if cfg!(windows) {
        &["ffmpeg.exe", "ffmpeg.cmd", "ffmpeg"]
    } else {
        &["ffmpeg", "ffmpeg.exe"]
    };

    // 1. Check relative to current exe
    if let Ok(exe) = std::env::current_exe() {
        let clean_exe = clean_path(&exe);
        if let Some(parent) = clean_exe.parent() {
            let dirs = [
                parent.to_path_buf(),
                parent.join("bin"),
                parent.join("..").join("bin"),
                parent.join("..").join("..").join("bin"),
                parent.join("..").join("..").join("..").join("bin"),
            ];
            for dir in &dirs {
                for name in names {
                    let p = dir.join(name);
                    if p.is_file() {
                        return Some(clean_path(&p).to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    // 2. Check beside yt-dlp binary if discovered
    let yt_bin = get_ytdlp_bin();
    let yt_path = clean_path(std::path::Path::new(&yt_bin));
    if let Some(yt_parent) = yt_path.parent() {
        let dirs = [
            yt_parent.to_path_buf(),
            yt_parent.join("bin"),
        ];
        for dir in &dirs {
            for name in names {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(clean_path(&p).to_string_lossy().to_string());
                }
            }
        }
    }

    // 3. Check local relative paths from CWD
    for dir_sub in &["", "bin", "sidm-core/bin"] {
        for name in names {
            let p = if dir_sub.is_empty() {
                std::path::PathBuf::from(name)
            } else {
                std::path::Path::new(dir_sub).join(name)
            };
            if p.is_file() {
                return Some(clean_path(&p).to_string_lossy().to_string());
            }
        }
    }

    // 4. Search system PATH
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for name in names {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(clean_path(&p).to_string_lossy().to_string());
                }
            }
        }
    }

    // 5. Standard platform directories
    let standard = if cfg!(windows) {
        let mut list = vec![
            "C:\\Program Files\\SIDM\\bin\\ffmpeg.exe".to_string(),
            "C:\\Program Files\\SIDM\\ffmpeg.exe".to_string(),
            "C:\\Program Files (x86)\\SIDM\\bin\\ffmpeg.exe".to_string(),
            "C:\\Program Files (x86)\\SIDM\\ffmpeg.exe".to_string(),
            "C:\\Program Files\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
            "C:\\ffmpeg\\bin\\ffmpeg.exe".to_string(),
            "C:\\ProgramData\\chocolatey\\bin\\ffmpeg.exe".to_string(),
        ];
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            list.push(format!("{}\\Programs\\SIDM\\bin\\ffmpeg.exe", local_app_data));
            list.push(format!("{}\\Programs\\SIDM\\ffmpeg.exe", local_app_data));
        }
        if let Ok(app_data) = std::env::var("APPDATA") {
            list.push(format!("{}\\SIDM\\bin\\ffmpeg.exe", app_data));
            list.push(format!("{}\\SIDM\\ffmpeg.exe", app_data));
        }
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            list.push(format!("{}\\scoop\\shims\\ffmpeg.exe", user_profile));
            list.push(format!("{}\\Downloads\\SIDM\\bin\\ffmpeg.exe", user_profile));
            list.push(format!("{}\\Downloads\\SIDM\\ffmpeg.exe", user_profile));
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

#[allow(dead_code)]
pub fn get_ffmpeg_dir() -> Option<String> {
    if let Some(bin_path) = get_ffmpeg_bin() {
        let p = std::path::Path::new(&bin_path);
        if let Some(parent) = p.parent() {
            let dir_str = clean_path(parent).to_string_lossy().to_string();
            if !dir_str.is_empty() {
                return Some(dir_str);
            }
        }
    }
    None
}

fn get_ffmpeg_location() -> Option<String> {
    // Pass direct path to ffmpeg binary (supported by yt-dlp --ffmpeg-location)
    get_ffmpeg_bin()
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
        let ffmpeg_bin_opt = get_ffmpeg_bin();

        tracing::info!("🔧 yt-dlp binary: {}", bin);
        tracing::info!("🔧 ffmpeg binary: {:?}", ffmpeg_bin_opt);

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

        // ─── TWO-PHASE STRATEGY (Windows-safe, 100% reliable) ─────────────────────
        // If FFmpeg is available, we do NOT rely on yt-dlp's internal merger
        // (which silently fails on Windows due to quoting / PATH issues).
        // Instead we:
        //   1. Download video-only  → temp_dir
        //   2. Download audio-only  → temp_dir
        //   3. Mux with FFmpeg      → final save_dir
        //   4. Delete temp files
        //
        // For audio-only ("best_audio") we keep the single-phase path unchanged.
        // ──────────────────────────────────────────────────────────────────────────
        if format_id != Some("best_audio") {
            if let Some(ref ffmpeg_bin) = ffmpeg_bin_opt {
                return download_two_phase(
                    &bin,
                    ffmpeg_bin,
                    url,
                    save_dir,
                    &format_arg,
                    cookies,
                    on_progress,
                ).await;
            }
        }

        // ─── FALLBACK / AUDIO-ONLY: original single-phase path ────────────────────
        let mut cmd = Command::new(&bin);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        // Output template: saves into target folder with video title
        let output_template = save_dir
            .join("%(title).150s.%(ext)s")
            .to_string_lossy()
            .to_string();

        cmd.arg("-N")
            .arg("16")
            .arg("--buffer-size")
            .arg("16M")
            .arg("--no-playlist")
            .arg("--newline")
            .arg("--progress")
            .arg("--windows-filenames")
            .arg("--ignore-config")
            .arg("--no-write-subs")
            .arg("--no-write-auto-subs")
            .arg("--no-embed-subs")
            .arg("--sub-langs").arg("")
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

        if let Some(ref ffmpeg_loc) = ffmpeg_bin_opt {
            cmd.arg("--ffmpeg-location").arg(ffmpeg_loc);
            let path_key = if cfg!(windows) { "Path" } else { "PATH" };
            let mut paths = Vec::new();
            if let Some(parent) = std::path::Path::new(ffmpeg_loc).parent() {
                paths.push(parent.to_path_buf());
            } else {
                paths.push(std::path::PathBuf::from(ffmpeg_loc));
            }
            if let Some(curr) = std::env::var_os(path_key).or_else(|| std::env::var_os("PATH")) {
                paths.extend(std::env::split_paths(&curr));
            }
            if let Ok(joined) = std::env::join_paths(paths) {
                cmd.env(path_key, joined);
            }
        }

        apply_cookies(&mut cmd, url, cookies);

        // Audio-only path
        cmd.arg("-x")
            .arg("--audio-format").arg("mp3")
            .arg("--audio-quality").arg("0");

        cmd.arg(url);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            anyhow!("Failed to start yt-dlp at '{}': {}", bin, e)
        })?;

        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

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

            if (trimmed.starts_with('/') || (trimmed.len() > 3 && &trimmed[1..3] == ":\\"))
                && (trimmed.ends_with(".mp4") || trimmed.ends_with(".mp3") || trimmed.ends_with(".mkv") || trimmed.ends_with(".webm") || trimmed.ends_with(".m4a"))
            {
                final_filepath = trimmed.to_string();
            }

            if line.contains("[download]") && line.contains('%') {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut percent = 0.0f32;
                let mut speed = 0u64;
                let mut total_bytes: Option<u64> = None;
                let mut eta_seconds: Option<u64> = None;

                for (i, p) in parts.iter().enumerate() {
                    if p.ends_with('%') {
                        let num = p.trim_end_matches('%');
                        if let Ok(val) = num.parse::<f32>() {
                            percent = val;
                        }
                    }
                    if *p == "of" && i + 1 < parts.len() {
                        let size_token = parts[i + 1].trim_start_matches('~');
                        if let Some(bytes) = parse_size_to_bytes(size_token) {
                            total_bytes = Some(bytes);
                        }
                    }
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

        let file_exists = !final_filepath.is_empty() && std::path::Path::new(&final_filepath).exists();

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

/// Two-phase download: download video+audio separately into a unique temp folder,
/// then mux with FFmpeg into the final save_dir. This completely bypasses yt-dlp's
/// internal merger which silently fails on Windows due to PATH / quoting issues.
async fn download_two_phase<F>(
    ytdlp_bin: &str,
    ffmpeg_bin: &str,
    url: &str,
    save_dir: &Path,
    format_arg: &str,
    cookies: Option<&str>,
    on_progress: F,
) -> Result<String>
where
    F: Fn(u64, Option<u64>, u64, f32, Option<u64>, Vec<ChunkInfo>) + Send + Sync + 'static,
{
    // Create a unique temp directory inside save_dir so we avoid cross-device rename issues
    let session_id = uuid::Uuid::new_v4().to_string().replace('-', "");
    let temp_dir = save_dir.join(format!(".sidm_tmp_{}", session_id));
    let _ = std::fs::create_dir_all(&temp_dir);

    // Helper closure to build a base yt-dlp command with common flags
    let make_cmd = |bin: &str| -> Command {
        let mut cmd = Command::new(bin);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        cmd.arg("--no-playlist")
            .arg("--newline")
            .arg("--progress")
            .arg("--windows-filenames")
            .arg("--ignore-config")
            .arg("--no-write-subs")
            .arg("--no-write-auto-subs")
            .arg("--no-embed-subs")
            .arg("--sub-langs").arg("")
            .arg("--compat-options").arg("no-youtube-unavailable-subtitles")
            .arg("--no-check-certificates")
            .arg("-N").arg("16")
            .arg("--buffer-size").arg("16M");
        // Add extractor args
        if url.contains("youtube.com") || url.contains("youtu.be") {
            cmd.arg("--extractor-args").arg("youtube:player_client=android,ios,web");
        } else {
            cmd.arg("--user-agent")
                .arg("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36");
        }
        // Always give yt-dlp the ffmpeg path so it can probe without merging
        cmd.arg("--ffmpeg-location").arg(ffmpeg_bin);
        // Inject ffmpeg dir into PATH for child process
        let path_key = if cfg!(windows) { "Path" } else { "PATH" };
        let mut paths = Vec::new();
        if let Some(parent) = std::path::Path::new(ffmpeg_bin).parent() {
            paths.push(parent.to_path_buf());
        }
        if let Some(curr) = std::env::var_os(path_key).or_else(|| std::env::var_os("PATH")) {
            paths.extend(std::env::split_paths(&curr));
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env(path_key, joined);
        }
        cmd
    };

    // ── PHASE 1: Download video-only stream ──────────────────────────────────────
    // Strip the "+bestaudio" part to get video-only format
    let video_format = {
        let f = format_arg.split('+').next().unwrap_or("bestvideo[height<=?1080]");
        // If format_arg already has no +, use bestvideo fallback
        if format_arg.contains('+') { f.to_string() } else { "bestvideo[height<=?1080]".to_string() }
    };

    let video_tmpl = temp_dir.join("video.%(ext)s").to_string_lossy().to_string();
    let mut vcmd = make_cmd(ytdlp_bin);
    vcmd.arg("--print").arg("after_move:filepath")
        .arg("-f").arg(&video_format)
        .arg("--no-audio")  // yt-dlp skips audio automatically when video-only format selected
        .arg("-o").arg(&video_tmpl);
    apply_cookies(&mut vcmd, url, cookies);
    vcmd.arg(url);
    vcmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    tracing::info!("📥 [Two-Phase] Downloading video stream: {}", video_format);

    let mut vchild = vcmd.spawn().map_err(|e| anyhow!("Failed to start yt-dlp (video): {}", e))?;
    let vstdout = vchild.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
    let vstderr = vchild.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;
    let _verr = tokio::spawn(async move {
        let mut r = BufReader::new(vstderr).lines();
        let mut lines = Vec::new();
        while let Ok(Some(l)) = r.next_line().await { lines.push(l); }
        lines
    });

    let mut vreader = BufReader::new(vstdout).lines();
    let mut video_file = String::new();
    while let Ok(Some(line)) = vreader.next_line().await {
        let t = line.trim();
        if t.len() > 3 && (t.starts_with('/') || &t[1..2] == ":") {
            video_file = t.to_string();
        }
        // Forward progress (phase 1 = 0–50%)
        if line.contains("[download]") && line.contains('%') {
            if let Some(pct) = parse_percent_from_line(&line) {
                let scaled = pct * 0.5;
                on_progress(
                    (scaled * 1024.0 * 1024.0) as u64, None, 0, scaled, None,
                    vec![ChunkInfo { index: 0, start_byte: 0, end_byte: 100_000_000, downloaded_bytes: (scaled * 1024.0 * 1024.0) as u64, active: true }],
                );
            }
        }
    }
    let vstatus = vchild.wait().await?;

    // If video-only format failed (some sites serve combined), fallback to full format
    if !vstatus.success() || video_file.is_empty() {
        tracing::warn!("⚠️ [Two-Phase] Video-only download failed, falling back to single-phase yt-dlp merge");
        let _ = std::fs::remove_dir_all(&temp_dir);
        return download_single_phase_fallback(ytdlp_bin, ffmpeg_bin, url, save_dir, format_arg, cookies, on_progress).await;
    }

    // Scan temp_dir for the actual video file (yt-dlp may rename it)
    let actual_video = find_largest_file_in_dir(&temp_dir).unwrap_or_else(|| std::path::PathBuf::from(&video_file));
    tracing::info!("✅ [Two-Phase] Video file: {:?}", actual_video);

    // ── PHASE 2: Download audio-only stream ──────────────────────────────────────
    let audio_tmpl = temp_dir.join("audio.%(ext)s").to_string_lossy().to_string();
    let mut acmd = make_cmd(ytdlp_bin);
    acmd.arg("--print").arg("after_move:filepath")
        .arg("-f").arg("bestaudio")
        .arg("-o").arg(&audio_tmpl);
    apply_cookies(&mut acmd, url, cookies);
    acmd.arg(url);
    acmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    tracing::info!("🎵 [Two-Phase] Downloading audio stream");

    let mut achild = acmd.spawn().map_err(|e| anyhow!("Failed to start yt-dlp (audio): {}", e))?;
    let astdout = achild.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
    let astderr = achild.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;
    let aerr_lines_task = tokio::spawn(async move {
        let mut r = BufReader::new(astderr).lines();
        let mut lines = Vec::new();
        while let Ok(Some(l)) = r.next_line().await { lines.push(l); }
        lines
    });

    let mut areader = BufReader::new(astdout).lines();
    let mut audio_file = String::new();
    while let Ok(Some(line)) = areader.next_line().await {
        let t = line.trim();
        if t.len() > 3 && (t.starts_with('/') || &t[1..2] == ":") {
            audio_file = t.to_string();
        }
        // Forward progress (phase 2 = 50–90%)
        if line.contains("[download]") && line.contains('%') {
            if let Some(pct) = parse_percent_from_line(&line) {
                let scaled = 50.0 + pct * 0.4;
                on_progress(
                    (scaled * 1024.0 * 1024.0) as u64, None, 0, scaled, None,
                    vec![ChunkInfo { index: 0, start_byte: 0, end_byte: 100_000_000, downloaded_bytes: (scaled * 1024.0 * 1024.0) as u64, active: true }],
                );
            }
        }
    }
    let astatus = achild.wait().await?;
    let _aerr_lines = aerr_lines_task.await.unwrap_or_default();

    // Find actual audio file
    let actual_audio = find_file_by_prefix(&temp_dir, "audio")
        .unwrap_or_else(|| std::path::PathBuf::from(&audio_file));

    if !astatus.success() || !actual_audio.is_file() {
        tracing::warn!("⚠️ [Two-Phase] Audio download failed, falling back to single-phase");
        let _ = std::fs::remove_dir_all(&temp_dir);
        return download_single_phase_fallback(ytdlp_bin, ffmpeg_bin, url, save_dir, format_arg, cookies, on_progress).await;
    }
    tracing::info!("✅ [Two-Phase] Audio file: {:?}", actual_audio);

    // ── PHASE 3: FFmpeg mux → save_dir ──────────────────────────────────────────
    // Derive output filename from video file title
    let video_stem = actual_video.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    let merged_name = format!("{}.mp4", video_stem);
    let merged_path = save_dir.join(&merged_name);
    let temp_merged = temp_dir.join("merged_output.mp4");

    tracing::info!("🎬 [Two-Phase] FFmpeg muxing → {:?}", merged_path);
    on_progress(1, None, 0, 92.0, Some(5), vec![ChunkInfo { index: 0, start_byte: 0, end_byte: 100_000_000, downloaded_bytes: 92_000_000, active: true }]);

    let mut mux = Command::new(ffmpeg_bin);
    #[cfg(windows)]
    mux.creation_flags(0x08000000);
    mux.arg("-y")
        .arg("-i").arg(&actual_video)
        .arg("-i").arg(&actual_audio)
        .arg("-c:v").arg("copy")
        .arg("-c:a").arg("aac")
        .arg("-b:a").arg("192k")
        .arg("-movflags").arg("+faststart")
        .arg(&temp_merged);

    let mux_status = mux.status().await.map_err(|e| anyhow!("Failed to run ffmpeg: {}", e))?;

    if !mux_status.success() || !temp_merged.is_file() {
        tracing::error!("❌ [Two-Phase] FFmpeg mux failed");
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(anyhow!("FFmpeg failed to merge video and audio streams"));
    }

    // Move merged file to final destination
    if let Err(e) = std::fs::rename(&temp_merged, &merged_path) {
        tracing::warn!("rename failed ({}), trying copy+delete", e);
        std::fs::copy(&temp_merged, &merged_path)?;
        let _ = std::fs::remove_file(&temp_merged);
    }

    // Cleanup temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    let final_path = clean_path(&merged_path).to_string_lossy().to_string();
    tracing::info!("✅ [Two-Phase] Complete! Final file: {}", final_path);
    on_progress(100_000_000, Some(100_000_000), 0, 100.0, Some(0), vec![ChunkInfo { index: 0, start_byte: 0, end_byte: 100_000_000, downloaded_bytes: 100_000_000, active: false }]);

    Ok(final_path)
}

/// Single-phase fallback: use yt-dlp's own merger (original behavior) when two-phase fails.
async fn download_single_phase_fallback<F>(
    ytdlp_bin: &str,
    ffmpeg_bin: &str,
    url: &str,
    save_dir: &Path,
    format_arg: &str,
    cookies: Option<&str>,
    on_progress: F,
) -> Result<String>
where
    F: Fn(u64, Option<u64>, u64, f32, Option<u64>, Vec<ChunkInfo>) + Send + Sync + 'static,
{
    let output_template = save_dir.join("%(title).150s.%(ext)s").to_string_lossy().to_string();
    let mut cmd = Command::new(ytdlp_bin);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);

    cmd.arg("-N").arg("16")
        .arg("--buffer-size").arg("16M")
        .arg("--no-playlist")
        .arg("--newline").arg("--progress")
        .arg("--windows-filenames")
        .arg("--ignore-config")
        .arg("--no-write-subs").arg("--no-write-auto-subs").arg("--no-embed-subs")
        .arg("--sub-langs").arg("")
        .arg("--compat-options").arg("no-youtube-unavailable-subtitles")
        .arg("--no-check-certificates")
        .arg("--print").arg("after_move:filepath")
        .arg("--ffmpeg-location").arg(ffmpeg_bin)
        .arg("-f").arg(format_arg)
        .arg("--merge-output-format").arg("mp4")
        .arg("-o").arg(&output_template);

    if url.contains("youtube.com") || url.contains("youtu.be") {
        cmd.arg("--extractor-args").arg("youtube:player_client=android,ios,web");
    }

    apply_cookies(&mut cmd, url, cookies);
    cmd.arg(url);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| anyhow!("Failed to start yt-dlp (fallback): {}", e))?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;
    let stderr = child.stderr.take().ok_or_else(|| anyhow!("No stderr"))?;

    let err_task = tokio::spawn(async move {
        let mut r = BufReader::new(stderr).lines();
        let mut lines = Vec::new();
        while let Ok(Some(l)) = r.next_line().await { lines.push(l); }
        lines
    });

    let mut reader = BufReader::new(stdout).lines();
    let mut final_filepath = String::new();

    while let Ok(Some(line)) = reader.next_line().await {
        let t = line.trim();
        if (t.starts_with('/') || (t.len() > 3 && &t[1..3] == ":\\"))
            && (t.ends_with(".mp4") || t.ends_with(".mkv") || t.ends_with(".webm"))
        {
            final_filepath = t.to_string();
        }
        if line.contains("[download]") && line.contains('%') {
            if let Some(pct) = parse_percent_from_line(&line) {
                on_progress((pct * 1024.0 * 1024.0) as u64, None, 0, pct, None,
                    vec![ChunkInfo { index: 0, start_byte: 0, end_byte: 100_000_000, downloaded_bytes: (pct * 1024.0 * 1024.0) as u64, active: true }]);
            }
        }
    }

    let status = child.wait().await?;
    let err_lines = err_task.await.unwrap_or_default();

    // Safety fallback post-merge check
    if let Ok(Some(muxed)) = postprocess_merge_if_needed(save_dir, if !final_filepath.is_empty() { Some(&final_filepath) } else { None }).await {
        final_filepath = muxed;
    }

    if !status.success() && (final_filepath.is_empty() || !std::path::Path::new(&final_filepath).exists()) {
        let err = err_lines.into_iter().filter(|l| l.contains("ERROR:")).collect::<Vec<_>>().join(" ");
        return Err(anyhow!("Media download failed: {}", err));
    }

    Ok(final_filepath)
}

fn parse_percent_from_line(line: &str) -> Option<f32> {
    for token in line.split_whitespace() {
        if token.ends_with('%') {
            if let Ok(v) = token.trim_end_matches('%').parse::<f32>() {
                return Some(v);
            }
        }
    }
    None
}

fn find_largest_file_in_dir(dir: &Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten()
        .filter(|e| e.path().is_file())
        .max_by_key(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .map(|e| e.path())
}

fn find_file_by_prefix(dir: &Path, prefix: &str) -> Option<std::path::PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_file() && p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix))
                .unwrap_or(false)
        })
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
        if matches!(ext.as_str(), "m4a" | "mp3" | "opus" | "aac" | "ogg" | "flac" | "wav") {
            return true;
        }
        for tag in &[
            "139", "140", "141", "249", "250", "251", "599", "600", "233", "234", "ba", "audio",
        ] {
            if name.contains(&format!(".f{}", tag)) {
                return true;
            }
        }
        false
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
                        let _ = std::fs::remove_file(&merged_path);
                        if let Err(e) = std::fs::rename(&temp_path, &merged_path) {
                            if std::fs::copy(&temp_path, &merged_path).is_ok() {
                                let _ = std::fs::remove_file(&temp_path);
                            } else {
                                eprintln!("⚠️ [SIDM FFmpeg Merger] Failed to move temp file: {}", e);
                            }
                        }
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
