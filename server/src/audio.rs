use std::{
    collections::HashMap,
    path::Path,
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::Context;
use rodio::{Decoder, Player, Source, buffer::SamplesBuffer};
use tokio::sync::oneshot;

use crate::protocol::{PlayingSound, SoundInfo};

const SUPPORTED: &[&str] = &["mp3", "ogg", "oga", "wav", "flac", "m4a"];
const POLL: Duration = Duration::from_millis(50);

pub struct Library {
    pub sounds: Vec<SoundInfo>,
    buffers: HashMap<String, SamplesBuffer>,
}

/// Decodes every supported file in `dir` into memory so playback starts instantly.
pub fn load_library(dir: &Path) -> anyhow::Result<Library> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| SUPPORTED.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    entries.sort();

    let mut library = Library { sounds: Vec::new(), buffers: HashMap::new() };
    for path in entries {
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let id = slug(stem);
        if library.buffers.contains_key(&id) {
            tracing::warn!(path = %path.display(), "duplicate sound id {id}, skipping");
            continue;
        }
        match decode(&path) {
            Ok(buffer) => {
                let duration_ms = buffer.total_duration().map_or(0, |d| d.as_millis() as u32);
                library.sounds.push(SoundInfo {
                    id: id.clone(),
                    label: stem.replace(['-', '_'], " "),
                    duration_ms,
                });
                library.buffers.insert(id, buffer);
            }
            Err(e) => tracing::warn!(path = %path.display(), "skipping: {e:#}"),
        }
    }
    tracing::info!(dir = %dir.display(), count = library.sounds.len(), "loaded sound library");
    Ok(library)
}

fn decode(path: &Path) -> anyhow::Result<SamplesBuffer> {
    let file = std::fs::File::open(path)?;
    let decoder = Decoder::try_from(file)?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<_> = decoder.collect();
    anyhow::ensure!(!samples.is_empty(), "no audio samples");
    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

type Reply = oneshot::Sender<Result<(), String>>;

enum Command {
    Play { sound_id: String, device_id: String, reply: Reply },
    Stop { sound_id: String, reply: Reply },
    StopAll { reply: Reply },
}

#[derive(Clone)]
pub struct AudioHandle {
    tx: mpsc::Sender<Command>,
}

impl AudioHandle {
    pub async fn play(&self, sound_id: String, device_id: String) -> Result<(), String> {
        self.call(|reply| Command::Play { sound_id, device_id, reply }).await
    }

    pub async fn stop(&self, sound_id: String) -> Result<(), String> {
        self.call(|reply| Command::Stop { sound_id, reply }).await
    }

    pub async fn stop_all(&self) -> Result<(), String> {
        self.call(|reply| Command::StopAll { reply }).await
    }

    async fn call(&self, cmd: impl FnOnce(Reply) -> Command) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.tx.send(cmd(reply)).map_err(|_| "audio thread stopped".to_string())?;
        rx.await.map_err(|_| "audio thread stopped".to_string())?
    }
}

struct Voice {
    sound_id: String,
    device_id: String,
    player: Player,
}

/// Starts the audio thread. The output device must be owned by a single thread,
/// so everything goes through a command channel.
pub fn spawn(
    library: Library,
    master_volume: f32,
    on_change: impl Fn(Vec<PlayingSound>) + Send + 'static,
) -> anyhow::Result<AudioHandle> {
    let (tx, rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<anyhow::Result<()>>();
    let buffers = library.buffers;

    thread::Builder::new().name("audio".into()).spawn(move || {
        let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                let _ = ready_tx.send(Ok(()));
                sink
            }
            Err(e) => {
                let _ = ready_tx.send(Err(anyhow::anyhow!("opening audio output: {e}")));
                return;
            }
        };
        let mut voices: Vec<Voice> = Vec::new();

        loop {
            let mut changed = false;
            match rx.recv_timeout(POLL) {
                Ok(Command::Play { sound_id, device_id, reply }) => {
                    let result = match buffers.get(&sound_id) {
                        Some(buffer) => {
                            let player = Player::connect_new(sink.mixer());
                            player.set_volume(master_volume);
                            player.append(buffer.clone());
                            voices.push(Voice { sound_id, device_id, player });
                            changed = true;
                            Ok(())
                        }
                        None => Err(format!("unknown sound {sound_id}")),
                    };
                    let _ = reply.send(result);
                }
                Ok(Command::Stop { sound_id, reply }) => {
                    let before = voices.len();
                    voices.retain(|v| v.sound_id != sound_id);
                    changed = voices.len() != before;
                    let _ = reply.send(Ok(()));
                }
                Ok(Command::StopAll { reply }) => {
                    changed = !voices.is_empty();
                    voices.clear();
                    let _ = reply.send(Ok(()));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            let before = voices.len();
            voices.retain(|v| !v.player.empty());
            if changed || voices.len() != before {
                on_change(
                    voices
                        .iter()
                        .map(|v| PlayingSound {
                            sound_id: v.sound_id.clone(),
                            device_id: v.device_id.clone(),
                        })
                        .collect(),
                );
            }
        }
    })?;

    ready_rx.recv().context("audio thread exited")??;
    Ok(AudioHandle { tx })
}
