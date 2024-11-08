use std::{io, net::SocketAddr};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::any,
    Router,
};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let app = Router::new().route("/ws", any(ws_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await?;
    info!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    info!(who = %addr, "New connection");
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(txt))) => {
                info!(%who, message = %txt, "Received message");
                if let Err(err) = socket.send(Message::Text(txt)).await {
                    error!(%who, %err, "Failed to respond");
                } else {
                    info!(%who, "Sent response");
                }
            }
            Some(Ok(Message::Close(_))) | None => {
                info!(%who, "Connection closed");
                return;
            }
            Some(Ok(_)) => {
                warn!(%who, "Received unsupported message format");
            }
            Some(Err(err)) => {
                error!(%who, %err, "Connection error");
                return;
            }
        }
    }
}
