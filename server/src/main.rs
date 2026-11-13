#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod api;
mod audio;
mod config;
mod hotkey;
mod launch;
mod library;
mod obs;
mod protocol;
mod static_files;
mod store;
#[cfg(windows)]
mod tray;
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
    obs::Obs,
    protocol::{ServerMessage, ServerState},
    store::Store,
};

const LIBRARY_SCAN_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub config_path: PathBuf,
    pub server_state: Arc<RwLock<ServerState>>,
    pub broadcast: broadcast::Sender<ServerMessage>,
    pub audio: AudioHandle,
    pub store: Arc<Mutex<Store>>,
    pub obs: Arc<Obs>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        #[cfg(all(windows, not(debug_assertions)))]
        tray::alert(&format!("{err:#}"));
        std::process::exit(1);
    }
}

#[tokio::main]
async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "soundboard_server=info".into()),
        )
        .init();

    let requested = parse_config_arg();
    let config_path = requested
        .clone()
        .unwrap_or_else(|| config::config_dir().join("config.toml"));
    let config = Config::load(requested.as_deref())?;
    #[cfg(windows)]
    if tray::focus_existing(&tray::board_url_from_bind(&config.server.bind)) {
        tracing::info!("TheBoard is already running");
        return Ok(());
    }
    let data_dir = config::data_dir();
    tracing::info!(data_dir = %data_dir.display(), "starting");
    tracing::info!(
        outputs = ?audio::output_device_names(),
        selected = %config.audio.output,
        "audio output devices"
    );

    let store = Store::open(
        config.audio.sounds_dir(),
        &data_dir,
        config.audio.max_upload_mb,
    )?;

    let (broadcast, _) = broadcast::channel(64);
    let server_state = Arc::new(RwLock::new(ServerState::default()));

    let audio = {
        let server_state = server_state.clone();
        let broadcast = broadcast.clone();
        audio::spawn(
            config.audio.output.clone(),
            config.audio.master_volume,
            move |playing| {
                let mut state = server_state.blocking_write();
                state.playing = playing;
                let _ = broadcast.send(ServerMessage::State {
                    state: state.clone(),
                });
            },
        )?
    };

    let upload_limit = usize::try_from(config.audio.max_upload_mb * 1024 * 1024)?;
    tracing::info!(
        host = %config.obs.host,
        port = config.obs.port,
        "OBS websocket (connects when a scene button is used)"
    );
    let obs = Arc::new(Obs::new(&config.obs));
    let state = AppState {
        config: Arc::new(config),
        config_path,
        server_state,
        broadcast,
        audio,
        store: Arc::new(Mutex::new(store)),
        obs,
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
        .route("/api/apps", get(api::list_apps).post(api::add_app))
        .route(
            "/api/obs/scenes",
            get(api::obs_scenes).post(api::add_obs_scene),
        )
        .route(
            "/api/obs/settings",
            get(api::get_obs_settings).put(api::put_obs_settings),
        )
        .route(
            "/api/buttons/{id}",
            patch(api::update_button).delete(api::delete_button),
        )
        .route("/ws", get(ws::handler))
        .fallback(static_files::handler)
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(&state.config.server.bind).await?;
    let addr = listener.local_addr()?;
    tracing::info!("listening on http://{addr}");
    if addr.ip().is_unspecified() {
        match lan_ip() {
            Some(ip) => tracing::info!("open http://{ip}:{} on your other devices", addr.port()),
            None => tracing::warn!("couldn't detect this PC's LAN address"),
        }
    }
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(addr))
        .await?;
    Ok(())
}

/// Stops the server on Ctrl+C. On Windows, Quit from the tray icon does the same.
async fn shutdown_signal(addr: std::net::SocketAddr) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    #[cfg(windows)]
    {
        let url = tray::board_url(addr);
        tray::spawn(url.clone(), tx);
        if std::env::args().skip(1).any(|arg| arg == "--open") {
            tray::open_url(&url);
        }
    }
    #[cfg(not(windows))]
    let _hold = tx;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = rx => {}
    }
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

/// The address other devices on the LAN can reach. Connecting a UDP socket sends nothing;
/// it only asks the OS which local address it would route through.
fn lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    socket
        .local_addr()
        .ok()
        .map(|a| a.ip())
        .filter(|ip| !ip.is_loopback())
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
