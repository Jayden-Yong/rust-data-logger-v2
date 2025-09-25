use rusqlite::{Connection, params};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;
use tracing::{info, warn};

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
    pub tb_device_id: Option<String>,
    pub tb_group_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTag {
    pub id: Option<i64>,
    pub device_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusTcpTagRegister {
    pub id: Option<i64>,
    pub device_brand: String,
    pub device_model: String,
    pub ava_type: String,
    pub mppt: Option<i32>,
    pub input: Option<i32>,
    pub data_label: String,
    pub address: i32,
    pub size: i32,
    pub modbus_type: String,
    pub divider: f64,
    pub register_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModbusTcpTagRegister {
    pub device_brand: String,
    pub device_model: String,
    pub ava_type: String,
    pub mppt: Option<i32>,
    pub input: Option<i32>,
    pub data_label: String,
    pub address: i32,
    pub size: i32,
    pub modbus_type: String,
    pub divider: f64,
    pub register_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvModbusTcpTagRecord {
    #[serde(rename = "Device Brand")]
    pub device_brand: String,
    #[serde(rename = "Device Model")]
    pub device_model: String,
    #[serde(rename = "AVA Type")]
    pub ava_type: String,
    #[serde(rename = "MPPT")]
    pub mppt: String,
    #[serde(rename = "INPUT")]
    pub input: String,
    #[serde(rename = "Data Label")]
    pub data_label: String,
    #[serde(rename = "Address")]
    pub address: i32,
    #[serde(rename = "Size")]
    pub size: i32,
    #[serde(rename = "Modbus Type")]
    pub modbus_type: String,
    #[serde(rename = "Divider")]
    pub divider: f64,
    #[serde(rename = "Register Type")]
    pub register_type: String,
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
                size INTEGER NOT NULL DEFAULT 1,
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

        // Add size column to device_tags if it doesn't exist (migration)
        let _ = conn.execute(
            "ALTER TABLE device_tags ADD COLUMN size INTEGER DEFAULT 1",
            [],
        ); // Ignore error if column already exists
        
        // Add tb_device_id column to devices table if it doesn't exist (migration)
        let _ = conn.execute(
            "ALTER TABLE devices ADD COLUMN tb_device_id TEXT",
            [],
        ); // Ignore error if column already exists
        
        // Add tb_group_id column to devices table if it doesn't exist (migration)
        let _ = conn.execute(
            "ALTER TABLE devices ADD COLUMN tb_group_id TEXT",
            [],
        ); // Ignore error if column already exists
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

        // Modbus TCP tag registers table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS modbus_tcp_tag_registers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_brand TEXT NOT NULL,
                device_model TEXT NOT NULL,
                ava_type TEXT NOT NULL,
                mppt INTEGER,
                input INTEGER,
                data_label TEXT NOT NULL,
                address INTEGER NOT NULL,
                size INTEGER NOT NULL,
                modbus_type TEXT NOT NULL,
                divider REAL NOT NULL DEFAULT 1.0,
                register_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(device_brand, device_model, address, mppt, input)
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

        // Modbus TCP tag registers indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_modbus_tcp_device ON modbus_tcp_tag_registers(device_brand, device_model)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_modbus_tcp_address ON modbus_tcp_tag_registers(address)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_modbus_tcp_ava_type ON modbus_tcp_tag_registers(ava_type)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_modbus_tcp_mppt_input ON modbus_tcp_tag_registers(mppt, input)",
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

    pub async fn create_device_model(&self, name: &str, manufacturer: Option<&str>, protocol_type: &str, description: Option<&str>) -> Result<DeviceModel> {
        let conn = self.connection.lock().await;
        let model_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let created_str = now.to_rfc3339();
        let updated_str = now.to_rfc3339();

        conn.execute(
            "INSERT INTO device_models (id, name, description, manufacturer, protocol_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                model_id,
                name,
                description,
                manufacturer,
                protocol_type,
                created_str,
                updated_str
            ],
        )?;

        Ok(DeviceModel {
            id: model_id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            manufacturer: manufacturer.map(|s| s.to_string()),
            protocol_type: protocol_type.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn create_tag_template(&self, template: &TagTemplate) -> Result<TagTemplate> {
        let conn = self.connection.lock().await;

        let _result = conn.execute(
            "INSERT INTO tag_templates 
             (model_id, name, address, data_type, description, scaling_multiplier, scaling_offset, unit, read_only)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                template.model_id,
                template.name,
                template.address as i32,
                template.data_type,
                template.description,
                template.scaling_multiplier,
                template.scaling_offset,
                template.unit,
                template.read_only
            ],
        )?;

        let id = conn.last_insert_rowid();

        let mut new_template = template.clone();
        new_template.id = Some(id);

        Ok(new_template)
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
        let _deleted_models = stmt.execute([model_id])?;
        
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
             (id, name, model_id, enabled, polling_interval_ms, timeout_ms, retry_count, protocol_config, tb_device_id, tb_group_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                device.id,
                device.name,
                device.model_id,
                device.enabled,
                device.polling_interval_ms,
                device.timeout_ms,
                device.retry_count,
                device.protocol_config,
                device.tb_device_id,
                device.tb_group_id,
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
                    protocol_config, tb_device_id, tb_group_id, created_at, updated_at 
             FROM devices ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get("created_at")?;
            let updated_str: String = row.get("updated_at")?;
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
                tb_device_id: row.get("tb_device_id")?,
                tb_group_id: row.get("tb_group_id")?,
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
                    protocol_config, tb_device_id, tb_group_id, created_at, updated_at 
             FROM devices WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map([device_id], |row| {
            let created_str: String = row.get(10)?;
            let updated_str: String = row.get(11)?;
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
                tb_device_id: row.get("tb_device_id")?,
                tb_group_id: row.get("tb_group_id")?,
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
                 (device_id, name, address, size, data_type, description, scaling_multiplier, scaling_offset, unit, read_only, enabled, schedule_group_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    device_id,
                    tag.name,
                    tag.address,
                    tag.size,
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
            "SELECT id, device_id, name, address, size, data_type, description, 
                    scaling_multiplier, scaling_offset, unit, read_only, enabled, schedule_group_id
             FROM device_tags WHERE device_id = ?1 ORDER BY address"
        )?;

        let rows = stmt.query_map([device_id], |row| {
            Ok(DeviceTag {
                id: Some(row.get(0)?),
                device_id: row.get(1)?,
                name: row.get(2)?,
                address: row.get::<_, i32>(3)? as u16,
                size: row.get(4)?,
                data_type: row.get(5)?,
                description: row.get(6)?,
                scaling_multiplier: row.get(7)?,
                scaling_offset: row.get(8)?,
                unit: row.get(9)?,
                read_only: row.get(10)?,
                enabled: row.get(11)?,
                schedule_group_id: row.get(12)?,
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
                 timeout_ms = ?5, retry_count = ?6, protocol_config = ?7, tb_device_id = ?8, tb_group_id = ?9, updated_at = ?10
             WHERE id = ?11",
            params![
                device.name,
                device.model_id,
                device.enabled,
                device.polling_interval_ms,
                device.timeout_ms,
                device.retry_count,
                device.protocol_config,
                device.tb_device_id,
                device.tb_group_id,
                updated_str,
                device.id
            ],
        )?;

        Ok(())
    }

    /// Get unsynced devices (devices without ThingsBoard device ID and group ID)
    pub async fn get_unsynced_devices(&self) -> Result<Vec<DeviceInstance>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, model_id, enabled, polling_interval_ms, timeout_ms, retry_count, 
                    protocol_config, tb_device_id, tb_group_id, created_at, updated_at 
             FROM devices 
             WHERE (tb_device_id IS NULL OR tb_device_id = '') AND (tb_group_id IS NULL OR tb_group_id = '')
             ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get("created_at")?;
            let updated_str: String = row.get("updated_at")?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(8, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(9, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceInstance {
                id: row.get("id")?,
                name: row.get("name")?,
                model_id: row.get("model_id")?,
                enabled: row.get("enabled")?,
                polling_interval_ms: row.get::<_, i32>("polling_interval_ms")? as u32,
                timeout_ms: row.get::<_, i32>("timeout_ms")? as u32,
                retry_count: row.get::<_, i32>("retry_count")? as u32,
                protocol_config: row.get("protocol_config")?,
                tb_device_id: row.get("tb_device_id")?,
                tb_group_id: row.get("tb_group_id")?,
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

    /// Get devices by ThingsBoard group ID
    pub async fn get_devices_by_group_id(&self, group_id: &str) -> Result<Vec<DeviceInstance>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, name, model_id, enabled, polling_interval_ms, timeout_ms, retry_count, 
                    protocol_config, tb_device_id, tb_group_id, created_at, updated_at 
             FROM devices 
             WHERE tb_group_id = ?1
             ORDER BY name"
        )?;

        let rows = stmt.query_map([group_id], |row| {
            let created_str: String = row.get("created_at")?;
            let updated_str: String = row.get("updated_at")?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(8, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(9, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(DeviceInstance {
                id: row.get("id")?,
                name: row.get("name")?,
                model_id: row.get("model_id")?,
                enabled: row.get("enabled")?,
                polling_interval_ms: row.get::<_, i32>("polling_interval_ms")? as u32,
                timeout_ms: row.get::<_, i32>("timeout_ms")? as u32,
                retry_count: row.get::<_, i32>("retry_count")? as u32,
                protocol_config: row.get("protocol_config")?,
                tb_device_id: row.get("tb_device_id")?,
                tb_group_id: row.get("tb_group_id")?,
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

    pub async fn delete_device_tags(&self, device_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "DELETE FROM device_tags WHERE device_id = ?1",
            params![device_id],
        )?;

        Ok(())
    }

    pub async fn delete_device(&self, device_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        // Start a transaction to ensure data consistency
        conn.execute("BEGIN TRANSACTION", [])?;
        
        // First, check if the device exists
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM devices WHERE id = ?1")?;
        let count: i64 = stmt.query_row([device_id], |row| row.get(0))?;
        
        if count == 0 {
            conn.execute("ROLLBACK", [])?;
            return Err(anyhow::anyhow!("Device not found"));
        }
        
        // Delete all device tags first
        let mut stmt = conn.prepare("DELETE FROM device_tags WHERE device_id = ?1")?;
        let deleted_tags = stmt.execute([device_id])?;
        
        // Delete device status
        let mut stmt = conn.prepare("DELETE FROM device_status WHERE device_id = ?1")?;
        let _deleted_status = stmt.execute([device_id])?;
        
        // Finally, delete the device itself
        let mut stmt = conn.prepare("DELETE FROM devices WHERE id = ?1")?;
        let _deleted_device = stmt.execute([device_id])?;
        
        // Commit the transaction
        conn.execute("COMMIT", [])?;
        
        info!("Deleted device {} and {} associated tags", device_id, deleted_tags);
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

    // Modbus TCP Tag Register CRUD operations
    pub async fn create_modbus_tcp_tag_register(&self, tag_register: &CreateModbusTcpTagRegister) -> Result<ModbusTcpTagRegister> {
        let conn = self.connection.lock().await;
        let now = Utc::now();
        let created_str = now.to_rfc3339();
        let updated_str = now.to_rfc3339();

        conn.execute(
            "INSERT INTO modbus_tcp_tag_registers (
                device_brand, device_model, ava_type, mppt, input, data_label, 
                address, size, modbus_type, divider, register_type, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                tag_register.device_brand,
                tag_register.device_model,
                tag_register.ava_type,
                tag_register.mppt,
                tag_register.input,
                tag_register.data_label,
                tag_register.address,
                tag_register.size,
                tag_register.modbus_type,
                tag_register.divider,
                tag_register.register_type,
                created_str,
                updated_str
            ],
        )?;

        let id = conn.last_insert_rowid();

        Ok(ModbusTcpTagRegister {
            id: Some(id),
            device_brand: tag_register.device_brand.clone(),
            device_model: tag_register.device_model.clone(),
            ava_type: tag_register.ava_type.clone(),
            mppt: tag_register.mppt,
            input: tag_register.input,
            data_label: tag_register.data_label.clone(),
            address: tag_register.address,
            size: tag_register.size,
            modbus_type: tag_register.modbus_type.clone(),
            divider: tag_register.divider,
            register_type: tag_register.register_type.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn bulk_insert_modbus_tcp_tag_registers(&self, tag_registers: Vec<CreateModbusTcpTagRegister>) -> Result<u64> {
        let conn = self.connection.lock().await;
        let tx = conn.unchecked_transaction()?;
        let now = Utc::now();
        let created_str = now.to_rfc3339();
        let updated_str = now.to_rfc3339();

        let mut inserted_count = 0;

        for tag_register in tag_registers {
            tx.execute(
                "INSERT OR REPLACE INTO modbus_tcp_tag_registers (
                    device_brand, device_model, ava_type, mppt, input, data_label, 
                    address, size, modbus_type, divider, register_type, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    tag_register.device_brand,
                    tag_register.device_model,
                    tag_register.ava_type,
                    tag_register.mppt,
                    tag_register.input,
                    tag_register.data_label,
                    tag_register.address,
                    tag_register.size,
                    tag_register.modbus_type,
                    tag_register.divider,
                    tag_register.register_type,
                    created_str,
                    updated_str
                ],
            )?;
            
            inserted_count += 1;
        }

        tx.commit()?;
        Ok(inserted_count)
    }

    pub async fn get_modbus_tcp_tag_registers_by_device(&self, device_brand: &str, device_model: &str) -> Result<Vec<ModbusTcpTagRegister>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, device_brand, device_model, ava_type, mppt, input, data_label, 
                    address, size, modbus_type, divider, register_type, created_at, updated_at 
             FROM modbus_tcp_tag_registers 
             WHERE device_brand = ?1 AND device_model = ?2 
             ORDER BY ava_type, mppt, input, address ASC"
        )?;

        let rows = stmt.query_map([device_brand, device_model], |row| {
            let created_str: String = row.get(12)?;
            let updated_str: String = row.get(13)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(12, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(13, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(ModbusTcpTagRegister {
                id: Some(row.get(0)?),
                device_brand: row.get(1)?,
                device_model: row.get(2)?,
                ava_type: row.get(3)?,
                mppt: row.get(4)?,
                input: row.get(5)?,
                data_label: row.get(6)?,
                address: row.get(7)?,
                size: row.get(8)?,
                modbus_type: row.get(9)?,
                divider: row.get(10)?,
                register_type: row.get(11)?,
                created_at,
                updated_at,
            })
        })?;

        let mut tag_registers = Vec::new();
        for row in rows {
            tag_registers.push(row?);
        }

        Ok(tag_registers)
    }

    pub async fn get_modbus_tcp_tag_registers_by_model(&self, device_model: &str) -> Result<Vec<ModbusTcpTagRegister>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, device_brand, device_model, ava_type, mppt, input, data_label, 
                    address, size, modbus_type, divider, register_type, created_at, updated_at 
             FROM modbus_tcp_tag_registers 
             WHERE device_model = ?1 
             ORDER BY ava_type, mppt, input, address ASC"
        )?;

        let rows = stmt.query_map([device_model], |row| {
            let created_str: String = row.get(12)?;
            let updated_str: String = row.get(13)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(12, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(13, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(ModbusTcpTagRegister {
                id: Some(row.get(0)?),
                device_brand: row.get(1)?,
                device_model: row.get(2)?,
                ava_type: row.get(3)?,
                mppt: row.get(4)?,
                input: row.get(5)?,
                data_label: row.get(6)?,
                address: row.get(7)?,
                size: row.get(8)?,
                modbus_type: row.get(9)?,
                divider: row.get(10)?,
                register_type: row.get(11)?,
                created_at,
                updated_at,
            })
        })?;

        let mut tag_registers = Vec::new();
        for row in rows {
            tag_registers.push(row?);
        }

        Ok(tag_registers)
    }

    pub async fn get_modbus_tcp_tag_registers_by_model_id(&self, model_id: &str) -> Result<Vec<ModbusTcpTagRegister>> {
        let conn = self.connection.lock().await;
        
        // Join with device_models table to get unique tags by model_id
        // Use DISTINCT to eliminate any potential duplicates
        let mut stmt = conn.prepare(
            "SELECT DISTINCT mtr.id, mtr.device_brand, mtr.device_model, mtr.ava_type, mtr.mppt, mtr.input, 
                    mtr.data_label, mtr.address, mtr.size, mtr.modbus_type, mtr.divider, mtr.register_type, 
                    mtr.created_at, mtr.updated_at 
             FROM modbus_tcp_tag_registers mtr
             JOIN device_models dm ON dm.name = mtr.device_model AND dm.manufacturer = mtr.device_brand
             WHERE dm.id = ?1 
             ORDER BY mtr.ava_type, mtr.mppt, mtr.input, mtr.address ASC"
        )?;

        let rows = stmt.query_map([model_id], |row| {
            let created_str: String = row.get(12)?;
            let updated_str: String = row.get(13)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(12, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(13, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(ModbusTcpTagRegister {
                id: Some(row.get(0)?),
                device_brand: row.get(1)?,
                device_model: row.get(2)?,
                ava_type: row.get(3)?,
                mppt: row.get(4)?,
                input: row.get(5)?,
                data_label: row.get(6)?,
                address: row.get(7)?,
                size: row.get(8)?,
                modbus_type: row.get(9)?,
                divider: row.get(10)?,
                register_type: row.get(11)?,
                created_at,
                updated_at,
            })
        })?;

        let mut tag_registers = Vec::new();
        for row in rows {
            tag_registers.push(row?);
        }

        Ok(tag_registers)
    }

    pub async fn get_all_modbus_tcp_tag_registers(&self) -> Result<Vec<ModbusTcpTagRegister>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT id, device_brand, device_model, ava_type, mppt, input, data_label, 
                    address, size, modbus_type, divider, register_type, created_at, updated_at 
             FROM modbus_tcp_tag_registers 
             ORDER BY device_brand, device_model, ava_type, mppt, input, address ASC"
        )?;

        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(12)?;
            let updated_str: String = row.get(13)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(12, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(13, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(ModbusTcpTagRegister {
                id: Some(row.get(0)?),
                device_brand: row.get(1)?,
                device_model: row.get(2)?,
                ava_type: row.get(3)?,
                mppt: row.get(4)?,
                input: row.get(5)?,
                data_label: row.get(6)?,
                address: row.get(7)?,
                size: row.get(8)?,
                modbus_type: row.get(9)?,
                divider: row.get(10)?,
                register_type: row.get(11)?,
                created_at,
                updated_at,
            })
        })?;

        let mut tag_registers = Vec::new();
        for row in rows {
            tag_registers.push(row?);
        }

        Ok(tag_registers)
    }

    pub async fn delete_modbus_tcp_tag_registers_by_device(&self, device_brand: &str, device_model: &str) -> Result<u64> {
        let conn = self.connection.lock().await;
        
        let result = conn.execute(
            "DELETE FROM modbus_tcp_tag_registers WHERE device_brand = ?1 AND device_model = ?2",
            params![device_brand, device_model],
        )?;

        Ok(result as u64)
    }

    /// Update local device with ThingsBoard device ID after sync
    /// This maintains the relationship between local and ThingsBoard devices
    /// without changing the primary key
    pub async fn update_device_thingsboard_id(&self, local_device_id: &str, thingsboard_device_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        
        // Check if the local device exists
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM devices WHERE id = ?1")?;
        let count: i64 = stmt.query_row([local_device_id], |row| row.get(0))?;
        
        if count == 0 {
            return Err(anyhow::anyhow!("Local device with ID {} not found", local_device_id));
        }
        
        // Update the tb_device_id field
        let updated_str = Utc::now().to_rfc3339();
        let mut stmt = conn.prepare(
            "UPDATE devices SET tb_device_id = ?1, updated_at = ?2 WHERE id = ?3"
        )?;
        stmt.execute([thingsboard_device_id, &updated_str, local_device_id])?;
        
        info!("Successfully updated device {} with ThingsBoard ID: {}", local_device_id, thingsboard_device_id);
        Ok(())
    }

    /// Batch update multiple devices with ThingsBoard device IDs and group IDs after sync
    /// This is more efficient when updating many devices at once
    pub async fn batch_update_devices_thingsboard_ids(&self, device_id_mappings: &[(String, String, String)]) -> Result<Vec<String>> {
        if device_id_mappings.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.connection.lock().await;
        let mut successful_updates = Vec::new();
        let mut failed_updates = Vec::new();
        
        // Start a transaction
        conn.execute("BEGIN TRANSACTION", [])?;
        
        for (local_id, thingsboard_id, group_id) in device_id_mappings {
            // Check if local device exists
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM devices WHERE id = ?1")?;
            let local_exists: i64 = stmt.query_row([local_id], |row| row.get(0))?;
            
            if local_exists == 0 {
                failed_updates.push(format!("Local device {} not found", local_id));
                continue;
            }
            
            // Update both tb_device_id and tb_group_id fields
            let updated_str = Utc::now().to_rfc3339();
            let mut stmt = conn.prepare("UPDATE devices SET tb_device_id = ?1, tb_group_id = ?2, updated_at = ?3 WHERE id = ?4")?;
            
            match stmt.execute([thingsboard_id, group_id, &updated_str, local_id]) {
                Ok(_) => {
                    successful_updates.push(format!("{} -> {} (group: {})", local_id, thingsboard_id, group_id));
                    info!("Updated device {} with ThingsBoard ID: {} and Group ID: {}", local_id, thingsboard_id, group_id);
                }
                Err(e) => {
                    failed_updates.push(format!("Failed to update {}: {}", local_id, e));
                }
            }
        }
        
        if !failed_updates.is_empty() {
            warn!("Some ThingsBoard ID updates failed: {:?}", failed_updates);
        }
        
        // Commit the transaction
        conn.execute("COMMIT", [])?;
        
        info!("Batch ThingsBoard ID update completed. Success: {}, Failed: {}", 
              successful_updates.len(), failed_updates.len());
        
        Ok(successful_updates)
    }

    /// Get device AVA type based on the database flow:
    /// devices.model_id -> device_models.id -> device_models.name -> modbus_tcp_tag_registers.device_model -> modbus_tcp_tag_registers.ava_type
    /// For devices with multiple AVA types, prioritize in order: Inverter, PowerMeter, Meter, MPPT, String
    pub async fn get_device_ava_type(&self, device_id: &str) -> Result<Option<String>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare("
            SELECT DISTINCT mtr.ava_type
            FROM devices d
            JOIN device_models dm ON d.model_id = dm.id
            JOIN modbus_tcp_tag_registers mtr ON dm.name = mtr.device_model
            WHERE d.id = ?1
            ORDER BY
                CASE mtr.ava_type
                    WHEN 'Inverter' THEN 1
                    WHEN 'Weather Station' THEN 2
                    WHEN 'PowerMeter' THEN 3
                    WHEN 'Meter' THEN 4
                    WHEN 'MPPT' THEN 5
                    WHEN 'String' THEN 6
                    ELSE 7
                END
            LIMIT 1
        ")?;
        
        let result = stmt.query_row([device_id], |row| {
            Ok(row.get::<_, String>(0)?)
        });
        
        match result {
            Ok(ava_type) => Ok(Some(ava_type)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get device model name for a given device ID
    pub async fn get_device_model_name(&self, device_id: &str) -> Result<Option<String>> {
        let conn = self.connection.lock().await;
        
        let mut stmt = conn.prepare("
            SELECT dm.name
            FROM devices d
            JOIN device_models dm ON d.model_id = dm.id
            WHERE d.id = ?1
        ")?;
        
        let result = stmt.query_row([device_id], |row| {
            Ok(row.get::<_, String>(0)?)
        });
        
        match result {
            Ok(model_name) => Ok(Some(model_name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
