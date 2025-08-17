#[derive(Deserialize)]
pub struct CreateDeviceModelRequest {
    pub name: String,
    pub manufacturer: String,
    pub protocol_type: String,
    pub description: Option<String>,
}

pub async fn create_device_model(
    State(state): State<AppState>,
    Json(request): Json<CreateDeviceModelRequest>,
) -> Result<Json<ApiResponse<DeviceModel>>, StatusCode> {
    let now = chrono::Utc::now();
    
    let model = DeviceModel {
        id: None, // Let the database generate the ID
        name: request.name,
        manufacturer: request.manufacturer,
        protocol_type: request.protocol_type,
        description: request.description,
        created_at: now,
        updated_at: now,
    };

    match state.database.create_device_model(&model).await {
        Ok(created_model) => {
            info!("Created device model: {}", created_model.name);
            Ok(Json(ApiResponse::success(created_model)))
        },
        Err(e) => {
            error!("Failed to create device model: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
