use axum::{
    response::Response,
};
use socketioxide::extract::SocketRef;
use tracing::{info};

pub async fn socket_handler() -> Response {
    Response::builder()
        .status(200)
        .body("Socket.IO endpoint".into())
        .unwrap()
}

pub async fn on_connect(socket: SocketRef) {
    info!("Socket.IO client connected: {}", socket.id);

    socket.on_disconnect(|socket: SocketRef| async move {
        info!("Socket.IO client disconnected: {}", socket.id);
    });
}
