mod api;
mod audio;
mod config;
mod hotkey;
mod library;
mod protocol;
mod static_files;
mod store;
mod ws;

use std::{path::PathBuf, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::DefaultBodyLimit,
    routing::{get, patch, post},
};
use serde_json::json;
use tokio::sync::{Mutex, RwLock, broadcast};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::{
    audio::AudioHandle,
    config::Config,
    protocol::{ServerMessage, ServerState},
    store::Store,
};

const LIBRARY_SCAN_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub server_state: Arc<RwLock<ServerState>>,
    pub broadcast: broadcast::Sender<ServerMessage>,
    pub audio: AudioHandle,
    pub store: Arc<Mutex<Store>>,
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
    let data_dir = config::data_dir();
    tracing::info!(data_dir = %data_dir.display(), "starting");
    if config.audio.output != "default" {
        tracing::warn!(
            output = %config.audio.output,
            "named audio outputs are not supported yet, using the default output"
        );
    }

    let store = Store::open(config.audio.sounds_dir(), &data_dir, config.audio.max_upload_mb)?;

    let (broadcast, _) = broadcast::channel(64);
    let server_state = Arc::new(RwLock::new(ServerState::default()));

    let audio = {
        let server_state = server_state.clone();
        let broadcast = broadcast.clone();
        audio::spawn(config.audio.master_volume, move |playing| {
            let mut state = server_state.blocking_write();
            state.playing = playing;
            let _ = broadcast.send(ServerMessage::State { state: state.clone() });
        })?
    };

    let upload_limit = usize::try_from(config.audio.max_upload_mb * 1024 * 1024)?;
    let state = AppState {
        config: Arc::new(config),
        server_state,
        broadcast,
        audio,
        store: Arc::new(Mutex::new(store)),
    };

    sync_library(&state).await;
    {
        let store = state.store.lock().await;
        store.save()?;
        let view = store.view();
        tracing::info!(
            dir = %view.library_dir,
            sounds = view.sounds.len(),
            buttons = view.buttons.len(),
            "library ready"
        );
    }
    tokio::spawn(watch_library(state.clone()));

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/board", get(api::get_board))
        .route(
            "/api/sounds",
            post(api::upload_sound).layer(DefaultBodyLimit::max(upload_limit)),
        )
        .route("/api/hotkeys", post(api::add_hotkey))
        .route("/api/buttons/{id}", patch(api::update_button).delete(api::delete_button))
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

/// Picks up files added to or removed from the sounds folder outside the app.
async fn watch_library(state: AppState) {
    let mut tick = tokio::time::interval(LIBRARY_SCAN_INTERVAL);
    tick.tick().await;
    loop {
        tick.tick().await;
        if sync_library(&state).await {
            api::commit(&state, &*state.store.lock().await);
        }
    }
}

/// Loads new files and forgets deleted ones. Returns true if the board changed.
async fn sync_library(state: &AppState) -> bool {
    let (new_files, missing) = {
        let store = state.store.lock().await;
        (store.unknown_files(), store.missing_sounds())
    };
    let mut changed = false;

    if !missing.is_empty() {
        let mut store = state.store.lock().await;
        for id in missing {
            tracing::info!(sound_id = %id, "sound file removed");
            store.forget_sound(&id);
            state.audio.unload(id);
        }
        changed = true;
    }

    for path in new_files {
        let p = path.clone();
        let decoded = tokio::task::spawn_blocking(move || library::decode_file(&p)).await;
        let mut store = state.store.lock().await;
        match decoded {
            Ok(Ok(decoded)) => {
                if let Some(id) = store.register(&path, decoded.duration_ms, None) {
                    tracing::info!(sound_id = %id, "loaded sound");
                    state.audio.load(id, decoded.buffer);
                    changed = true;
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(path = %path.display(), "skipping: {e:#}");
                store.mark_failed(&path);
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), "decoder crashed: {e}");
                store.mark_failed(&path);
            }
        }
    }

    changed | state.store.lock().await.prune_dangling()
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
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
