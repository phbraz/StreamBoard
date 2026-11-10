use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub audio: AudioConfig,
    pub mic: MicConfig,
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
        self.sounds_dir.clone().unwrap_or_else(|| data_dir().join("sounds"))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MicConfig {
    pub source: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            audio: AudioConfig::default(),
            mic: MicConfig::default(),
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
