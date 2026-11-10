use std::time::Duration;

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::{
    AppState,
    protocol::{ClientMessage, ServerMessage},
};

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle(socket, state))
}

async fn handle(socket: WebSocket, state: AppState) {
    let (mut sink, mut stream) = socket.split();

    let Some(device_id) = authenticate(&mut stream).await else {
        let _ = send(&mut sink, &ServerMessage::Error {
            message: "expected hello".into(),
        })
        .await;
        return;
    };
    tracing::info!(%device_id, "client connected");

    let welcome = ServerMessage::Welcome {
        device_id: device_id.clone(),
        server_version: env!("CARGO_PKG_VERSION").into(),
        state: state.server_state.read().await.clone(),
    };
    if send(&mut sink, &welcome).await.is_err() {
        return;
    }

    // Single writer task so replies and broadcasts never interleave on the socket.
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(32);
    let mut broadcasts = state.broadcast.subscribe();
    let writer = tokio::spawn(async move {
        loop {
            let msg = tokio::select! {
                Some(msg) = rx.recv() => msg,
                Ok(msg) = broadcasts.recv() => msg,
                else => break,
            };
            if send(&mut sink, &msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(frame)) = stream.next().await {
        let Message::Text(text) = frame else { continue };
        let reply = match serde_json::from_str::<ClientMessage>(&text) {
            Ok(msg) => dispatch(msg),
            Err(e) => ServerMessage::Error {
                message: format!("invalid message: {e}"),
            },
        };
        if tx.send(reply).await.is_err() {
            break;
        }
    }

    writer.abort();
    tracing::info!(%device_id, "client disconnected");
}

/// Waits for the first `hello`. Token validation arrives with pairing (milestone 2).
async fn authenticate(stream: &mut futures_util::stream::SplitStream<WebSocket>) -> Option<String> {
    let frame = tokio::time::timeout(HELLO_TIMEOUT, stream.next())
        .await
        .ok()??
        .ok()?;
    let Message::Text(text) = frame else { return None };
    match serde_json::from_str::<ClientMessage>(&text).ok()? {
        ClientMessage::Hello { token: _ } => Some("dev".into()),
        _ => None,
    }
}

fn dispatch(msg: ClientMessage) -> ServerMessage {
    match msg {
        ClientMessage::Ping => ServerMessage::Pong,
        ClientMessage::Hello { .. } => ServerMessage::Error {
            message: "already authenticated".into(),
        },
        ClientMessage::Play { request_id, .. }
        | ClientMessage::Stop { request_id, .. }
        | ClientMessage::StopAll { request_id }
        | ClientMessage::RunAction { request_id, .. } => ServerMessage::Ack {
            request_id,
            ok: false,
            error: Some("not implemented yet".into()),
        },
    }
}

async fn send(
    sink: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    msg: &ServerMessage,
) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("ServerMessage serializes");
    sink.send(Message::Text(text.into())).await
}
