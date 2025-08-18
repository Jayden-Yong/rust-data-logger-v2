use axum::{
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use socketioxide::SocketIo;
use std::{sync::Arc, net::SocketAddr};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;

mod config;
mod modbus;
mod iec104;
mod database;
mod logging;
mod api;
mod websocket;
mod csv_parser;

use config::{AppConfig, load_config};
use database::Database;
use logging::LoggingService;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub database: Arc<Database>,
    pub logging_service: Arc<LoggingService>,
}

async fn serve_index() -> impl IntoResponse {
    let html_content = "<!DOCTYPE html>
<html lang=\"en\">
  <head>
    <meta charset=\"utf-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
    <meta name=\"theme-color\" content=\"#000000\" />
    <meta name=\"description\" content=\"AVA Device Logger - Industrial data logging and monitoring\" />
    <title>AVA Device Logger</title>
    <style>
      body {
        margin: 0;
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue', sans-serif;
        -webkit-font-smoothing: antialiased;
        -moz-osx-font-smoothing: grayscale;
      }
      .loading {
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100vh;
        font-size: 18px;
        color: #1890ff;
      }
    </style>
  </head>
  <body>
    <noscript>You need to enable JavaScript to run this app.</noscript>
    <div id=\"root\">
      <div class=\"loading\">
        <div>
          <h2>AVA Device Logger</h2>
          <p>Backend server is running successfully!</p>
          <p><strong>API Base URL:</strong> http://localhost:8080/api</p>
          <br>
          <p><em>For the full web interface, build the React frontend with:</em></p>
          <code>cd web && npm install && npm start</code>
        </div>
      </div>
    </div>
  </body>
</html>";
    Html(html_content)
}

async fn serve_static() -> impl IntoResponse {
    // Fallback to index for client-side routing
    serve_index().await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Arc::new(load_config().await?);
    info!("Configuration loaded successfully");

    // Initialize database
    let database = Arc::new(Database::new(&config.database.path).await?);
    info!("Database initialized");

    // Initialize logging service
    let logging_service = Arc::new(LoggingService::new(
        database.clone(),
        config.clone(),
    ).await?);
    info!("Logging service initialized");

    // Create app state
    let app_state = AppState {
        config: config.clone(),
        database,
        logging_service,
    };

    // Create Socket.IO layer
    let (socket_layer, socket_io) = SocketIo::new_layer();
    
    // Set up Socket.IO event handlers
    socket_io.ns("/", websocket::on_connect);

    // Create router
    let app = Router::new()
        // API routes
        .route("/api/config", get(api::get_config).post(api::update_config))
        .route("/api/devices", get(api::get_devices).post(api::create_device))
        .route("/api/devices/:id", get(api::get_device).put(api::update_device).delete(api::delete_device))
        .route("/api/devices-enhanced/:id/start", post(api::start_device))
        .route("/api/devices-enhanced/:id/stop", post(api::stop_device))
        .route("/api/devices-debug", get(api::debug_devices))
        .route("/api/logs", get(api::get_logs))
        .route("/api/logs/:device_id", get(api::get_device_logs))
        .route("/api/status", get(api::get_status))
        
        // Enhanced device management with models and tags
        .route("/api/device-models", get(api::get_device_models).post(api::create_device_model))
        .route("/api/device-models/:id", get(api::get_device_model))
        .route("/api/device-models/:id/delete", post(api::delete_device_model))
        .route("/api/device-models/:id/tags", get(api::get_tag_templates))
        .route("/api/devices-enhanced", get(api::get_devices_enhanced).post(api::create_device_with_tags))
        .route("/api/devices-enhanced/:id", get(api::get_device_enhanced).put(api::update_device_with_tags).delete(api::delete_device))
        .route("/api/devices/:id/tags", get(api::get_device_tags_api))
        
        // Schedule group management
        .route("/api/schedule-groups", get(api::get_schedule_groups).post(api::create_schedule_group))
        .route("/api/schedule-groups/:id", get(api::get_schedule_group).put(api::update_schedule_group).delete(api::delete_schedule_group))
        
        // Modbus TCP tag register management
        .route("/api/modbus-tcp-tag-registers", get(api::get_modbus_tcp_tag_registers))
        .route("/api/modbus-tcp-tag-registers/upload-csv", post(api::upload_modbus_tcp_csv_tags))
        
        // WebSocket endpoint
        .route("/socket.io/*path", get(websocket::socket_handler))
        
        // Static file serving
        .route("/", get(serve_index))
        .route("/*path", get(serve_static))
        
        // Add middleware
        .layer(CorsLayer::permissive())
        .layer(socket_layer)
        .with_state(app_state);

    // Start the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    let listener = TcpListener::bind(addr).await?;
    
    info!("Server starting on http://{}", addr);
    info!("Web UI available at http://localhost:{}", config.server.port);
    
    axum::serve(listener, app).await?;

    Ok(())
}
