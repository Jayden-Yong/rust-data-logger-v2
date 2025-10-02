use axum::{
    extract::{Path, Query, State, Multipart},
    http::{StatusCode, HeaderMap},
    response::Json,
    middleware::Next,
    response::Response,
    extract::Request,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::{AppState};
use crate::config::{AppConfig, DeviceConfig, save_config};
use crate::database::{LogEntry, DeviceModel, TagTemplate, DeviceInstance, DeviceTag, ScheduleGroup, ModbusTcpTagRegister, PlantConfiguration};
use crate::csv_parser::ModbusTcpCsvParserService;

use serde_json::{json, Value};

// docker health check endpoint
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "AVA Device Logger",
        "version": env!("CARGO_PKG_VERSION")
    })))
}

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

// Authentication structures
#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub session_token: String,
    pub user: UserInfo,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: String,
}

#[derive(Deserialize)]
pub struct PlantConfigRequest {
    pub plant_name: String,
    pub thingsboard_entity_group_id: Option<String>,
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

/// Parses a ThingsBoard device name to extract device type and index
/// Expected format: "PREFIX-T##" where T is type abbreviation and ## is index
/// Examples:
/// - "ACCV-P002-I01" -> Some(("Inverter", 1))
/// - "GR-P001-S05" -> Some(("String", 5))
/// - "CMES-PR084-I12" -> Some(("Inverter", 12))
fn parse_device_name_for_type_and_index(device_name: &str, entity_group_name: &str) -> Option<(String, u32)> {
    // Extract expected prefix from entity group name
    let expected_prefix = extract_group_prefix_from_name(entity_group_name);
    
    // Check if device name starts with the expected prefix
    if !device_name.starts_with(&expected_prefix) {
        return None;
    }
    
    // Remove prefix and dash to get the type-index part
    let remaining = device_name.strip_prefix(&format!("{}-", expected_prefix))?;
    
    // Parse the type-index part (e.g., "I01", "S05", "PM01", "MT01", "WS01")
    if remaining.len() >= 3 {
        // Handle multi-character prefixes first (PM, MT, WS)
        let device_type = if remaining.starts_with("PM") && remaining.len() >= 4 {
            let index_str = &remaining[2..];
            if let Ok(index) = index_str.parse::<u32>() {
                return Some(("PowerMeter".to_string(), index));
            }
            return None;
        } else if remaining.starts_with("MT") && remaining.len() >= 4 {
            let index_str = &remaining[2..];
            if let Ok(index) = index_str.parse::<u32>() {
                return Some(("Meter".to_string(), index));
            }
            return None;
        } else if remaining.starts_with("WS") && remaining.len() >= 4 {
            let index_str = &remaining[2..];
            if let Ok(index) = index_str.parse::<u32>() {
                return Some(("Weather Station".to_string(), index));
            }
            return None;
        } else {
            // Handle single character prefixes
            let type_char = remaining.chars().next()?;
            let _index_str = &remaining[1..];
            
            // Convert type abbreviation back to full type name
            match type_char {
                'I' => "Inverter",
                'S' => "String", 
                'P' => "PlantBlock",
                'D' => "Device",
                _ => return None,
            }
        };
        
        // Parse the numeric index for single character prefixes
        if let Ok(index) = remaining[1..].parse::<u32>() {
            return Some((device_type.to_string(), index));
        }
    }
    
    None
}

/// Extracts the prefix from entity group name (same logic as in tb_rust_client.rs)
fn extract_group_prefix_from_name(entity_group_name: &str) -> String {
    let parts: Vec<&str> = entity_group_name.split('-').collect();
    
    if parts.len() >= 3 {
        // Take first two parts separated by dash
        format!("{}-{}", parts[0], parts[1])
    } else if parts.len() == 2 {
        // If only two parts, take both
        format!("{}-{}", parts[0], parts[1])
    } else {
        // If only one part or empty, use as is
        entity_group_name.to_string()
    }
}

pub async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<AppConfig>>, StatusCode> {
    let config = (*state.config).clone();
    Ok(Json(ApiResponse::success(config)))
}

pub async fn update_config(
    State(_state): State<AppState>,
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
    info!("Attempting to delete device with ID: {}", device_id);
    
    // Stop the device first if it's running
    if let Err(e) = state.logging_service.stop_device(&device_id).await {
        warn!("Failed to stop device before deletion: {}", e);
        // Continue with deletion even if stop fails
    }

    // Delete from database (matching the enhanced endpoints approach)
    match state.database.delete_device(&device_id).await {
        Ok(()) => {
            info!("Device {} deleted successfully", device_id);
            Ok(Json(ApiResponse::success("Device deleted successfully".to_string())))
        }
        Err(e) => {
            error!("Failed to delete device {}: {}", device_id, e);
            if e.to_string().contains("not found") {
                Ok(Json(ApiResponse::error("Device not found".to_string())))
            } else {
                Ok(Json(ApiResponse::error(format!("Failed to delete device: {}", e))))
            }
        }
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

// NOTE: This struct is unused - the create_device_model handler uses multipart form parsing instead
// The actual struct used is in api_additions.rs
// #[derive(Deserialize)]
// pub struct CreateDeviceModelRequest {
//     pub name: String,
//     pub manufacturer: Option<String>,
//     pub protocol_type: String,  
//     pub description: Option<String>,
// }

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
    pub serial_no: Option<String>,
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
    pub size: i32,
    pub data_type: String,
    pub description: Option<String>,
    pub scaling_multiplier: f64,
    pub scaling_offset: f64,
    pub unit: Option<String>,
    pub read_only: bool,
    pub enabled: bool,
    pub schedule_group_id: Option<String>,
    pub agg_to_field: Option<String>,
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
        serial_no: request.serial_no,
        model_id: request.model_id,
        enabled: request.enabled,
        polling_interval_ms: request.polling_interval_ms,
        timeout_ms: request.timeout_ms,
        retry_count: request.retry_count,
        protocol_config: serde_json::to_string(&request.protocol_config).unwrap_or_default(),
        tb_device_id: None,
        tb_group_id: None,
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
        size: tag.size,
        data_type: tag.data_type,
        description: tag.description,
        scaling_multiplier: tag.scaling_multiplier,
        scaling_offset: tag.scaling_offset,
        unit: tag.unit,
        read_only: tag.read_only,
        enabled: tag.enabled,
        schedule_group_id: tag.schedule_group_id,
        agg_to_field: tag.agg_to_field,
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

/// Get all devices that are not synced to ThingsBoard (tb_device_id is NULL)
pub async fn get_unsynced_devices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceWithTags>>>, StatusCode> {
    let devices = match state.database.get_unsynced_devices().await {
        Ok(devices) => devices,
        Err(e) => {
            error!("Failed to get unsynced devices: {}", e);
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

/// Get all devices that belong to a specific ThingsBoard entity group
pub async fn get_devices_by_group(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<DeviceWithTags>>>, StatusCode> {
    let devices = match state.database.get_devices_by_group_id(&group_id).await {
        Ok(devices) => devices,
        Err(e) => {
            error!("Failed to get devices for group {}: {}", group_id, e);
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

    // Get existing device to preserve tb_device_id and tb_group_id
    let existing_device = match state.database.get_device(&device_id).await {
        Ok(Some(device)) => device,
        Ok(None) => {
            error!("Device not found: {}", device_id);
            return Ok(Json(ApiResponse::error(format!("Device not found: {}", device_id))));
        }
        Err(e) => {
            error!("Failed to get existing device: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Check if serial number changed BEFORE moving values
    let serial_number_changed = existing_device.serial_no != request.serial_no;
    let tb_device_id_clone = existing_device.tb_device_id.clone();
    let tb_group_id_clone = existing_device.tb_group_id.clone();

    // Update device instance - preserve tb_device_id and tb_group_id from existing device
    let device = DeviceInstance {
        id: device_id.clone(),
        name: request.name,
        serial_no: request.serial_no,
        model_id: request.model_id,
        enabled: request.enabled,
        polling_interval_ms: request.polling_interval_ms,
        timeout_ms: request.timeout_ms,
        retry_count: request.retry_count,
        protocol_config: serde_json::to_string(&request.protocol_config).unwrap_or_default(),
        tb_device_id: existing_device.tb_device_id,  // Preserve existing ThingsBoard ID
        tb_group_id: existing_device.tb_group_id,    // Preserve existing ThingsBoard group
        created_at: existing_device.created_at,      // Preserve creation time
        updated_at: now,
    };

    // Update device in database
    if let Err(e) = state.database.update_device(&device).await {
        error!("Failed to update device: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Check if serial number changed and device is synced to ThingsBoard
    if serial_number_changed && tb_device_id_clone.is_some() {
        info!("Serial number changed for synced device {}, updating ThingsBoard attributes", device_id);
        
        // Get device type from database to determine which attributes to update
        match state.database.get_device_ava_type(&device_id).await {
            Ok(Some(device_type)) => {
                // Only update attributes for Inverter and Meter devices
                if device_type == "Inverter" || device_type == "Meter" || device_type == "PowerMeter" {
                    // Get entity group name from tb_group_id
                    if let Some(tb_group_id) = &tb_group_id_clone {
                        // Connect to ThingsBoard
                        use crate::tb_rust_client::ThingsBoardClient;
                        let base_url = "https://monitoring.avaasia.co".to_string();
                        let mut tb_client = ThingsBoardClient::new(&base_url);
                        
                        match tb_client.login("jaydenyong28@gmail.com", "lalala88").await {
                            Ok(()) => {
                                // Get entity group name and TB device name
                                match tb_client.get_all_entity_groups("DEVICE").await {
                                    Ok(groups) => {
                                        if let Some(group) = groups.iter().find(|g| &g.id.id == tb_group_id) {
                                            let entity_group_name = &group.name;
                                            
                                            // Fetch ThingsBoard device to get its name
                                            if let Some(tb_device_id) = &tb_device_id_clone {
                                                match tb_client.get_device_by_id(tb_device_id).await {
                                                    Ok(tb_device) => {
                                                        let tb_device_name = &tb_device.name;
                                                        
                                                        // Build and update attributes using TB device name
                                                        match tb_client.build_device_attributes(&device, tb_device_name, &device_type, entity_group_name, &state.database).await {
                                                            Ok(attributes) => {
                                                                match tb_client.update_device_attributes(tb_device_id, attributes).await {
                                                                    Ok(_) => {
                                                                        info!("✅ Successfully updated ThingsBoard attributes for device {}", device_id);
                                                                    }
                                                                    Err(e) => {
                                                                        warn!("⚠️ Failed to update ThingsBoard attributes for device {}: {}", device_id, e);
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                warn!("⚠️ Failed to build attributes for device {}: {}", device_id, e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!("⚠️ Failed to fetch ThingsBoard device {}: {}", tb_device_id, e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("⚠️ Failed to get entity groups for attribute update: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("⚠️ Failed to login to ThingsBoard for attribute update: {}", e);
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                warn!("Device type not found for {}", device_id);
            }
            Err(e) => {
                warn!("Failed to get device type for {}: {}", device_id, e);
            }
        }
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
        size: tag.size,
        data_type: tag.data_type,
        description: tag.description,
        scaling_multiplier: tag.scaling_multiplier,
        scaling_offset: tag.scaling_offset,
        unit: tag.unit,
        read_only: tag.read_only,
        enabled: tag.enabled,
        schedule_group_id: tag.schedule_group_id,
        agg_to_field: tag.agg_to_field,
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

// Modbus TCP Tag Register API Endpoints

#[derive(Serialize, Deserialize)]
pub struct CsvUploadResponse {
    pub success: bool,
    pub message: String,
    pub records_processed: u64,
    pub device_brand: String,
    pub device_model: String,
    pub summary: String,
    pub validation_errors: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ModbusTcpTagQuery {
    pub device_brand: Option<String>,
    pub device_model: Option<String>,
    pub model_id: Option<String>,
    // NOTE: ava_type field is unused in the handler - only model_id, device_brand, and device_model are checked
    // pub ava_type: Option<String>,
}

pub async fn upload_modbus_tcp_csv_tags(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<CsvUploadResponse>, StatusCode> {
    let csv_parser = ModbusTcpCsvParserService::new();
    let mut csv_data: Option<bytes::Bytes> = None;
    let mut device_model_name: Option<String> = None;
    let mut manufacturer: Option<String> = None;
    
    // Parse multipart form to extract CSV file, device model name, and manufacturer
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(name) = field.name() {
            match name {
                "csv_file" => {
                    csv_data = Some(field.bytes().await.unwrap());
                }
                "device_model_name" => {
                    if let Ok(text) = field.text().await {
                        device_model_name = Some(text);
                    }
                }
                "manufacturer" => {
                    if let Ok(text) = field.text().await {
                        manufacturer = Some(text);
                    }
                }
                _ => {
                    // Ignore other fields
                }
            }
        }
    }
    
    let csv_data = match csv_data {
        Some(data) => data,
        None => {
            return Ok(Json(CsvUploadResponse {
                success: false,
                message: "No CSV file found in request".to_string(),
                records_processed: 0,
                device_brand: "".to_string(),
                device_model: "".to_string(),
                summary: "".to_string(),
                validation_errors: vec!["No file uploaded".to_string()],
            }));
        }
    };
    
    let device_model_name = match device_model_name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => {
            return Ok(Json(CsvUploadResponse {
                success: false,
                message: "Device model name is required".to_string(),
                records_processed: 0,
                device_brand: "".to_string(),
                device_model: "".to_string(),
                summary: "".to_string(),
                validation_errors: vec!["Device model name not provided".to_string()],
            }));
        }
    };
    
    let manufacturer = match manufacturer {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => {
            return Ok(Json(CsvUploadResponse {
                success: false,
                message: "Manufacturer name is required".to_string(),
                records_processed: 0,
                device_brand: "".to_string(),
                device_model: "".to_string(),
                summary: "".to_string(),
                validation_errors: vec!["Manufacturer name not provided".to_string()],
            }));
        }
    };
    
    // Validate CSV headers first
    if let Err(e) = csv_parser.validate_csv_headers(csv_data.as_ref()) {
        return Ok(Json(CsvUploadResponse {
            success: false,
            message: format!("CSV validation failed: {}", e),
            records_processed: 0,
            device_brand: "".to_string(),
            device_model: "".to_string(),
            summary: "".to_string(),
            validation_errors: vec![e.to_string()],
        }));
    }

    // Parse CSV with the provided device model name and manufacturer
    match csv_parser.parse_csv_with_device_model_and_manufacturer(csv_data.as_ref(), &device_model_name, &manufacturer) {
        Ok(tag_registers) => {
            if tag_registers.is_empty() {
                return Ok(Json(CsvUploadResponse {
                    success: false,
                    message: "No valid records found in CSV".to_string(),
                    records_processed: 0,
                    device_brand: "".to_string(),
                    device_model: device_model_name.clone(),
                    summary: "".to_string(),
                    validation_errors: vec!["Empty CSV or no valid records".to_string()],
                }));
            }

            // Validate record data
            if let Err(e) = csv_parser.validate_record_data(&tag_registers) {
                return Ok(Json(CsvUploadResponse {
                    success: false,
                    message: format!("Data validation failed: {}", e),
                    records_processed: 0,
                    device_brand: "".to_string(),
                    device_model: device_model_name.clone(),
                    summary: "".to_string(),
                    validation_errors: vec![e.to_string()],
                }));
            }

            let device_brand = tag_registers[0].device_brand.clone();
            let summary = csv_parser.get_summary(&tag_registers);

            // Insert records
            match state.database.bulk_insert_modbus_tcp_tag_registers(tag_registers).await {
                Ok(count) => {
                    info!("Successfully inserted {} Modbus TCP tag registers for {} {}", 
                        count, device_brand, device_model_name);
                    
                    return Ok(Json(CsvUploadResponse {
                        success: true,
                        message: format!("Successfully processed {} records for {} {}", 
                            count, device_brand, device_model_name),
                        records_processed: count,
                        device_brand,
                        device_model: device_model_name.clone(),
                        summary,
                        validation_errors: vec![],
                    }));
                }
                Err(e) => {
                    error!("Failed to insert Modbus TCP tag registers: {}", e);
                    return Ok(Json(CsvUploadResponse {
                        success: false,
                        message: format!("Database error: {}", e),
                        records_processed: 0,
                        device_brand,
                        device_model: device_model_name.clone(),
                        summary,
                        validation_errors: vec![e.to_string()],
                    }));
                }
            }
        }
        Err(e) => {
            error!("Failed to parse CSV: {}", e);
            return Ok(Json(CsvUploadResponse {
                success: false,
                message: format!("Failed to parse CSV: {}", e),
                records_processed: 0,
                device_brand: "".to_string(),
                device_model: device_model_name.clone(),
                summary: "".to_string(),
                validation_errors: vec![e.to_string()],
            }));
        }
    }
}

pub async fn get_modbus_tcp_tag_registers(
    State(state): State<AppState>,
    Query(params): Query<ModbusTcpTagQuery>,
) -> Result<Json<ApiResponse<Vec<ModbusTcpTagRegister>>>, StatusCode> {
    // Debug logging
    eprintln!("API DEBUG: Received query params: {:?}", params);
    
    let result = match (params.model_id, params.device_brand, params.device_model) {
        // Prefer model_id if provided (most accurate, no duplicates)
        (Some(model_id), _, _) => {
            eprintln!("API DEBUG: Using model_id filter: {}", model_id);
            state.database.get_modbus_tcp_tag_registers_by_model_id(&model_id).await
        }
        // Fallback to legacy device_brand + device_model
        (None, Some(brand), Some(model)) => {
            eprintln!("API DEBUG: Using brand + model filter: {} + {}", brand, model);
            state.database.get_modbus_tcp_tag_registers_by_device(&brand, &model).await
        }
        // Fallback to legacy device_model only
        (None, None, Some(model)) => {
            eprintln!("API DEBUG: Using model only filter: {}", model);
            state.database.get_modbus_tcp_tag_registers_by_model(&model).await
        }
        // Return all if no specific filters
        _ => {
            eprintln!("API DEBUG: No filters provided - returning all records");
            state.database.get_all_modbus_tcp_tag_registers().await
        }
    };

    match result {
        Ok(tag_registers) => {
            eprintln!("API DEBUG: Returning {} records", tag_registers.len());
            Ok(Json(ApiResponse::success(tag_registers)))
        }
        Err(e) => {
            error!("Database error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn debug_devices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    match state.database.get_devices().await {
        Ok(devices) => {
            let device_ids: Vec<String> = devices.iter().map(|d| d.id.clone()).collect();
            info!("DEBUG: All device IDs in database: {:?}", device_ids);
            Ok(Json(ApiResponse::success(device_ids)))
        }
        Err(e) => {
            error!("Failed to get devices for debugging: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ThingsBoard API endpoints
#[derive(Deserialize)]
pub struct EntityGroupQuery {
    pub group_type: Option<String>,
}

pub async fn get_thingsboard_entity_groups(
    Query(params): Query<EntityGroupQuery>,
) -> Result<Json<ApiResponse<Vec<crate::tb_rust_client::EntityGroup>>>, StatusCode> {
    use crate::tb_rust_client::ThingsBoardClient;
    
    // Get group type from query parameters, default to "DEVICE" if not specified
    let group_type = params.group_type.unwrap_or_else(|| "DEVICE".to_string());
    
    info!("Fetching ThingsBoard entity groups of type: {}", group_type);
    
    // TODO: These credentials should be configurable via environment variables or config file
    let base_url = "https://monitoring.avaasia.co"; // Default ThingsBoard URL
    let username = "jaydenyong28@gmail.com";
    let password = "lalala88";
    
    let mut client = ThingsBoardClient::new(base_url);
    
    match client.login(username, password).await {
        Ok(_) => {
            info!("Successfully logged in to ThingsBoard");
            
            match client.get_all_entity_groups(&group_type).await {
                Ok(entity_groups) => {
                    info!("Successfully fetched {} entity groups", entity_groups.len());
                    Ok(Json(ApiResponse::success(entity_groups)))
                }
                Err(e) => {
                    error!("Failed to fetch entity groups: {}", e);
                    Ok(Json(ApiResponse::error(format!("Failed to fetch entity groups: {}", e))))
                }
            }
        }
        Err(e) => {
            error!("Failed to login to ThingsBoard: {}", e);
            Ok(Json(ApiResponse::error(format!("Failed to login to ThingsBoard: {}", e))))
        }
    }
}


// create devices on thingsboard for selected device group
#[derive(Deserialize)]
pub struct SyncDevicesRequest {
    pub entity_group_id: String,
}

#[derive(Serialize)]
pub struct SyncDevicesResponse {
    pub total_devices: usize,
    pub created_count: usize,
    pub failed_count: usize,
    pub failed_devices: Vec<FailedDevice>,
    pub updated_device_ids: Vec<DeviceIdUpdate>,
    pub update_failed_count: usize,
}

#[derive(Serialize)]
pub struct DeviceIdUpdate {
    pub local_id: String,
    pub thingsboard_id: String,
    pub device_name: String,
    pub device_type: String, // Add AVA type information
}

#[derive(Serialize)]
pub struct FailedDevice {
    pub device_name: String,
    pub error: String,
}

/// Sync all local devices to ThingsBoard entity group
pub async fn sync_devices_to_thingsboard(
    State(state): State<AppState>,
    Json(request): Json<SyncDevicesRequest>,
) -> Result<Json<ApiResponse<SyncDevicesResponse>>, StatusCode> {
    use crate::tb_rust_client::{ThingsBoardClient, to_thingsboard_device, to_thingsboard_device_with_type};
    
    info!("Starting sync of local devices to ThingsBoard entity group: {}", request.entity_group_id);
    
    // Get only unsynced devices from local database (those without tb_device_id)
    let devices = match state.database.get_unsynced_devices().await {
        Ok(devices) => devices,
        Err(e) => {
            error!("Failed to get unsynced devices from database: {}", e);
            return Ok(Json(ApiResponse::error(format!("Failed to get unsynced devices from database: {}", e))));
        }
    };
    
    if devices.is_empty() {
        warn!("No unsynced devices found in local database");
        return Ok(Json(ApiResponse::success(SyncDevicesResponse {
            total_devices: 0,
            created_count: 0,
            failed_count: 0,
            failed_devices: vec![],
            updated_device_ids: vec![],
            update_failed_count: 0,
        })));
    }
    
    info!("Found {} unsynced devices in local database", devices.len());
    
    // Connect to ThingsBoard
    let base_url = "https://monitoring.avaasia.co".to_string();
    let mut tb_client = ThingsBoardClient::new(&base_url);
    
    match tb_client.login("jaydenyong28@gmail.com", "lalala88").await {
        Ok(()) => {
            info!("Successfully authenticated with ThingsBoard");
            
            // Get entity group information to extract the name
            let entity_groups = match tb_client.get_all_entity_groups("DEVICE").await {
                Ok(groups) => groups,
                Err(e) => {
                    error!("Failed to get entity groups: {}", e);
                    return Ok(Json(ApiResponse::error(format!("Failed to get entity groups: {}", e))));
                }
            };
            
            let entity_group_name = entity_groups
                .iter()
                .find(|group| group.id.id == request.entity_group_id)
                .map(|group| group.name.clone())
                .unwrap_or_else(|| "Unknown Group".to_string());
            
            info!("Target entity group: {} ({})", entity_group_name, request.entity_group_id);
            
            // Get existing devices in the ThingsBoard group to determine current device indices
            let existing_devices = match tb_client.get_all_group_devices(&request.entity_group_id, 50).await {
                Ok(devices) => {
                    info!("Found {} existing devices in ThingsBoard group", devices.len());
                    devices
                }
                Err(e) => {
                    warn!("Failed to get existing devices from ThingsBoard group: {}", e);
                    Vec::new() // Continue with empty list if we can't fetch existing devices
                }
            };
            
            // Initialize device type counters based on existing devices
            let mut device_type_counters: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            
            // Parse existing device names to find the highest index for each device type
            for existing_device in &existing_devices {
                // Extract device type and index from existing device name
                // Expected format: "PREFIX-T##" where T is type abbreviation and ## is index
                if let Some((device_type, index)) = parse_device_name_for_type_and_index(&existing_device.name, &entity_group_name) {
                    let current_max = device_type_counters.entry(device_type.clone()).or_insert(0);
                    if index > *current_max {
                        *current_max = index;
                        info!("Updated max index for device type '{}' to {} based on existing device '{}'", 
                              device_type, index, existing_device.name);
                    }
                }
            }
            
            info!("Device type counters initialized: {:?}", device_type_counters);
            
            let mut created_count = 0;
            let mut failed_count = 0;
            let mut failed_devices = Vec::new();
            let mut device_id_mappings = Vec::new(); // Store mappings for batch update
            
            // Process each device
            for (index, device) in devices.iter().enumerate() {
                info!("Processing device {} of {}: {}", index + 1, devices.len(), device.name);
                
                // Get the actual device type from database for proper indexing
                let device_type = match state.database.get_device_ava_type(&device.id).await {
                    Ok(Some(ava_type)) => ava_type,
                    Ok(None) => {
                        warn!("No AVA type found for device {}, defaulting to Inverter", device.id);
                        "Inverter".to_string()
                    }
                    Err(e) => {
                        warn!("Failed to get AVA type for device {}: {}, defaulting to Inverter", device.id, e);
                        "Inverter".to_string()
                    }
                };
                
                // Increment counter for this device type
                let device_index = device_type_counters.entry(device_type.clone()).or_insert(0);
                *device_index += 1;
                
                // Convert local device to ThingsBoard format with proper device type lookup
                let create_request = match to_thingsboard_device_with_type(device, &entity_group_name, *device_index, &state.database).await {
                    Ok(request) => request,
                    Err(e) => {
                        warn!("Failed to create ThingsBoard device request for {}: {}, using fallback", device.name, e);
                        to_thingsboard_device(device, &entity_group_name, *device_index)
                    }
                };
                
                // Attempt to create device in ThingsBoard
                match tb_client.create_device(&create_request, &request.entity_group_id, None).await {
                    Ok(created_device) => {
                        created_count += 1;
                        let tb_device_id = created_device.id.as_ref().map(|id| &id.id).unwrap_or(&String::from("Unknown")).clone();
                        
                        info!("Successfully created device: {} [{}] (TB ID: {})", 
                              create_request.name, create_request.device_type, tb_device_id);
                        
                        // Step 2.5: Update device attributes for Inverter and Meter devices
                        if tb_device_id != "Unknown" && (create_request.device_type == "Inverter" || create_request.device_type == "Meter" || create_request.device_type == "PowerMeter") {
                            info!("Updating attributes for {} device: {}", create_request.device_type, create_request.name);
                            
                            // Use the ThingsBoard device name from created_device
                            let tb_device_name = &created_device.name;
                            
                            match tb_client.build_device_attributes(device, tb_device_name, &create_request.device_type, &entity_group_name, &state.database).await {
                                Ok(attributes) => {
                                    match tb_client.update_device_attributes(&tb_device_id, attributes).await {
                                        Ok(_) => {
                                            info!("✅ Successfully updated attributes for device: {}", create_request.name);
                                        }
                                        Err(e) => {
                                            warn!("⚠️ Failed to update attributes for device {}: {}", create_request.name, e);
                                            // Don't fail the sync, just log the warning
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("⚠️ Failed to build attributes for device {}: {}", create_request.name, e);
                                }
                            }
                        }
                        
                        // Create hierarchical devices (MPPT and String devices) only for Inverters
                        if tb_device_id != "Unknown" && create_request.device_type == "Inverter" {
                            match tb_client.sync_device_hierarchy_to_thingsboard(
                                device, 
                                &request.entity_group_id,
                                &entity_group_name,
                                &state.database,
                                *device_index  // Pass the correct inverter index
                            ).await {
                                Ok(hierarchy_devices) => {
                                    info!("Successfully created {} hierarchical devices for inverter {}", 
                                          hierarchy_devices.len(), create_request.name);
                                }
                                Err(e) => {
                                    warn!("Failed to create hierarchical devices for inverter {}: {}", 
                                          create_request.name, e);
                                    // Continue with the main process - don't fail the entire sync
                                }
                            }
                        } else if tb_device_id != "Unknown" {
                            info!("Device {} is not an Inverter (type: {}), skipping hierarchical device creation", 
                                  create_request.name, create_request.device_type);
                        }
                        
                        // Store mapping for later batch update
                        if tb_device_id != "Unknown" {
                            device_id_mappings.push((device.id.clone(), tb_device_id, create_request.name.clone(), create_request.device_type.clone()));
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        let error_msg = e.to_string();
                        error!("Failed to create device {}: {}", create_request.name, error_msg);
                        failed_devices.push(FailedDevice {
                            device_name: create_request.name.clone(),
                            error: error_msg,
                        });
                    }
                }
                
                // Add a small delay to avoid rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            // Step 2: Update local database with ThingsBoard device IDs
            let mut updated_device_ids = Vec::new();
            let mut update_failed_count = 0;
            
            if !device_id_mappings.is_empty() {
                info!("Updating {} local devices with ThingsBoard device IDs and group ID...", device_id_mappings.len());
                
                // Prepare mappings for batch update (local_id, thingsboard_id, group_id)
                let id_mappings: Vec<(String, String, String)> = device_id_mappings
                    .iter()
                    .map(|(local_id, tb_id, _device_name, _device_type)| (local_id.clone(), tb_id.clone(), request.entity_group_id.clone()))
                    .collect();
                
                // Perform batch update using the new method
                match state.database.batch_update_devices_thingsboard_ids(&id_mappings).await {
                    Ok(successful_updates) => {
                        info!("Successfully updated {} devices with ThingsBoard IDs and group ID {}", successful_updates.len(), request.entity_group_id);
                        
                        // Create response data for successful updates
                        for (local_id, tb_id, device_name, device_type) in device_id_mappings {
                            if successful_updates.iter().any(|update| update.contains(&format!("{} -> {} (group: {})", local_id, tb_id, request.entity_group_id))) {
                                updated_device_ids.push(DeviceIdUpdate {
                                    local_id,
                                    thingsboard_id: tb_id,
                                    device_name,
                                    device_type,
                                });
                            } else {
                                update_failed_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to update devices with ThingsBoard IDs and group ID in local database: {}", e);
                        update_failed_count = device_id_mappings.len();
                        // Note: Devices were still created in ThingsBoard, just local records weren't updated
                    }
                }
            }
            
            let response = SyncDevicesResponse {
                total_devices: devices.len(),
                created_count,
                failed_count,
                failed_devices,
                updated_device_ids,
                update_failed_count,
            };
            
            info!("Sync completed. Total: {}, Created: {}, Failed: {}, ID Updates: {}, Update Failures: {}", 
                  response.total_devices, response.created_count, response.failed_count,
                  response.updated_device_ids.len(), response.update_failed_count);
            
            // Update plant sync timestamp after successful sync
            if let Err(e) = state.database.update_plant_sync_timestamp(&entity_group_name, &request.entity_group_id).await {
                warn!("Failed to update plant sync timestamp: {}", e);
                // Don't fail the entire sync operation if timestamp update fails
            }
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to login to ThingsBoard: {}", e);
            Ok(Json(ApiResponse::error(format!("Failed to login to ThingsBoard: {}", e))))
        }
    }
}

// Generate device catalog request and response structures
#[derive(Deserialize)]
pub struct GenerateDeviceCatalogRequest {
    pub entity_group_id: String,
    pub output_dir: String,
}

#[derive(Serialize)]
pub struct GenerateDeviceCatalogResponse {
    pub message: String,
    pub file_path: String,
}

/// Generate a CSV device catalog for the specified entity group
pub async fn generate_device_catalog(
    State(state): State<AppState>,
    Json(request): Json<GenerateDeviceCatalogRequest>,
) -> Result<Json<ApiResponse<GenerateDeviceCatalogResponse>>, StatusCode> {
    use crate::tb_rust_client::ThingsBoardClient;
    
    info!("Generating device catalog for entity group: {}", request.entity_group_id);
    
    // Connect to ThingsBoard
    let base_url = "https://monitoring.avaasia.co".to_string();
    let mut tb_client = ThingsBoardClient::new(&base_url);
    
    match tb_client.login("jaydenyong28@gmail.com", "lalala88").await {
        Ok(()) => {
            info!("Successfully authenticated with ThingsBoard for catalog generation");
            
            // Generate the device catalog CSV with database access
            match tb_client.generate_detailed_device_catalog_csv(&request.entity_group_id, &request.output_dir, &state.database).await {
                Ok(result) => {
                    let response = GenerateDeviceCatalogResponse {
                        message: result.clone(),
                        file_path: format!("{}/[entity-group-name]-device-catalog.csv", request.output_dir),
                    };
                    
                    info!("Device catalog generated successfully");
                    Ok(Json(ApiResponse::success(response)))
                }
                Err(e) => {
                    error!("Failed to generate device catalog: {}", e);
                    Ok(Json(ApiResponse::error(format!("Failed to generate device catalog: {}", e))))
                }
            }
        }
        Err(e) => {
            error!("Failed to login to ThingsBoard for catalog generation: {}", e);
            Ok(Json(ApiResponse::error(format!("Failed to login to ThingsBoard: {}", e))))
        }
    }
}

// File Management API endpoints

#[derive(Serialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub modified: String,
    pub download_url: String,
}

/// List all CSV files in the catalogs directory
pub async fn list_catalog_files() -> Result<Json<ApiResponse<Vec<FileInfo>>>, StatusCode> {
    use std::fs;
    use chrono::{DateTime, Utc};
    
    let catalog_dir = "catalogs";
    
    match fs::read_dir(catalog_dir) {
        Ok(entries) => {
            let mut files = Vec::new();
            
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            // Only include CSV files
                            if filename.ends_with(".csv") {
                                if let Ok(metadata) = entry.metadata() {
                                    let modified = metadata.modified()
                                        .map(|time| {
                                            let datetime: DateTime<Utc> = time.into();
                                            datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
                                        })
                                        .unwrap_or_else(|_| "Unknown".to_string());
                                    
                                    files.push(FileInfo {
                                        name: filename.to_string(),
                                        size: metadata.len(),
                                        modified,
                                        download_url: format!("/api/files/catalogs/{}", filename),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            // Sort by modification time (newest first)
            files.sort_by(|a, b| b.modified.cmp(&a.modified));
            
            Ok(Json(ApiResponse::success(files)))
        }
        Err(e) => {
            error!("Failed to read catalog directory: {}", e);
            Ok(Json(ApiResponse::error(format!("Failed to read catalog directory: {}", e))))
        }
    }
}

/// Download a specific CSV file
pub async fn download_catalog_file(Path(filename): Path<String>) -> Result<impl axum::response::IntoResponse, StatusCode> {
    use axum::response::Response;
    use axum::body::Body;
    use axum::http::{header, HeaderMap};
    use std::fs;
    use std::path::Path as StdPath;
    
    // Security: Only allow CSV files and prevent directory traversal
    if !filename.ends_with(".csv") || filename.contains("..") || filename.contains("/") || filename.contains("\\") {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let file_path = StdPath::new("catalogs").join(&filename);
    
    match fs::read(&file_path) {
        Ok(contents) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                "text/csv; charset=utf-8".parse().unwrap(),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename).parse().unwrap(),
            );
            
            Ok(Response::builder()
                .status(200)
                .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
                .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
                .body(Body::from(contents))
                .unwrap())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Delete a specific CSV file
pub async fn delete_catalog_file(Path(filename): Path<String>) -> Result<Json<ApiResponse<String>>, StatusCode> {
    use std::fs;
    use std::path::Path as StdPath;
    
    // Security: Only allow CSV files and prevent directory traversal
    if !filename.ends_with(".csv") || filename.contains("..") || filename.contains("/") || filename.contains("\\") {
        return Ok(Json(ApiResponse::error("Invalid filename or file type".to_string())));
    }
    
    let file_path = StdPath::new("catalogs").join(&filename);
    
    match fs::remove_file(&file_path) {
        Ok(_) => {
            info!("Deleted catalog file: {}", filename);
            Ok(Json(ApiResponse::success(format!("File '{}' deleted successfully", filename))))
        }
        Err(e) => {
            error!("Failed to delete catalog file {}: {}", filename, e);
            Ok(Json(ApiResponse::error(format!("Failed to delete file: {}", e))))
        }
    }
}

// Authentication endpoints

/// Login endpoint - validates credentials and creates session
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, StatusCode> {
    // Verify user credentials
    match state.database.verify_user(&request.username, &request.password).await {
        Ok(Some(user)) => {
            // Generate session token
            let session_token = Uuid::new_v4().to_string();
            let expires_at = Utc::now() + Duration::hours(24); // 24-hour sessions
            
            // Create session in database
            match state.database.create_session(user.id.unwrap(), &session_token, expires_at).await {
                Ok(_) => {
                    info!("User '{}' logged in successfully", user.username);
                    
                    let response = LoginResponse {
                        session_token,
                        user: UserInfo {
                            id: user.id.unwrap(),
                            username: user.username,
                            role: user.role,
                        },
                        expires_at,
                    };
                    
                    Ok(Json(ApiResponse::success(response)))
                }
                Err(e) => {
                    error!("Failed to create session: {}", e);
                    Ok(Json(ApiResponse::error("Failed to create session".to_string())))
                }
            }
        }
        Ok(None) => {
            warn!("Invalid login attempt for username: {}", request.username);
            Ok(Json(ApiResponse::error("Invalid username or password".to_string())))
        }
        Err(e) => {
            error!("Database error during login: {}", e);
            Ok(Json(ApiResponse::error("Internal server error".to_string())))
        }
    }
}

/// Logout endpoint - revokes session
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                match state.database.revoke_session(token).await {
                    Ok(true) => {
                        info!("Session revoked successfully");
                        Ok(Json(ApiResponse::success("Logged out successfully".to_string())))
                    }
                    Ok(false) => {
                        Ok(Json(ApiResponse::error("Session not found".to_string())))
                    }
                    Err(e) => {
                        error!("Failed to revoke session: {}", e);
                        Ok(Json(ApiResponse::error("Failed to logout".to_string())))
                    }
                }
            } else {
                Ok(Json(ApiResponse::error("Invalid authorization header format".to_string())))
            }
        } else {
            Ok(Json(ApiResponse::error("Invalid authorization header".to_string())))
        }
    } else {
        Ok(Json(ApiResponse::error("No authorization header provided".to_string())))
    }
}

/// Verify session endpoint - checks if session is valid
pub async fn verify_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<UserInfo>>, StatusCode> {
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                match state.database.verify_session(token).await {
                    Ok(Some(user)) => {
                        Ok(Json(ApiResponse::success(UserInfo {
                            id: user.id.unwrap(),
                            username: user.username,
                            role: user.role,
                        })))
                    }
                    Ok(None) => {
                        Ok(Json(ApiResponse::error("Invalid or expired session".to_string())))
                    }
                    Err(e) => {
                        error!("Database error during session verification: {}", e);
                        Ok(Json(ApiResponse::error("Internal server error".to_string())))
                    }
                }
            } else {
                Ok(Json(ApiResponse::error("Invalid authorization header format".to_string())))
            }
        } else {
            Ok(Json(ApiResponse::error("Invalid authorization header".to_string())))
        }
    } else {
        Ok(Json(ApiResponse::error("No authorization header provided".to_string())))
    }
}

/// Get plant configuration
pub async fn get_plant_config(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<PlantConfiguration>>, StatusCode> {
    match state.database.get_plant_configuration().await {
        Ok(Some(config)) => Ok(Json(ApiResponse::success(config))),
        Ok(None) => Ok(Json(ApiResponse::error("No plant configuration found".to_string()))),
        Err(e) => {
            error!("Failed to get plant configuration: {}", e);
            Ok(Json(ApiResponse::error("Failed to get plant configuration".to_string())))
        }
    }
}

/// Update plant configuration
pub async fn update_plant_config(
    State(state): State<AppState>,
    Json(request): Json<PlantConfigRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.database.update_plant_configuration(&request.plant_name, request.thingsboard_entity_group_id.as_deref()).await {
        Ok(_) => {
            info!("Plant configuration updated: {}", request.plant_name);
            Ok(Json(ApiResponse::success("Plant configuration updated successfully".to_string())))
        }
        Err(e) => {
            error!("Failed to update plant configuration: {}", e);
            Ok(Json(ApiResponse::error("Failed to update plant configuration".to_string())))
        }
    }
}

/// Get all plant sync information (for admin view)
pub async fn get_all_plant_sync_info(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PlantConfiguration>>>, StatusCode> {
    match state.database.get_all_plant_sync_info().await {
        Ok(plants) => Ok(Json(ApiResponse::success(plants))),
        Err(e) => {
            error!("Failed to get plant sync info: {}", e);
            Ok(Json(ApiResponse::error("Failed to get plant sync info".to_string())))
        }
    }
}

/// Get devices filtered by plant configuration
pub async fn get_devices_filtered(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DeviceWithTags>>>, StatusCode> {
    // Get plant configuration to determine filtering
    let plant_config = match state.database.get_plant_configuration().await {
        Ok(Some(config)) => config,
        Ok(None) => {
            // No plant config exists - return error for installers
            return Ok(Json(ApiResponse::error("Plant configuration not found. Please contact your administrator.".to_string())));
        }
        Err(e) => {
            error!("Failed to get plant configuration: {}", e);
            return Ok(Json(ApiResponse::error("Failed to get plant configuration".to_string())));
        }
    };

    // Check if plant is properly configured (not default)
    if plant_config.plant_name == "Default Plant" || plant_config.thingsboard_entity_group_id.is_none() {
        return Ok(Json(ApiResponse::error("Plant has not been configured yet. Please contact your administrator to configure the plant settings.".to_string())));
    }

    // If ThingsBoard group ID is configured, filter devices by that group
    if let Some(tb_group_id) = &plant_config.thingsboard_entity_group_id {
        // Get devices that belong to this specific ThingsBoard group
        let devices = match state.database.get_devices_by_group_id(tb_group_id).await {
            Ok(devices) => devices,
            Err(e) => {
                error!("Failed to get devices for plant group {}: {}", tb_group_id, e);
                return Ok(Json(ApiResponse::error("Failed to get devices for plant group".to_string())));
            }
        };

        let mut devices_with_tags = Vec::new();
        for device in devices {
            let tags = match state.database.get_device_tags(&device.id).await {
                Ok(tags) => tags,
                Err(e) => {
                    error!("Failed to get tags for device {}: {}", device.id, e);
                    continue; // Skip this device but continue with others
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
    } else {
        // No filtering configured, return error
        Ok(Json(ApiResponse::error("Plant ThingsBoard group not configured. Please contact your administrator.".to_string())))
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication for login, health check, static files, and HTML pages
    let path = request.uri().path();
    if path == "/api/login" 
        || path == "/api/health" 
        || path.starts_with("/static/") 
        || path.starts_with("/web/")
        || path == "/" 
        || path == "/favicon.ico"
        || path == "/manifest.json"
        || !path.starts_with("/api/") // Allow non-API routes (React app routes)
    {
        return Ok(next.run(request).await);
    }
    
    // Check for authorization header
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                match state.database.verify_session(token).await {
                    Ok(Some(user)) => {
                        // Add user info to request extensions for use in handlers
                        request.extensions_mut().insert(user);
                        return Ok(next.run(request).await);
                    }
                    Ok(None) => {
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                    Err(_) => {
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
        }
    }
    
    Err(StatusCode::UNAUTHORIZED)
}
