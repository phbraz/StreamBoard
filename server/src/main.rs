mod audio;
mod config;
mod protocol;
mod static_files;
mod ws;

use std::{path::PathBuf, sync::Arc};

use axum::{Json, Router, extract::State, routing::get};
use serde_json::json;
use tokio::sync::{RwLock, broadcast};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::{
    audio::AudioHandle,
    config::Config,
    protocol::{ServerMessage, ServerState, SoundList},
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub server_state: Arc<RwLock<ServerState>>,
    pub broadcast: broadcast::Sender<ServerMessage>,
    pub audio: AudioHandle,
    pub sounds: Arc<SoundList>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "soundboard_server=info".into()),
        )
        .init();

    let config_path = parse_config_arg();
    let config = Config::load(config_path.as_deref())?;
    tracing::info!(data_dir = %config::data_dir().display(), "starting");
    if config.audio.output != "default" {
        tracing::warn!(
            output = %config.audio.output,
            "named audio outputs are not supported yet, using the default output"
        );
    }

    let sounds_dir = config.audio.sounds_dir();
    let library = audio::load_library(&sounds_dir)?;
    let sounds = Arc::new(SoundList {
        library_dir: sounds_dir.display().to_string(),
        sounds: library.sounds.clone(),
    });

    let (broadcast, _) = broadcast::channel(64);
    let server_state = Arc::new(RwLock::new(ServerState::default()));

    let audio = {
        let server_state = server_state.clone();
        let broadcast = broadcast.clone();
        audio::spawn(library, config.audio.master_volume, move |playing| {
            let mut state = server_state.blocking_write();
            state.playing = playing;
            let _ = broadcast.send(ServerMessage::State { state: state.clone() });
        })?
    };

    let state = AppState {
        config: Arc::new(config),
        server_state,
        broadcast,
        audio,
        sounds,
    };

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/sounds", get(list_sounds))
        .route("/ws", get(ws::handler))
        .fallback(static_files::handler)
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&state.config.server.bind).await?;
    tracing::info!("listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn list_sounds(State(state): State<AppState>) -> Json<SoundList> {
    Json((*state.sounds).clone())
}

fn parse_config_arg() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" || arg == "-c" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}
