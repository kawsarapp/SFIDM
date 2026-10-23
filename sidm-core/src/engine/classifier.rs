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
}
