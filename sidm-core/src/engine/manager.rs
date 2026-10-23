use crate::config::AppConfig;
use crate::engine::classifier::Classifier;
use crate::engine::media::MediaEngine;
use crate::engine::segmenter::Segmenter;
use crate::licensing::LicenseManager;
use crate::model::{
    ChunkInfo, CreateDownloadRequest, DownloadEvent, DownloadStatus, DownloadTask, DownloadType,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};
use uuid::Uuid;

pub struct TaskManager {
    config: AppConfig,
    tasks: Arc<RwLock<HashMap<Uuid, DownloadTask>>>,
    pause_senders: Arc<RwLock<HashMap<Uuid, watch::Sender<bool>>>>,
    event_tx: broadcast::Sender<DownloadEvent>,
    segmenter: Arc<Segmenter>,
    pub license: Arc<LicenseManager>,
}

impl TaskManager {
    pub fn new(config: AppConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            config,
            tasks: Arc::new(RwLock::new(HashMap::new())),
            pause_senders: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            segmenter: Arc::new(Segmenter::new()),
            license: Arc::new(LicenseManager::new()),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DownloadEvent> {
        self.event_tx.subscribe()
    }

    pub async fn list_tasks(&self) -> Vec<DownloadTask> {
        let map = self.tasks.read().await;
        let mut list: Vec<DownloadTask> = map.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    pub async fn get_task(&self, id: Uuid) -> Option<DownloadTask> {
        let map = self.tasks.read().await;
        map.get(&id).cloned()
    }

    pub async fn add_task(&self, req: CreateDownloadRequest) -> Result<DownloadTask> {
        let id = Uuid::new_v4();
        let download_type = Classifier::classify(&req.url);

        let file_name = req
            .file_name
            .clone()
            .unwrap_or_else(|| Classifier::extract_filename(&req.url));

        let save_dir = req
            .save_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.default_download_dir.clone());

        let _ = std::fs::create_dir_all(&save_dir);
        let save_path = save_dir.join(&file_name).to_string_lossy().to_string();

        let connections = req
            .connections
            .unwrap_or(self.config.default_connections)
            .clamp(1, self.config.max_connections);

        let task = DownloadTask {
            id,
            url: req.url.clone(),
            file_name: file_name.clone(),
            save_path: save_path.clone(),
            total_bytes: None,
            downloaded_bytes: 0,
            speed_bps: 0,
            progress_percent: 0.0,
            eta_seconds: None,
            status: DownloadStatus::Queued,
            download_type,
            connections,
            chunks: Vec::new(),
            created_at: Utc::now(),
            error_message: None,
        };

        // Insert into state
        {
            let mut map = self.tasks.write().await;
            map.insert(id, task.clone());
        }

        let (pause_tx, pause_rx) = watch::channel(false);
        {
            let mut p_map = self.pause_senders.write().await;
            p_map.insert(id, pause_tx);
        }

        let _ = self.event_tx.send(DownloadEvent::TaskAdded(task.clone()));

        // Spawn background downloader
        let tasks_ref = self.tasks.clone();
        let event_tx = self.event_tx.clone();
        let segmenter = self.segmenter.clone();
        let format_id = req.format_id.clone();
        let cookies = req.cookies.clone();

        tokio::spawn(async move {
            Self::run_download_task(
                id,
                tasks_ref,
                event_tx,
                segmenter,
                save_path,
                format_id,
                cookies,
                pause_rx,
            )
            .await;
        });

        Ok(task)
    }

    async fn run_download_task(
        id: Uuid,
        tasks: Arc<RwLock<HashMap<Uuid, DownloadTask>>>,
        event_tx: broadcast::Sender<DownloadEvent>,
        segmenter: Arc<Segmenter>,
        save_path: String,
        format_id: Option<String>,
        cookies: Option<String>,
        pause_rx: watch::Receiver<bool>,
    ) {
        // Set to Connecting
        {
            let mut map = tasks.write().await;
            if let Some(t) = map.get_mut(&id) {
                t.status = DownloadStatus::Connecting;
            }
        }

        let task = {
            let map = tasks.read().await;
            match map.get(&id) {
                Some(t) => t.clone(),
                None => return,
            }
        };

        println!("\n📥 [SIDM] New Download Task Started: {}", task.url);
        println!("   Type: {:?} | Format: {:?} | Threads: {}", task.download_type, format_id, task.connections);
        println!("   Destination: {}", save_path);

        match task.download_type {
            DownloadType::MediaStream => {
                // Media Engine with yt-dlp + FFmpeg
                let p_tasks = tasks.clone();
                let p_event_tx = event_tx.clone();
                let on_progress = move |downloaded: u64, speed: u64, chunks: Vec<ChunkInfo>| {
                    let p_tasks = p_tasks.clone();
                    let p_event_tx = p_event_tx.clone();
                    tokio::spawn(async move {
                        let mut map = p_tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Downloading;
                            t.downloaded_bytes = downloaded;
                            t.speed_bps = speed;
                            if let Some(tot) = t.total_bytes {
                                if tot > 0 {
                                    t.progress_percent = (downloaded as f32 / tot as f32) * 100.0;
                                }
                            }
                            let _ = p_event_tx.send(DownloadEvent::TaskProgress {
                                id,
                                downloaded_bytes: downloaded,
                                total_bytes: t.total_bytes,
                                speed_bps: speed,
                                progress_percent: t.progress_percent,
                                eta_seconds: t.eta_seconds,
                                chunks,
                            });
                        }
                    });
                };

                match MediaEngine::download(
                    &task.url,
                    PathBuf::from(&save_path).as_path(),
                    format_id.as_deref(),
                    cookies.as_deref(),
                    on_progress,
                )
                .await
                {
                    Ok(_) => {
                        println!("✅ [SIDM] Media Download Completed! Saved to folder: {}", save_path);
                        let mut map = tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Completed;
                            t.progress_percent = 100.0;
                        }
                        let _ = event_tx.send(DownloadEvent::TaskCompleted { id, save_path });
                    }
                    Err(e) => {
                        eprintln!("❌ [SIDM] Media Download Error: {}", e);
                        let mut map = tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Failed;
                            t.error_message = Some(e.to_string());
                        }
                        let _ = event_tx.send(DownloadEvent::TaskFailed {
                            id,
                            error: e.to_string(),
                        });
                    }
                }
            }
            DownloadType::DirectFile | DownloadType::HlsDash => {
                // Probe server
                let probe = match segmenter.probe(&task.url, cookies.as_deref()).await {
                    Ok(p) => p,
                    Err(e) => {
                        let mut map = tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Failed;
                            t.error_message = Some(format!("Probe failed: {}", e));
                        }
                        let _ = event_tx.send(DownloadEvent::TaskFailed {
                            id,
                            error: e.to_string(),
                        });
                        return;
                    }
                };

