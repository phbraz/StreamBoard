//! WebSocket messages. TypeScript bindings are generated into
//! `web/src/protocol/generated` by `cargo test`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ClientMessage {
    Hello {
        token: String,
    },
    #[serde(rename_all = "camelCase")]
    Play {
        request_id: String,
        sound_id: String,
    },
    #[serde(rename_all = "camelCase")]
    Stop {
        request_id: String,
        sound_id: String,
    },
    #[serde(rename_all = "camelCase")]
    StopAll {
        request_id: String,
    },
    #[serde(rename_all = "camelCase")]
    RunAction {
        request_id: String,
        action_id: String,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Welcome {
        device_id: String,
        server_version: String,
        state: ServerState,
    },
    #[serde(rename_all = "camelCase")]
    Ack {
        request_id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        error: Option<String>,
    },
    State {
        state: ServerState,
    },
    BoardUpdated,
    Error {
        message: String,
    },
    Pong,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ServerState {
    pub mic_muted: bool,
    pub playing: Vec<PlayingSound>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlayingSound {
    pub sound_id: String,
    pub device_id: String,
}
