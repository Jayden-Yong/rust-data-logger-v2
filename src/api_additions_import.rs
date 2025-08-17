use axum::{
    extract::Multipart,
    http::StatusCode,
};
use serde_json::Value;
use std::io::Read;

pub async fn import_tag_templates(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    info!("Importing tag templates for model: {}", model_id);

    // Check if device model exists
    if let Ok(Some(_)) = state.database.get_device_model(&model_id).await {
        while let Some(field) = multipart.next_field().await.map_err(|e| {
            error!("Failed to read multipart form: {}", e);
            StatusCode::BAD_REQUEST
        })? {
            if field.name() == Some("file") {
                let data = field.bytes().await.map_err(|e| {
                    error!("Failed to read file data: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                let templates: Vec<TagTemplate> = match serde_json::from_slice(&data) {
                    Ok(templates) => templates,
                    Err(e) => {
                        error!("Failed to parse tag templates: {}", e);
                        return Ok(Json(ApiResponse::error(format!(
                            "Invalid tag template format: {}",
                            e
                        ))));
                    }
                };

                // Delete existing templates
                if let Err(e) = state.database.delete_tag_templates(&model_id).await {
                    error!("Failed to delete existing templates: {}", e);
                    return Ok(Json(ApiResponse::error(format!(
                        "Failed to delete existing templates: {}",
                        e
                    ))));
                }

                // Import new templates
                if let Err(e) = state.database.create_tag_templates(&model_id, &templates).await {
                    error!("Failed to create tag templates: {}", e);
                    return Ok(Json(ApiResponse::error(format!(
                        "Failed to create tag templates: {}",
                        e
                    ))));
                }

                return Ok(Json(ApiResponse::success(
                    "Tag templates imported successfully".to_string(),
                )));
            }
        }
        Ok(Json(ApiResponse::error("No file provided".to_string())))
    } else {
        Ok(Json(ApiResponse::error("Device model not found".to_string())))
    }
}
