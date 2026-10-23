use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Queued,
    Connecting,
    Downloading,
    Paused,
    Merging,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadType {
    DirectFile,
    MediaStream,
    HlsDash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub index: usize,
    pub start_byte: u64,
    pub end_byte: u64,
    pub downloaded_bytes: u64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: Uuid,
    pub url: String,
    pub file_name: String,
    pub save_path: String,
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub speed_bps: u64,
    pub progress_percent: f32,
    pub eta_seconds: Option<u64>,
    pub status: DownloadStatus,
    pub download_type: DownloadType,
    pub connections: usize,
    pub chunks: Vec<ChunkInfo>,
    pub created_at: DateTime<Utc>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFormatOption {
    pub format_id: String,
    pub label: String,
    pub ext: String,
    pub resolution: Option<String>,
    pub fps: Option<f32>,
    pub filesize_approx: Option<u64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub direct_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectResult {
    pub url: String,
    pub title: String,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration_seconds: Option<u64>,
    pub formats: Vec<MediaFormatOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum DownloadEvent {
    TaskAdded(DownloadTask),
    TaskProgress {
        id: Uuid,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        speed_bps: u64,
        progress_percent: f32,
        eta_seconds: Option<u64>,
        chunks: Vec<ChunkInfo>,
    },
    TaskPaused { id: Uuid },
    TaskResumed { id: Uuid },
    TaskCompleted { id: Uuid, save_path: String },
    TaskFailed { id: Uuid, error: String },
}

#[derive(Debug, Deserialize)]
pub struct CreateDownloadRequest {
    pub url: String,
    pub file_name: Option<String>,
    pub save_dir: Option<String>,
    pub connections: Option<usize>,
    pub format_id: Option<String>,
    pub cookies: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InspectRequest {
    pub url: String,
    pub cookies: Option<String>,
}
