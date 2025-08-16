use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{Result, anyhow};
use tracing::{info, warn, error};
use chrono::Utc;
use bytes::{Bytes, BytesMut, BufMut};

use crate::config::{DeviceConfig, ProtocolConfig};
use crate::database::{LogEntry, Database};

// IEC 104 Protocol constants
const START_BYTE: u8 = 0x68;
const APDU_MIN_LEN: u8 = 4;

// APCI types
const I_FORMAT: u8 = 0x00;
const S_FORMAT: u8 = 0x01;
const U_FORMAT: u8 = 0x03;

// U-format commands
const STARTDT_ACT: u8 = 0x07;
const STARTDT_CON: u8 = 0x0B;
const STOPDT_ACT: u8 = 0x13;
const STOPDT_CON: u8 = 0x23;
const TESTFR_ACT: u8 = 0x43;
const TESTFR_CON: u8 = 0x83;

// ASDU Types
const M_SP_NA_1: u8 = 1;  // Single-point information
const M_DP_NA_1: u8 = 3;  // Double-point information
const M_ME_NA_1: u8 = 9;  // Measured value, normalized value
const M_ME_NB_1: u8 = 11; // Measured value, scaled value
const M_ME_NC_1: u8 = 13; // Measured value, short floating point value

pub struct Iec104Client {
    device_config: DeviceConfig,
    stream: Option<TcpStream>,
    send_sequence: u16,
    receive_sequence: u16,
}

impl Iec104Client {
    pub fn new(device_config: DeviceConfig) -> Self {
        Self {
            device_config,
            stream: None,
            send_sequence: 0,
            receive_sequence: 0,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        if let ProtocolConfig::Iec104 { host, port, .. } = &self.device_config.protocol {
            let socket_addr: SocketAddr = format!("{}:{}", host, port).parse()?;
            info!("Connecting to IEC 104 device at {}", socket_addr);

            let stream = TcpStream::connect(socket_addr).await
                .map_err(|e| anyhow!("Failed to connect to IEC 104 device: {}", e))?;

            self.stream = Some(stream);
            
            // Send STARTDT_ACT to activate data transfer
            self.send_u_format(STARTDT_ACT).await?;
            
            // Wait for STARTDT_CON
            let response = self.receive_frame().await?;
            if let Some(u_type) = self.parse_u_format(&response) {
                if u_type == STARTDT_CON {
                    info!("IEC 104 data transfer activated");
                } else {
                    return Err(anyhow!("Unexpected U-format response: {}", u_type));
                }
            } else {
                return Err(anyhow!("Expected U-format STARTDT_CON"));
            }

            info!("Successfully connected to IEC 104 device");
            Ok(())
        } else {
            Err(anyhow!("Invalid protocol for IEC 104 client"))
        }
    }

    pub async fn read_tags(&mut self, database: &Database) -> Result<Vec<LogEntry>> {
        let mut log_entries = Vec::new();
        let timestamp = Utc::now();

        // Send interrogation command to get all current values
        self.send_interrogation().await?;

        // Read multiple frames to get all data
        for _ in 0..10 { // Limit to prevent infinite loop
            match tokio::time::timeout(
                tokio::time::Duration::from_millis(1000),
                self.receive_frame()
            ).await {
                Ok(Ok(frame)) => {
                    if let Some(entries) = self.parse_data_frame(&frame, timestamp) {
                        for entry in entries {
                            // Insert into database
                            if let Err(e) = database.insert_log_entry(&entry).await {
                                error!("Failed to insert log entry: {}", e);
                            }
                            log_entries.push(entry);
                        }
                    }
                },
                Ok(Err(e)) => {
                    error!("Error receiving frame: {}", e);
                    break;
                },
                Err(_) => {
                    // Timeout - no more data
                    break;
                }
            }
        }

        Ok(log_entries)
    }

    async fn send_u_format(&mut self, control: u8) -> Result<()> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut frame = BytesMut::new();
        frame.put_u8(START_BYTE);
        frame.put_u8(4); // Length
        frame.put_u8(control);
        frame.put_u8(0);
        frame.put_u8(0);
        frame.put_u8(0);

        stream.write_all(&frame).await?;
        Ok(())
    }

    async fn send_interrogation(&mut self) -> Result<()> {
        let common_address = self.get_common_address();
        
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow!("Not connected"))?;

        let mut frame = BytesMut::new();
        frame.put_u8(START_BYTE);
        frame.put_u8(14); // Length
        
        // APCI
        frame.put_u16_le(self.send_sequence << 1); // Send sequence number
        frame.put_u16_le(self.receive_sequence << 1); // Receive sequence number
        
