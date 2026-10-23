use crate::engine::manager::TaskManager;
use crate::engine::media::MediaEngine;
use crate::model::{CreateDownloadRequest, InspectRequest};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::Json;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub async fn serve_dashboard() -> impl IntoResponse {
    Html(include_str!("../../static/index.html"))
}

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "app": "SIDM",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn list_downloads(State(manager): State<Arc<TaskManager>>) -> impl IntoResponse {
    let tasks = manager.list_tasks().await;
    Json(tasks)
}

pub async fn get_download(
    State(manager): State<Arc<TaskManager>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match manager.get_task(id).await {
        Some(task) => (StatusCode::OK, Json(json!({ "success": true, "task": task }))),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "error": "Task not found" })),
        ),
    }
}

pub async fn create_download(
    State(manager): State<Arc<TaskManager>>,
    Json(payload): Json<CreateDownloadRequest>,
) -> impl IntoResponse {
    match manager.add_task(payload).await {
        Ok(task) => (StatusCode::CREATED, Json(json!({ "success": true, "task": task }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

pub async fn inspect_media(Json(payload): Json<InspectRequest>) -> impl IntoResponse {
    match MediaEngine::inspect(&payload.url, payload.cookies.as_deref()).await {
        Ok(result) => (StatusCode::OK, Json(json!({ "success": true, "data": result }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

pub async fn pause_download(
    State(manager): State<Arc<TaskManager>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match manager.pause_task(id).await {
        Ok(_) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

pub async fn resume_download(
    State(manager): State<Arc<TaskManager>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match manager.resume_task(id).await {
        Ok(_) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

pub async fn cancel_download(
    State(manager): State<Arc<TaskManager>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match manager.cancel_task(id).await {
        Ok(_) => Json(json!({ "success": true })),
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

pub async fn get_license(State(manager): State<Arc<TaskManager>>) -> impl IntoResponse {
    let info = manager.license.check_license();
    Json(json!({ "success": true, "data": info }))
}

pub async fn open_download_folder(
    State(manager): State<Arc<TaskManager>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match manager.open_task_folder(id).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

pub async fn open_download_file(
    State(manager): State<Arc<TaskManager>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match manager.open_task_file(id).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

pub async fn open_default_folder(
    State(manager): State<Arc<TaskManager>>,
) -> impl IntoResponse {
    match manager.open_default_folder() {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true }))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e.to_string() })),
        ),
    }
}

pub async fn activate_license(
    State(manager): State<Arc<TaskManager>>,
    Json(payload): Json<crate::licensing::ActivateRequest>,
) -> impl IntoResponse {
    match manager.license.activate(&payload.license_key, payload.customer_email) {
        Ok(info) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": info })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "error": e })),
        ),
    }
}
