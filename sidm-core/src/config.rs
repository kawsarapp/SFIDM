use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_download_dir: PathBuf,
    pub default_connections: usize,
    pub max_connections: usize,
    pub server_host: String,
    pub server_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        let download_dir = dirs_or_home();
        Self {
            default_download_dir: download_dir,
            default_connections: 16,
            max_connections: 64, // Upgraded to 64 extreme connections
            server_host: "127.0.0.1".to_string(),
            server_port: 45678,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("SIDM").join("config.json")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("sidm").join("config.json")
    } else {
        PathBuf::from("config.json")
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let path = get_config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    let _ = std::fs::create_dir_all(&config.default_download_dir);
                    for sub in &["Videos", "Music", "Compressed", "Programs", "Documents"] {
                        let _ = std::fs::create_dir_all(config.default_download_dir.join(sub));
                    }
                    return config;
                }
            }
        }
        let default_config = Self::default();
        let _ = default_config.save();
        default_config
    }

    pub fn save(&self) -> Result<()> {
        let path = get_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| anyhow!("Failed to write config file to '{}': {}", path.display(), e))?;
        Ok(())
    }
}

pub fn dirs_or_home() -> PathBuf {
    let base = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let p = base.join("Downloads").join("SIDM");
    let _ = std::fs::create_dir_all(&p);
    for sub in &["Videos", "Music", "Compressed", "Programs", "Documents"] {
        let _ = std::fs::create_dir_all(p.join(sub));
    }
    p
}

/// Spawns native OS folder picker dialog:
/// - Windows: PowerShell System.Windows.Forms.FolderBrowserDialog (silent, no black CMD window)
/// - Linux: Zenity / Kdialog GUI folder picker
pub async fn pick_folder_dialog(current_path: Option<&Path>) -> Result<PathBuf> {
    #[cfg(windows)]
    {
        use tokio::process::Command;
        use std::os::windows::process::CommandExt;

        let mut cmd = Command::new("powershell");
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        
        let initial_folder = current_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let script = format!(
            r#"
            Add-Type -AssemblyName System.Windows.Forms
            $f = New-Object System.Windows.Forms.FolderBrowserDialog
            $f.Description = "Select SIDM Download Directory"
            $f.ShowNewFolderButton = $true
            if ("{}" -ne "" -and (Test-Path "{}")) {{
                $f.SelectedPath = "{}"
            }}
            if ($f.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{
                Write-Output $f.SelectedPath
            }}
            "#,
            initial_folder, initial_folder, initial_folder
        );

        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        let output = cmd.output().await.map_err(|e| anyhow!("Failed to launch PowerShell folder picker: {}", e))?;
        
        if output.status.success() {
            let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !selected.is_empty() {
                let p = PathBuf::from(&selected);
                let _ = std::fs::create_dir_all(&p);
                for sub in &["Videos", "Music", "Compressed", "Programs", "Documents"] {
                    let _ = std::fs::create_dir_all(p.join(sub));
                }
                return Ok(p);
            }
        }
        Err(anyhow!("Folder selection was cancelled by user"))
    }

    #[cfg(not(windows))]
    {
        use tokio::process::Command;

        // 1. Try zenity (GNOME, Ubuntu, Zorin OS, Mint)
        let mut cmd = Command::new("zenity");
        cmd.args(["--file-selection", "--directory", "--title=Select SIDM Download Directory"]);
        if let Some(curr) = current_path {
            cmd.arg(format!("--filename={}/", curr.display()));
        }
        if let Ok(output) = cmd.output().await {
            if output.status.success() {
                let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !selected.is_empty() {
                    let p = PathBuf::from(&selected);
                    let _ = std::fs::create_dir_all(&p);
                    for sub in &["Videos", "Music", "Compressed", "Programs", "Documents"] {
                        let _ = std::fs::create_dir_all(p.join(sub));
                    }
                    return Ok(p);
                }
            }
        }

        // 2. Try kdialog (KDE Plasma)
        let mut kcmd = Command::new("kdialog");
        kcmd.args(["--getexistingdirectory"]);
        if let Some(curr) = current_path {
            kcmd.arg(curr.display().to_string());
        }
        if let Ok(output) = kcmd.output().await {
            if output.status.success() {
                let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !selected.is_empty() {
                    let p = PathBuf::from(&selected);
                    let _ = std::fs::create_dir_all(&p);
                    for sub in &["Videos", "Music", "Compressed", "Programs", "Documents"] {
                        let _ = std::fs::create_dir_all(p.join(sub));
                    }
                    return Ok(p);
                }
            }
        }

        Err(anyhow!("Folder selection was cancelled or no GUI dialog tool found"))
    }
}
