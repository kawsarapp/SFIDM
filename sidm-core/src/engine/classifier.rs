use crate::model::DownloadType;
use url::Url;

pub struct Classifier;

impl Classifier {
    pub fn classify(raw_url: &str) -> DownloadType {
        let Ok(parsed) = Url::parse(raw_url) else {
            return DownloadType::DirectFile;
        };

        let host = parsed.host_str().unwrap_or("").to_lowercase();
        let path = parsed.path().to_lowercase();

        // 1. YouTube
        if host.contains("youtube.com") || host.contains("youtu.be") {
            return DownloadType::MediaStream;
        }

        // 2. Facebook
        if host.contains("facebook.com") || host.contains("fb.watch") || host.contains("fb.com") {
            return DownloadType::MediaStream;
        }

        // 3. TikTok
        if host.contains("tiktok.com") {
            return DownloadType::MediaStream;
        }

        // 4. Instagram
        if host.contains("instagram.com") {
            return DownloadType::MediaStream;
        }

        // 5. Twitter / X
        if host.contains("twitter.com") || host.contains("x.com") {
            return DownloadType::MediaStream;
        }

        // 6. Vimeo, Dailymotion, Reddit video, etc.
        if host.contains("vimeo.com") || host.contains("dailymotion.com") || host.contains("reddit.com") {
            return DownloadType::MediaStream;
        }

        // 7. HLS (.m3u8) / DASH (.mpd)
        if path.ends_with(".m3u8") || path.ends_with(".mpd") {
            return DownloadType::HlsDash;
        }

        // 8. Otherwise treat as Direct File
        DownloadType::DirectFile
    }

    /// Extract filename from URL or default fallback
    pub fn extract_filename(raw_url: &str) -> String {
        if let Ok(parsed) = Url::parse(raw_url) {
            let path = parsed.path();
            if let Some(segment) = path.split('/').filter(|s| !s.is_empty()).last() {
                // If the segment has an extension or looks like a file
                if segment.contains('.') {
                    return segment.to_string();
                }
            }
        }
        format!("sidm_download_{}", chrono::Utc::now().timestamp())
    }

    /// Automatically determine the organized destination folder (Videos, Music, Compressed, Programs, Documents)
    pub fn get_category_subfolder(
        filename: &str,
        download_type: &DownloadType,
        format_id: Option<&str>,
    ) -> &'static str {
        if format_id == Some("best_audio") {
            return "Music";
        }

        let lower = filename.to_lowercase();
        let ext = lower.rsplit('.').next().unwrap_or("");

        match ext {
            // Videos
            "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "flv" | "m4v" | "ts" | "3gp" => "Videos",
            // Music / Audio
            "mp3" | "m4a" | "wav" | "flac" | "aac" | "ogg" | "wma" | "opus" | "alac" | "aiff" => "Music",
            // Compressed Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "dmg" | "tgz" => "Compressed",
            // Programs / Installers
            "exe" | "msi" | "deb" | "rpm" | "apk" | "appimage" => "Programs",
            // Documents
            "pdf" | "docx" | "doc" | "xlsx" | "xls" | "pptx" | "ppt" | "txt" | "rtf" | "odt" | "csv" | "epub" => "Documents",
            // Images
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tiff" => "Images",
            // Fallback based on stream type
            _ => {
                if *download_type == DownloadType::MediaStream || *download_type == DownloadType::HlsDash {
                    "Videos"
                } else {
                    "General"
                }
            }
        }
    }
}
