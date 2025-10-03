// Cargo.toml dependencies needed:
// [dependencies]
// reqwest = { version = "1.0", features = ["json"] }
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// tokio = { version = "1.0", features = ["full"] }
// uuid = { version = "1.0", features = ["v4"] }

use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use csv::Writer;
use crate::database::{DeviceInstance, Database, DeviceTag}; // Import for hierarchical device analysis
use tracing::warn;

#[derive(Debug)]
pub enum TbError {
    Http(ReqwestError),
    Auth(String),
    Api(String),
}

impl std::fmt::Display for TbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TbError::Http(err) => write!(f, "HTTP error: {}", err),
            TbError::Auth(msg) => write!(f, "Authentication error: {}", msg),
            TbError::Api(msg) => write!(f, "API error: {}", msg),
        }
    }
}

impl std::error::Error for TbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TbError::Http(err) => Some(err),
            TbError::Auth(_) => None,
            TbError::Api(_) => None,
        }
    }
}

impl From<ReqwestError> for TbError {
    fn from(err: ReqwestError) -> Self {
        TbError::Http(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: Option<DeviceId>,
    pub name: String,
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub device_type: String,
    #[serde(rename = "deviceProfileId")]
    pub device_profile_id: Option<DeviceId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceId {
    pub id: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCredentials {
    pub id: Option<DeviceCredentialsId>,
    #[serde(rename = "createdTime")]
    pub created_time: Option<i64>,
    #[serde(rename = "deviceId")]
    pub device_id: Option<DeviceCredentialsId>,
    #[serde(rename = "credentialsType")]
    pub credentials_type: Option<String>,
    #[serde(rename = "credentialsId")]
    pub credentials_id: String,
    #[serde(rename = "credentialsValue")]
    pub credentials_value: Option<String>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCredentialsId {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityId {
    pub id: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceConfiguration {
    pub configuration: HashMap<String, serde_json::Value>,
    #[serde(rename = "transportConfiguration")]
    pub transport_configuration: HashMap<String, serde_json::Value>,
}


// TB Device Creation Request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDeviceRequest {
    pub id: Option<DeviceId>,
    pub tenant_id: Option<DeviceId>,
    pub customer_id: Option<DeviceId>,
    pub owner_id: Option<DeviceId>,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub label: Option<String>,
    pub device_profile_id: Option<DeviceId>,
    pub device_data: Option<DeviceData>,
    pub firmware_id: Option<DeviceId>,
    pub software_id: Option<DeviceId>,
    pub additional_info: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceData {
    pub id: EntityId,
    pub created_time: Option<i64>,
    pub tenant_id: Option<EntityId>,
    pub customer_id: Option<EntityId>,
    pub owner_id: Option<EntityId>,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub label: String,
    pub device_profile_id: Option<EntityId>,
    pub device_data: Option<DeviceConfiguration>,
    pub firmware_id: Option<EntityId>,
    pub software_id: Option<EntityId>,
    pub additional_info: Option<HashMap<String, serde_json::Value>>,
}


// TB get devices of entity group response
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupDevicesResponse {
    pub data: Vec<DeviceData>,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
    #[serde(rename = "totalElements")]
    pub total_elements: i32,
    pub has_next: Option<bool>,
}

// TB Entity Group structure
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityGroup {
    pub id: EntityId,
    #[serde(rename = "createdTime")]
    pub created_time: Option<i64>,
    #[serde(rename = "ownerId")]
    pub owner_id: EntityId,
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    #[serde(rename = "additionalInfo")]
    pub additional_info: Option<HashMap<String, serde_json::Value>>,
    pub configuration: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "groupAll")]
    pub group_all: bool,
    #[serde(rename = "edgeGroupAll")]
    pub edge_group_all: bool,
    #[serde(rename = "ownerIds")]
    pub owner_ids: Option<Vec<EntityId>>,
}

// TB get entity groups response
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityGroupsResponse {
    pub data: Vec<EntityGroup>,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
    #[serde(rename = "totalElements")]
    pub total_elements: i32,
    pub has_next: Option<bool>,
}

/// Converts a local DeviceInstance to a ThingsBoard CreateDeviceRequest with actual device type lookup
/// 
/// This async function maps AVA Device Logger device data to ThingsBoard format,
/// looking up the actual AVA type from the database for accurate device classification.
pub async fn to_thingsboard_device_with_type(
    device: &DeviceInstance, 
    entity_group_name: &str, 
    device_index: u32,
    database: &Database
) -> Result<CreateDeviceRequest, TbError> {
    // Get the actual device type from database
    let device_type = match database.get_device_ava_type(&device.id).await {
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

    // Generate device name based on entity group and device type
    let device_name = format!("{}-{}{:02}", 
        extract_group_prefix_static(entity_group_name),
        match device_type.as_str() {
            "Inverter" => "I",
            "PowerMeter" => "PM",
            "Meter" => "MT",
            "Weather Station" => "WS",
            "String" => "S", 
            "PlantBlock" => "P",
            _ => "D"
        },
        device_index
    );

    Ok(CreateDeviceRequest {
        id: None,
        tenant_id: None,
        customer_id: None,
        owner_id: None,
        name: device_name.clone(),
        device_type,
        label: Some(device_name.clone()),
        device_profile_id: None,
        device_data: None,
        firmware_id: None,
        software_id: None,
        additional_info: None,
    })
}

/// Converts a local DeviceInstance to a ThingsBoard CreateDeviceRequest
/// 
/// This function maps AVA Device Logger device data to ThingsBoard format,
/// preserving all relevant configuration in the additional_info field.
/// The device name follows a specific naming scheme based on the entity group.
pub fn to_thingsboard_device(device: &DeviceInstance, entity_group_name: &str, device_index: u32) -> CreateDeviceRequest {
    // Parse protocol config to determine device type
    let device_type = match serde_json::from_str::<serde_json::Value>(&device.protocol_config) {
        Ok(config) => {
            if let Some(protocol_type) = config.get("type").and_then(|v| v.as_str()) {
                match protocol_type {
                    "modbus_tcp" => "Inverter".to_string(),
                    "modbus_rtu" => "Inverter".to_string(),
                    "iec104" => "Inverter".to_string(),
                    _ => "Inverter".to_string(),
                }
            } else {
                "Default".to_string()
            }
        }
        Err(_) => "Default".to_string(),
    };

    // Generate device name based on entity group and device type
    let device_name = format!("{}-{}{:02}", 
        extract_group_prefix_static(entity_group_name),
        match device_type.as_str() {
            "Inverter" => "I",
            "PowerMeter" => "PM",
            "Meter" => "MT",
            "Weather Station" => "WS",
            "String" => "S", 
            "PlantBlock" => "P",
            _ => "D"
        },
        device_index
    );

    CreateDeviceRequest {
        id: None,
        tenant_id: None,
        customer_id: None,
        owner_id: None,
        name: device_name.clone(),
        device_type,
        label: Some(device_name.clone()),
        device_profile_id: None,
        device_data: None,
        firmware_id: None,
        software_id: None,
        additional_info: None,
    }
}

/// Static helper function to extract group prefix
fn extract_group_prefix_static(entity_group_name: &str) -> String {
    let parts: Vec<&str> = entity_group_name.split('-').collect();
    
    if parts.len() >= 3 {
        format!("{}-{}", parts[0], parts[1])
    } else if parts.len() == 2 {
        format!("{}-{}", parts[0], parts[1])
    } else {
        entity_group_name.to_string()
    }
}

// Device hierarchy structures for ThingsBoard sync
#[derive(Debug, Clone)]
pub struct InverterInfo {
    pub name: String,
    pub model: String,
    pub tags: Vec<DeviceTag>,
}

#[derive(Debug, Clone)]
pub struct MpptInfo {
    pub name: String,
    pub mppt_number: u32,
    pub parent_inverter: String,
    pub tags: Vec<DeviceTag>,
}

#[derive(Debug, Clone)]
pub struct StringInfo {
    pub name: String,
    pub mppt_number: u32,
    pub input_number: u32,
    pub parent_mppt: String,
    pub parent_inverter: String,
    pub idc_tag: Option<DeviceTag>,
    pub udc_tag: Option<DeviceTag>,
}

#[derive(Debug, Clone)]
pub struct DeviceHierarchy {
    pub inverter: InverterInfo,
    pub mppets: Vec<MpptInfo>,
    pub strings: Vec<StringInfo>,
    pub entity_group_prefix: String,
    pub inverter_index: u32,
}

#[derive(Debug, PartialEq)]
pub enum DeviceType {
    Inverter,
    Mppt(u32), // MPPT number
    String(u32, u32), // MPPT number, Input number
    Unknown,
}

pub struct ThingsBoardClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl ThingsBoardClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            token: None,
        }
    }

    /// Parse device tag description to determine device type and hierarchy
    /// 
    /// Examples:
    /// - "Inverter (SG150CX)" -> DeviceType::Inverter
    /// - "MPPT - MPPT 1 (SG150CX)" -> DeviceType::Mppt(1)
    /// - "String - MPPT 1 - Input 1 (SG150CX)" -> DeviceType::String(1, 1)
    fn parse_device_description(description: &str) -> DeviceType {
        if description.starts_with("Inverter") {
            return DeviceType::Inverter;
        }
        
        if description.starts_with("MPPT - MPPT ") {
            // Extract MPPT number from "MPPT - MPPT 1 (SG150CX)"
            if let Some(captures) = description.split(" ").nth(3) {
                if let Ok(mppt_num) = captures.parse::<u32>() {
                    return DeviceType::Mppt(mppt_num);
                }
            }
        }
        
        if description.starts_with("String - MPPT ") {
            // Extract MPPT and Input from "String - MPPT 1 - Input 1 (SG150CX)"
            let parts: Vec<&str> = description.split(" - ").collect();
            if parts.len() >= 3 {
                // parts[0] = "String"
                // parts[1] = "MPPT 1" 
                // parts[2] = "Input 1 (SG150CX)"
                
                if let Some(mppt_part) = parts.get(1) {
                    if let Some(mppt_str) = mppt_part.strip_prefix("MPPT ") {
                        if let Ok(mppt_num) = mppt_str.parse::<u32>() {
                            if let Some(input_part) = parts.get(2) {
                                if let Some(input_str) = input_part.strip_prefix("Input ") {
                                    // Extract just the number before the space/parenthesis
                                    let input_num_str = input_str.split_whitespace().next().unwrap_or("0");
                                    if let Ok(input_num) = input_num_str.parse::<u32>() {
                                        return DeviceType::String(mppt_num, input_num);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        DeviceType::Unknown
    }

    /// Extract model name from device tag description
    /// 
    /// Example: "String - MPPT 1 - Input 1 (SG150CX)" -> "SG150CX"
    fn extract_model_from_description(description: &str) -> String {
        if let Some(start) = description.rfind('(') {
            if let Some(end) = description.rfind(')') {
                if end > start {
                    return description[start + 1..end].to_string();
                }
            }
        }
        "Unknown".to_string()
    }

    /// Analyze device tags to create hierarchical device structure
    /// 
    /// This function takes all device tags from a single physical inverter device
    /// and groups them into Inverter/MPPT/String categories based on description patterns.
    pub async fn analyze_device_hierarchy(
        &self,  
        device_tags: Vec<DeviceTag>, 
        entity_group_name: &str,
        inverter_index: u32,  // Use the passed inverter index instead of extracting from device name
    ) -> Result<DeviceHierarchy, TbError> {        
        // Extract entity group prefix for naming
        let entity_group_prefix = self.extract_group_prefix(entity_group_name);
        // Use the passed inverter_index instead of extracting from device name
        
        // Extract model from first tag description
        let model = if let Some(first_tag) = device_tags.first() {
            if let Some(desc) = &first_tag.description {
                Self::extract_model_from_description(desc)
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };
        
        // Create inverter name
        let inverter_name = format!("{}-I{:02}", entity_group_prefix, inverter_index);
        
        // Group tags by type
        let mut inverter_tags = Vec::new();
        let mut mppt_groups: HashMap<u32, Vec<DeviceTag>> = HashMap::new();
        let mut string_groups: HashMap<(u32, u32), Vec<DeviceTag>> = HashMap::new();
        
        for tag in device_tags {
            if let Some(description) = &tag.description {
                match Self::parse_device_description(description) {
                    DeviceType::Inverter => {
                        inverter_tags.push(tag);
                    }
                    DeviceType::Mppt(mppt_num) => {
                        mppt_groups.entry(mppt_num).or_insert_with(Vec::new).push(tag);
                    }
                    DeviceType::String(mppt_num, input_num) => {
                        string_groups.entry((mppt_num, input_num)).or_insert_with(Vec::new).push(tag);
                    }
                    DeviceType::Unknown => {
                        println!("⚠️  Unknown device type for tag: {} - {}", tag.name, description);
                    }
                }
            }
        }
        
        // Create inverter info
        let inverter = InverterInfo {
            name: inverter_name.clone(),
            model: model.clone(),
            tags: inverter_tags,
        };
        
        // Create MPPT info
        let mut mppets = Vec::new();
        for (mppt_num, tags) in mppt_groups {
            let mppt_name = format!("{}-M{:02}", inverter_name, mppt_num);
            mppets.push(MpptInfo {
                name: mppt_name,
                mppt_number: mppt_num,
                parent_inverter: inverter_name.clone(),
                tags,
            });
        }
        mppets.sort_by_key(|m| m.mppt_number);
        
        // Create String info
        let mut strings = Vec::new();
        for ((mppt_num, input_num), tags) in string_groups {
            let mppt_name = format!("{}-M{:02}", inverter_name, mppt_num);
            let string_name = format!("{}-PV{:02}", mppt_name, (input_num - 1) % 3 + 1); // PV01, PV02, PV03 per MPPT
            
            // Find Idc and Udc tags
            let mut idc_tag = None;
            let mut udc_tag = None;
            for tag in &tags {
                if tag.name == "Idc" {
                    idc_tag = Some(tag.clone());
                } else if tag.name == "Udc" {
                    udc_tag = Some(tag.clone());
                }
            }
            
            strings.push(StringInfo {
                name: string_name,
                mppt_number: mppt_num,
                input_number: input_num,
                parent_mppt: mppt_name,
                parent_inverter: inverter_name.clone(),
                idc_tag,
                udc_tag,
            });
        }
        strings.sort_by_key(|s| (s.mppt_number, s.input_number));
        
        Ok(DeviceHierarchy {
            inverter,
            mppets,
            strings,
            entity_group_prefix,
            inverter_index,
        })
    }

    /// Create hierarchical devices in ThingsBoard based on device hierarchy analysis
    /// 
    /// This method creates devices in the proper sequence:
    /// 1. Create Inverter device
    /// 2. Create MPPT devices (with Inverter as parent context)  
    /// 3. Create String devices (with MPPT as parent context)
    pub async fn create_hierarchical_devices(
        &self,
        hierarchy: &DeviceHierarchy,
        entity_group_id: &str,
    ) -> Result<Vec<Device>, TbError> {
        let mut created_devices = Vec::new();
        
        // Step 1: Create Inverter device (or skip if exists)
        let inverter_request = CreateDeviceRequest {
            id: None,
            tenant_id: None,
            customer_id: None,
            owner_id: None,
            name: hierarchy.inverter.name.clone(),
            device_type: "Inverter".to_string(),
            label: Some(format!("{} ({})", hierarchy.inverter.name, hierarchy.inverter.model)),
            device_profile_id: None,
            device_data: None,
            firmware_id: None,
            software_id: None,
            additional_info: Some({
                let mut info = HashMap::new();
                info.insert("model".to_string(), serde_json::json!(hierarchy.inverter.model));
                info.insert("tag_count".to_string(), serde_json::json!(hierarchy.inverter.tags.len()));
                info.insert("device_type".to_string(), serde_json::json!("Inverter"));
                info
            }),
        };
        
        let inverter_device = match self.create_device(&inverter_request, entity_group_id, None).await {
            Ok(device) => device,
            Err(TbError::Api(error_msg)) if error_msg.contains("already exists") => {
                // Create a placeholder device for the hierarchy (we don't have the actual device ID)
                Device {
                    id: Some(DeviceId {
                        id: "existing-device".to_string(),
                        entity_type: "DEVICE".to_string(),
                    }),
                    name: hierarchy.inverter.name.clone(),
                    label: Some(format!("{} ({})", hierarchy.inverter.name, hierarchy.inverter.model)),
                    device_type: "Inverter".to_string(),
                    device_profile_id: None,
                }
            }
            Err(e) => return Err(e),
        };
        created_devices.push(inverter_device);
        
        // Step 2: Create MPPT devices
        for mppt_info in &hierarchy.mppets {
            let mppt_request = CreateDeviceRequest {
                id: None,
                tenant_id: None,
                customer_id: None,
                owner_id: None,
                name: mppt_info.name.clone(),
                device_type: "MPPT".to_string(),
                label: Some(format!("MPPT {} - {}", mppt_info.mppt_number, hierarchy.inverter.model)),
                device_profile_id: None,
                device_data: None,
                firmware_id: None,
                software_id: None,
                additional_info: Some({
                    let mut info = HashMap::new();
                    info.insert("mppt_number".to_string(), serde_json::json!(mppt_info.mppt_number));
                    info.insert("parent_inverter".to_string(), serde_json::json!(mppt_info.parent_inverter));
                    info.insert("tag_count".to_string(), serde_json::json!(mppt_info.tags.len()));
                    info.insert("device_type".to_string(), serde_json::json!("MPPT"));
                    info
                }),
            };
            
            let mppt_device = match self.create_device(&mppt_request, entity_group_id, None).await {
                Ok(device) => device,
                Err(TbError::Api(error_msg)) if error_msg.contains("already exists") => {
                    // Create placeholder for tracking
                    Device {
                        id: Some(DeviceId {
                            id: "existing-mppt".to_string(),
                            entity_type: "DEVICE".to_string(),
                        }),
                        name: mppt_info.name.clone(),
                        label: Some(format!("MPPT {} - {}", mppt_info.mppt_number, hierarchy.inverter.model)),
                        device_type: "MPPT".to_string(),
                        device_profile_id: None,
                    }
                }
                Err(_) => continue, // Skip this MPPT but continue with others
            };
            created_devices.push(mppt_device);
        }
        
        // Step 3: Create String devices  
        for string_info in &hierarchy.strings {
            let mut additional_info = HashMap::new();
            additional_info.insert("mppt_number".to_string(), serde_json::json!(string_info.mppt_number));
            additional_info.insert("input_number".to_string(), serde_json::json!(string_info.input_number));
            additional_info.insert("parent_mppt".to_string(), serde_json::json!(string_info.parent_mppt));
            additional_info.insert("parent_inverter".to_string(), serde_json::json!(string_info.parent_inverter));
            additional_info.insert("device_type".to_string(), serde_json::json!("String"));
            
            // Add Idc and Udc tag information
            if let Some(idc_tag) = &string_info.idc_tag {
                additional_info.insert("idc_address".to_string(), serde_json::json!(idc_tag.address));
                additional_info.insert("idc_scaling".to_string(), serde_json::json!(idc_tag.scaling_multiplier));
            }
            if let Some(udc_tag) = &string_info.udc_tag {
                additional_info.insert("udc_address".to_string(), serde_json::json!(udc_tag.address));
                additional_info.insert("udc_scaling".to_string(), serde_json::json!(udc_tag.scaling_multiplier));
            }
            
            let string_request = CreateDeviceRequest {
                id: None,
                tenant_id: None,
                customer_id: None,
                owner_id: None,
                name: string_info.name.clone(),
                device_type: "String".to_string(),
                label: Some(format!("String MPPT {} Input {} - {}", 
                    string_info.mppt_number, string_info.input_number, hierarchy.inverter.model)),
                device_profile_id: None,
                device_data: None,
                firmware_id: None,
                software_id: None,
                additional_info: Some(additional_info),
            };
            
            let string_device = match self.create_device(&string_request, entity_group_id, None).await {
                Ok(device) => device,
                Err(TbError::Api(error_msg)) if error_msg.contains("already exists") => {
                    // Create placeholder for tracking
                    Device {
                        id: Some(DeviceId {
                            id: "existing-string".to_string(),
                            entity_type: "DEVICE".to_string(),
                        }),
                        name: string_info.name.clone(),
                        label: Some(format!("String MPPT {} Input {} - {}", 
                            string_info.mppt_number, string_info.input_number, hierarchy.inverter.model)),
                        device_type: "String".to_string(),
                        device_profile_id: None,
                    }
                }
                Err(_) => continue, // Skip this String but continue with others
            };
            created_devices.push(string_device);
        }
        
        Ok(created_devices)
    }

    /// Create only MPPT and String devices (skip inverter creation)
    /// 
    /// This method is called when the main inverter device already exists
    /// and we only need to create the hierarchical MPPT and String devices.
    pub async fn create_mppt_and_string_devices(
        &self,
        hierarchy: &DeviceHierarchy,
        entity_group_id: &str,
    ) -> Result<Vec<Device>, TbError> {
        let mut created_devices = Vec::new();
        
        // Step 1: Create MPPT devices
        for mppt_info in &hierarchy.mppets {
            let mppt_request = CreateDeviceRequest {
                id: None,
                tenant_id: None,
                customer_id: None,
                owner_id: None,
                name: mppt_info.name.clone(),
                device_type: "Mppt".to_string(), // Fixed: "Mppt" instead of "MPPT"
                label: Some(mppt_info.name.clone()), // Fixed: Use device name as label
                device_profile_id: None,
                device_data: None,
                firmware_id: None,
                software_id: None,
                additional_info: Some({
                    let mut info = HashMap::new();
                    info.insert("mppt_number".to_string(), serde_json::json!(mppt_info.mppt_number));
                    info.insert("parent_inverter".to_string(), serde_json::json!(mppt_info.parent_inverter));
                    info.insert("tag_count".to_string(), serde_json::json!(mppt_info.tags.len()));
                    info.insert("device_type".to_string(), serde_json::json!("Mppt"));
                    info
                }),
            };
            
            let mppt_device = match self.create_device(&mppt_request, entity_group_id, None).await {
                Ok(device) => device,
                Err(TbError::Api(error_msg)) if error_msg.contains("already exists") => {
                    // Create placeholder for tracking
                    Device {
                        id: Some(DeviceId {
                            id: "existing-mppt".to_string(),
                            entity_type: "DEVICE".to_string(),
                        }),
                        name: mppt_info.name.clone(),
                        label: Some(mppt_info.name.clone()),
                        device_type: "Mppt".to_string(),
                        device_profile_id: None,
                    }
                }
                Err(_) => continue, // Skip this MPPT but continue with others
            };
            created_devices.push(mppt_device);
        }
        
        // Step 2: Create String devices with continuous PV indexing
        let mut global_pv_index = 1; // Start from 1 and continue across all MPPTs
        for string_info in &hierarchy.strings {
            // Use global PV index instead of resetting per MPPT
            let string_name = format!("{}-PV{:02}", 
                format!("{}-M{:02}", hierarchy.inverter.name, string_info.mppt_number), 
                global_pv_index
            );
            
            let mut additional_info = HashMap::new();
            additional_info.insert("mppt_number".to_string(), serde_json::json!(string_info.mppt_number));
            additional_info.insert("input_number".to_string(), serde_json::json!(string_info.input_number));
            additional_info.insert("global_pv_index".to_string(), serde_json::json!(global_pv_index));
            additional_info.insert("parent_mppt".to_string(), serde_json::json!(string_info.parent_mppt));
            additional_info.insert("parent_inverter".to_string(), serde_json::json!(string_info.parent_inverter));
            additional_info.insert("device_type".to_string(), serde_json::json!("String"));
            
            // Add Idc and Udc tag information
            if let Some(idc_tag) = &string_info.idc_tag {
                additional_info.insert("idc_address".to_string(), serde_json::json!(idc_tag.address));
                additional_info.insert("idc_scaling".to_string(), serde_json::json!(idc_tag.scaling_multiplier));
            }
            if let Some(udc_tag) = &string_info.udc_tag {
                additional_info.insert("udc_address".to_string(), serde_json::json!(udc_tag.address));
                additional_info.insert("udc_scaling".to_string(), serde_json::json!(udc_tag.scaling_multiplier));
            }
            
            let string_request = CreateDeviceRequest {
                id: None,
                tenant_id: None,
                customer_id: None,
                owner_id: None,
                name: string_name.clone(),
                device_type: "String".to_string(),
                label: Some(string_name.clone()), // Fixed: Use device name as label
                device_profile_id: None,
                device_data: None,
                firmware_id: None,
                software_id: None,
                additional_info: Some(additional_info),
            };
            
            let string_device = match self.create_device(&string_request, entity_group_id, None).await {
                Ok(device) => device,
                Err(TbError::Api(error_msg)) if error_msg.contains("already exists") => {
                    // Create placeholder for tracking
                    Device {
                        id: Some(DeviceId {
                            id: "existing-string".to_string(),
                            entity_type: "DEVICE".to_string(),
                        }),
                        name: string_name.clone(),
                        label: Some(string_name.clone()),
                        device_type: "String".to_string(),
                        device_profile_id: None,
                    }
                }
                Err(_) => continue, // Skip this String but continue with others
            };
            created_devices.push(string_device);
            global_pv_index += 1; // Increment for next string
        }
        
        Ok(created_devices)
    }

    /// Complete workflow: analyze device hierarchy and create hierarchical devices in ThingsBoard
    /// 
    /// This method combines hierarchy analysis and device creation for a single local device instance.
    /// It retrieves device tags, analyzes the hierarchy, and creates all devices in ThingsBoard.
    /// Set skip_inverter_creation to true when the main inverter device is already created.
    pub async fn sync_device_hierarchy_to_thingsboard(
        &self,
        device: &DeviceInstance,
        entity_group_id: &str,
        entity_group_name: &str,
        database: &Database,
        inverter_index: u32,  // Pass the correct inverter index explicitly
    ) -> Result<Vec<Device>, TbError> {        
        // Step 1: Get device tags from database
        let device_tags = database.get_device_tags(&device.id).await
            .map_err(|e| TbError::Api(format!("Failed to get device tags: {}", e)))?;
        
        if device_tags.is_empty() {
            return Err(TbError::Api("No device tags found for hierarchical sync".to_string()));
        }
        
        // Step 2: Analyze device hierarchy
        let hierarchy = self.analyze_device_hierarchy(device_tags, entity_group_name, inverter_index).await?;
        
        // Step 3: Create only MPPT and String devices (skip inverter - already created in main sync)
        let created_devices = self.create_mppt_and_string_devices(&hierarchy, entity_group_id).await?;
        
        Ok(created_devices)
    }

    /// Extracts the prefix from entity group name
    /// 
    /// Examples:
    /// - "ACCV-P002-King Jade" -> "ACCV-P002"
    /// - "GR-P001-Toyota Boshoku HN" -> "GR-P001"
    /// - "CMES-PR084-Han Young" -> "CMES-PR084"
    fn extract_group_prefix(&self, entity_group_name: &str) -> String {
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

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), TbError> {
        let login_request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(&format!("{}/api/auth/login", self.base_url))
            .json(&login_request)
            .send()
            .await?;

        if response.status().is_success() {
            let login_response: LoginResponse = response.json().await?;
            self.token = Some(login_response.token);
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(TbError::Auth(format!("Login failed: {}", error_text)))
        }
    }

    fn get_auth_header(&self) -> Result<String, TbError> {
        match &self.token {
            Some(token) => {
                Ok(format!("Bearer {}", token))
            }
            None => {
                Err(TbError::Auth("Not authenticated".to_string()))
            }
        }
    }

    pub fn get_token(&self) -> Option<String> {
        self.token.clone()
    }

    pub async fn create_device(
        &self,
        device: &CreateDeviceRequest,
        entity_group_id: &str,
        device_token: Option<&str>,
    ) -> Result<Device, TbError> {
        let auth_header = self.get_auth_header()?;

        let mut url = format!("{}/api/device?entityGroupId={}", self.base_url, entity_group_id);
        if let Some(token) = device_token {
            url.push_str(&format!("&deviceToken={}", token));
        }

        println!("🔗 POST Request URL: {}", url);
        println!("📝 Device creation payload: {}", serde_json::to_string_pretty(device).unwrap_or_else(|_| "Failed to serialize".to_string()));

        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .json(device)
            .send()
            .await?;

        let status_code = response.status();
        println!("📊 ThingsBoard Response Status: {}", status_code);

        if response.status().is_success() {
            let created_device: Device = response.json().await?;
            println!("✨ Device creation response received successfully");
            Ok(created_device)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            println!("❌ ThingsBoard API Error Response: {}", error_text);
            Err(TbError::Api(format!("Device creation failed (Status: {}): {}", status_code, error_text)))
        }
    }

    /// Update device attributes on ThingsBoard
    /// POST /api/plugins/telemetry/{deviceId}/SERVER_SCOPE
    /// Only uses SERVER_SCOPE for server-side attributes
    pub async fn update_device_attributes(
        &self,
        device_id: &str,
        attributes: serde_json::Value,
    ) -> Result<(), TbError> {
        let auth_header = self.get_auth_header()?;
        
        // Always use SERVER_SCOPE for attributes
        let url = format!(
            "{}/api/plugins/telemetry/{}/SERVER_SCOPE",
            self.base_url, device_id
        );
        
        println!("🔗 POST Request URL: {}", url);
        println!("📝 Attributes payload: {}", serde_json::to_string_pretty(&attributes).unwrap_or_else(|_| "Failed to serialize".to_string()));
        
        let response = self
            .client
            .post(&url)
            .header("Authorization", auth_header)
            .json(&attributes)
            .send()
            .await?;
        
        let status_code = response.status();
        println!("📊 ThingsBoard Attributes Response Status: {}", status_code);
        
        if response.status().is_success() {
            println!("✨ Device attributes updated successfully");
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            println!("❌ ThingsBoard API Error Response: {}", error_text);
            Err(TbError::Api(format!(
                "Update device attributes failed (Status: {}): {}",
                status_code, error_text
            )))
        }
    }

    /// Build device attributes based on device type
    /// This helper creates the appropriate attributes for Inverter and Meter devices
    pub async fn build_device_attributes(
        &self,
        device: &DeviceInstance,
        tb_device_name: &str,
        device_type: &str,
        entity_group_name: &str,
        database: &Database,
    ) -> Result<serde_json::Value, TbError> {
        let mut attributes = serde_json::Map::new();
        
        // Common attributes for all device types
        if let Some(serial_no) = &device.serial_no {
            attributes.insert("SN".to_string(), serde_json::Value::String(serial_no.clone()));
        }
        
        // Device type specific attributes
        match device_type {
            "Inverter" => {
                // INV - inverter index (extract from ThingsBoard device name)
                let inv_index = self.extract_inv_index_from_device_name(tb_device_name);
                attributes.insert("INV".to_string(), serde_json::Value::Number(inv_index.into()));
                
                // Device Model - get from database
                if let Some(model_id) = &device.model_id {
                    if let Ok(Some(model_name)) = database.get_device_model_name(model_id).await {
                        attributes.insert("Device Model".to_string(), serde_json::Value::String(model_name));
                    }
                }
                
                // Device Brand - get from database
                if let Some(model_id) = &device.model_id {
                    if let Ok(Some(model)) = database.get_device_model(model_id).await {
                        if let Some(manufacturer) = model.manufacturer {
                            attributes.insert("Device Brand".to_string(), serde_json::Value::String(manufacturer));
                        }
                    }
                }
                
                // customer - extract from ThingsBoard device name (e.g., ACCV-P001-I01 -> ACCV)
                let customer = self.extract_customer_from_group_name(tb_device_name);
                attributes.insert("customer".to_string(), serde_json::Value::String(customer));
                
                // ava_name - the ThingsBoard device name itself
                attributes.insert("ava_name".to_string(), serde_json::Value::String(tb_device_name.to_string()));
            }
            "Meter" | "PowerMeter" => {
                // plant - the entity group name
                attributes.insert("plant".to_string(), serde_json::Value::String(entity_group_name.to_string()));
                
                // ava_name - the ThingsBoard device name itself
                attributes.insert("ava_name".to_string(), serde_json::Value::String(tb_device_name.to_string()));
            }
            _ => {
                // For other device types, just add ava_name
                attributes.insert("ava_name".to_string(), serde_json::Value::String(tb_device_name.to_string()));
            }
        }
        
        Ok(serde_json::Value::Object(attributes))
    }


    // get devices of entity group - with pagination controlled by page size and page number
    pub async fn get_group_devices(&self, entity_group_id: &str, page_size: i32, page: i32) -> Result<GroupDevicesResponse, TbError> {
        let auth_header = self.get_auth_header()?;

        let response = self
            .client
            .get(&format!("{}/api/entityGroup/{}/devices?pageSize={}&page={}", self.base_url, entity_group_id, page_size, page))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            let devices_response: GroupDevicesResponse = response.json().await?;
            Ok(devices_response)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(TbError::Api(format!("Get group devices failed: {}", error_text)))
        }
    }

    // gets all devices of entity group by iterating through pages
    pub async fn get_all_group_devices(&self, entity_group_id: &str, page_size: i32) -> Result<Vec<DeviceData>, TbError> {
        let mut all_devices = Vec::new();
        let mut page = 0;
        
        loop {
            println!("Fetching page {} with page size {}...", page, page_size);
            let response = self.get_group_devices(entity_group_id, page_size, page).await?;
            
            let data_len = response.data.len();
            println!("Received {} devices on page {}", data_len, page);
            
            // Add devices from this page to our collection
            all_devices.extend(response.data);
            
            // Break if we got fewer devices than requested (indicates last page)
            if data_len < page_size as usize {
                println!("Last page reached (got {} < {} devices)", data_len, page_size);
                break;
            }
            
            // Also check the has_next flag if available
            if response.has_next == Some(false) {
                println!("has_next is false, stopping pagination");
                break;
            }
            
            page += 1;
            
            // Safety check to prevent infinite loops
            if page > 1000 {
                println!("Warning: Reached page limit of 1000, stopping pagination");
                break;
            }
        }
        
        println!("Total devices collected: {}", all_devices.len());
        Ok(all_devices)
    }

    // get all entity groups by group type (no pagination needed)
    pub async fn get_all_entity_groups(&self, group_type: &str) -> Result<Vec<EntityGroup>, TbError> {
        let auth_header = self.get_auth_header()?;

        let response = self
            .client
            .get(&format!("{}/api/entityGroups/{}", self.base_url, group_type))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            let groups: Vec<EntityGroup> = response.json().await?;
            println!("Retrieved {} entity groups of type '{}'", groups.len(), group_type);
            Ok(groups)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(TbError::Api(format!("Get entity groups failed: {}", error_text)))
        }
    }


    pub async fn get_device_access_token(&self, device_id: &str) -> Result<String, TbError> {
        let auth_header = self.get_auth_header()?;

        let response = self
            .client
            .get(&format!("{}/api/device/{}/credentials", self.base_url, device_id))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            let credentials: DeviceCredentials = response.json().await?;
            Ok(credentials.credentials_id)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(TbError::Api(format!("Get device access token failed: {}", error_text)))
        }
    }

    /// Generate a CSV catalog of devices for the specified entity group
    /// 
    /// This function retrieves all devices from an entity group and their access tokens,
    /// then exports them to a CSV file with columns: Index, Device Name, Device ID, AVA Type, Label, Token
    /// The filename will be formatted as "ENTITY-GROUP-NAME-device-catalog.csv"
    pub async fn generate_device_catalog_csv(&self, entity_group_id: &str, output_dir: &str) -> Result<String, TbError> {
        println!("🔍 Retrieving entity group information and devices for: {}", entity_group_id);
        
        // Step 1: Get entity group information to extract the name
        let entity_groups = self.get_all_entity_groups("DEVICE").await?;
        let entity_group_name = entity_groups
            .iter()
            .find(|group| group.id.id == entity_group_id)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "Unknown-Group".to_string());
        
        println!("📋 Entity Group: {}", entity_group_name);
        
        // Generate filename based on entity group name
        let safe_group_name = entity_group_name
            .replace(" ", "-")
            .replace("/", "-")
            .replace("\\", "-")
            .replace(":", "-")
            .replace("*", "-")
            .replace("?", "-")
            .replace("\"", "-")
            .replace("<", "-")
            .replace(">", "-")
            .replace("|", "-");
        
        let output_path = format!("{}/{}-device-catalog.csv", output_dir, safe_group_name);
        println!("📄 Output file: {}", output_path);
        
        // Step 2: Get all devices from the entity group
        let devices = self.get_all_group_devices(entity_group_id, 50).await?;
        
        if devices.is_empty() {
            println!("⚠️  No devices found in entity group");
            return Err(TbError::Api("No devices found in entity group".to_string()));
        }

        println!("📱 Found {} devices, retrieving access tokens...", devices.len());

        // Step 3: Create CSV file
        let file = File::create(&output_path).map_err(|e| TbError::Api(format!("Failed to create CSV file: {}", e)))?;
        let mut writer = Writer::from_writer(file);

        // Write CSV header with AVA Type column
        writer.write_record(&["Index", "Device Name", "Device ID", "AVA Type", "Label", "Token"])
            .map_err(|e| TbError::Api(format!("Failed to write CSV header: {}", e)))?;

        // Step 4: Process each device and get access token
        let mut successful_count = 0;
        let mut failed_count = 0;

        for (index, device) in devices.iter().enumerate() {
            println!("Processing device {} of {}: {}", index + 1, devices.len(), device.name);
            
            // Get access token for this device
            let token = match self.get_device_access_token(&device.id.id).await {
                Ok(token) => {
                    successful_count += 1;
                    token
                }
                Err(e) => {
                    failed_count += 1;
                    println!("  ❌ Failed to get token for {}: {}", device.name, e);
                    format!("ERROR: {}", e)
                }
            };

            // Write device row to CSV
            writer.write_record(&[
                &index.to_string(),           // Index (starting from 0)
                &device.name,                 // Device Name
                &device.id.id,               // Device ID
                &device.device_type,         // AVA Type (device type from ThingsBoard)
                &device.label,               // Label
                &token,                      // Token
            ]).map_err(|e| TbError::Api(format!("Failed to write device row: {}", e)))?;

            // Small delay to avoid rate limiting
            if index < devices.len() - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        // Flush and close the writer
        writer.flush().map_err(|e| TbError::Api(format!("Failed to flush CSV file: {}", e)))?;

        // Generate summary
        let summary = format!(
            "✅ CSV catalog generated successfully!\n\
            📄 File: {}\n\
            🏷️  Entity Group: {}\n\
            📱 Total devices: {}\n\
            ✅ Successful token retrievals: {}\n\
            ❌ Failed token retrievals: {}",
            output_path, entity_group_name, devices.len(), successful_count, failed_count
        );

        println!("{}", summary);
        Ok(summary)
    }

    /// Generate a CSV catalog of devices with detailed database information
    /// 
    /// This function retrieves all devices from an entity group, their access tokens,
    /// and detailed information from the database including device models, tags, and schedules.
    /// Each device tag generates a separate row in the CSV file.
    pub async fn generate_detailed_device_catalog_csv(&self, entity_group_id: &str, output_dir: &str, database: &Database) -> Result<String, TbError> {
        println!("📊 Generating CSV catalog...");
        
        // Step 1: Get entity group information to extract the name
        let entity_groups = self.get_all_entity_groups("DEVICE").await?;
        let entity_group_name = entity_groups
            .iter()
            .find(|group| group.id.id == entity_group_id)
            .map(|group| group.name.clone())
            .unwrap_or_else(|| "Unknown-Group".to_string());
        
        // Generate filename based on entity group name
        let safe_group_name = entity_group_name
            .replace(" ", "-")
            .replace("/", "-")
            .replace("\\", "-")
            .replace(":", "-")
            .replace("*", "-")
            .replace("?", "-")
            .replace("\"", "-")
            .replace("<", "-")
            .replace(">", "-")
            .replace("|", "-");
        
        let output_path = format!("{}/{}-device-catalog.csv", output_dir, safe_group_name);
        
        // get all devices from the entity group
        let devices = self.get_all_group_devices(entity_group_id, 50).await?;
        
        if devices.is_empty() {
            return Err(TbError::Api("No devices found in entity group".to_string()));
        }

        // create CSV file
        let file = File::create(&output_path).map_err(|e| TbError::Api(format!("Failed to create CSV file: {}", e)))?;
        let mut writer = Writer::from_writer(file);

        writer.write_record(&[
            "IOA",           // 1
            "Index",         // 2
            "Serial Number", // 3 ← NEW
            "Device Name",   // 4
            "Device Brand",  // 5
            "Device Model",  // 6
            "Customer",      // 7
            "AVA Type",      // 8
            "Token",         // 9
            "Parent",        // 10
            "Plant",         // 11
            "INV",           // 12
            "MPPT",          // 13
            "INPUT",         // 14
            "Label",         // 15
            "Device ID",     // 16
            "Host",          // 17
            "Port",          // 18
            "Forwarding Modbus ID", // 19
            "Protocol",      // 20
            "Data Label",    // 21
            "Address",       // 22
            "Size",          // 23
            "Modbus Type",   // 24
            "Divider",       // 25
            "Register Type", // 26
            "Frequency",     // 27
            "Agg To Field",  // 28 ← NEW
        ]).map_err(|e| TbError::Api(format!("Failed to write CSV header: {}", e)))?;

        // get local database devices first to determine which devices to process
        let local_devices = match database.get_devices_by_group_id(entity_group_id).await {
            Ok(devices) => devices,
            Err(e) => {
                return Err(TbError::Api(format!("Failed to get local devices from database: {}", e)));
            }
        };

        // filter ThingsBoard devices to only include those with local database records
        let devices_to_process: Vec<&DeviceData> = devices.iter()
            .filter(|device| {
                // Check if this ThingsBoard device has a corresponding local database record
                local_devices.iter().any(|local_device| {
                    local_device.tb_device_id.as_ref() == Some(&device.id.id)
                }) ||
                // Also include hierarchical devices (MPPT/String) that might not have direct local records
                // but are created from local inverter devices
                (device.device_type == "Mppt" || device.device_type == "String")
            })
            .collect();

        if devices_to_process.is_empty() {
            return Err(TbError::Api("No devices found that exist in local database".to_string()));
        }

        // process only the filtered devices
        let mut successful_count = 0;
        let mut failed_count = 0;
        let mut total_rows = 0;
        let mut processed_devices = Vec::new(); // Track processed devices for summary

        for (device_index, device) in devices_to_process.iter().enumerate() {
            // Get access token for this device
            let token = match self.get_device_access_token(&device.id.id).await {
                Ok(token) => {
                    successful_count += 1;
                    token
                }
                Err(e) => {
                    failed_count += 1;
                    format!("ERROR: {}", e)
                }
            };

            // Track processed device info for summary
            processed_devices.push((device.name.clone(), device.device_type.clone()));

            // Extract common device information
            let customer = self.extract_customer_from_group_name(&entity_group_name);
            let inv_index = self.extract_inv_index_from_device_name(&device.name);
            let mppt_index = self.extract_mppt_index_from_device_name(&device.name);
            let input_index = self.extract_input_index_from_device_name(&device.name);
            let pm_index = self.extract_pm_index_from_device_name(&device.name);
            let mt_index = self.extract_mt_index_from_device_name(&device.name);
            
            // Use the appropriate index based on device type
            let device_index_for_csv = match device.device_type.as_str() {
                "PowerMeter" => pm_index,
                "Meter" => mt_index,
                _ => inv_index // For inverters, MPPT, String, and others
            };
            
            // Find the matching local device by ThingsBoard device ID
            if let Some(local_device) = local_devices.iter().find(|d| {
                d.tb_device_id.as_ref() == Some(&device.id.id)
            }) {
                // This is a main inverter device with local database record
                self.process_main_device(&mut writer, &mut total_rows, device, local_device, &customer, device_index_for_csv, mppt_index, input_index, &entity_group_name, &token, database).await?;
            } else {
                // This might be a hierarchical MPPT/String device without local record
                // Look for UDC/IDC data in parent inverter devices
                self.process_hierarchical_device(&mut writer, &mut total_rows, device, &local_devices, &customer, device_index_for_csv, mppt_index, input_index, &entity_group_name, &token, database).await?;
            }

            // Small delay to avoid rate limiting
            if device_index < devices_to_process.len() - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        // Flush and close the writer
        writer.flush().map_err(|e| TbError::Api(format!("Failed to flush CSV file: {}", e)))?;

        // Group devices by type for detailed summary
        let mut device_summary = std::collections::HashMap::new();
        let mut parent_devices = Vec::new(); // Main devices (non-hierarchical)
        let mut child_devices = Vec::new();  // Hierarchical devices (MPPT/String)

        for (device_name, device_type) in processed_devices {
            // Add to type summary count
            *device_summary.entry(device_type.clone()).or_insert(0) += 1;
            
            // Categorize as parent or child device
            match device_type.as_str() {
                "Inverter" | "PowerMeter" | "Meter" => {
                    parent_devices.push((device_name, device_type));
                }
                "Mppt" | "String" => {
                    child_devices.push((device_name, device_type));
                }
                _ => {
                    parent_devices.push((device_name, device_type));
                }
            }
        }

        // Generate detailed summary with device breakdown
        let mut summary = format!(
            "✅ CSV catalog generated successfully!\n\
            📄 File: {}\n\
            🏷️  Entity Group: {}\n\
            📊 Total rows: {} (device tags + hierarchical data)\n\
            ✅ Successful token retrievals: {}\n\
            ❌ Failed token retrievals: {}\n\n",
            output_path, entity_group_name, total_rows, successful_count, failed_count
        );

        // Add parent devices section
        if !parent_devices.is_empty() {
            summary.push_str("🏭 Parent Devices Generated:\n");
            let mut parent_by_type = std::collections::HashMap::new();
            for (name, device_type) in parent_devices {
                parent_by_type.entry(device_type).or_insert_with(Vec::new).push(name);
            }

            for (device_type, devices) in parent_by_type {
                let type_emoji = match device_type.as_str() {
                    "Inverter" => "⚡",
                    "PowerMeter" => "📊",
                    "Meter" => "📏",
                    "Weather Station" => "🌤️",
                    _ => "🔧"
                };
                summary.push_str(&format!("  {} {} ({}): {}\n", 
                    type_emoji, device_type, devices.len(), devices.join(", ")));
            }
        }

        println!("{}", summary);
        Ok(summary)
    }

    /// Extract customer name from entity group name (everything before first dash)
    fn extract_customer_from_group_name(&self, entity_group_name: &str) -> String {
        entity_group_name
            .split('-')
            .next()
            .unwrap_or("Unknown")
            .to_string()
    }

    /// Extract INV index from device name (extract number from pattern like "I01" -> 1)
    fn extract_inv_index_from_device_name(&self, device_name: &str) -> u32 {
        // Look for pattern like "I01", "I12", etc.
        for part in device_name.split('-') {
            if part.starts_with('I') && part.len() >= 2 {
                let number_str = &part[1..];
                if let Ok(num) = number_str.parse::<u32>() {
                    return num;
                }
            }
        }
        0
    }

    /// Extract MPPT index from device name (extract number from pattern like "M01" -> 1)
    fn extract_mppt_index_from_device_name(&self, device_name: &str) -> u32 {
        for part in device_name.split('-') {
            if part.starts_with('M') && part.len() >= 2 {
                let number_str = &part[1..];
                if let Ok(num) = number_str.parse::<u32>() {
                    return num;
                }
            }
        }
        0
    }

    /// Extract INPUT index from device name (extract number from pattern like "PV01" -> 1)
    fn extract_input_index_from_device_name(&self, device_name: &str) -> u32 {
        for part in device_name.split('-') {
            if part.starts_with("PV") && part.len() >= 3 {
                let number_str = &part[2..];
                if let Ok(num) = number_str.parse::<u32>() {
                    return num;
                }
            }
        }
        0
    }

    /// Extract PowerMeter index from device name (extract number from pattern like "PM01" -> 1)
    fn extract_pm_index_from_device_name(&self, device_name: &str) -> u32 {
        for part in device_name.split('-') {
            if part.starts_with("PM") && part.len() >= 3 {
                let number_str = &part[2..];
                if let Ok(num) = number_str.parse::<u32>() {
                    return num;
                }
            }
        }
        0
    }

    /// Extract Meter index from device name (extract number from pattern like "MT01" -> 1)
    fn extract_mt_index_from_device_name(&self, device_name: &str) -> u32 {
        for part in device_name.split('-') {
            if part.starts_with("MT") && part.len() >= 3 {
                let number_str = &part[2..];
                if let Ok(num) = number_str.parse::<u32>() {
                    return num;
                }
            }
        }
        0
    }

    /// Process main device (inverter with local database record)
    async fn process_main_device(
        &self,
        writer: &mut Writer<File>,
        total_rows: &mut usize,
        device: &DeviceData,
        local_device: &DeviceInstance,
        customer: &str,
        device_index: u32, // Renamed from inv_index to be more generic
        mppt_index: u32,
        input_index: u32,
        entity_group_name: &str,
        token: &str,
        database: &Database,
    ) -> Result<(), TbError> {
        // Get device model information
        let (device_brand, device_model) = if let Some(model_id) = &local_device.model_id {
            match database.get_device_model(model_id).await {
                Ok(Some(model)) => (
                    model.manufacturer.unwrap_or("Unknown".to_string()),
                    model.name
                ),
                Ok(None) => ("Unknown".to_string(), "Unknown".to_string()),
                Err(_) => ("Unknown".to_string(), "Unknown".to_string()),
            }
        } else {
            ("Unknown".to_string(), "Unknown".to_string())
        };

        // Get device tags
        match database.get_device_tags(&local_device.id).await {
            Ok(tags) => {
                if tags.is_empty() {
                    // If no tags, create one row with device info only
                    let (host, port, modbus_id, modbus_type) = self.parse_protocol_config(&local_device.protocol_config);
                    
                    // Determine INV, MPPT and INPUT columns based on device type
                    let (inv_col, mppt_col, input_col) = match device.device_type.as_str() {
                        "Mppt" => (device_index.to_string(), mppt_index.to_string(), "".to_string()),
                        "String" => (device_index.to_string(), mppt_index.to_string(), input_index.to_string()),
                        "Inverter" => (device_index.to_string(), "".to_string(), "".to_string()),
                        "PowerMeter" | "Meter" => ("".to_string(), "".to_string(), "".to_string()), // Empty for meters
                        _ => ("".to_string(), "".to_string(), "".to_string()) // Other devices
                    };
                    
                    let parent_name = self.get_parent_name(device, entity_group_name, device_index, mppt_index);
                    
                    writer.write_record(&[
                        &total_rows.to_string(),              // IOA (starting from 0)
                        &total_rows.to_string(),              // Index (starting from 0) 
                        local_device.serial_no.as_deref().unwrap_or(""), // Serial Number ← NEW
                        &device.name,                         // Device Name
                        &device_brand,                        // Device Brand
                        &device_model,                        // Device Model
                        customer,                             // Customer
                        &device.device_type,                  // AVA Type
                        token,                               // Token
                        &parent_name,                        // Parent
                        entity_group_name,                   // Plant
                        &inv_col,                            // INV
                        &mppt_col,                           // MPPT 
                        &input_col,                          // INPUT
                        &device.label,                       // Label
                        &device.id.id,                      // Device ID
                        &host,                               // Host
                        &port,                               // Port
                        &modbus_id,                          // Modbus ID
                        &modbus_type,                        // Modbus Type
                        "",                                  // Data Label (empty for no tags)
                        "",                                  // Address
                        "",                                  // Size
                        "",                                  // Data Type
                        "",                                  // Divider
                        "",                                  // Register Type
                        "",                                  // Frequency
                        "",                                  // Agg To Field (empty for no tags) ← NEW
                    ]).map_err(|e| TbError::Api(format!("Failed to write device row: {}", e)))?;
                    *total_rows += 1;
                } else {
                    // Create one row for each tag, with different filtering based on device type
                    let (host, port, modbus_id, modbus_type) = self.parse_protocol_config(&local_device.protocol_config);
                    
                    for tag in tags {
                        // For Inverter devices: skip Idc and Udc tags (current behavior)
                        // For MPPT/String devices: only include Idc and Udc tags
                        // For PowerMeter/Meter devices: include all tags
                        let should_include_tag = match device.device_type.as_str() {
                            "Inverter" => tag.name != "Idc" && tag.name != "Udc",
                            "Mppt" | "String" => tag.name == "Idc" || tag.name == "Udc",
                            "PowerMeter" | "Meter" => true, // Include all tags for meter devices
                            _ => true // Other device types include all tags
                        };

                        if !should_include_tag {
                            continue;
                        }

                        let divider = self.convert_scaling_multiplier_to_divider(tag.scaling_multiplier);
                        let frequency = self.get_schedule_group_frequency(database, &tag.schedule_group_id).await;

                        // Determine INV, MPPT and INPUT columns based on device type
                        let (inv_col, mppt_col, input_col) = match device.device_type.as_str() {
                            "Mppt" => (device_index.to_string(), mppt_index.to_string(), "".to_string()),
                            "String" => (device_index.to_string(), mppt_index.to_string(), input_index.to_string()),
                            "Inverter" => (device_index.to_string(), "".to_string(), "".to_string()),
                            "PowerMeter" | "Meter" => ("".to_string(), "".to_string(), "".to_string()), // Empty for meters
                            _ => ("".to_string(), "".to_string(), "".to_string()) // Other devices
                        };

                        let parent_name = self.get_parent_name(device, entity_group_name, device_index, mppt_index);

                        writer.write_record(&[
                            &total_rows.to_string(),              // IOA (starting from 0)
                            &total_rows.to_string(),              // Index (starting from 0)
                            local_device.serial_no.as_deref().unwrap_or(""), // Serial Number ← NEW
                            &device.name,                         // Device Name
                            &device_brand,                        // Device Brand
                            &device_model,                        // Device Model
                            customer,                             // Customer
                            &device.device_type,                  // AVA Type
                            token,                               // Token
                            &parent_name,                        // Parent
                            entity_group_name,                   // Plant
                            &inv_col,                            // INV
                            &mppt_col,                           // MPPT
                            &input_col,                          // INPUT
                            &device.label,                       // Label
                            &device.id.id,                      // Device ID
                            &host,                               // Host
                            &port,                               // Port
                            &modbus_id,                          // Modbus ID
                            &modbus_type,                        // Modbus Type
                            &tag.name,                           // Data Label
                            &tag.address.to_string(),            // Address
                            &tag.size.to_string(),               // Size
                            &tag.data_type,                      // Data Type
                            &divider,                            // Divider
                            &tag.unit.as_ref().unwrap_or(&"".to_string()), // Register Type
                            &frequency,                          // Frequency
                            &tag.agg_to_field.as_ref().unwrap_or(&"".to_string()), // Agg To Field ← NEW
                        ]).map_err(|e| TbError::Api(format!("Failed to write tag row: {}", e)))?;
                        *total_rows += 1;
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Failed to get tags for device {}: {}", local_device.id, e);
                // Create a single row without tag information
                let (host, port, modbus_id, modbus_type) = self.parse_protocol_config(&local_device.protocol_config);
                
                // Determine INV, MPPT and INPUT columns based on device type
                let (inv_col, mppt_col, input_col) = match device.device_type.as_str() {
                    "Mppt" => (device_index.to_string(), mppt_index.to_string(), "".to_string()),
                    "String" => (device_index.to_string(), mppt_index.to_string(), input_index.to_string()),
                    "Inverter" => (device_index.to_string(), "".to_string(), "".to_string()),
                    "PowerMeter" | "Meter" => ("".to_string(), "".to_string(), "".to_string()), // Empty for meters
                    _ => ("".to_string(), "".to_string(), "".to_string()) // Other devices
                };
                
                let parent_name = self.get_parent_name(device, entity_group_name, device_index, mppt_index);
                
                writer.write_record(&[
                    &total_rows.to_string(),              // IOA (starting from 0)
                    &total_rows.to_string(),              // Index (starting from 0)
                    local_device.serial_no.as_deref().unwrap_or(""), // Serial Number ← NEW
                    &device.name,                         // Device Name
                    &device_brand,                        // Device Brand
                    &device_model,                        // Device Model
                    customer,                             // Customer
                    &device.device_type,                  // AVA Type
                    token,                               // Token
                    &parent_name,                        // Parent
                    entity_group_name,                   // Plant
                    &inv_col,                            // INV
                    &mppt_col,                           // MPPT
                    &input_col,                          // INPUT
                    &device.label,                       // Label
                    &device.id.id,                      // Device ID
                    &host,                               // Host
                    &port,                               // Port
                    &modbus_id,                          // Modbus ID
                    &modbus_type,                        // Modbus Type
                    "ERROR: Failed to get tags",        // Data Label
                    "",                                  // Address
                    "",                                  // Size
                    "",                                  // Data Type
                    "",                                  // Divider
                    "",                                  // Register Type
                    "",                                  // Frequency
                    "",                                  // Agg To Field (empty for error) ← NEW
                ]).map_err(|e| TbError::Api(format!("Failed to write error row: {}", e)))?;
                *total_rows += 1;
            }
        }
        Ok(())
    }

    /// Process hierarchical device (MPPT/String without local database record)
    /// Find UDC/IDC data from parent inverter devices based on tag descriptions
    async fn process_hierarchical_device(
        &self,
        writer: &mut Writer<File>,
        total_rows: &mut usize,
        device: &DeviceData,
        local_devices: &[DeviceInstance],
        customer: &str,
        device_index: u32, // Renamed from inv_index to be more generic
        mppt_index: u32,
        input_index: u32,
        entity_group_name: &str,
        token: &str,
        database: &Database,
    ) -> Result<(), TbError> {
        match device.device_type.as_str() {
            "Mppt" => {
                // Find parent inverter device with matching INV index
                if let Some(parent_device) = self.find_parent_inverter_device(local_devices, device_index) {
                    // Look for UDC/IDC tags with MPPT description matching this MPPT index
                    match database.get_device_tags(&parent_device.id).await {
                        Ok(tags) => {
                            let mut found_tags = false;
                            let (host, port, modbus_id, modbus_type) = self.parse_protocol_config(&parent_device.protocol_config);
                            
                            // Get device model info from parent
                            let (device_brand, device_model) = self.get_device_model_info(database, parent_device).await;
                            
                            for tag in tags {
                                if (tag.name == "Udc" || tag.name == "Idc") && 
                                   self.tag_matches_mppt(tag.description.as_deref(), mppt_index) {
                                    found_tags = true;
                                    
                                    let divider = self.convert_scaling_multiplier_to_divider(tag.scaling_multiplier);
                                    let frequency = self.get_schedule_group_frequency(database, &tag.schedule_group_id).await;
                                    
                                    let parent_name = self.get_parent_name(device, entity_group_name, device_index, mppt_index);
                                    
                                    writer.write_record(&[
                                        &total_rows.to_string(),              // IOA
                                        &total_rows.to_string(),              // Index
                                        parent_device.serial_no.as_deref().unwrap_or(""), // Serial Number (from parent) ← NEW
                                        &device.name,                         // Device Name
                                        &device_brand,                        // Device Brand
                                        &device_model,                        // Device Model
                                        customer,                             // Customer
                                        &device.device_type,                  // AVA Type
                                        token,                               // Token
                                        &parent_name,                        // Parent
                                        entity_group_name,                   // Plant
                                        &device_index.to_string(),              // INV
                                        &mppt_index.to_string(),             // MPPT
                                        "",                                  // INPUT (empty for MPPT)
                                        &device.label,                       // Label
                                        &device.id.id,                      // Device ID
                                        &host,                               // Host
                                        &port,                               // Port
                                        &modbus_id,                          // Modbus ID
                                        &modbus_type,                        // Modbus Type
                                        &tag.name,                           // Data Label (Udc/Idc)
                                        &tag.address.to_string(),            // Address
                                        &tag.size.to_string(),               // Size
                                        &tag.data_type,                      // Data Type
                                        &divider,                            // Divider
                                        &tag.unit.as_ref().unwrap_or(&"".to_string()), // Register Type
                                        &frequency,                          // Frequency
                                        &tag.agg_to_field.as_ref().unwrap_or(&"".to_string()), // Agg To Field ← NEW
                                    ]).map_err(|e| TbError::Api(format!("Failed to write MPPT row: {}", e)))?;
                                    *total_rows += 1;
                                }
                            }
                            
                            if !found_tags {
                                println!("  ⚠️ No UDC/IDC tags found for MPPT {}", mppt_index);
                                // Create placeholder row
                                self.create_placeholder_row(writer, total_rows, device, customer, device_index, mppt_index, 0, entity_group_name, token, "No UDC/IDC data found").await?;
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Failed to get tags from parent device: {}", e);
                            self.create_placeholder_row(writer, total_rows, device, customer, device_index, mppt_index, 0, entity_group_name, token, "Failed to get parent device tags").await?;
                        }
                    }
                } else {
                    println!("  ❌ Parent inverter device not found for MPPT {}", device.name);
                    self.create_placeholder_row(writer, total_rows, device, customer, device_index, mppt_index, 0, entity_group_name, token, "Parent inverter not found").await?;
                }
            }
            "String" => {
                // Find parent inverter device with matching INV index
                if let Some(parent_device) = self.find_parent_inverter_device(local_devices, device_index) {
                    // Look for UDC/IDC tags with String description matching this MPPT and Input index
                    match database.get_device_tags(&parent_device.id).await {
                        Ok(tags) => {
                            let mut found_tags = false;
                            let (host, port, modbus_id, modbus_type) = self.parse_protocol_config(&parent_device.protocol_config);
                            
                            // Get device model info from parent
                            let (device_brand, device_model) = self.get_device_model_info(database, parent_device).await;
                            
                            for tag in tags {
                                if (tag.name == "Udc" || tag.name == "Idc") && 
                                   self.tag_matches_string(tag.description.as_deref(), mppt_index, input_index) {
                                    found_tags = true;
                                    
                                    let divider = self.convert_scaling_multiplier_to_divider(tag.scaling_multiplier);
                                    let frequency = self.get_schedule_group_frequency(database, &tag.schedule_group_id).await;
                                    
                                    let parent_name = self.get_parent_name(device, entity_group_name, device_index, mppt_index);
                                    
                                    writer.write_record(&[
                                        &total_rows.to_string(),              // IOA
                                        &total_rows.to_string(),              // Index
                                        parent_device.serial_no.as_deref().unwrap_or(""), // Serial Number (from parent) ← NEW
                                        &device.name,                         // Device Name
                                        &device_brand,                        // Device Brand
                                        &device_model,                        // Device Model
                                        customer,                             // Customer
                                        &device.device_type,                  // AVA Type
                                        token,                               // Token
                                        &parent_name,                        // Parent
                                        entity_group_name,                   // Plant
                                        &device_index.to_string(),              // INV
                                        &mppt_index.to_string(),             // MPPT
                                        &input_index.to_string(),            // INPUT
                                        &device.label,                       // Label
                                        &device.id.id,                      // Device ID
                                        &host,                               // Host
                                        &port,                               // Port
                                        &modbus_id,                          // Modbus ID
                                        &modbus_type,                        // Modbus Type
                                        &tag.name,                           // Data Label (Udc/Idc)
                                        &tag.address.to_string(),            // Address
                                        &tag.size.to_string(),               // Size
                                        &tag.data_type,                      // Data Type
                                        &divider,                            // Divider
                                        &tag.unit.as_ref().unwrap_or(&"".to_string()), // Register Type
                                        &frequency,                          // Frequency
                                        &tag.agg_to_field.as_ref().unwrap_or(&"".to_string()), // Agg To Field ← NEW
                                    ]).map_err(|e| TbError::Api(format!("Failed to write String row: {}", e)))?;
                                    *total_rows += 1;
                                }
                            }
                            
                            if !found_tags {
                                println!("  ⚠️ No UDC/IDC tags found for String MPPT {} Input {}", mppt_index, input_index);
                                // Create placeholder row
                                self.create_placeholder_row(writer, total_rows, device, customer, device_index, mppt_index, input_index, entity_group_name, token, "No UDC/IDC data found").await?;
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Failed to get tags from parent device: {}", e);
                            self.create_placeholder_row(writer, total_rows, device, customer, device_index, mppt_index, input_index, entity_group_name, token, "Failed to get parent device tags").await?;
                        }
                    }
                } else {
                    println!("  ❌ Parent inverter device not found for String {}", device.name);
                    self.create_placeholder_row(writer, total_rows, device, customer, device_index, mppt_index, input_index, entity_group_name, token, "Parent inverter not found").await?;
                }
            }
            _ => {
                println!("  ⚠️ Unknown device type for hierarchical processing: {}", device.device_type);
                self.create_placeholder_row(writer, total_rows, device, customer, device_index, 0, 0, entity_group_name, token, "Unknown device type").await?;
            }
        }
        Ok(())
    }

    /// Find parent inverter device by INV index
    fn find_parent_inverter_device<'a>(&self, local_devices: &'a [DeviceInstance], inv_index: u32) -> Option<&'a DeviceInstance> {
        // Look for the inverter device that matches this INV index
        // First try ThingsBoard naming pattern: -I{inv_index:02}
        let inv_pattern = format!("-I{:02}", inv_index);
        
        for device in local_devices {
            if device.name.contains(&inv_pattern) {
                return Some(device);
            }
        }
        
        // Try local device naming patterns: "inv 1", "inv 2", etc.
        let local_pattern = format!("inv {}", inv_index);
        
        for device in local_devices {
            if device.name.eq_ignore_ascii_case(&local_pattern) {
                return Some(device);
            }
        }
        
        // Try numeric index matching: if we have 2 devices and inv_index=2, use second device
        if inv_index > 0 && (inv_index as usize) <= local_devices.len() {
            let device_index = (inv_index as usize) - 1; // Convert to 0-based index
            let matched_device = &local_devices[device_index];
            return Some(matched_device);
        }
        
        // FALLBACK: If all pattern matching fails, return the first device
        if let Some(first_device) = local_devices.first() {
            return Some(first_device);
        }
        
        None
    }

    /// Get device model info from a device instance
    async fn get_device_model_info(&self, database: &Database, device: &DeviceInstance) -> (String, String) {
        if let Some(model_id) = &device.model_id {
            match database.get_device_model(model_id).await {
                Ok(Some(model)) => (
                    model.manufacturer.unwrap_or("Unknown".to_string()),
                    model.name
                ),
                Ok(None) => ("Unknown".to_string(), "Unknown".to_string()),
                Err(_) => ("Unknown".to_string(), "Unknown".to_string()),
            }
        } else {
            ("Unknown".to_string(), "Unknown".to_string())
        }
    }

    /// Determine the correct parent for a device based on hierarchy:
    /// - Inverter devices: Entity Group name
    /// - MPPT devices: Parent Inverter name
    /// - String devices: Parent MPPT name
    fn get_parent_name(&self, device: &DeviceData, entity_group_name: &str, inv_index: u32, mppt_index: u32) -> String {
        match device.device_type.as_str() {
            "Inverter" => {
                // Inverters parent is the entity group
                entity_group_name.to_string()
            },
            "Mppt" => {
                // MPPT parent is the inverter name
                // Extract base name and construct inverter name
                // From "ACCV-P002-I01-M01" get "ACCV-P002-I01"
                if let Some(pos) = device.name.rfind("-M") {
                    device.name[..pos].to_string()
                } else {
                    // Fallback: construct from pattern
                    format!("ACCV-P002-I{:02}", inv_index)
                }
            },
            "String" => {
                // String parent is the MPPT name
                // Extract base name and construct MPPT name
                // From "ACCV-P002-I01-M01-PV01" get "ACCV-P002-I01-M01"
                if let Some(pos) = device.name.rfind("-PV") {
                    device.name[..pos].to_string()
                } else {
                    // Fallback: construct from pattern
                    format!("ACCV-P002-I{:02}-M{:02}", inv_index, mppt_index)
                }
            },
            _ => {
                // Unknown device type, use entity group as fallback
                entity_group_name.to_string()
            }
        }
    }

    /// Check if tag description matches MPPT pattern
    /// Example: "MPPT - MPPT 1 (SG125CX-P2)" should match mppt_index = 1
    fn tag_matches_mppt(&self, description: Option<&str>, mppt_index: u32) -> bool {
        if let Some(desc) = description {
            // Look for pattern "MPPT - MPPT {number}"
            if desc.starts_with("MPPT - MPPT ") {
                let parts: Vec<&str> = desc.split(' ').collect();
                if parts.len() >= 4 {
                    if let Ok(num) = parts[3].parse::<u32>() {
                        return num == mppt_index;
                    }
                }
            }
        }
        false
    }

    /// Check if tag description matches String pattern
    /// Example: "String - MPPT 1 - Input 2" should match mppt_index = 1, input_index = 2
    fn tag_matches_string(&self, description: Option<&str>, mppt_index: u32, input_index: u32) -> bool {
        if let Some(desc) = description {
            // Look for pattern "String - MPPT {mppt_number} - Input {input_number}"
            if desc.starts_with("String - MPPT ") {
                let parts: Vec<&str> = desc.split(' ').collect();
                if parts.len() >= 7 {
                    if let (Ok(mppt_num), Ok(input_num)) = (parts[3].parse::<u32>(), parts[6].parse::<u32>()) {
                        return mppt_num == mppt_index && input_num == input_index;
                    }
                }
            }
        }
        false
    }

    /// Create a placeholder row for devices without data
    async fn create_placeholder_row(
        &self,
        writer: &mut Writer<File>,
        total_rows: &mut usize,
        device: &DeviceData,
        customer: &str,
        device_index: u32, // Renamed from inv_index to be more generic
        mppt_index: u32,
        input_index: u32,
        entity_group_name: &str,
        token: &str,
        error_msg: &str,
    ) -> Result<(), TbError> {
        let (mppt_col, input_col) = match device.device_type.as_str() {
            "Mppt" => (mppt_index.to_string(), "".to_string()),
            "String" => (mppt_index.to_string(), input_index.to_string()),
            _ => ("".to_string(), "".to_string())
        };

        let parent_name = self.get_parent_name(device, entity_group_name, device_index, mppt_index);

        writer.write_record(&[
            &total_rows.to_string(),              // IOA
            &total_rows.to_string(),              // Index
            "",                                   // Serial Number (empty for placeholder) ← NEW
            &device.name,                         // Device Name
            "Unknown",                            // Device Brand
            "Unknown",                            // Device Model
            customer,                             // Customer
            &device.device_type,                  // AVA Type
            token,                               // Token
            &parent_name,                        // Parent
            entity_group_name,                   // Plant
            &device_index.to_string(),              // INV
            &mppt_col,                           // MPPT
            &input_col,                          // INPUT
            &device.label,                       // Label
            &device.id.id,                      // Device ID
            "",                                  // Host (empty)
            "",                                  // Port (empty)
            "",                                  // Modbus ID (empty)
            "",                                  // Modbus Type (empty)
            error_msg,                           // Data Label
            "",                                  // Address
            "",                                  // Size
            "",                                  // Data Type
            "",                                  // Divider
            "",                                  // Register Type
            "",                                  // Frequency
            "",                                  // Agg To Field (empty for placeholder) ← NEW
        ]).map_err(|e| TbError::Api(format!("Failed to write placeholder row: {}", e)))?;
        *total_rows += 1;
        Ok(())
    }

    /// Convert scaling multiplier to divider format
    /// Examples: 0.1 -> "10", 0.001 -> "1000", 1.0 -> "1"
    fn convert_scaling_multiplier_to_divider(&self, multiplier: f64) -> String {
        if multiplier == 0.0 {
            return "1".to_string();
        }
        
        let divider = 1.0 / multiplier;
        if divider.fract() == 0.0 {
            format!("{:.0}", divider)
        } else {
            format!("{}", divider)
        }
    }

    /// Get frequency information from schedule group (numeric value only)
    async fn get_schedule_group_frequency(&self, database: &Database, schedule_group_id: &Option<String>) -> String {
        if let Some(group_id) = schedule_group_id {
            match database.get_schedule_group(group_id).await {
                Ok(Some(group)) => group.polling_interval_ms.to_string(),
                Ok(None) => "Unknown".to_string(),
                Err(_) => "Error".to_string(),
            }
        } else {
            "None".to_string()
        }
    }

    /// Parse protocol config JSON and extract connection details
    fn parse_protocol_config(&self, protocol_config: &str) -> (String, String, String, String) {
        match serde_json::from_str::<serde_json::Value>(protocol_config) {
            Ok(config) => {
                let host = config.get("host")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let port = config.get("port")
                    .and_then(|v| v.as_u64())
                    .map(|p| p.to_string())
                    .unwrap_or("".to_string());
                
                let modbus_id = config.get("slave_id")
                    .and_then(|v| v.as_u64())
                    .map(|s| s.to_string())
                    .unwrap_or("".to_string());
                
                let modbus_type = config.get("type")
                    .and_then(|v| v.as_str())
                    .map(|t| t.to_string())
                    .unwrap_or("".to_string());
                
                (host, port, modbus_id, modbus_type)
            }
            Err(_) => ("".to_string(), "".to_string(), "".to_string(), "".to_string())
        }
    }

    pub async fn get_device_by_id(&self, device_id: &str) -> Result<Device, TbError> {
        let auth_header = self.get_auth_header()?;

        let response = self
            .client
            .get(&format!("{}/api/device/{}", self.base_url, device_id))
            .header("Authorization", auth_header)
            .send()
            .await?;

        if response.status().is_success() {
            let device: Device = response.json().await?;
            Ok(device)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(TbError::Api(format!("Get device failed: {}", error_text)))
        }
    }

    // pub async fn get_device_credentials(&self, device_id: &str) -> Result<DeviceCredentials, TbError> {
    //     let auth_header = self.get_auth_header()?;

    //     let response = self
    //         .client
    //         .get(&format!("{}/api/device/{}/credentials", self.base_url, device_id))
    //         .header("Authorization", auth_header)
    //         .send()
    //         .await?;

    //     if response.status().is_success() {
    //         let credentials: DeviceCredentials = response.json().await?;
    //         Ok(credentials)
    //     } else {
    //         let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
    //         Err(TbError::Api(format!("Get credentials failed: {}", error_text)))
    //     }
    // }

    pub async fn save_device_telemetry(
        &self,
        device_id: &str,
        telemetry: &HashMap<String, serde_json::Value>,
    ) -> Result<(), TbError> {
        let auth_header = self.get_auth_header()?;

        let response = self
            .client
            .post(&format!("{}/api/plugins/telemetry/DEVICE/{}/timeseries/ANY", self.base_url, device_id))
            .header("Authorization", auth_header)
            .json(telemetry)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(TbError::Api(format!("Save telemetry failed: {}", error_text)))
        }
    }
}