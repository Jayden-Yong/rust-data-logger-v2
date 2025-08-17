use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Add the functions below to impl Database in database.rs
impl Database {
    pub async fn create_device_model(&self, device_model: &DeviceModel) -> Result<DeviceModel> {
        let conn = self.connection.lock().await;
        let id = device_model.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
        let created_str = device_model.created_at.to_rfc3339();
        let updated_str = device_model.updated_at.to_rfc3339();

        conn.execute(
            "INSERT INTO device_models (id, name, description, manufacturer, protocol_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                device_model.name,
                device_model.description,
                device_model.manufacturer,
                device_model.protocol_type,
                created_str,
                updated_str
            ],
        )?;

        // Return the created model with the generated ID
        Ok(DeviceModel {
            id,
            name: device_model.name.clone(),
            description: device_model.description.clone(),
            manufacturer: device_model.manufacturer.clone(),
            protocol_type: device_model.protocol_type.clone(),
            created_at: device_model.created_at,
            updated_at: device_model.updated_at,
        })
    }
}
