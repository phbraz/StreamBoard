//! OBS scene changes go through obs-websocket (OBS: Tools → WebSocket Server Settings).
//! The connection is opened on demand and reused, so the server still starts when OBS is closed.

use std::time::Duration;

use obws::Client;
use tokio::sync::Mutex;

use crate::config::ObsConfig;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

pub struct Obs {
    host: String,
    port: u16,
    password: Option<String>,
    client: Mutex<Option<Client>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene {
    pub name: String,
    pub current: bool,
}

#[derive(Debug)]
pub enum ObsError {
    /// OBS isn't reachable, or the websocket handshake failed.
    Offline(String),
    /// OBS answered and refused the request.
    Rejected(String),
}

impl std::fmt::Display for ObsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Offline(msg) | Self::Rejected(msg) => f.write_str(msg),
        }
    }
}

/// Runs an OBS request, reconnecting once when the cached socket has died.
/// Inlined so the caller's future stays `Send` (a generic async closure does not).
macro_rules! obs_call {
    ($obs:expr, |$client:ident| $body:expr) => {{
        let obs: &Obs = $obs;
        let mut slot = obs.client.lock().await;
        let mut out = Err(ObsError::Offline(obs.hint("OBS request failed")));
        for attempt in 0..2 {
            if slot.is_none() {
                match obs.connect().await {
                    Ok(client) => *slot = Some(client),
                    Err(err) => {
                        out = Err(err);
                        break;
                    }
                }
            }
            let $client = slot.as_ref().expect("client was just connected");
            match $body.await {
                Ok(value) => {
                    out = Ok(value);
                    break;
                }
                Err(err) if attempt == 0 && is_disconnect(&err) => {
                    tracing::warn!("OBS request failed, reconnecting: {err}");
                    *slot = None;
                }
                Err(err) => {
                    out = Err(obs.classify(err));
                    break;
                }
            }
        }
        out
    }};
}

impl Obs {
    pub fn new(config: &ObsConfig) -> Self {
        let password = {
            let password = config.password.trim();
            (!password.is_empty()).then(|| password.to_string())
        };
        Self {
            host: config.host.clone(),
            port: config.port,
            password,
            client: Mutex::new(None),
        }
    }

    pub async fn scenes(&self) -> Result<Vec<Scene>, ObsError> {
        let list = obs_call!(self, |client| client.scenes().list())?;
        let current = list.current_program_scene.map(|scene| scene.name);
        let mut scenes = list.scenes;
        // Higher indexes sit higher in the OBS scenes dock.
        scenes.sort_by(|a, b| b.index.cmp(&a.index));
        Ok(scenes
            .into_iter()
            .map(|scene| Scene {
                current: current.as_deref() == Some(scene.id.name.as_str()),
                name: scene.id.name,
            })
            .collect())
    }

    pub async fn set_scene(&self, scene: &str) -> Result<(), ObsError> {
        let scene = clean_scene_name(scene).map_err(ObsError::Rejected)?;
        obs_call!(self, |client| client
            .scenes()
            .set_current_program_scene(scene.as_str()))
    }

    /// Returns the scene name as OBS spells it, or an error when it isn't in the current list.
    pub async fn require_scene(&self, scene: &str) -> Result<String, ObsError> {
        let scene = clean_scene_name(scene).map_err(ObsError::Rejected)?;
        let scenes = self.scenes().await?;
        scenes
            .into_iter()
            .find(|candidate| candidate.name == scene)
            .map(|candidate| candidate.name)
            .ok_or_else(|| ObsError::Rejected(format!("OBS has no scene \"{scene}\"")))
    }

    async fn connect(&self) -> Result<Client, ObsError> {
        let connect = Client::connect(&self.host, self.port, self.password.as_deref());
        match tokio::time::timeout(CONNECT_TIMEOUT, connect).await {
            Ok(Ok(client)) => Ok(client),
            Ok(Err(err)) => Err(self.classify(err)),
            Err(_) => Err(ObsError::Offline(self.hint("timed out waiting for OBS"))),
        }
    }

    fn classify(&self, err: obws::error::Error) -> ObsError {
        match &err {
            obws::error::Error::Api { message, .. } => {
                let detail = message.clone().unwrap_or_else(|| err.to_string());
                ObsError::Rejected(format!("OBS at {}: {detail}", self.endpoint()))
            }
            obws::error::Error::InvalidUri(_)
            | obws::error::Error::Connect(_)
            | obws::error::Error::Timeout
            | obws::error::Error::Handshake(_)
            | obws::error::Error::Send(_)
            | obws::error::Error::ReceiveMessage(_)
            | obws::error::Error::Disconnected => ObsError::Offline(self.hint(err)),
            _ => ObsError::Rejected(format!("OBS at {}: {err}", self.endpoint())),
        }
    }

    fn hint(&self, err: impl std::fmt::Display) -> String {
        format!(
            "couldn't reach OBS at {} ({err}). In OBS, open Tools → WebSocket Server Settings and check the server is enabled, plus the port and password.",
            self.endpoint()
        )
    }

    fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

pub fn clean_scene_name(scene: &str) -> Result<String, String> {
    let scene = scene.trim();
    if scene.is_empty() {
        return Err("scene name is empty".into());
    }
    if scene.chars().count() > 256 {
        return Err("scene name is too long".into());
    }
    if scene.chars().any(char::is_control) {
        return Err("scene name has invalid characters".into());
    }
    Ok(scene.to_string())
}

fn is_disconnect(err: &obws::error::Error) -> bool {
    matches!(
        err,
        obws::error::Error::Send(_)
            | obws::error::Error::ReceiveMessage(_)
            | obws::error::Error::Disconnected
            | obws::error::Error::Timeout
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_scene_names() {
        assert_eq!(clean_scene_name("  Game  ").unwrap(), "Game");
        assert_eq!(
            clean_scene_name("BRB — starting soon").unwrap(),
            "BRB — starting soon"
        );
        assert!(clean_scene_name("   ").is_err());
        assert!(clean_scene_name("a\nb").is_err());
        assert!(clean_scene_name(&"x".repeat(257)).is_err());
    }
}