        // ASDU
        frame.put_u8(100); // C_IC_NA_1 - Interrogation command
        frame.put_u8(0x01); // SQ=0, Number of objects=1
        frame.put_u8(0x06); // COT=6 (activation)
        frame.put_u8(0); // Originator address
        frame.put_u16_le(common_address); // Common address
        frame.put_u8(0); // Information object address (3 bytes)
        frame.put_u8(0);
        frame.put_u8(0);
        frame.put_u8(20); // QOI=20 (station interrogation)

        stream.write_all(&frame).await?;
        self.send_sequence = (self.send_sequence + 1) % 32768;
        Ok(())
    }

    async fn receive_frame(&mut self) -> Result<Bytes> {
        let stream = self.stream.as_mut()
            .ok_or_else(|| anyhow!("Not connected"))?;

        // Read start byte
        let mut start = [0u8; 1];
        stream.read_exact(&mut start).await?;
        
        if start[0] != START_BYTE {
            return Err(anyhow!("Invalid start byte: {}", start[0]));
        }

        // Read length
        let mut length = [0u8; 1];
        stream.read_exact(&mut length).await?;
        let len = length[0] as usize;

        if len < APDU_MIN_LEN as usize {
            return Err(anyhow!("Frame too short: {}", len));
        }

        // Read the rest of the frame
        let mut frame_data = vec![0u8; len];
        stream.read_exact(&mut frame_data).await?;

        let mut full_frame = BytesMut::new();
        full_frame.put_u8(START_BYTE);
        full_frame.put_u8(length[0]);
        full_frame.put_slice(&frame_data);

        Ok(full_frame.freeze())
    }

    fn parse_u_format(&self, frame: &Bytes) -> Option<u8> {
        if frame.len() >= 6 {
            let control = frame[2];
            if (control & 0x03) == U_FORMAT {
                return Some(control);
            }
        }
        None
    }

    fn parse_data_frame(&self, frame: &Bytes, timestamp: chrono::DateTime<Utc>) -> Option<Vec<LogEntry>> {
        if frame.len() < 6 {
            return None;
        }

        let control = frame[2];
        if (control & 0x01) != I_FORMAT {
            return None; // Not an I-format frame
        }

        if frame.len() < 12 {
            return None; // Too short for ASDU
        }

        let type_id = frame[6];
        let _vsq = frame[7];
        let _cot = frame[8];
        let _common_addr = u16::from_le_bytes([frame[10], frame[11]]);

        let mut entries = Vec::new();

        match type_id {
            M_ME_NC_1 => {
                // Short floating point value
                if frame.len() >= 19 {
                    let ioa = u32::from_le_bytes([frame[12], frame[13], frame[14], 0]);
                    let value_bytes = [frame[15], frame[16], frame[17], frame[18]];
                    let value = f32::from_le_bytes(value_bytes) as f64;

                    entries.push(LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: format!("float_{}", ioa),
                        value,
                        quality: "Good".to_string(),
                        timestamp,
                        unit: None,
                    });
                }
            },
            M_ME_NB_1 => {
                // Scaled value
                if frame.len() >= 17 {
                    let ioa = u32::from_le_bytes([frame[12], frame[13], frame[14], 0]);
                    let value = i16::from_le_bytes([frame[15], frame[16]]) as f64;

                    entries.push(LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: format!("scaled_{}", ioa),
                        value,
                        quality: "Good".to_string(),
                        timestamp,
                        unit: None,
                    });
                }
            },
            M_SP_NA_1 => {
                // Single point
                if frame.len() >= 16 {
                    let ioa = u32::from_le_bytes([frame[12], frame[13], frame[14], 0]);
                    let siq = frame[15];
                    let value = if (siq & 0x01) != 0 { 1.0 } else { 0.0 };

                    entries.push(LogEntry {
                        id: None,
                        device_id: self.device_config.id.clone(),
                        tag_name: format!("sp_{}", ioa),
                        value,
                        quality: "Good".to_string(),
                        timestamp,
                        unit: None,
                    });
                }
            },
            _ => {
                warn!("Unsupported ASDU type: {}", type_id);
            }
        }

        if entries.is_empty() {
            None
        } else {
            Some(entries)
        }
    }

    fn get_common_address(&self) -> u16 {
        if let ProtocolConfig::Iec104 { common_address, .. } = &self.device_config.protocol {
            *common_address
        } else {
            1
        }
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            // Send STOPDT_ACT before disconnecting
            self.send_u_format(STOPDT_ACT).await?;
            self.stream = None;
            info!("Disconnected from IEC 104 device {}", self.device_config.id);
        }
        Ok(())
    }
}
