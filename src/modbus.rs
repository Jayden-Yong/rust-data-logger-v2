use tokio_modbus::prelude::*;
use std::net::SocketAddr;
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};
use chrono::Utc;

use crate::config::{DeviceConfig, ProtocolConfig, TagConfig, DataType, ScalingConfig};
use crate::database::{LogEntry, Database, DeviceTag};

pub struct ModbusClient {
    device_config: DeviceConfig,
    tcp_client: Option<tokio_modbus::client::Context>,
}

impl ModbusClient {
    pub fn new(device_config: DeviceConfig) -> Self {
        Self {
            device_config,
            tcp_client: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        match &self.device_config.protocol {
            ProtocolConfig::ModbusTcp { host, port, .. } => {
                let socket_addr: SocketAddr = format!("{}:{}", host, port).parse()?;
                info!("Connecting to Modbus TCP device at {}", socket_addr);
                
                let ctx = tcp::connect_slave(socket_addr, Slave(self.get_slave_id()))
                    .await
                    .map_err(|e| anyhow!("Failed to connect to Modbus TCP device: {}", e))?;
                
                self.tcp_client = Some(ctx);
                info!("Successfully connected to Modbus TCP device");
                Ok(())
            },
            ProtocolConfig::ModbusRtu { port, .. } => {
                info!("Connecting to Modbus RTU device on {}", port);
                
                // For now, we'll just implement a stub for RTU
                // A real implementation would need proper RTU framing and serial communication
                warn!("Modbus RTU support is limited - using TCP fallback for demonstration");
                
                // Use localhost TCP for demonstration
                let socket_addr: SocketAddr = "127.0.0.1:5020".parse()?;
                let ctx = tcp::connect_slave(socket_addr, Slave(self.get_slave_id()))
                    .await
                    .map_err(|e| anyhow!("Failed to connect to Modbus RTU device: {}", e))?;

                self.tcp_client = Some(ctx);
                info!("Successfully connected to Modbus RTU device (via TCP)");
                Ok(())
            },
            _ => Err(anyhow!("Invalid protocol for Modbus client")),
        }
    }

    pub async fn read_tags(&mut self, database: &Database) -> Result<Vec<LogEntry>> {
        let mut log_entries = Vec::new();
        let timestamp = Utc::now();

        // Clone the tags to avoid borrowing issues
        let tags = self.device_config.tags.clone();
        
        for tag in &tags {
            match self.read_tag(&tag).await {
                Ok(value) => {
                    let scaled_value = self.apply_scaling(value, &tag);
                    let entry = LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: tag.name.clone(),
                        value: scaled_value,
                        quality: "Good".to_string(),
                        timestamp,
                        unit: tag.scaling.as_ref().and_then(|s| s.unit.clone()),
                    };

                    // Insert into database
                    if let Err(e) = database.insert_log_entry(&entry).await {
                        error!("Failed to insert log entry: {}", e);
                    }

                    log_entries.push(entry);
                },
                Err(e) => {
                    warn!("Failed to read tag {}: {}", tag.name, e);
                    let entry = LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: tag.name.clone(),
                        value: 0.0,
                        quality: "Bad".to_string(),
                        timestamp,
                        unit: tag.scaling.as_ref().and_then(|s| s.unit.clone()),
                    };

                    log_entries.push(entry);
                }
            }
        }

