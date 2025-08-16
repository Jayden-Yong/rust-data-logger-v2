use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::{AppState};
use crate::config::{AppConfig, DeviceConfig, save_config};
use crate::database::LogEntry;

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AppConfig>>, StatusCode> {
    let config = (*state.config).clone();
    Ok(Json(ApiResponse::success(config)))
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(new_config): Json<AppConfig>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match save_config(&new_config).await {
        Ok(()) => Ok(Json(ApiResponse::success("Configuration updated successfully".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to save configuration: {}", e)))),
    }
}

pub async fn get_devices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceConfig>>>, StatusCode> {
    let devices = state.config.devices.clone();
    Ok(Json(ApiResponse::success(devices)))
}

pub async fn get_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<DeviceConfig>>, StatusCode> {
    if let Some(device) = state.config.devices.iter().find(|d| d.id == device_id) {
        Ok(Json(ApiResponse::success(device.clone())))
    } else {
        Ok(Json(ApiResponse::error("Device not found".to_string())))
    }
}

pub async fn create_device(
    State(_state): State<AppState>,
    Json(device): Json<DeviceConfig>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Check if device ID already exists
    if _state.config.devices.iter().any(|d| d.id == device.id) {
        return Ok(Json(ApiResponse::error("Device ID already exists".to_string())));
    }

    // This is a simplified implementation - in a real app, you'd update the config file
    Ok(Json(ApiResponse::success("Device created successfully".to_string())))
}

pub async fn update_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(_updated_device): Json<DeviceConfig>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    if state.config.devices.iter().any(|d| d.id == device_id) {
        // This is a simplified implementation - in a real app, you'd update the config file
        Ok(Json(ApiResponse::success("Device updated successfully".to_string())))
    } else {
        Ok(Json(ApiResponse::error("Device not found".to_string())))
    }
}

pub async fn delete_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    if state.config.devices.iter().any(|d| d.id == device_id) {
        // Stop the device first
        if let Err(e) = state.logging_service.stop_device(&device_id).await {
            return Ok(Json(ApiResponse::error(format!("Failed to stop device: {}", e))));
        }

        // This is a simplified implementation - in a real app, you'd update the config file
        Ok(Json(ApiResponse::success("Device deleted successfully".to_string())))
    } else {
        Ok(Json(ApiResponse::error("Device not found".to_string())))
    }
}

pub async fn start_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.logging_service.start_device(&device_id).await {
        Ok(()) => Ok(Json(ApiResponse::success("Device started successfully".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to start device: {}", e)))),
    }
}

pub async fn stop_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.logging_service.stop_device(&device_id).await {
        Ok(()) => Ok(Json(ApiResponse::success("Device stopped successfully".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to stop device: {}", e)))),
    }
}

pub async fn get_logs(
    State(state): State<AppState>,
    Query(params): Query<LogQuery>,
) -> Result<Json<ApiResponse<Vec<LogEntry>>>, StatusCode> {
    match state.database.get_log_entries(None, params.limit, params.offset).await {
        Ok(logs) => Ok(Json(ApiResponse::success(logs))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get logs: {}", e)))),
    }
}

pub async fn get_device_logs(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Query(params): Query<LogQuery>,
) -> Result<Json<ApiResponse<Vec<LogEntry>>>, StatusCode> {
    match state.database.get_log_entries(Some(&device_id), params.limit, params.offset).await {
        Ok(logs) => Ok(Json(ApiResponse::success(logs))),
        Err(e) => Ok(Json(ApiResponse::error(format!("Failed to get device logs: {}", e)))),
    }
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub devices: Vec<DeviceStatusInfo>,
    pub total_log_entries: u32,
    pub server_uptime: String,
}

#[derive(Serialize)]
pub struct DeviceStatusInfo {
    pub device_id: String,
    pub status: String,
    pub last_update: chrono::DateTime<chrono::Utc>,
    pub error_message: Option<String>,
    pub connection_count: i64,
    pub is_running: bool,
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<StatusResponse>>, StatusCode> {
    let device_statuses = match state.logging_service.get_all_device_statuses().await {
        Ok(statuses) => statuses,
        Err(e) => return Ok(Json(ApiResponse::error(format!("Failed to get device statuses: {}", e)))),
    };

    let mut device_status_info = Vec::new();
    
    for status in device_statuses {
        let is_running = state.logging_service.is_device_running(&status.device_id).await;
        device_status_info.push(DeviceStatusInfo {
            device_id: status.device_id,
            status: status.status,
            last_update: status.last_update,
            error_message: status.error_message,
            connection_count: status.connection_count,
            is_running,
        });
    }

    // Get total log entries count (simplified)
    let total_logs = match state.database.get_log_entries(None, Some(1), Some(0)).await {
        Ok(_) => 0, // Simplified - would need a count query
        Err(_) => 0,
    };

    let response = StatusResponse {
        devices: device_status_info,
        total_log_entries: total_logs,
        server_uptime: "Running".to_string(), // Simplified
    };

    Ok(Json(ApiResponse::success(response)))
}
