use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use anyhow::Result;
use tracing::{info, warn, error};
use chrono::Utc;

use crate::config::{AppConfig, DeviceConfig, ProtocolConfig};
use crate::database::{Database, DeviceStatus, DeviceTag, ScheduleGroup};
use crate::modbus::ModbusClient;
use crate::iec104::Iec104Client;

pub struct LoggingService {
    database: Arc<Database>,
    config: Arc<AppConfig>,
    device_tasks: Arc<RwLock<HashMap<String, Vec<JoinHandle<()>>>>>, // Device ID -> list of schedule group tasks
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
        // Get device configuration from database
        let device_instance = self.database.get_device(device_id).await?
            .ok_or_else(|| anyhow::anyhow!("Device not found: {}", device_id))?;

        if !device_instance.enabled {
            warn!("Device {} is disabled, cannot start", device_id);
            return Ok(());
        }

        // Parse protocol config from JSON
        let protocol_config: crate::config::ProtocolConfig = serde_json::from_str(&device_instance.protocol_config)
            .map_err(|e| anyhow::anyhow!("Failed to parse protocol config for device {}: {}", device_id, e))?;

        // Create device config from database instance
        let device_config = crate::config::DeviceConfig {
            id: device_instance.id.clone(),
            name: device_instance.name.clone(),
            enabled: device_instance.enabled,
            protocol: protocol_config,
            polling_interval_ms: device_instance.polling_interval_ms as u64,
            timeout_ms: device_instance.timeout_ms as u64,
            retry_count: device_instance.retry_count,
            tags: Vec::new(), // We'll get tags from database separately
        };

        // Stop existing tasks if running
        self.stop_device(device_id).await?;

        info!("Starting device: {}", device_id);

        // Get device tags from database grouped by schedule group
        let device_tags = self.database.get_device_tags(device_id).await?;
        let schedule_groups = self.database.get_schedule_groups().await?;
        
        // Group tags by schedule group
        let mut schedule_group_tags: HashMap<String, (ScheduleGroup, Vec<DeviceTag>)> = HashMap::new();
        
        for tag in device_tags {
            if !tag.enabled {
                continue;
            }
            
            let schedule_group_id = tag.schedule_group_id
                .clone()
                .unwrap_or_else(|| "medium_freq".to_string()); // Default fallback
                
            if let Some(schedule_group) = schedule_groups.iter().find(|sg| sg.id == schedule_group_id) {
                if schedule_group.enabled {
                    schedule_group_tags
                        .entry(schedule_group_id.clone())
                        .or_insert_with(|| (schedule_group.clone(), Vec::new()))
                        .1
                        .push(tag);
                }
            }
        }

        if schedule_group_tags.is_empty() {
            warn!("No enabled tags with valid schedule groups found for device: {}", device_id);
            return Ok(());
        }

        let database = self.database.clone();
        let device_clients = self.device_clients.clone();
        let device_config_clone = device_config.clone();

        let mut tasks = Vec::new();

        // Create a task for each schedule group that has tags
        for (_schedule_group_id, (schedule_group, tags)) in schedule_group_tags {
            let database_clone = database.clone();
            let device_clients_clone = device_clients.clone();
            let device_config_task = device_config_clone.clone();
            let device_id_clone = device_id.to_string();

            let task = tokio::spawn(async move {
                Self::schedule_group_loop(
                    device_id_clone,
                    device_config_task,
                    schedule_group,
                    tags,
                    database_clone,
                    device_clients_clone,
                ).await;
            });

            tasks.push(task);
        }

        self.device_tasks.write().await.insert(device_id.to_string(), tasks);

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

        // Stop all tasks for this device
        if let Some(tasks) = self.device_tasks.write().await.remove(device_id) {
            for task in tasks {
                task.abort();
            }
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

    async fn schedule_group_loop(
        device_id: String,
        device_config: DeviceConfig,
        schedule_group: ScheduleGroup,
        tags: Vec<DeviceTag>,
        database: Arc<Database>,
        device_clients: Arc<Mutex<HashMap<String, DeviceClient>>>,
    ) {
        let mut connection_count = 0;
        let polling_interval = tokio::time::Duration::from_millis(schedule_group.polling_interval_ms as u64);

        info!(
            "Starting schedule group '{}' for device '{}' with {} tags, polling every {}ms",
            schedule_group.name, device_id, tags.len(), schedule_group.polling_interval_ms
        );

        loop {
            // Create client if not exists (shared across all schedule groups for a device)
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
                    
                    // Update status to connected (only on first connection)
                    if connection_count == 1 {
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
                    }

                    // Start polling loop for this schedule group
                    Self::schedule_group_polling_loop(
                        &mut client,
                        &device_config,
                        &schedule_group,
                        &tags,
                        &database,
                        polling_interval,
                    ).await;
                },
                Err(e) => {
                    error!("Failed to connect to device {} for schedule group {}: {}", 
                           device_id, schedule_group.name, e);
                    
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

            // Wait before retry (use longer interval for failed connections)
            tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        }
    }

    async fn schedule_group_polling_loop(
        client: &mut DeviceClient,
        device_config: &DeviceConfig,
        schedule_group: &ScheduleGroup,
        tags: &[DeviceTag],
        database: &Database,
        polling_interval: tokio::time::Duration,
    ) {
        let mut retry_count = 0;

        info!(
            "Starting polling loop for device '{}', schedule group '{}' with {} tags",
            device_config.id, schedule_group.name, tags.len()
        );

        loop {
            let result = match client {
                DeviceClient::Modbus(modbus) => modbus.read_specific_tags(database, tags).await,
                DeviceClient::Iec104(iec104) => iec104.read_specific_tags(database, tags).await,
            };

            match result {
                Ok(log_entries) => {
                    info!(
                        "Read {} values from device '{}' schedule group '{}'",
                        log_entries.len(), device_config.id, schedule_group.name
                    );
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
                    warn!(
                        "Failed to read from device '{}' schedule group '{}': {}",
                        device_config.id, schedule_group.name, e
                    );
                    retry_count += 1;

                    if retry_count >= device_config.retry_count {
                        error!(
                            "Max retries reached for device '{}' schedule group '{}'",
                            device_config.id, schedule_group.name
                        );
                        break;
                    }
                }
            }

            // Wait for next poll using schedule group interval
            tokio::time::sleep(polling_interval).await;
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
        if let Some(tasks) = self.device_tasks.read().await.get(device_id) {
            !tasks.is_empty()
        } else {
            false
        }
    }
}
