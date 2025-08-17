use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub devices: Vec<DeviceConfig>,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub max_log_entries: u32,
    pub cleanup_interval_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub protocol: ProtocolConfig,
    pub polling_interval_ms: u64,
    pub timeout_ms: u64,
    pub retry_count: u32,
    pub tags: Vec<TagConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolConfig {
    #[serde(rename = "modbus_rtu")]
    ModbusRtu {
        port: String,
        baud_rate: u32,
        data_bits: u8,
        stop_bits: u8,
        parity: String,
        slave_id: u8,
    },
    #[serde(rename = "modbus_tcp")]
    ModbusTcp {
        host: String,
        port: u16,
        slave_id: u8,
    },
    #[serde(rename = "iec104")]
    Iec104 {
        host: String,
        port: u16,
        common_address: u16,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagConfig {
    pub name: String,
    pub address: u16,
    pub data_type: DataType,
    pub scaling: Option<ScalingConfig>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    #[serde(rename = "coil")]
    Coil,
    #[serde(rename = "discrete_input")]
    DiscreteInput,
    #[serde(rename = "holding_register")]
    HoldingRegister,
    #[serde(rename = "input_register")]
    InputRegister,
    #[serde(rename = "float32")]
    Float32,
    #[serde(rename = "uint16")]
    UInt16,
    #[serde(rename = "int16")]
    Int16,
    #[serde(rename = "uint32")]
    UInt32,
    #[serde(rename = "int32")]
    Int32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub multiplier: f64,
    pub offset: f64,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file_path: Option<String>,
    pub max_file_size_mb: u32,
    pub max_files: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                port: 8080,
                host: "0.0.0.0".to_string(),
            },
            database: DatabaseConfig {
                path: "data.db".to_string(),
                max_log_entries: 1000000,
                cleanup_interval_hours: 24,
            },
            devices: vec![
                DeviceConfig {
                    id: "device1".to_string(),
                    name: "Example Modbus TCP Device".to_string(),
                    enabled: false,
                    protocol: ProtocolConfig::ModbusTcp {
                        host: "192.168.1.100".to_string(),
                        port: 502,
                        slave_id: 1,
                    },
                    polling_interval_ms: 1000,
                    timeout_ms: 5000,
                    retry_count: 3,
                    tags: vec![
                        TagConfig {
                            name: "temperature".to_string(),
                            address: 1,
                            data_type: DataType::HoldingRegister,
                            scaling: Some(ScalingConfig {
                                multiplier: 0.1,
                                offset: 0.0,
                                unit: Some("°C".to_string()),
                            }),
                            description: Some("Temperature sensor".to_string()),
                        },
                    ],
                },
            ],
            logging: LoggingConfig {
                level: "info".to_string(),
                file_path: Some("app.log".to_string()),
                max_file_size_mb: 10,
                max_files: 5,
            },
        }
    }
}

pub async fn load_config() -> Result<AppConfig> {
    let config_path = "config.toml";
    
    match tokio::fs::read_to_string(config_path).await {
        Ok(content) => {
            match toml::from_str(&content) {
                Ok(config) => {
                    info!("Configuration loaded from {}", config_path);
                    Ok(config)
                },
                Err(e) => {
                    warn!("Failed to parse config file: {}. Using default configuration.", e);
                    let default_config = AppConfig::default();
                    save_config(&default_config).await?;
                    Ok(default_config)
                }
            }
        },
        Err(_) => {
            info!("Config file not found. Creating default configuration.");
            let default_config = AppConfig::default();
            save_config(&default_config).await?;
            Ok(default_config)
        }
    }
}

pub async fn save_config(config: &AppConfig) -> Result<()> {
    let config_content = toml::to_string_pretty(config)?;
    tokio::fs::write("config.toml", config_content).await?;
    info!("Configuration saved to config.toml");
    Ok(())
}
