use std::path::PathBuf;

#[derive(Debug, Clone)]
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

fn dirs_or_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join("Downloads").join("SIDM");
        let _ = std::fs::create_dir_all(&p);
        p
    } else {
        PathBuf::from("./downloads")
    }
}
