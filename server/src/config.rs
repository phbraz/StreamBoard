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

/// Writes the OBS host, port, and password into `path`, leaving the rest of the file as it is.
pub fn write_obs(path: &Path, host: &str, port: u16, password: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let existing = if path.exists() {
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?
    } else {
        String::new()
    };
    let updated = upsert_obs_section(&existing, host, port, password);
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, updated).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

/// Replaces `host`, `port`, and `password` inside `[obs]`, or appends that section.
fn upsert_obs_section(text: &str, host: &str, port: u16, password: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = text.split_inclusive(['\n']).collect();
    let mut start = None;
    let mut end = lines.len();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "[obs]" {
            start = Some(index);
            continue;
        }
        if start.is_some() && trimmed.starts_with('[') && trimmed.ends_with(']') {
            end = index;
            break;
        }
    }

    let Some(start) = start else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        if !out.is_empty() && !out.ends_with("\n\n") && !out.ends_with("\r\n\r\n") {
            out.push_str(newline);
        }
        out.push_str("[obs]");
        out.push_str(newline);
        out.push_str(&obs_keys(host, port, password, newline));
        return out;
    };

    let mut section = Vec::new();
    let mut wrote_host = false;
    let mut wrote_port = false;
    let mut wrote_password = false;
    for line in &lines[start + 1..end] {
        let key = obs_key(line);
        let replacement = match key {
            Some("host") => {
                wrote_host = true;
                Some(format!("host = {}{newline}", toml_basic(host)))
            }
            Some("port") => {
                wrote_port = true;
                Some(format!("port = {port}{newline}"))
            }
            Some("password") => {
                wrote_password = true;
                Some(format!("password = {}{newline}", toml_basic(password)))
            }
            _ => None,
        };
        section.push(replacement.unwrap_or_else(|| (*line).to_string()));
    }
    if !wrote_host {
        section.push(format!("host = {}{newline}", toml_basic(host)));
    }
    if !wrote_port {
        section.push(format!("port = {port}{newline}"));
    }
    if !wrote_password {
        section.push(format!("password = {}{newline}", toml_basic(password)));
    }

    let mut out = String::new();
    for line in &lines[..start + 1] {
        out.push_str(line);
    }
    for line in section {
        out.push_str(&line);
    }
    for line in &lines[end..] {
        out.push_str(line);
    }
    out
}

fn obs_keys(host: &str, port: u16, password: &str, newline: &str) -> String {
    format!(
        "host = {host}{newline}port = {port}{newline}password = {password}{newline}",
        host = toml_basic(host),
        password = toml_basic(password),
    )
}

fn obs_key(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let key = trimmed.split_once('=')?.0.trim();
    matches!(key, "host" | "port" | "password").then_some(key)
}

fn toml_basic(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::upsert_obs_section;

    #[test]
    fn updates_an_existing_obs_section_and_keeps_the_rest() {
        let text = "\
[audio]\n\
output = \"Sonar - Aux\"\n\
\n\
[obs]\n\
# websocket\n\
host = \"127.0.0.1\"\n\
port = 4455\n\
password = \"old\"\n\
\n\
[server]\n\
bind = \"0.0.0.0:7878\"\n";
        let updated = upsert_obs_section(text, "192.168.1.101", 4455, "p\"ass");
        assert!(updated.contains("output = \"Sonar - Aux\""));
        assert!(updated.contains("# websocket"));
        assert!(updated.contains("host = \"192.168.1.101\""));
        assert!(updated.contains("password = \"p\\\"ass\""));
        assert!(updated.contains("bind = \"0.0.0.0:7878\""));
        let parsed: super::Config = toml::from_str(&updated).unwrap();
        assert_eq!(parsed.obs.host, "192.168.1.101");
        assert_eq!(parsed.obs.password, "p\"ass");
        assert_eq!(parsed.audio.output, "Sonar - Aux");
    }

    #[test]
    fn appends_obs_when_the_section_is_missing() {
        let updated = upsert_obs_section("[audio]\noutput = \"default\"\n", "localhost", 4456, "");
        let parsed: super::Config = toml::from_str(&updated).unwrap();
        assert_eq!(parsed.obs.host, "localhost");
        assert_eq!(parsed.obs.port, 4456);
        assert!(parsed.obs.password.is_empty());
        assert_eq!(parsed.audio.output, "default");
    }
}
