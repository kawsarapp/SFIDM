mod config;
mod engine;
mod licensing;
mod model;
mod server;

use clap::{Parser, Subcommand};
use config::AppConfig;
use engine::manager::TaskManager;
use model::CreateDownloadRequest;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "sidm")]
#[command(about = "SIDM - Next-Gen Smart Internet Download Manager Core Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the background bridge server for browser extensions & GUI
    Server {
        #[arg(short, long, default_value_t = 45678)]
        port: u16,
    },
    /// Direct command-line high speed download
    Download {
        /// URL to download
        url: String,

        /// Number of parallel connections (default: 16)
        #[arg(short, long, default_value_t = 16)]
        connections: usize,

        /// Output filename
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Inspect media streams and available video qualities
    Inspect {
        /// Media URL (YouTube, Facebook, etc.)
        url: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sidm_core=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    let mut config = AppConfig::default();

    match cli.command {
        Some(Commands::Download { url, connections, output }) => {
            println!("⚡ SIDM High-Speed Download starting...");
            println!("🔗 URL: {}", url);
            println!("🚀 Connections: {}", connections);

            let manager = Arc::new(TaskManager::new(config));
            let mut rx = manager.subscribe();

            let task = manager
                .add_task(CreateDownloadRequest {
                    url,
                    file_name: output,
                    save_dir: None,
                    connections: Some(connections),
                    format_id: None,
                    cookies: None,
                })
                .await?;

            println!("📁 Destination: {}", task.save_path);

            while let Ok(event) = rx.recv().await {
                match event {
                    model::DownloadEvent::TaskProgress {
                        id,
                        downloaded_bytes,
                        total_bytes,
                        speed_bps,
                        progress_percent,
                        eta_seconds,
                        ..
                    } if id == task.id => {
                        let speed_mb = speed_bps as f64 / (1024.0 * 1024.0);
                        let total_mb = total_bytes.unwrap_or(0) as f64 / (1024.0 * 1024.0);
                        let curr_mb = downloaded_bytes as f64 / (1024.0 * 1024.0);
                        let eta = eta_seconds.map(|s| format!("{}s", s)).unwrap_or_else(|| "--".into());

                        print!(
                            "\r⏳ [{:.1}%] {:.2} MB / {:.2} MB | Speed: {:.2} MB/s | ETA: {}    ",
                            progress_percent, curr_mb, total_mb, speed_mb, eta
                        );
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                    }
                    model::DownloadEvent::TaskCompleted { id, save_path } if id == task.id => {
                        println!("\n✅ Download completed successfully! Saved to: {}", save_path);
                        break;
                    }
                    model::DownloadEvent::TaskFailed { id, error } if id == task.id => {
                        println!("\n❌ Download failed: {}", error);
                        break;
                    }
                    _ => {}
                }
            }
        }
        Some(Commands::Inspect { url }) => {
            println!("🔍 Inspecting media for URL: {}", url);
            match engine::media::MediaEngine::inspect(&url, None).await {
                Ok(info) => {
                    println!("\n🎥 Title: {}", info.title);
                    if let Some(uploader) = info.uploader {
                        println!("👤 Channel / Author: {}", uploader);
                    }
                    println!("\n📋 Available Qualities:");
                    for (i, f) in info.formats.iter().enumerate() {
                        println!("  {:2}. {:<40} [ID: {}]", i + 1, f.label, f.format_id);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Inspection failed: {}", e);
                }
            }
        }
        Some(Commands::Server { port }) => {
            config.server_port = port;
            println!("==================================================");
            println!("       🚀 SIDM Download Manager Core Daemon       ");
            println!("==================================================");
            let manager = Arc::new(TaskManager::new(config.clone()));
            let lic = manager.license.check_license();
            println!("🔑 Hardware ID (HWID): {}", lic.hwid);
            println!("🛡️  License Status: {:?}", lic.status);
            server::start_server(config, manager).await?;
        }
        None => {
            // Default to running the server
            println!("==================================================");
            println!("       🚀 SIDM Download Manager Core Daemon       ");
            println!("==================================================");
            let manager = Arc::new(TaskManager::new(config.clone()));
            let lic = manager.license.check_license();
            println!("🔑 Hardware ID (HWID): {}", lic.hwid);
            println!("🛡️  License Status: {:?}", lic.status);
            server::start_server(config, manager).await?;
        }
    }

    Ok(())
}
