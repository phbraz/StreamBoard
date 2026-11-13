use std::path::Path;

use axum::{
    Json,
    extract::{Multipart, Path as UrlPath, State, multipart::MultipartError},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::{
    AppState,
    library::{self, SUPPORTED},
    protocol::{
        AppInfo, AppList, BoardView, Button, ButtonPatch, NewApp, NewHotkey, NewObsScene,
        ObsSceneInfo, ObsSceneList, ServerMessage,
    },
    store::{Store, clean_label},
};

pub struct ApiError(StatusCode, String);

impl ApiError {
    fn bad_request(msg: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, msg.into())
    }

    fn internal(err: impl std::fmt::Display) -> Self {
        tracing::error!("{err}");
        Self(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
    }

    fn unavailable(msg: impl Into<String>) -> Self {
        Self(StatusCode::SERVICE_UNAVAILABLE, msg.into())
    }
}

fn map_obs(err: crate::obs::ObsError) -> ApiError {
    match err {
        crate::obs::ObsError::Offline(msg) => ApiError::unavailable(msg),
        crate::obs::ObsError::Rejected(msg) => ApiError::bad_request(msg),
    }
}

impl From<MultipartError> for ApiError {
    fn from(err: MultipartError) -> Self {
        Self(err.status(), err.body_text())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

/// Persists the board and tells every client to refetch it.
pub fn commit(state: &AppState, store: &Store) {
    if let Err(e) = store.save() {
        tracing::error!("saving board: {e:#}");
    }
    let _ = state.broadcast.send(ServerMessage::BoardUpdated);
}

pub async fn get_board(State(state): State<AppState>) -> Json<BoardView> {
    Json(state.store.lock().await.view())
}

/// Multipart upload with a `file` field and an optional `label` field.
pub async fn upload_sound(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Button>, ApiError> {
    let mut label = None;
    let mut file = None;
    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("label") => label = clean_label(&field.text().await?),
            Some("file") => {
                let name = field.file_name().unwrap_or("sound").to_string();
                file = Some((name, field.bytes().await?));
            }
            _ => {}
        }
    }

    let (name, bytes) = file.ok_or_else(|| ApiError::bad_request("missing file"))?;
    let ext = library::extension(Path::new(&name)).ok_or_else(|| {
        ApiError::bad_request(format!(
            "unsupported file type, use {}",
            SUPPORTED.join(", ")
        ))
    })?;

    let data = bytes.clone();
    let decoded = tokio::task::spawn_blocking(move || library::decode_bytes(data.to_vec()))
        .await
        .map_err(ApiError::internal)?
        .map_err(|e| ApiError::bad_request(format!("not a playable audio file: {e}")))?;

    let stem = Path::new(&name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string());
    let mut store = state.store.lock().await;
    let id = store.unique_sound_id(label.as_deref().or(stem.as_deref()).unwrap_or("sound"));
    let path = store.sounds_dir().join(format!("{id}.{ext}"));
    let part = store.sounds_dir().join(format!("{id}.{ext}.part"));
    tokio::fs::write(&part, &bytes)
        .await
        .map_err(ApiError::internal)?;
    tokio::fs::rename(&part, &path)
        .await
        .map_err(ApiError::internal)?;

    let label = label.or_else(|| stem.as_deref().map(library::label_from_stem));
    let id = store
        .register(&path, decoded.duration_ms, label)
        .ok_or_else(|| ApiError::internal(format!("registering {}", path.display())))?;
    state.audio.load(id.clone(), decoded.buffer);
    let button = store
        .sound_button(&id)
        .ok_or_else(|| ApiError::internal("button missing"))?;
    commit(&state, &store);
    tracing::info!(sound_id = %id, "uploaded sound");
    Ok(Json(button))
}

pub async fn add_hotkey(
    State(state): State<AppState>,
    Json(req): Json<NewHotkey>,
) -> Result<Json<Button>, ApiError> {
    let mut store = state.store.lock().await;
    let button = store
        .add_hotkey(&req.label, &req.keys)
        .map_err(ApiError::bad_request)?;
    commit(&state, &store);
    Ok(Json(button))
}

pub async fn list_apps() -> Json<AppList> {
    let apps = crate::launch::list().await;
    Json(AppList {
        apps: apps
            .into_iter()
            .map(|app| AppInfo {
                name: app.name,
                path: app.path,
            })
            .collect(),
    })
}

pub async fn add_app(
    State(state): State<AppState>,
    Json(req): Json<NewApp>,
) -> Result<Json<Button>, ApiError> {
    let mut store = state.store.lock().await;
    let button = store
        .add_app(&req.label, &req.path)
        .map_err(ApiError::bad_request)?;
    commit(&state, &store);
    Ok(Json(button))
}

pub async fn obs_scenes(State(state): State<AppState>) -> Result<Json<ObsSceneList>, ApiError> {
    let scenes = state.obs.scenes().await.map_err(map_obs)?;
    Ok(Json(ObsSceneList {
        scenes: scenes
            .into_iter()
            .map(|scene| ObsSceneInfo {
                name: scene.name,
                current: scene.current,
            })
            .collect(),
    }))
}

pub async fn add_obs_scene(
    State(state): State<AppState>,
    Json(req): Json<NewObsScene>,
) -> Result<Json<Button>, ApiError> {
    let scene = state.obs.require_scene(&req.scene).await.map_err(map_obs)?;
    let mut store = state.store.lock().await;
    let button = store
        .add_obs_scene(&req.label, &scene)
        .map_err(ApiError::bad_request)?;
    commit(&state, &store);
    Ok(Json(button))
}

pub async fn update_button(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<String>,
    Json(mut patch): Json<ButtonPatch>,
) -> Result<Json<Button>, ApiError> {
    if patch.scene.is_some() {
        match state.store.lock().await.button(&id) {
            Some(Button::Obs { .. }) => {
                let scene = state
                    .obs
                    .require_scene(patch.scene.as_deref().unwrap_or(""))
                    .await
                    .map_err(map_obs)?;
                patch.scene = Some(scene);
            }
            Some(_) => return Err(ApiError::bad_request("only OBS buttons have a scene")),
            None => return Err(ApiError::bad_request("unknown button")),
        }
    }
    let mut store = state.store.lock().await;
    let button = store
        .update_button(&id, patch)
        .map_err(ApiError::bad_request)?;
    commit(&state, &store);
    Ok(Json(button))
}

pub async fn delete_button(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<String>,
) -> Result<StatusCode, ApiError> {
    let mut store = state.store.lock().await;
    let removed = store.delete_button(&id).map_err(ApiError::bad_request)?;
    if let Some(sound_id) = removed {
        state.audio.unload(sound_id);
    }
    commit(&state, &store);
    Ok(StatusCode::NO_CONTENT)
}
