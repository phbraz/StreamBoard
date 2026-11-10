use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{
    hotkey,
    library::{self, label_from_stem, slug},
    protocol::{BoardView, Button, ButtonPatch, SoundInfo},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
struct Board {
    version: u32,
    buttons: Vec<Button>,
}

impl Default for Board {
    fn default() -> Self {
        Self { version: 1, buttons: Vec::new() }
    }
}

struct SoundEntry {
    info: SoundInfo,
    file_name: OsString,
}

/// The board (persisted to `board.json`) plus the index of loaded sound files.
pub struct Store {
    sounds_dir: PathBuf,
    board_path: PathBuf,
    max_upload_mb: u64,
    board: Board,
    sounds: BTreeMap<String, SoundEntry>,
    /// Files that failed to decode; skipped until they change name.
    failed: HashSet<OsString>,
}

impl Store {
    pub fn open(sounds_dir: PathBuf, data_dir: &Path, max_upload_mb: u64) -> anyhow::Result<Self> {
        for dir in [&sounds_dir, &data_dir.to_path_buf()] {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let board_path = data_dir.join("board.json");
        let board = match std::fs::read_to_string(&board_path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", board_path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Board::default(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", board_path.display())),
        };
        Ok(Self {
            sounds_dir,
            board_path,
            max_upload_mb,
            board,
            sounds: BTreeMap::new(),
            failed: HashSet::new(),
        })
    }

    pub fn sounds_dir(&self) -> &Path {
        &self.sounds_dir
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let tmp = self.board_path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&self.board)?)?;
        std::fs::rename(&tmp, &self.board_path)?;
        Ok(())
    }

    pub fn view(&self) -> BoardView {
        BoardView {
            buttons: self.board.buttons.clone(),
            sounds: self.sounds.values().map(|s| s.info.clone()).collect(),
            library_dir: self.sounds_dir.display().to_string(),
            max_upload_mb: self.max_upload_mb,
            server_os: std::env::consts::OS.to_string(),
        }
    }

    pub fn button(&self, id: &str) -> Option<Button> {
        self.board.buttons.iter().find(|b| b.id() == id).cloned()
    }

    pub fn sound_button(&self, sound_id: &str) -> Option<Button> {
        self.board
            .buttons
            .iter()
            .find(|b| matches!(b, Button::Sound { sound_id: s, .. } if s == sound_id))
            .cloned()
    }

    /// Supported files in the sounds folder that aren't loaded yet.
    pub fn unknown_files(&self) -> Vec<PathBuf> {
        let known: HashSet<&OsString> = self.sounds.values().map(|s| &s.file_name).collect();
        let Ok(entries) = std::fs::read_dir(&self.sounds_dir) else { return Vec::new() };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|e| !known.contains(&e.file_name()) && !self.failed.contains(&e.file_name()))
            .map(|e| e.path())
            .filter(|p| p.is_file() && library::extension(p).is_some())
            .collect();
        files.sort();
        files
    }

    /// Loaded sounds whose file has disappeared from the folder.
    pub fn missing_sounds(&self) -> Vec<String> {
        self.sounds
            .iter()
            .filter(|(_, s)| !self.sounds_dir.join(&s.file_name).exists())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Adds a decoded file to the library and gives it a button if none exists.
    /// Returns the sound id, or `None` if its id collides with another file.
    pub fn register(&mut self, path: &Path, duration_ms: u32, label: Option<String>) -> Option<String> {
        let file_name = path.file_name()?.to_os_string();
        if self.sounds.values().any(|s| s.file_name == file_name) {
            return None;
        }
        let stem = path.file_stem()?.to_string_lossy().to_string();
        let id = slug(&stem);
        if self.sounds.contains_key(&id) {
            tracing::warn!(path = %path.display(), "another file already uses sound id {id}, skipping");
            self.failed.insert(file_name);
            return None;
        }

        let label = label.unwrap_or_else(|| label_from_stem(&stem));
        self.sounds.insert(
            id.clone(),
            SoundEntry {
                info: SoundInfo { id: id.clone(), label: label.clone(), duration_ms },
                file_name,
            },
        );
        let has_button = self
            .board
            .buttons
            .iter()
            .any(|b| matches!(b, Button::Sound { sound_id, .. } if *sound_id == id));
        if !has_button {
            self.board.buttons.push(Button::Sound {
                id: new_id(),
                label,
                sound_id: id.clone(),
            });
        }
        Some(id)
    }

    pub fn mark_failed(&mut self, path: &Path) {
        if let Some(name) = path.file_name() {
            self.failed.insert(name.to_os_string());
        }
    }

    /// Removes a sound from the index and drops its buttons. Doesn't touch the file.
    pub fn forget_sound(&mut self, sound_id: &str) {
        self.sounds.remove(sound_id);
        self.board
            .buttons
            .retain(|b| !matches!(b, Button::Sound { sound_id: s, .. } if s == sound_id));
    }

    /// Drops sound buttons that point at sounds which aren't loaded. Returns true if any were removed.
    pub fn prune_dangling(&mut self) -> bool {
        let before = self.board.buttons.len();
        let sounds = &self.sounds;
        self.board
            .buttons
            .retain(|b| !matches!(b, Button::Sound { sound_id, .. } if !sounds.contains_key(sound_id)));
        self.board.buttons.len() != before
    }

    /// A sound id derived from `base` that no loaded sound or file in the folder uses.
    pub fn unique_sound_id(&self, base: &str) -> String {
        let taken: HashSet<String> = std::fs::read_dir(&self.sounds_dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|e| e.path().file_stem().map(|s| slug(&s.to_string_lossy())))
            .chain(self.sounds.keys().cloned())
            .collect();
        let base = slug(base);
        (1..)
            .map(|n| if n == 1 { base.clone() } else { format!("{base}-{n}") })
            .find(|id| !taken.contains(id))
            .expect("unbounded range")
    }

    pub fn add_hotkey(&mut self, label: &str, keys: &str) -> Result<Button, String> {
        let keys = hotkey::normalize(keys)?;
        let label = clean_label(label).unwrap_or_else(|| keys.clone());
        let button = Button::Hotkey { id: new_id(), label, keys };
        self.board.buttons.push(button.clone());
        Ok(button)
    }

    pub fn update_button(&mut self, id: &str, patch: ButtonPatch) -> Result<Button, String> {
        let button = self
            .board
            .buttons
            .iter_mut()
            .find(|b| b.id() == id)
            .ok_or("unknown button")?;

        let new_keys = match (&*button, &patch.keys) {
            (Button::Hotkey { .. }, Some(keys)) => Some(hotkey::normalize(keys)?),
            (Button::Sound { .. }, Some(_)) => return Err("sound buttons have no keys".into()),
            _ => None,
        };
        let new_label = patch.label.as_deref().and_then(clean_label);

        match button {
            Button::Sound { label, .. } => {
                if let Some(l) = new_label {
                    *label = l;
                }
            }
            Button::Hotkey { label, keys, .. } => {
                if let Some(l) = new_label {
                    *label = l;
                }
                if let Some(k) = new_keys {
                    *keys = k;
                }
            }
        }
        Ok(button.clone())
    }

    /// Removes a button. Deleting a sound button also deletes its file from the server.
    /// Returns the removed sound id, if any, so the caller can unload it.
    pub fn delete_button(&mut self, id: &str) -> Result<Option<String>, String> {
        let pos = self.board.buttons.iter().position(|b| b.id() == id).ok_or("unknown button")?;
        let sound_id = match self.board.buttons.remove(pos) {
            Button::Sound { sound_id, .. } => sound_id,
            Button::Hotkey { .. } => return Ok(None),
        };
        if let Some(entry) = self.sounds.remove(&sound_id) {
            let path = self.sounds_dir.join(&entry.file_name);
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!(path = %path.display(), "deleting sound file: {e}");
            }
        }
        Ok(Some(sound_id))
    }
}

pub fn clean_label(label: &str) -> Option<String> {
    let label = label.trim();
    (!label.is_empty()).then(|| label.chars().take(40).collect())
}

fn new_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    format!("b{:x}{:x}", nanos, COUNTER.fetch_add(1, Ordering::Relaxed))
}
