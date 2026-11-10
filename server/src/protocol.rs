//! WebSocket messages and HTTP payloads. TypeScript bindings are generated into
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
    Press {
        request_id: String,
        button_id: String,
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

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SoundInfo {
    pub id: String,
    pub label: String,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum Button {
    #[serde(rename_all = "camelCase")]
    Sound {
        id: String,
        label: String,
        sound_id: String,
    },
    Hotkey {
        id: String,
        label: String,
        keys: String,
    },
}

impl Button {
    pub fn id(&self) -> &str {
        match self {
            Button::Sound { id, .. } | Button::Hotkey { id, .. } => id,
        }
    }
}

/// Everything a client needs to render the board.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BoardView {
    pub buttons: Vec<Button>,
    pub sounds: Vec<SoundInfo>,
    pub library_dir: String,
    #[ts(type = "number")]
    pub max_upload_mb: u64,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct NewHotkey {
    pub label: String,
    pub keys: String,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct ButtonPatch {
    #[ts(optional)]
    pub label: Option<String>,
    #[ts(optional)]
    pub keys: Option<String>,
}
