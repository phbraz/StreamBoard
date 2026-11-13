use std::{collections::HashMap, sync::mpsc, thread, time::Duration};

use anyhow::Context;
use rodio::{
    DeviceSinkBuilder, Player,
    buffer::SamplesBuffer,
    cpal::{self, traits::HostTrait},
    stream::MixerDeviceSink,
};
use tokio::sync::oneshot;

use crate::protocol::PlayingSound;

const POLL: Duration = Duration::from_millis(50);

type Reply = oneshot::Sender<Result<(), String>>;

enum Command {
    Load {
        sound_id: String,
        buffer: SamplesBuffer,
    },
    Unload {
        sound_id: String,
    },
    Play {
        sound_id: String,
        device_id: String,
        reply: Reply,
    },
    Stop {
        sound_id: String,
        reply: Reply,
    },
    StopAll {
        reply: Reply,
    },
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
        self.call(|reply| Command::Play {
            sound_id,
            device_id,
            reply,
        })
        .await
    }

    pub async fn stop(&self, sound_id: String) -> Result<(), String> {
        self.call(|reply| Command::Stop { sound_id, reply }).await
    }

    pub async fn stop_all(&self) -> Result<(), String> {
        self.call(|reply| Command::StopAll { reply }).await
    }

    async fn call(&self, cmd: impl FnOnce(Reply) -> Command) -> Result<(), String> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(cmd(reply))
            .map_err(|_| "audio thread stopped".to_string())?;
        rx.await.map_err(|_| "audio thread stopped".to_string())?
    }
}

struct Voice {
    sound_id: String,
    device_id: String,
    player: Player,
}

/// Names of every output device the system offers, for the startup log and error messages.
pub fn output_device_names() -> Vec<String> {
    cpal::default_host()
        .output_devices()
        .map(|devices| devices.map(|d| d.to_string()).collect())
        .unwrap_or_default()
}

/// Opens the output named `output`, or the system default for `"default"`.
/// Matching ignores case and accepts a unique substring, so `"Sonar - Stream"` is enough.
fn open_sink(output: &str) -> anyhow::Result<MixerDeviceSink> {
    if output.eq_ignore_ascii_case("default") {
        return DeviceSinkBuilder::open_default_sink().map_err(|e| anyhow::anyhow!("{e}"));
    }

    let wanted = output.to_lowercase();
    let devices: Vec<_> = cpal::default_host()
        .output_devices()
        .map_err(|e| anyhow::anyhow!("listing output devices: {e}"))?
        .map(|d| (d.to_string(), d))
        .collect();

    let exact = devices
        .iter()
        .position(|(name, _)| name.to_lowercase() == wanted);
    let index = match exact {
        Some(i) => i,
        None => {
            let partial: Vec<usize> = devices
                .iter()
                .enumerate()
                .filter(|(_, (name, _))| name.to_lowercase().contains(&wanted))
                .map(|(i, _)| i)
                .collect();
            match partial.as_slice() {
                [i] => *i,
                [] => anyhow::bail!(
                    "no output device matches {output:?}. Available: {}",
                    devices
                        .iter()
                        .map(|(n, _)| format!("{n:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                many => anyhow::bail!(
                    "{output:?} matches several output devices, be more specific: {}",
                    many.iter()
                        .map(|&i| format!("{:?}", devices[i].0))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        }
    };

    let (name, device) = devices.into_iter().nth(index).expect("index is in range");
    tracing::info!(device = %name, "using audio output");
    DeviceSinkBuilder::from_device(device)
        .and_then(DeviceSinkBuilder::open_stream)
        .map_err(|e| anyhow::anyhow!("opening {name:?}: {e}"))
}

/// Starts the audio thread. The output device must be owned by a single thread,
/// so everything goes through a command channel.
pub fn spawn(
    output: String,
    master_volume: f32,
    on_change: impl Fn(Vec<PlayingSound>) + Send + 'static,
) -> anyhow::Result<AudioHandle> {
    let (tx, rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<anyhow::Result<()>>();

    thread::Builder::new().name("audio".into()).spawn(move || {
        let sink = match open_sink(&output) {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                let _ = ready_tx.send(Ok(()));
                sink
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e.context("opening audio output")));
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
                Ok(Command::Play {
                    sound_id,
                    device_id,
                    reply,
                }) => {
                    let result = match buffers.get(&sound_id) {
                        Some(buffer) => {
                            let player = Player::connect_new(sink.mixer());
                            player.set_volume(master_volume);
                            player.append(buffer.clone());
                            voices.push(Voice {
                                sound_id,
                                device_id,
                                player,
                            });
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
