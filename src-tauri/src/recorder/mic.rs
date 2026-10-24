//! Default microphone via cpal. The callback only mixes to mono and hands ~100 ms chunks to a
//! bounded channel with `try_send` (never blocks; a full channel counts a drop). All processing
//! happens on the mixer thread. `cpal::Stream` is not `Send`, so this lives on the capture thread.

use std::sync::atomic::Ordering;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

use super::mixer::Shared;
use crate::error::AppError;

pub struct Mic {
    _stream: Stream,
    pub sample_rate: u32,
}

pub fn start(tx: SyncSender<Vec<f32>>, shared: Arc<Shared>) -> Result<Mic> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| AppError::new("no_microphone", "No microphone found"))?;
    let supported = device.default_input_config().context("reading the microphone format")?;
    let config = supported.config();
    let (rate, channels) = (config.sample_rate, config.channels.max(1) as usize);
    let chunk = (rate / 10) as usize; // 100 ms

    let mut pending: Vec<f32> = Vec::with_capacity(chunk * 2);
    let mut push = move |frame_mono: &mut dyn Iterator<Item = f32>| {
        pending.extend(frame_mono);
        if pending.len() >= chunk {
            let full = std::mem::replace(&mut pending, Vec::with_capacity(chunk * 2));
            if tx.try_send(full).is_err() {
                shared.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    };
    let on_error = |err| tracing::warn!("microphone stream error: {err}");

    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _: &_| push(&mut data.chunks(channels).map(|f| f.iter().sum::<f32>() / channels as f32)),
            on_error,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _: &_| {
                push(&mut data.chunks(channels).map(|f| f.iter().map(|s| *s as f32 / i16::MAX as f32).sum::<f32>() / channels as f32))
            },
            on_error,
            None,
        ),
        other => bail!(AppError::new("unsupported_microphone", format!("Microphone sample format {other:?} is not supported"))),
    }
    .context("opening the microphone")?;
    stream.play().context("starting the microphone")?;
    tracing::info!("microphone started: {:?} at {rate} Hz, {channels} ch", supported.sample_format());
    Ok(Mic { _stream: stream, sample_rate: rate })
}

#[cfg(test)]
mod tests {
    /// Live (`cargo test mic_raw_levels -- --ignored --nocapture`): raw mono chunks for ~1 s.
    #[test]
    #[ignore]
    fn mic_raw_levels() {
        let _ = tracing_subscriber::fmt().with_env_filter("info").with_test_writer().try_init();
        let (tx, rx) = std::sync::mpsc::sync_channel(50);
        let _mic = super::start(tx, Default::default()).unwrap();
        let speak = std::env::var("NEWDAWN_SPEAK").is_ok();
        let mut speaker = speak.then(|| std::process::Command::new("say").arg("New Dawn recorder test").spawn().unwrap());
        let mut all = Vec::new();
        while all.len() < if speak { 96_000 } else { 48_000 } {
            all.extend(rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap());
        }
        let peak = all.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let rms = (all.iter().map(|s| s * s).sum::<f32>() / all.len() as f32).sqrt();
        let nan = all.iter().filter(|s| !s.is_finite()).count();
        if let Some(s) = speaker.as_mut() {
            s.wait().unwrap();
        }
        eprintln!("raw mic: n={} peak={peak:.4} rms={rms:.4} non_finite={nan} first={:?}", all.len(), &all[..8]);
    }
}
