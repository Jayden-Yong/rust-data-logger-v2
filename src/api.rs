use axum::{
    extract::{Path, Query, State, Multipart},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use std::collections::HashMap;

use crate::{AppState};
use crate::config::{AppConfig, DeviceConfig, save_config};
use crate::database::{LogEntry, DeviceModel, TagTemplate, DeviceInstance, DeviceTag, ScheduleGroup};

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

// Device Model API endpoints
pub async fn get_device_models(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceModel>>>, StatusCode> {
    match state.database.get_device_models().await {
        Ok(models) => Ok(Json(ApiResponse::success(models))),
        Err(e) => {
            error!("Failed to get device models: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Deserialize)]
pub struct CreateDeviceModelRequest {
    pub name: String,
    pub manufacturer: Option<String>,
    pub protocol_type: String,
    pub description: Option<String>,
}

pub async fn create_device_model(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<DeviceModel>>, StatusCode> {
    let mut name = String::new();
    let mut manufacturer: Option<String> = None;
    let mut protocol_type = String::new();
    let mut description: Option<String> = None;
    let mut csv_data: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart.next_field().await.unwrap() {
        let field_name = field.name().unwrap_or("").to_string();
        
        match field_name.as_str() {
            "name" => {
                name = field.text().await.unwrap_or_default();
            }
            "manufacturer" => {
                let value = field.text().await.unwrap_or_default();
                if !value.is_empty() {
                    manufacturer = Some(value);
                }
            }
            "protocol_type" => {
                protocol_type = field.text().await.unwrap_or_default();
            }
            "description" => {
                let value = field.text().await.unwrap_or_default();
                if !value.is_empty() {
                    description = Some(value);
                }
            }
            "csv_file" => {
                if let Ok(bytes) = field.bytes().await {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        csv_data = Some(text);
                    }
                }
            }
            _ => {}
        }
    }

    // Validate required fields
    if name.is_empty() || protocol_type.is_empty() {
        return Ok(Json(ApiResponse::error("Name and protocol type are required".to_string())));
    }

    // Validate protocol type
    if !["modbus_tcp", "modbus_rtu", "iec104"].contains(&protocol_type.as_str()) {
        return Ok(Json(ApiResponse::error("Invalid protocol type".to_string())));
    }

    // Create device model
    match state.database.create_device_model(&name, manufacturer.as_deref(), &protocol_type, description.as_deref()).await {
        Ok(model) => {
            // Process CSV if provided
            if let Some(csv_content) = csv_data {
                if let Err(e) = process_csv_tags(&state, &model.id, &csv_content).await {
                    error!("Failed to process CSV: {}", e);
                    return Ok(Json(ApiResponse::error(format!("Device model created but failed to process CSV: {}", e))));
                }
            }
            
            info!("Device model {} created successfully", model.id);
            Ok(Json(ApiResponse::success(model)))
        }
        Err(e) => {
            error!("Failed to create device model: {}", e);
            Ok(Json(ApiResponse::error(format!("Failed to create device model: {}", e))))
        }
    }
}

async fn process_csv_tags(state: &AppState, device_model_id: &str, csv_content: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Cursor;
    use csv::Reader;

    let mut reader = Reader::from_reader(Cursor::new(csv_content));
    
    for result in reader.records() {
        let record = result?;
        
        if record.len() < 3 {
            continue; // Skip incomplete records
        }
        
        let name = record.get(0).unwrap_or("").trim();
        let address_str = record.get(1).unwrap_or("").trim();
        let data_type = record.get(2).unwrap_or("").trim();
        let unit = record.get(3).map(|s| s.trim()).filter(|s| !s.is_empty());
        let description = record.get(4).map(|s| s.trim()).filter(|s| !s.is_empty());
        
        if name.is_empty() || address_str.is_empty() || data_type.is_empty() {
            continue;
        }
        
        let address: u16 = address_str.parse().unwrap_or(0);
        
        let tag_template = TagTemplate {
            id: None,
            model_id: device_model_id.to_string(),
            name: name.to_string(),
            address,
            data_type: data_type.to_string(),
            description: description.map(|s| s.to_string()),
            scaling_multiplier: 1.0,
            scaling_offset: 0.0,
            unit: unit.map(|s| s.to_string()),
            read_only: false,
        };
        
        if let Err(e) = state.database.create_tag_template(&tag_template).await {
            error!("Failed to create tag template {}: {}", name, e);
        }
    }
    
    Ok(())
}

pub async fn get_device_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<Json<ApiResponse<DeviceModel>>, StatusCode> {
    match state.database.get_device_model(&model_id).await {
        Ok(Some(model)) => Ok(Json(ApiResponse::success(model))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get device model {}: {}", model_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_device_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.database.delete_device_model(&model_id).await {
        Ok(_) => {
            info!("Device model {} deleted successfully", model_id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            error!("Failed to delete device model {}: {}", model_id, e);
            if e.to_string().contains("not found") {
                Err(StatusCode::NOT_FOUND)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn get_tag_templates(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<TagTemplate>>>, StatusCode> {
    match state.database.get_tag_templates(&model_id).await {
        Ok(templates) => Ok(Json(ApiResponse::success(templates))),
        Err(e) => {
            error!("Failed to get tag templates for model {}: {}", model_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Enhanced Device API endpoints with model support
#[derive(Deserialize)]
pub struct CreateDeviceRequest {
    pub id: String,
    pub name: String,
    pub model_id: Option<String>,
    pub enabled: bool,
    pub polling_interval_ms: u32,
    pub timeout_ms: u32,
    pub retry_count: u32,
    pub protocol_config: serde_json::Value,
    pub tags: Vec<CreateTagRequest>,
}

#[derive(Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub address: u16,
    pub data_type: String,
    pub description: Option<String>,
    pub scaling_multiplier: f64,
    pub scaling_offset: f64,
    pub unit: Option<String>,
    pub read_only: bool,
    pub enabled: bool,
    pub schedule_group_id: Option<String>,
}

pub async fn create_device_with_tags(
    State(state): State<AppState>,
    Json(request): Json<CreateDeviceRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let now = chrono::Utc::now();

    // Create device instance
    let device = DeviceInstance {
        id: request.id.clone(),
        name: request.name,
        model_id: request.model_id,
        enabled: request.enabled,
        polling_interval_ms: request.polling_interval_ms,
        timeout_ms: request.timeout_ms,
        retry_count: request.retry_count,
        protocol_config: serde_json::to_string(&request.protocol_config).unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };

    // Create device in database
    if let Err(e) = state.database.create_device(&device).await {
        error!("Failed to create device: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Create device tags
    let device_tags: Vec<DeviceTag> = request.tags.into_iter().map(|tag| DeviceTag {
        id: None,
        device_id: request.id.clone(),
        name: tag.name,
        address: tag.address,
        data_type: tag.data_type,
        description: tag.description,
        scaling_multiplier: tag.scaling_multiplier,
        scaling_offset: tag.scaling_offset,
        unit: tag.unit,
        read_only: tag.read_only,
        enabled: tag.enabled,
        schedule_group_id: tag.schedule_group_id,
    }).collect();

    if let Err(e) = state.database.create_device_tags(&request.id, &device_tags).await {
        error!("Failed to create device tags: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Created device {} with {} tags", request.id, device_tags.len());
    Ok(Json(ApiResponse::success("Device created successfully".to_string())))
}

pub async fn get_devices_enhanced(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceWithTags>>>, StatusCode> {
    let devices = match state.database.get_devices().await {
        Ok(devices) => devices,
        Err(e) => {
            error!("Failed to get devices: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut devices_with_tags = Vec::new();
    for device in devices {
        let tags = match state.database.get_device_tags(&device.id).await {
            Ok(tags) => tags,
            Err(e) => {
                error!("Failed to get tags for device {}: {}", device.id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

        // Get device status from database
        let device_status = state.database.get_device_status(&device.id).await.ok().flatten();
        let status = device_status.as_ref().map(|s| s.status.clone());
        let last_update = device_status.as_ref().map(|s| s.last_update.to_rfc3339());
        
        // Check if device is currently running
        let is_running = state.logging_service.is_device_running(&device.id).await;

        devices_with_tags.push(DeviceWithTags { 
            device, 
            tags, 
            status,
            is_running,
            last_update,
        });
    }

    Ok(Json(ApiResponse::success(devices_with_tags)))
}

pub async fn get_device_enhanced(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<DeviceWithTags>>, StatusCode> {
    let device = match state.database.get_device(&device_id).await {
        Ok(Some(device)) => device,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get device {}: {}", device_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let tags = match state.database.get_device_tags(&device_id).await {
        Ok(tags) => tags,
        Err(e) => {
            error!("Failed to get tags for device {}: {}", device_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get device status from database
    let device_status = state.database.get_device_status(&device_id).await.ok().flatten();
    let status = device_status.as_ref().map(|s| s.status.clone());
    let last_update = device_status.as_ref().map(|s| s.last_update.to_rfc3339());
    
    // Check if device is currently running
    let is_running = state.logging_service.is_device_running(&device_id).await;

    Ok(Json(ApiResponse::success(DeviceWithTags { 
        device, 
        tags, 
        status,
        is_running,
        last_update,
    })))
}

pub async fn get_device_tags_api(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<DeviceTag>>>, StatusCode> {
    match state.database.get_device_tags(&device_id).await {
        Ok(tags) => Ok(Json(ApiResponse::success(tags))),
        Err(e) => {
            error!("Failed to get device tags for {}: {}", device_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_device_with_tags(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(request): Json<CreateDeviceRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let now = chrono::Utc::now();

    // Update device instance
    let device = DeviceInstance {
        id: device_id.clone(),
        name: request.name,
        model_id: request.model_id,
        enabled: request.enabled,
        polling_interval_ms: request.polling_interval_ms,
        timeout_ms: request.timeout_ms,
        retry_count: request.retry_count,
        protocol_config: serde_json::to_string(&request.protocol_config).unwrap_or_default(),
        created_at: now, // This will be ignored in update
        updated_at: now,
    };

    // Update device in database
    if let Err(e) = state.database.update_device(&device).await {
        error!("Failed to update device: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Delete existing tags and recreate them
    if let Err(e) = state.database.delete_device_tags(&device_id).await {
        error!("Failed to delete existing device tags: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Create updated device tags
    let device_tags: Vec<DeviceTag> = request.tags.into_iter().map(|tag| DeviceTag {
        id: None,
        device_id: device_id.clone(),
        name: tag.name,
        address: tag.address,
        data_type: tag.data_type,
        description: tag.description,
        scaling_multiplier: tag.scaling_multiplier,
        scaling_offset: tag.scaling_offset,
        unit: tag.unit,
        read_only: tag.read_only,
        enabled: tag.enabled,
        schedule_group_id: tag.schedule_group_id,
    }).collect();

    if let Err(e) = state.database.create_device_tags(&device_id, &device_tags).await {
        error!("Failed to update device tags: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Updated device {} with {} tags", device_id, device_tags.len());
    Ok(Json(ApiResponse::success("Device updated successfully".to_string())))
}

#[derive(Serialize)]
pub struct DeviceWithTags {
    pub device: DeviceInstance,
    pub tags: Vec<DeviceTag>,
    pub status: Option<String>,
    pub is_running: bool,
    pub last_update: Option<String>,
}

// Schedule Group API endpoints
pub async fn get_schedule_groups(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ScheduleGroup>>>, StatusCode> {
    let schedule_groups = match state.database.get_schedule_groups().await {
        Ok(groups) => groups,
        Err(e) => {
            error!("Failed to get schedule groups: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(ApiResponse::success(schedule_groups)))
}

pub async fn get_schedule_group(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
) -> Result<Json<ApiResponse<ScheduleGroup>>, StatusCode> {
    let schedule_group = match state.database.get_schedule_group(&group_id).await {
        Ok(Some(group)) => group,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            error!("Failed to get schedule group {}: {}", group_id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(ApiResponse::success(schedule_group)))
}

#[derive(Deserialize)]
pub struct CreateScheduleGroupRequest {
    pub id: String,
    pub name: String,
    pub polling_interval_ms: u32,
    pub description: Option<String>,
    pub enabled: bool,
}

pub async fn create_schedule_group(
    State(state): State<AppState>,
    Json(request): Json<CreateScheduleGroupRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let now = chrono::Utc::now();

    let schedule_group = ScheduleGroup {
        id: request.id.clone(),
        name: request.name,
        polling_interval_ms: request.polling_interval_ms,
        description: request.description,
        enabled: request.enabled,
        created_at: now,
        updated_at: now,
    };

    if let Err(e) = state.database.create_schedule_group(&schedule_group).await {
        error!("Failed to create schedule group: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Created schedule group {}", request.id);
    Ok(Json(ApiResponse::success("Schedule group created successfully".to_string())))
}

pub async fn update_schedule_group(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
    Json(request): Json<CreateScheduleGroupRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    let now = chrono::Utc::now();

    let schedule_group = ScheduleGroup {
        id: group_id.clone(),
        name: request.name,
        polling_interval_ms: request.polling_interval_ms,
        description: request.description,
        enabled: request.enabled,
        created_at: now, // This will be ignored in update
        updated_at: now,
    };

    if let Err(e) = state.database.update_schedule_group(&schedule_group).await {
        error!("Failed to update schedule group: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Updated schedule group {}", group_id);
    Ok(Json(ApiResponse::success("Schedule group updated successfully".to_string())))
}

pub async fn delete_schedule_group(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    if let Err(e) = state.database.delete_schedule_group(&group_id).await {
        error!("Failed to delete schedule group: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("Deleted schedule group {}", group_id);
    Ok(Json(ApiResponse::success("Schedule group deleted successfully".to_string())))
}
