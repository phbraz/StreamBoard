use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub audio: AudioConfig,
    pub mic: MicConfig,
    pub obs: ObsConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind: String,
    pub mdns: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub output: String,
    pub master_volume: f32,
    pub max_upload_mb: u64,
    /// Server-side sound library. Defaults to `~/.local/share/soundboard/sounds`.
    pub sounds_dir: Option<PathBuf>,
}

impl AudioConfig {
    pub fn sounds_dir(&self) -> PathBuf {
        self.sounds_dir
            .clone()
            .unwrap_or_else(|| data_dir().join("sounds"))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MicConfig {
    pub source: String,
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct ObsConfig {
    pub host: String,
    pub port: u16,
    /// Empty means OBS has no websocket password.
    pub password: String,
}

impl std::fmt::Debug for ObsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObsConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field(
                "password",
                &if self.password.is_empty() {
                    "none"
                } else {
                    "set"
                },
            )
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            audio: AudioConfig::default(),
            mic: MicConfig::default(),
            obs: ObsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:7878".into(),
            mdns: true,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            output: "default".into(),
            master_volume: 1.0,
            max_upload_mb: 20,
            sounds_dir: None,
        }
    }
}

impl Default for MicConfig {
    fn default() -> Self {
        Self {
            source: "@DEFAULT_AUDIO_SOURCE@".into(),
        }
    }
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 4455,
            password: String::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("soundboard")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("soundboard")
}

impl Config {
    /// Loads `path`, or `~/.config/soundboard/config.toml` when `None`.
    /// A missing default file yields the defaults; a missing explicit file is an error.
    pub fn load(path: Option<&Path>) -> anyhow::Result<Self> {
        let (path, explicit) = match path {
            Some(p) => (p.to_path_buf(), true),
            None => (config_dir().join("config.toml"), false),
        };

        if !explicit && !path.exists() {
            tracing::info!("no config at {}, using defaults", path.display());
            return Ok(Self::default());
        }

        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
}
