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
}