                // Update task with total bytes & suggested filename
                {
                    let mut map = tasks.write().await;
                    if let Some(t) = map.get_mut(&id) {
                        t.total_bytes = probe.total_bytes;
                        if let Some(ref sugg) = probe.suggested_filename {
                            t.file_name = sugg.clone();
                        }
                        t.status = DownloadStatus::Downloading;
                    }
                }

                let p_tasks = tasks.clone();
                let p_event_tx = event_tx.clone();
                let total_bytes = probe.total_bytes;

                let on_progress = move |downloaded: u64, speed: u64, chunks: Vec<ChunkInfo>| {
                    let p_tasks = p_tasks.clone();
                    let p_event_tx = p_event_tx.clone();
                    tokio::spawn(async move {
                        let mut map = p_tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.downloaded_bytes = downloaded;
                            t.speed_bps = speed;
                            t.chunks = chunks.clone();
                            if let Some(tot) = total_bytes {
                                if tot > 0 {
                                    t.progress_percent =
                                        ((downloaded as f32 / tot as f32) * 100.0).min(100.0);
                                    if speed > 0 {
                                        let remaining = tot.saturating_sub(downloaded);
                                        t.eta_seconds = Some(remaining / speed);
                                    }
                                }
                            }
                            let _ = p_event_tx.send(DownloadEvent::TaskProgress {
                                id,
                                downloaded_bytes: downloaded,
                                total_bytes,
                                speed_bps: speed,
                                progress_percent: t.progress_percent,
                                eta_seconds: t.eta_seconds,
                                chunks,
                            });
                        }
                    });
                };

                let res = segmenter
                    .download(
                        &task.url,
                        PathBuf::from(&save_path).as_path(),
                        probe.total_bytes,
                        probe.supports_range,
                        task.connections,
                        cookies.as_deref(),
                        pause_rx,
                        on_progress,
                    )
                    .await;

                match res {
                    Ok(_) => {
                        let mut map = tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Completed;
                            t.progress_percent = 100.0;
                            t.speed_bps = 0;
                        }
                        let _ = event_tx.send(DownloadEvent::TaskCompleted { id, save_path });
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let is_pause = err_str.contains("paused");
                        let mut map = tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            if is_pause {
                                t.status = DownloadStatus::Paused;
                                let _ = event_tx.send(DownloadEvent::TaskPaused { id });
                            } else {
                                t.status = DownloadStatus::Failed;
                                t.error_message = Some(err_str.clone());
                                let _ = event_tx.send(DownloadEvent::TaskFailed {
                                    id,
                                    error: err_str,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn pause_task(&self, id: Uuid) -> Result<()> {
        let p_map = self.pause_senders.read().await;
        if let Some(sender) = p_map.get(&id) {
            let _ = sender.send(true);
            let mut map = self.tasks.write().await;
            if let Some(t) = map.get_mut(&id) {
                t.status = DownloadStatus::Paused;
            }
            let _ = self.event_tx.send(DownloadEvent::TaskPaused { id });
            Ok(())
        } else {
            Err(anyhow!("Task not found or already finished"))
        }
    }

    pub async fn resume_task(&self, id: Uuid) -> Result<()> {
        let task = {
            let map = self.tasks.read().await;
            map.get(&id).cloned().ok_or_else(|| anyhow!("Task not found"))?
        };

        if task.status != DownloadStatus::Paused {
            return Err(anyhow!("Task is not paused"));
        }

        let (pause_tx, pause_rx) = watch::channel(false);
        {
            let mut p_map = self.pause_senders.write().await;
            p_map.insert(id, pause_tx);
        }

        let tasks_ref = self.tasks.clone();
        let event_tx = self.event_tx.clone();
        let segmenter = self.segmenter.clone();
        let save_path = task.save_path.clone();

        tokio::spawn(async move {
            Self::run_download_task(
                id, tasks_ref, event_tx, segmenter, save_path, None, None, pause_rx,
            )
            .await;
        });

        let _ = self.event_tx.send(DownloadEvent::TaskResumed { id });
        Ok(())
    }

    pub async fn cancel_task(&self, id: Uuid) -> Result<()> {
        let _ = self.pause_task(id).await;
        let mut map = self.tasks.write().await;
        map.remove(&id);
        Ok(())
    }
}