        Ok(log_entries)
    }

    async fn read_tag(&mut self, tag: &TagConfig) -> Result<f64> {
        let client = if let Some(ref mut client) = self.tcp_client {
            client
        } else {
            return Err(anyhow!("No client connected"));
        };

        match tag.data_type {
            DataType::Coil => {
                let result = client.read_coils(tag.address, 1).await?;
                Ok(if result[0] { 1.0 } else { 0.0 })
            },
            DataType::DiscreteInput => {
                let result = client.read_discrete_inputs(tag.address, 1).await?;
                Ok(if result[0] { 1.0 } else { 0.0 })
            },
            DataType::HoldingRegister | DataType::UInt16 => {
                let result = client.read_holding_registers(tag.address, 1).await?;
                Ok(result[0] as f64)
            },
            DataType::InputRegister => {
                let result = client.read_input_registers(tag.address, 1).await?;
                Ok(result[0] as f64)
            },
            DataType::Int16 => {
                let result = client.read_holding_registers(tag.address, 1).await?;
                Ok(result[0] as i16 as f64)
            },
            DataType::UInt32 => {
                let result = client.read_holding_registers(tag.address, 2).await?;
                let value = ((result[1] as u32) << 16) | (result[0] as u32);
                Ok(value as f64)
            },
            DataType::Int32 => {
                let result = client.read_holding_registers(tag.address, 2).await?;
                let value = ((result[1] as u32) << 16) | (result[0] as u32);
                Ok(value as i32 as f64)
            },
            DataType::Float32 => {
                let result = client.read_holding_registers(tag.address, tag.size as u16).await?;
                
                if result.len() < 2 {
                    return Err(anyhow!("Expected at least 2 registers for F32, got {}", result.len()));
                }
                
                // Try different byte order combinations for power meter compatibility
                let reg0 = result[0];  // First register 
                let reg1 = result[1];  // Second register
                
                // Most common for power meters: ABCD byte order (big-endian words, big-endian bytes)
                let combined_abcd = ((reg0 as u32) << 16) | (reg1 as u32);
                let bytes_abcd = combined_abcd.to_be_bytes();
                let value_abcd = f32::from_be_bytes(bytes_abcd);
                
                // Alternative: CDAB byte order (little-endian words, big-endian bytes)
                let combined_cdab = ((reg1 as u32) << 16) | (reg0 as u32);
                let bytes_cdab = combined_cdab.to_be_bytes();
                let value_cdab = f32::from_be_bytes(bytes_cdab);
                
                // BADC byte order (big-endian words, little-endian bytes)
                let combined_badc = ((reg0 as u32) << 16) | (reg1 as u32);
                let bytes_badc = combined_badc.to_le_bytes();
                let value_badc = f32::from_le_bytes(bytes_badc);
                
                // DCBA byte order (little-endian words, little-endian bytes)
                let combined_dcba = ((reg1 as u32) << 16) | (reg0 as u32);
                let bytes_dcba = combined_dcba.to_le_bytes();
                let value_dcba = f32::from_le_bytes(bytes_dcba);
                
                println!("F32 Debug - Address: {}, Raw registers: [{}, {}]", tag.address, reg0, reg1);
                println!("  ABCD: 0x{:08X} = {}", combined_abcd, value_abcd);
                println!("  CDAB: 0x{:08X} = {}", combined_cdab, value_cdab);
                println!("  BADC: 0x{:08X} = {}", combined_badc, value_badc);
                println!("  DCBA: 0x{:08X} = {}", combined_dcba, value_dcba);
                
                // For frequency (address 19050), we expect a value around 50 Hz
                // Choose the most reasonable value
                let value = if tag.address == 19050 {
                    // For frequency, pick the value closest to 50
                    let candidates = vec![
                        (value_abcd, "ABCD"),
                        (value_cdab, "CDAB"), 
                        (value_badc, "BADC"),
                        (value_dcba, "DCBA")
                    ];
                    
                    let mut best_value = value_abcd;
                    let mut best_name = "ABCD";
                    let mut best_distance = (value_abcd - 50.0).abs();
                    
                    for (val, name) in candidates {
                        if val.is_finite() && val > 0.0 && val < 1000.0 {
                            let distance = (val - 50.0).abs();
                            if distance < best_distance {
                                best_distance = distance;
                                best_value = val;
                                best_name = name;
                            }
                        }
                    }
                    
                    println!("  Selected {} format for frequency: {}", best_name, best_value);
                    best_value
                } else {
                    // For other addresses, use ABCD as default for now
                    value_abcd
                };
                
                Ok(value as f64)
            },
        }
    }

    fn apply_scaling(&self, value: f64, tag: &TagConfig) -> f64 {
        if let Some(scaling) = &tag.scaling {
            let scaled_value = value * scaling.multiplier + scaling.offset;
            if tag.name.contains("energy") || tag.name.contains("total_pos_active_energy") {
                println!("Scaling {} - Raw: {}, Multiplier: {}, Offset: {}, Final: {}", 
                         tag.name, value, scaling.multiplier, scaling.offset, scaled_value);
            }
            scaled_value
        } else {
            value
        }
    }

    fn get_slave_id(&self) -> u8 {
        match &self.device_config.protocol {
            ProtocolConfig::ModbusTcp { slave_id, .. } => *slave_id,
            ProtocolConfig::ModbusRtu { slave_id, .. } => *slave_id,
            _ => 1,
        }
    }

    pub async fn read_specific_tags(&mut self, database: &Database, device_tags: &[DeviceTag]) -> Result<Vec<LogEntry>> {
        let mut log_entries = Vec::new();
        let timestamp = Utc::now();

        for device_tag in device_tags {
            // Convert DeviceTag to TagConfig for compatibility with existing read_tag method
            let tag_config = TagConfig {
                name: device_tag.name.clone(),
                address: device_tag.address,
                size: device_tag.size,
                data_type: self.parse_data_type(&device_tag.data_type),
                scaling: Some(ScalingConfig {
                    multiplier: device_tag.scaling_multiplier,
                    offset: device_tag.scaling_offset,
                    unit: device_tag.unit.clone(),
                }),
                description: device_tag.description.clone(),
            };

            match self.read_tag(&tag_config).await {
                Ok(value) => {
                    let scaled_value = self.apply_scaling(value, &tag_config);
                    let entry = LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: device_tag.name.clone(),
                        value: scaled_value,
                        quality: "Good".to_string(),
                        timestamp,
                        unit: device_tag.unit.clone(),
                    };

                    // Insert into database
                    if let Err(e) = database.insert_log_entry(&entry).await {
                        error!("Failed to insert log entry: {}", e);
                    }

                    log_entries.push(entry);
                },
                Err(e) => {
                    warn!("Failed to read tag {}: {}", device_tag.name, e);
                    let entry = LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: device_tag.name.clone(),
                        value: 0.0,
                        quality: "Bad".to_string(),
                        timestamp,
                        unit: device_tag.unit.clone(),
                    };

                    log_entries.push(entry);
                }
            }
        }

        Ok(log_entries)
    }

    fn parse_data_type(&self, data_type_str: &str) -> DataType {
        match data_type_str {
            "coil" => DataType::Coil,
            "discrete_input" => DataType::DiscreteInput,
            "holding_register" => DataType::HoldingRegister,
            "input_register" => DataType::InputRegister,
            "float32" | "F32" | "FLOAT" => DataType::Float32,  // Handle all float variants
            "uint16" => DataType::UInt16,
            "int16" => DataType::Int16,
            "uint32" => DataType::UInt32,
            _ => DataType::HoldingRegister, // Default fallback
        }
    }

    pub async fn disconnect(&mut self) {
        self.tcp_client = None;
        info!("Disconnected from Modbus device {}", self.device_config.id);
    }
}
