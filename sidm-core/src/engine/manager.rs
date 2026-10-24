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

        let category = Classifier::get_category_subfolder(&file_name, &download_type, req.format_id.as_deref());
        let save_dir = req
            .save_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.default_download_dir.join(category));

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
                let on_progress = move |downloaded: u64, total: Option<u64>, speed: u64, percent: f32, eta: Option<u64>, chunks: Vec<ChunkInfo>| {
                    let p_tasks = p_tasks.clone();
                    let p_event_tx = p_event_tx.clone();
                    tokio::spawn(async move {
                        let mut map = p_tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Downloading;
                            t.downloaded_bytes = downloaded;
                            if total.is_some() {
                                t.total_bytes = total;
                            }
                            t.speed_bps = speed;
                            t.progress_percent = percent;
                            t.eta_seconds = eta;
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
                    Ok(final_path) => {
                        let actual_path = if !final_path.is_empty() { final_path } else { save_path.clone() };
                        let actual_name = std::path::Path::new(&actual_path)
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or(task.file_name.clone());

                        println!("✅ [SIDM] Media Download Completed! File: {}", actual_path);
                        let mut map = tasks.write().await;
                        if let Some(t) = map.get_mut(&id) {
                            t.status = DownloadStatus::Completed;
                            t.progress_percent = 100.0;
                            t.save_path = actual_path.clone();
                            t.file_name = actual_name.clone();
                        }
                        trigger_desktop_notification(&actual_name, &actual_path);
                        let _ = event_tx.send(DownloadEvent::TaskCompleted { id, save_path: actual_path });
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
                        trigger_desktop_notification(&task.file_name, &save_path);
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

    pub async fn open_task_folder(&self, id: Uuid) -> Result<()> {
        let task = self.get_task(id).await.ok_or_else(|| anyhow!("Task not found"))?;
        let p = std::path::PathBuf::from(&task.save_path);

        #[cfg(target_os = "windows")]
        {
            if p.exists() {
                // Windows Explorer /select,"C:\path\to\file.mp4" opens the folder AND highlights the file!
                let res = std::process::Command::new("explorer")
                    .arg(format!("/select,{}", p.display()))
                    .spawn();
                if res.is_ok() {
                    return Ok(());
                }
            }
            let folder = if p.is_dir() {
                p
            } else if let Some(parent) = p.parent() {
                if parent.exists() {
                    parent.to_path_buf()
                } else {
                    self.config.default_download_dir.clone()
                }
            } else {
                self.config.default_download_dir.clone()
            };
            std::process::Command::new("explorer")
                .arg(folder.display().to_string())
                .spawn()?;
            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let folder = if p.is_dir() {
                p
            } else if let Some(parent) = p.parent() {
                if parent.exists() {
                    parent.to_path_buf()
                } else {
                    self.config.default_download_dir.clone()
                }
            } else {
                self.config.default_download_dir.clone()
            };
            Self::launch_system_open(&folder)
        }
    }

    pub async fn open_task_file(&self, id: Uuid) -> Result<()> {
        let task = self.get_task(id).await.ok_or_else(|| anyhow!("Task not found"))?;
        let p = std::path::PathBuf::from(&task.save_path);
        
        let target = if p.exists() {
            p
        } else if let Some(parent) = p.parent() {
            // Check for downloaded video in target directory (including .mp4, .mkv, or stem matches)
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(entries) = std::fs::read_dir(parent) {
                let mut candidates: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        (!stem.is_empty() && name.contains(stem))
                            || (!name.ends_with(".part") && !name.ends_with(".ytdl"))
                    })
                    .collect();
                candidates.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
                if let Some(last) = candidates.last() {
                    last.path()
                } else {
                    parent.to_path_buf()
                }
            } else {
                parent.to_path_buf()
            }
        } else {
            self.config.default_download_dir.clone()
        };

        #[cfg(target_os = "windows")]
        {
            // Windows: "cmd /C start \"\" \"<target>\"" launches default media player (VLC, Windows Media Player, etc.)
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &target.to_string_lossy()])
                .spawn()?;
            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            Self::launch_system_open(&target)
        }
    }

    pub fn open_default_folder(&self) -> Result<()> {
        let dir = &self.config.default_download_dir;
        let _ = std::fs::create_dir_all(dir);

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(dir.display().to_string())
                .spawn()?;
            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            Self::launch_system_open(dir)
        }
    }

    pub fn get_extension_dir() -> PathBuf {
        let candidates = [
            "extension",
            "sidm-extension",
            "../sidm-extension",
            "../../sidm-extension",
        ];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.join("manifest.json").exists() {
                if let Ok(canon) = std::fs::canonicalize(&p) {
                    return canon;
                }
                return p;
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                for sub in &["extension", "sidm-extension", "../extension", "../sidm-extension"] {
                    let p = parent.join(sub);
                    if p.join("manifest.json").exists() {
                        if let Ok(canon) = std::fs::canonicalize(&p) {
                            return canon;
                        }
                        return p;
                    }
                }
            }
        }
        PathBuf::from("extension")
    }

    pub async fn launch_browser_extension(&self) -> Result<()> {
        let ext_dir = Self::get_extension_dir();

        #[cfg(target_os = "windows")]
        {
            let ext_arg = format!("--load-extension={}", ext_dir.display());
            let chrome_paths = [
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            ];
            let mut launched = false;
            for p in &chrome_paths {
                if std::path::Path::new(p).exists() {
                    let _ = std::process::Command::new(p)
                        .arg(&ext_arg)
                        .arg("http://127.0.0.1:45678")
                        .spawn();
                    launched = true;
                    break;
                }
            }

            if !launched {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", "chrome", &ext_arg])
                    .spawn();
            }

            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            let ext_arg = format!("--load-extension={}", ext_dir.display());
            if std::process::Command::new("google-chrome")
                .arg(&ext_arg)
                .arg("http://127.0.0.1:45678")
                .spawn().is_ok() {
                return Ok(());
            }
            if std::process::Command::new("chromium")
                .arg(&ext_arg)
                .arg("http://127.0.0.1:45678")
                .spawn().is_ok() {
                return Ok(());
            }
            Ok(())
        }
    }

    fn launch_system_open(path: &std::path::Path) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()?;
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(path.display().to_string())
                .spawn()?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(path)
                .spawn()?;
        }
        Ok(())
    }
}

fn trigger_desktop_notification(filename: &str, save_path: &str) {
    let fname = filename.to_string();
    let spath = save_path.to_string();
    tokio::spawn(async move {
        if cfg!(target_os = "windows") {
            let script = format!(
                r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$template = @"
<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>⚡ SIDM: Download Completed!</text>
            <text>{}</text>
        </binding>
    </visual>
</toast>
"@
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("SIDM").Show($toast)
"#,
                fname.replace('"', "")
            );
            let _ = tokio::process::Command::new("powershell")
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-Command")
                .arg(script)
                .spawn();
        } else if cfg!(target_os = "linux") {
            let body = format!("Saved to: {}", spath);
            let _ = tokio::process::Command::new("notify-send")
                .arg("⚡ SIDM: Download Completed!")
                .arg(format!("{}\n{}", fname, body))
                .arg("-i")
                .arg("video-x-generic")
                .spawn();
        }
    });
}
