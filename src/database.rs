use rusqlite::{Connection, params};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Option<i64>,
    pub device_id: String,
    pub tag_name: String,
    pub value: f64,
    pub quality: String,
    pub timestamp: DateTime<Utc>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub device_id: String,
    pub status: String,
    pub last_update: DateTime<Utc>,
    pub error_message: Option<String>,
    pub connection_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceModel {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub manufacturer: Option<String>,
    pub protocol_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagTemplate {
    pub id: Option<i64>,
    pub model_id: String,
    pub name: String,
    pub address: u16,
    pub data_type: String,
    pub description: Option<String>,
    pub scaling_multiplier: f64,
    pub scaling_offset: f64,
    pub unit: Option<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInstance {
    pub id: String,
    pub name: String,
    pub model_id: Option<String>,
    pub enabled: bool,
    pub polling_interval_ms: u32,
    pub timeout_ms: u32,
    pub retry_count: u32,
    pub protocol_config: String, // JSON serialized protocol config
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTag {
    pub id: Option<i64>,
    pub device_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleGroup {
    pub id: String,
    pub name: String,
    pub polling_interval_ms: u32,
    pub description: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS log_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                tag_name TEXT NOT NULL,
                value REAL NOT NULL,
                quality TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                unit TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS device_status (
                device_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                last_update TEXT NOT NULL,
                error_message TEXT,
                connection_count INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Device models table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS device_models (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                manufacturer TEXT,
                protocol_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Tag templates table for device models
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tag_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_id TEXT NOT NULL,
                name TEXT NOT NULL,
                address INTEGER NOT NULL,
                data_type TEXT NOT NULL,
                description TEXT,
                scaling_multiplier REAL DEFAULT 1.0,
                scaling_offset REAL DEFAULT 0.0,
                unit TEXT,
                read_only BOOLEAN DEFAULT FALSE,
                FOREIGN KEY (model_id) REFERENCES device_models (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Device instances table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                model_id TEXT,
                enabled BOOLEAN DEFAULT FALSE,
                polling_interval_ms INTEGER DEFAULT 1000,
                timeout_ms INTEGER DEFAULT 5000,
                retry_count INTEGER DEFAULT 3,
                protocol_config TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (model_id) REFERENCES device_models (id) ON DELETE SET NULL
            )",
            [],
        )?;

        // Device tags table (instances of tag templates)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS device_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                name TEXT NOT NULL,
                address INTEGER NOT NULL,
                data_type TEXT NOT NULL,
                description TEXT,
                scaling_multiplier REAL DEFAULT 1.0,
                scaling_offset REAL DEFAULT 0.0,
                unit TEXT,
                read_only BOOLEAN DEFAULT FALSE,
                enabled BOOLEAN DEFAULT TRUE,
                schedule_group_id TEXT,
                FOREIGN KEY (device_id) REFERENCES devices (id) ON DELETE CASCADE,
                FOREIGN KEY (schedule_group_id) REFERENCES schedule_groups (id) ON DELETE SET NULL
            )",
            [],
        )?;

        // Schedule groups table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schedule_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                polling_interval_ms INTEGER NOT NULL,
                description TEXT,
                enabled BOOLEAN DEFAULT TRUE,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_log_entries_device_timestamp 
             ON log_entries(device_id, timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_log_entries_timestamp 
             ON log_entries(timestamp)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tag_templates_model ON tag_templates(model_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_device_tags_device ON device_tags(device_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_devices_model ON devices(model_id)",
            [],
        )?;

        // Insert default device models
        Self::insert_default_device_models(&conn)?;

        // Insert default schedule groups
        Self::insert_default_schedule_groups(&conn)?;

        info!("Database initialized at {}", db_path);

        Ok(Database {
            connection: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn insert_log_entry(&self, entry: &LogEntry) -> Result<i64> {
        let conn = self.connection.lock().await;
        let timestamp_str = entry.timestamp.to_rfc3339();
        
        conn.execute(
            "INSERT INTO log_entries (device_id, tag_name, value, quality, timestamp, unit)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry.device_id,
                entry.tag_name,
                entry.value,
                entry.quality,
                timestamp_str,
                entry.unit
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub async fn get_log_entries(
        &self,
        device_id: Option<&str>,
        limit: Option<u32>,
        _offset: Option<u32>,
    ) -> Result<Vec<LogEntry>> {
        let conn = self.connection.lock().await;
        
        let (query, params_vec): (String, Vec<String>) = match (device_id, limit) {
            (Some(device_id), Some(limit)) => (
                "SELECT id, device_id, tag_name, value, quality, timestamp, unit 
                 FROM log_entries WHERE device_id = ?1 ORDER BY timestamp DESC LIMIT ?2".to_string(),
                vec![device_id.to_string(), limit.to_string()]
            ),
            (Some(device_id), None) => (
                "SELECT id, device_id, tag_name, value, quality, timestamp, unit 
                 FROM log_entries WHERE device_id = ?1 ORDER BY timestamp DESC".to_string(),
                vec![device_id.to_string()]
            ),
            (None, Some(limit)) => (
                "SELECT id, device_id, tag_name, value, quality, timestamp, unit 
                 FROM log_entries ORDER BY timestamp DESC LIMIT ?1".to_string(),
                vec![limit.to_string()]
            ),
            (None, None) => (
                "SELECT id, device_id, tag_name, value, quality, timestamp, unit 
                 FROM log_entries ORDER BY timestamp DESC".to_string(),
                vec![]
            ),
        };

        let mut stmt = conn.prepare(&query)?;
        
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();
        
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let timestamp_str: String = row.get(5)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(5, "timestamp".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(LogEntry {
                id: Some(row.get(0)?),
                device_id: row.get(1)?,
                tag_name: row.get(2)?,
                value: row.get(3)?,
                quality: row.get(4)?,
                timestamp,
                unit: row.get(6)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    pub async fn update_device_status(&self, status: &DeviceStatus) -> Result<()> {
        let conn = self.connection.lock().await;
        let timestamp_str = status.last_update.to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO device_status 
             (device_id, status, last_update, error_message, connection_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                status.device_id,
                status.status,
                timestamp_str,
                status.error_message,
                status.connection_count
            ],
        )?;

        Ok(())
    }

    pub async fn get_device_status(&self, device_id: &str) -> Result<Option<DeviceStatus>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT device_id, status, last_update, error_message, connection_count 
             FROM device_status WHERE device_id = ?1"
        )?;

        let mut rows = stmt.query_map([device_id], |row| {
            let timestamp_str: String = row.get(2)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(2, "last_update".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceStatus {
                device_id: row.get(0)?,
                status: row.get(1)?,
                last_update: timestamp,
                error_message: row.get(3)?,
                connection_count: row.get(4)?,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub async fn get_all_device_statuses(&self) -> Result<Vec<DeviceStatus>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT device_id, status, last_update, error_message, connection_count 
             FROM device_status"
        )?;

        let rows = stmt.query_map([], |row| {
            let timestamp_str: String = row.get(2)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(2, "last_update".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceStatus {
                device_id: row.get(0)?,
                status: row.get(1)?,
                last_update: timestamp,
                error_message: row.get(3)?,
                connection_count: row.get(4)?,
            })
        })?;

        let mut statuses = Vec::new();
        for row in rows {
            statuses.push(row?);
        }

        Ok(statuses)
    }

    pub async fn cleanup_old_entries(&self, max_entries: u32) -> Result<u32> {
        let conn = self.connection.lock().await;
        
        // First, count total entries
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM log_entries")?;
        let total_count: u32 = stmt.query_row([], |row| row.get(0))?;

        if total_count <= max_entries {
            return Ok(0);
        }

        let entries_to_delete = total_count - max_entries;
        
        // Delete oldest entries
        let deleted = conn.execute(
            "DELETE FROM log_entries WHERE id IN (
                SELECT id FROM log_entries ORDER BY timestamp ASC LIMIT ?1
            )",
            params![entries_to_delete],
        )?;

        info!("Cleaned up {} old log entries", deleted);
        Ok(deleted as u32)
    }

    // Device Model Management
    fn insert_default_device_models(conn: &Connection) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // Custom model (no predefined tags)
        conn.execute(
            "INSERT OR IGNORE INTO device_models (id, name, description, manufacturer, protocol_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "custom",
                "Custom Device",
                "Generic device model without predefined tags",
                "Various",
                "any",
                now,
                now
            ],
        )?;

        // Schneider Modicon M221 PLC
        conn.execute(
            "INSERT OR IGNORE INTO device_models (id, name, description, manufacturer, protocol_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "sungrow_1",
                "Sungrow Inverter",
                "Sungrow Solar Inverter",
                "Sungrow",
                "modbus_tcp",
                now,
                now
            ],
        )?;

        // IEC 104 RTU
        conn.execute(
            "INSERT OR IGNORE INTO device_models (id, name, description, manufacturer, protocol_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "iec104_rtu",
                "IEC 104 RTU",
                "Generic IEC 60870-5-104 Remote Terminal Unit",
                "Various",
                "iec104",
                now,
                now
            ],
        )?;

        // Insert default tag templates
        Self::insert_default_tag_templates(conn)?;

        Ok(())
    }

    fn insert_default_tag_templates(conn: &Connection) -> Result<()> {
        // Schneider M221 PLC tags
        let m221_tags = [
            ("system_status", 1, "uint16", "System status register", 1.0, 0.0, None),
            ("production_count", 100, "uint32", "Production counter", 1.0, 0.0, Some("units")),
            ("temperature_1", 200, "int16", "Temperature sensor 1", 0.1, 0.0, Some("°C")),
            ("temperature_2", 201, "int16", "Temperature sensor 2", 0.1, 0.0, Some("°C")),
            ("pressure_1", 300, "uint16", "Pressure sensor 1", 0.01, 0.0, Some("bar")),
            ("flow_rate", 400, "uint32", "Flow rate measurement", 0.1, 0.0, Some("L/min")),
            ("alarm_status", 500, "uint16", "Alarm status register", 1.0, 0.0, None),
        ];

        for (name, address, data_type, description, multiplier, offset, unit) in m221_tags {
            conn.execute(
                "INSERT OR IGNORE INTO tag_templates 
                 (model_id, name, address, data_type, description, scaling_multiplier, scaling_offset, unit, read_only)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    "sungrow_1",
                    name,
                    address,
                    data_type,
                    description,
                    multiplier,
                    offset,
                    unit,
                    false
                ],
            )?;
        }

        // file to be ingested here


        Ok(())
    }

    // Insert default schedule groups
    fn insert_default_schedule_groups(conn: &Connection) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // High frequency (100ms)
        conn.execute(
            "INSERT OR IGNORE INTO schedule_groups (id, name, polling_interval_ms, description, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "high_freq",
                "High Frequency",
                100,
                "High frequency polling for critical measurements",
                true,
                now,
                now
            ],
        )?;

        // Medium frequency (1000ms)
        conn.execute(
            "INSERT OR IGNORE INTO schedule_groups (id, name, polling_interval_ms, description, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "medium_freq",
                "Medium Frequency", 
                1000,
                "Standard polling frequency for most measurements",
                true,
                now,
                now
            ],
        )?;

        // Low frequency (5000ms)
        conn.execute(
            "INSERT OR IGNORE INTO schedule_groups (id, name, polling_interval_ms, description, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "low_freq",
                "Low Frequency",
                5000,
                "Low frequency polling for status and configuration data",
                true,
                now,
                now
            ],
        )?;

        // Energy monitoring (30000ms)
        conn.execute(
            "INSERT OR IGNORE INTO schedule_groups (id, name, polling_interval_ms, description, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "energy_monitor",
                "Energy Monitoring",
                30000,
                "Energy and power monitoring with 30 second intervals",
                true,
                now,
                now
            ],
        )?;

        Ok(())
    }

    // Device Model CRUD operations
    pub async fn get_device_models(&self) -> Result<Vec<DeviceModel>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, description, manufacturer, protocol_type, created_at, updated_at 
             FROM device_models ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(5, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(6, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceModel {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                manufacturer: row.get(3)?,
                protocol_type: row.get(4)?,
                created_at,
                updated_at,
            })
        })?;

        let mut models = Vec::new();
        for row in rows {
            models.push(row?);
        }

        Ok(models)
    }

    pub async fn get_device_model(&self, model_id: &str) -> Result<Option<DeviceModel>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, description, manufacturer, protocol_type, created_at, updated_at 
             FROM device_models WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([model_id], |row| {
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(5, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(6, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceModel {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                manufacturer: row.get(3)?,
                protocol_type: row.get(4)?,
                created_at,
                updated_at,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    pub async fn get_tag_templates(&self, model_id: &str) -> Result<Vec<TagTemplate>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, model_id, name, address, data_type, description, 
                    scaling_multiplier, scaling_offset, unit, read_only
             FROM tag_templates WHERE model_id = ?1 ORDER BY address"
        )?;

        let rows = stmt.query_map([model_id], |row| {
            Ok(TagTemplate {
                id: Some(row.get(0)?),
                model_id: row.get(1)?,
                name: row.get(2)?,
                address: row.get::<_, i32>(3)? as u16,
                data_type: row.get(4)?,
                description: row.get(5)?,
                scaling_multiplier: row.get(6)?,
                scaling_offset: row.get(7)?,
                unit: row.get(8)?,
                read_only: row.get(9)?,
            })
        })?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row?);
        }

        Ok(templates)
    }

    pub async fn delete_device_model(&self, model_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        // Start a transaction to ensure data consistency
        conn.execute("BEGIN TRANSACTION", [])?;
        
        // First, check if the model exists
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM device_models WHERE id = ?1")?;
        let count: i64 = stmt.query_row([model_id], |row| row.get(0))?;
        
        if count == 0 {
            conn.execute("ROLLBACK", [])?;
            return Err(anyhow::anyhow!("Device model not found"));
        }
        
        // Delete all tag templates associated with this model
        let mut stmt = conn.prepare("DELETE FROM tag_templates WHERE model_id = ?1")?;
        let deleted_tags = stmt.execute([model_id])?;
        
        // Delete any devices that use this model
        let mut stmt = conn.prepare("DELETE FROM devices WHERE model_id = ?1")?;
        let deleted_devices = stmt.execute([model_id])?;
        
        // Finally, delete the device model itself
        let mut stmt = conn.prepare("DELETE FROM device_models WHERE id = ?1")?;
        let deleted_models = stmt.execute([model_id])?;
        
        // Commit the transaction
        conn.execute("COMMIT", [])?;
        
        info!("Deleted device model {}, {} associated tag templates, and {} devices", 
              model_id, deleted_tags, deleted_devices);
        Ok(())
    }

    // Device Instance CRUD operations
    pub async fn create_device(&self, device: &DeviceInstance) -> Result<()> {
        let conn = self.connection.lock().await;
        let created_str = device.created_at.to_rfc3339();
        let updated_str = device.updated_at.to_rfc3339();

        conn.execute(
            "INSERT INTO devices 
             (id, name, model_id, enabled, polling_interval_ms, timeout_ms, retry_count, protocol_config, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                device.id,
                device.name,
                device.model_id,
                device.enabled,
                device.polling_interval_ms,
                device.timeout_ms,
                device.retry_count,
                device.protocol_config,
                created_str,
                updated_str
            ],
        )?;

        Ok(())
    }

    pub async fn get_devices(&self) -> Result<Vec<DeviceInstance>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, model_id, enabled, polling_interval_ms, timeout_ms, retry_count, 
                    protocol_config, created_at, updated_at 
             FROM devices ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(8)?;
            let updated_str: String = row.get(9)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(8, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(9, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceInstance {
                id: row.get(0)?,
                name: row.get(1)?,
                model_id: row.get(2)?,
                enabled: row.get(3)?,
                polling_interval_ms: row.get::<_, i32>(4)? as u32,
                timeout_ms: row.get::<_, i32>(5)? as u32,
                retry_count: row.get::<_, i32>(6)? as u32,
                protocol_config: row.get(7)?,
                created_at,
                updated_at,
            })
        })?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row?);
        }

        Ok(devices)
    }

    pub async fn get_device(&self, device_id: &str) -> Result<Option<DeviceInstance>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, model_id, enabled, polling_interval_ms, timeout_ms, retry_count, 
                    protocol_config, created_at, updated_at 
             FROM devices WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([device_id], |row| {
            let created_str: String = row.get(8)?;
            let updated_str: String = row.get(9)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(8, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(9, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceInstance {
                id: row.get(0)?,
                name: row.get(1)?,
                model_id: row.get(2)?,
                enabled: row.get(3)?,
                polling_interval_ms: row.get::<_, i32>(4)? as u32,
                timeout_ms: row.get::<_, i32>(5)? as u32,
                retry_count: row.get::<_, i32>(6)? as u32,
                protocol_config: row.get(7)?,
                created_at,
                updated_at,
            })
        })?;

        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    // Device Tag CRUD operations
    pub async fn create_device_tags(&self, device_id: &str, tags: &[DeviceTag]) -> Result<()> {
        let conn = self.connection.lock().await;

        for tag in tags {
            conn.execute(
                "INSERT INTO device_tags 
                 (device_id, name, address, data_type, description, scaling_multiplier, scaling_offset, unit, read_only, enabled, schedule_group_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    device_id,
                    tag.name,
                    tag.address,
                    tag.data_type,
                    tag.description,
                    tag.scaling_multiplier,
                    tag.scaling_offset,
                    tag.unit,
                    tag.read_only,
                    tag.enabled,
                    tag.schedule_group_id
                ],
            )?;
        }

        Ok(())
    }

    pub async fn get_device_tags(&self, device_id: &str) -> Result<Vec<DeviceTag>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, device_id, name, address, data_type, description, 
                    scaling_multiplier, scaling_offset, unit, read_only, enabled, schedule_group_id
             FROM device_tags WHERE device_id = ?1 ORDER BY address"
        )?;

        let rows = stmt.query_map([device_id], |row| {
            Ok(DeviceTag {
                id: Some(row.get(0)?),
                device_id: row.get(1)?,
                name: row.get(2)?,
                address: row.get::<_, i32>(3)? as u16,
                data_type: row.get(4)?,
                description: row.get(5)?,
                scaling_multiplier: row.get(6)?,
                scaling_offset: row.get(7)?,
                unit: row.get(8)?,
                read_only: row.get(9)?,
                enabled: row.get(10)?,
                schedule_group_id: row.get(11)?,
            })
        })?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }

        Ok(tags)
    }

    pub async fn update_device(&self, device: &DeviceInstance) -> Result<()> {
        let conn = self.connection.lock().await;
        let updated_str = device.updated_at.to_rfc3339();

        conn.execute(
            "UPDATE devices 
             SET name = ?1, model_id = ?2, enabled = ?3, polling_interval_ms = ?4, 
                 timeout_ms = ?5, retry_count = ?6, protocol_config = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                device.name,
                device.model_id,
                device.enabled,
                device.polling_interval_ms,
                device.timeout_ms,
                device.retry_count,
                device.protocol_config,
                updated_str,
                device.id
            ],
        )?;

        Ok(())
    }

    pub async fn delete_device_tags(&self, device_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "DELETE FROM device_tags WHERE device_id = ?1",
            params![device_id],
        )?;

        Ok(())
    }

    // Schedule Group CRUD operations
    pub async fn get_schedule_groups(&self) -> Result<Vec<ScheduleGroup>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, polling_interval_ms, description, enabled, created_at, updated_at 
             FROM schedule_groups ORDER BY polling_interval_ms"
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(5, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(6, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(ScheduleGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                polling_interval_ms: row.get::<_, i32>(2)? as u32,
                description: row.get(3)?,
                enabled: row.get(4)?,
                created_at,
                updated_at,
            })
        })?;

        let mut schedule_groups = Vec::new();
        for row in rows {
            schedule_groups.push(row?);
        }

        Ok(schedule_groups)
    }

    pub async fn get_schedule_group(&self, id: &str) -> Result<Option<ScheduleGroup>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, polling_interval_ms, description, enabled, created_at, updated_at 
             FROM schedule_groups WHERE id = ?1"
        )?;

        let result = stmt.query_row([id], |row| {
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(5, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(6, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(ScheduleGroup {
                id: row.get(0)?,
                name: row.get(1)?,
                polling_interval_ms: row.get::<_, i32>(2)? as u32,
                description: row.get(3)?,
                enabled: row.get(4)?,
                created_at,
                updated_at,
            })
        });

        match result {
            Ok(schedule_group) => Ok(Some(schedule_group)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn create_schedule_group(&self, schedule_group: &ScheduleGroup) -> Result<()> {
        let conn = self.connection.lock().await;
        let created_str = schedule_group.created_at.to_rfc3339();
        let updated_str = schedule_group.updated_at.to_rfc3339();

        conn.execute(
            "INSERT INTO schedule_groups (id, name, polling_interval_ms, description, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                schedule_group.id,
                schedule_group.name,
                schedule_group.polling_interval_ms,
                schedule_group.description,
                schedule_group.enabled,
                created_str,
                updated_str
            ],
        )?;

        Ok(())
    }

    pub async fn update_schedule_group(&self, schedule_group: &ScheduleGroup) -> Result<()> {
        let conn = self.connection.lock().await;
        let updated_str = schedule_group.updated_at.to_rfc3339();

        conn.execute(
            "UPDATE schedule_groups SET name = ?1, polling_interval_ms = ?2, description = ?3, 
             enabled = ?4, updated_at = ?5 WHERE id = ?6",
            params![
                schedule_group.name,
                schedule_group.polling_interval_ms,
                schedule_group.description,
                schedule_group.enabled,
                updated_str,
                schedule_group.id
            ],
        )?;

        Ok(())
    }

    pub async fn delete_schedule_group(&self, id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "DELETE FROM schedule_groups WHERE id = ?1",
            params![id],
        )?;

        Ok(())
    }
}
