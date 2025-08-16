use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use anyhow::Result;
use tracing::{info, warn, error};
use chrono::Utc;

use crate::config::{AppConfig, DeviceConfig, ProtocolConfig};
use crate::database::{Database, DeviceStatus};
use crate::modbus::ModbusClient;
use crate::iec104::Iec104Client;

pub struct LoggingService {
    database: Arc<Database>,
    config: Arc<AppConfig>,
    device_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    device_clients: Arc<Mutex<HashMap<String, DeviceClient>>>,
}

enum DeviceClient {
    Modbus(ModbusClient),
    Iec104(Iec104Client),
}

impl LoggingService {
    pub async fn new(database: Arc<Database>, config: Arc<AppConfig>) -> Result<Self> {
        let service = Self {
            database,
            config: config.clone(),
            device_tasks: Arc::new(RwLock::new(HashMap::new())),
            device_clients: Arc::new(Mutex::new(HashMap::new())),
        };

        // Start enabled devices
        for device in &config.devices {
            if device.enabled {
                if let Err(e) = service.start_device(&device.id).await {
                    error!("Failed to start device {}: {}", device.id, e);
                }
            }
        }

        // Start cleanup task
        service.start_cleanup_task().await;

        Ok(service)
    }

    pub async fn start_device(&self, device_id: &str) -> Result<()> {
        let device_config = self.config.devices.iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_id))?
            .clone();

        // Stop existing task if running
        self.stop_device(device_id).await?;

        info!("Starting device: {}", device_id);

        let database = self.database.clone();
        let device_clients = self.device_clients.clone();
        let device_config_clone = device_config.clone();

        let task = tokio::spawn(async move {
            Self::device_loop(device_config_clone, database, device_clients).await;
        });

        self.device_tasks.write().await.insert(device_id.to_string(), task);

        // Update device status
        let status = DeviceStatus {
            device_id: device_id.to_string(),
            status: "Starting".to_string(),
            last_update: Utc::now(),
            error_message: None,
            connection_count: 0,
        };
        self.database.update_device_status(&status).await?;

        Ok(())
    }

    pub async fn stop_device(&self, device_id: &str) -> Result<()> {
        info!("Stopping device: {}", device_id);

        // Stop the task
        if let Some(task) = self.device_tasks.write().await.remove(device_id) {
            task.abort();
        }

        // Disconnect the client
        let mut clients = self.device_clients.lock().await;
        if let Some(mut client) = clients.remove(device_id) {
            match &mut client {
                DeviceClient::Modbus(modbus) => {
                    modbus.disconnect().await;
                },
                DeviceClient::Iec104(iec104) => {
                    if let Err(e) = iec104.disconnect().await {
                        warn!("Error disconnecting IEC104 client: {}", e);
                    }
                },
            }
        }

        // Update device status
        let status = DeviceStatus {
            device_id: device_id.to_string(),
            status: "Stopped".to_string(),
            last_update: Utc::now(),
            error_message: None,
            connection_count: 0,
        };
        self.database.update_device_status(&status).await?;

        Ok(())
    }

    async fn device_loop(
        device_config: DeviceConfig,
        database: Arc<Database>,
        device_clients: Arc<Mutex<HashMap<String, DeviceClient>>>,
    ) {
        let device_id = device_config.id.clone();
        let mut connection_count = 0;

        loop {
            // Create client if not exists
            let mut clients = device_clients.lock().await;
            if !clients.contains_key(&device_id) {
                let client = match &device_config.protocol {
                    ProtocolConfig::ModbusTcp { .. } | ProtocolConfig::ModbusRtu { .. } => {
                        DeviceClient::Modbus(ModbusClient::new(device_config.clone()))
                    },
                    ProtocolConfig::Iec104 { .. } => {
                        DeviceClient::Iec104(Iec104Client::new(device_config.clone()))
                    },
                };
                clients.insert(device_id.clone(), client);
            }

            let mut client = clients.remove(&device_id).unwrap();
            drop(clients); // Release the lock

            // Try to connect
            let connect_result = match &mut client {
                DeviceClient::Modbus(modbus) => modbus.connect().await,
                DeviceClient::Iec104(iec104) => iec104.connect().await,
            };

            match connect_result {
                Ok(()) => {
                    connection_count += 1;
                    
                    // Update status to connected
                    let status = DeviceStatus {
                        device_id: device_id.clone(),
                        status: "Connected".to_string(),
                        last_update: Utc::now(),
                        error_message: None,
                        connection_count,
                    };
                    if let Err(e) = database.update_device_status(&status).await {
                        error!("Failed to update device status: {}", e);
                    }

                    // Start polling loop
                    Self::polling_loop(&mut client, &device_config, &database).await;
                },
                Err(e) => {
                    error!("Failed to connect to device {}: {}", device_id, e);
                    
                    // Update status to error
                    let status = DeviceStatus {
                        device_id: device_id.clone(),
                        status: "Error".to_string(),
                        last_update: Utc::now(),
                        error_message: Some(e.to_string()),
                        connection_count,
                    };
                    if let Err(e) = database.update_device_status(&status).await {
                        error!("Failed to update device status: {}", e);
                    }
                }
            }

            // Put client back
            device_clients.lock().await.insert(device_id.clone(), client);

            // Wait before retry
            tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        }
    }

    async fn polling_loop(
        client: &mut DeviceClient,
        device_config: &DeviceConfig,
        database: &Database,
    ) {
        let mut retry_count = 0;

        loop {
            let result = match client {
                DeviceClient::Modbus(modbus) => modbus.read_tags(database).await,
                DeviceClient::Iec104(iec104) => iec104.read_tags(database).await,
            };

            match result {
                Ok(log_entries) => {
                    info!("Read {} values from device {}", log_entries.len(), device_config.id);
                    retry_count = 0;

                    // Update status to reading
                    let status = DeviceStatus {
                        device_id: device_config.id.clone(),
                        status: "Reading".to_string(),
                        last_update: Utc::now(),
                        error_message: None,
                        connection_count: 0,
                    };
                    if let Err(e) = database.update_device_status(&status).await {
                        error!("Failed to update device status: {}", e);
                    }
                },
                Err(e) => {
                    warn!("Failed to read from device {}: {}", device_config.id, e);
                    retry_count += 1;

                    if retry_count >= device_config.retry_count {
                        error!("Max retries reached for device {}", device_config.id);
                        break;
                    }
                }
            }

            // Wait for next poll
            tokio::time::sleep(tokio::time::Duration::from_millis(device_config.polling_interval_ms)).await;
        }
    }

    async fn start_cleanup_task(&self) {
        let database = self.database.clone();
        let max_entries = self.config.database.max_log_entries;
        let cleanup_interval = self.config.database.cleanup_interval_hours;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(cleanup_interval as u64 * 3600)
            );

            loop {
                interval.tick().await;
                
                match database.cleanup_old_entries(max_entries).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            info!("Cleaned up {} old log entries", deleted);
                        }
                    },
                    Err(e) => {
                        error!("Failed to cleanup old entries: {}", e);
                    }
                }
            }
        });
    }

    pub async fn get_device_status(&self, device_id: &str) -> Result<Option<DeviceStatus>> {
        self.database.get_device_status(device_id).await
    }

    pub async fn get_all_device_statuses(&self) -> Result<Vec<DeviceStatus>> {
        self.database.get_all_device_statuses().await
    }

    pub async fn is_device_running(&self, device_id: &str) -> bool {
        self.device_tasks.read().await.contains_key(device_id)
    }
}
