use std::{collections::HashMap, sync::mpsc, thread, time::Duration};

use anyhow::Context;
use rodio::{Player, buffer::SamplesBuffer};
use tokio::sync::oneshot;

use crate::protocol::PlayingSound;

const POLL: Duration = Duration::from_millis(50);

type Reply = oneshot::Sender<Result<(), String>>;

enum Command {
    Load { sound_id: String, buffer: SamplesBuffer },
    Unload { sound_id: String },
    Play { sound_id: String, device_id: String, reply: Reply },
    Stop { sound_id: String, reply: Reply },
    StopAll { reply: Reply },
}

#[derive(Clone)]
pub struct AudioHandle {
    tx: mpsc::Sender<Command>,
}

impl AudioHandle {
    pub fn load(&self, sound_id: String, buffer: SamplesBuffer) {
        let _ = self.tx.send(Command::Load { sound_id, buffer });
    }

    pub fn unload(&self, sound_id: String) {
        let _ = self.tx.send(Command::Unload { sound_id });
    }

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
    master_volume: f32,
    on_change: impl Fn(Vec<PlayingSound>) + Send + 'static,
) -> anyhow::Result<AudioHandle> {
    let (tx, rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<anyhow::Result<()>>();

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
        let mut buffers: HashMap<String, SamplesBuffer> = HashMap::new();
        let mut voices: Vec<Voice> = Vec::new();

        loop {
            let mut changed = false;
            match rx.recv_timeout(POLL) {
                Ok(Command::Load { sound_id, buffer }) => {
                    buffers.insert(sound_id, buffer);
                }
                Ok(Command::Unload { sound_id }) => {
                    buffers.remove(&sound_id);
                    let before = voices.len();
                    voices.retain(|v| v.sound_id != sound_id);
                    changed = voices.len() != before;
                }
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
