pub mod routes;
pub mod ws;

use crate::config::AppConfig;
use crate::engine::manager::TaskManager;
use axum::http::Method;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn create_router(manager: Arc<TaskManager>) -> Router {
    // Permissive CORS for local bridge communication with browser extensions
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/", get(routes::serve_dashboard))
        .route("/dashboard", get(routes::serve_dashboard))
        .route("/health", get(routes::health_check))
        .route("/api/downloads", get(routes::list_downloads))
        .route("/api/downloads", post(routes::create_download))
        .route("/api/downloads/:id", get(routes::get_download))
        .route("/api/inspect", post(routes::inspect_media))
        .route("/api/downloads/:id/pause", post(routes::pause_download))
        .route("/api/downloads/:id/resume", post(routes::resume_download))
        .route("/api/downloads/:id", delete(routes::cancel_download))
        .route("/ws", get(ws::ws_handler))
        .layer(cors)
        .with_state(manager)
}

pub async fn start_server(config: AppConfig, manager: Arc<TaskManager>) -> anyhow::Result<()> {
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let app = create_router(manager);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🚀 SIDM Bridge Server running at http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
