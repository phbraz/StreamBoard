use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};

use rodio::{Decoder, Source, buffer::SamplesBuffer};

pub const SUPPORTED: &[&str] = &["mp3", "ogg", "oga", "wav", "flac", "m4a"];

pub struct Decoded {
    pub buffer: SamplesBuffer,
    pub duration_ms: u32,
}

pub fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| SUPPORTED.contains(&e.as_str()))
}

pub fn decode_file(path: &Path) -> anyhow::Result<Decoded> {
    finish(Decoder::try_from(std::fs::File::open(path)?)?)
}

pub fn decode_bytes(bytes: Vec<u8>) -> anyhow::Result<Decoded> {
    finish(Decoder::try_from(Cursor::new(bytes))?)
}

fn finish<R: Read + Seek + Send + Sync + 'static>(decoder: Decoder<R>) -> anyhow::Result<Decoded> {
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    let samples: Vec<_> = decoder.collect();
    anyhow::ensure!(!samples.is_empty(), "no audio samples");
    let buffer = SamplesBuffer::new(channels, sample_rate, samples);
    let duration_ms = buffer.total_duration().map_or(0, |d| d.as_millis() as u32);
    Ok(Decoded { buffer, duration_ms })
}

/// Lowercase, ASCII alphanumerics separated by single dashes.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    let out: String = out.chars().take(48).collect();
    if out.is_empty() { "sound".into() } else { out }
}

pub fn label_from_stem(stem: &str) -> String {
    stem.replace(['-', '_'], " ").trim().to_string()
}
