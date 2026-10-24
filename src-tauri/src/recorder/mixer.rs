//! The mixer thread: everything heavy happens here, never in the audio callbacks. Each source is
//! resampled to 16 kHz; the mic gets a soft limiter (Core Audio float input is not clipped, and
//! laptop speakers feeding the mic measured 4.7× full scale), an 80 Hz high-pass and loudness
//! normalisation towards −23 LUFS; both are aligned in 600 ms windows, summed with proportional scaling above ±1, and
//! written to the WAV unless paused. Per-source levels and the "system audio is silent" flag are
//! published through `Shared` for the 10 Hz meter.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use anyhow::{Context, Result};
use ebur128::{EbuR128, Mode};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, Resampler, SincInterpolationParameters};

use super::saver::{RATE, WavWriter};

const WINDOW: usize = RATE as usize * 600 / 1000;
/// If system audio runs ahead of the mic by more than this, the oldest excess is dropped.
const MAX_SYSTEM_BACKLOG: usize = 3 * WINDOW;
/// Exact zeros from the tap for this long, while the mic hears something, means no permission.
const SILENT_AFTER_MS: u32 = 3_000;
const TARGET_LUFS: f64 = -23.0;
const MAX_GAIN: f32 = 10.0; // +20 dB

/// State shared between the audio callbacks, the mixer and the session.
#[derive(Default)]
pub struct Shared {
    you: AtomicU32,  // f32 bits, RMS 0..1
    them: AtomicU32, // f32 bits, RMS 0..1
    pub paused: AtomicBool,
    pub system_silent: AtomicBool,
    /// Chunks dropped because a bounded channel was full (callbacks never wait).
    pub dropped: AtomicU64,
}

impl Shared {
    pub fn levels(&self) -> (f32, f32) {
        (
            f32::from_bits(self.you.load(Ordering::Relaxed)),
            f32::from_bits(self.them.load(Ordering::Relaxed)),
        )
    }
}

pub struct Source {
    pub rx: Receiver<Vec<f32>>,
    pub rate: u32,
}

/// Runs until the mic channel closes (the session dropped the streams). Returns samples written.
pub fn run(
    mic: Source,
    system: Option<Source>,
    shared: Arc<Shared>,
    mut wav: WavWriter,
) -> Result<u64> {
    let mut mic_chain = MicChain::new(mic.rate)?;
    let mut sys_resampler = match &system {
        Some(s) => Some(ToRate::new(s.rate)?),
        None => None,
    };
    let mut mic_q: VecDeque<f32> = VecDeque::new();
    let mut sys_q: VecDeque<f32> = VecDeque::new();
    let mut zero_ms = 0u32;

    loop {
        match mic.rx.recv_timeout(Duration::from_millis(200)) {
            Ok(chunk) => {
                store(&shared.you, rms(&chunk).min(1.0));
                mic_q.extend(mic_chain.process(&chunk)?);
            }
            Err(RecvTimeoutError::Timeout) => {
                // Mic stalled (device unplugged, Bluetooth drop): keep the call going with mic
                // silence once the other side has a full window waiting.
                if sys_q.len() >= WINDOW && mic_q.len() < WINDOW {
                    store(&shared.you, 0.0);
                    mic_q.extend(std::iter::repeat_n(0.0, WINDOW - mic_q.len()));
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if let (Some(system), Some(resampler)) = (&system, sys_resampler.as_mut()) {
            while let Ok(chunk) = system.rx.try_recv() {
                let them = rms(&chunk).min(1.0);
                store(&shared.them, them);
                zero_ms = if chunk.iter().all(|s| *s == 0.0) {
                    zero_ms + chunk_ms(&chunk, system.rate)
                } else {
                    0
                };
                let mic_active = shared.levels().0 > 0.005;
                shared
                    .system_silent
                    .store(zero_ms >= SILENT_AFTER_MS && mic_active, Ordering::Relaxed);
                sys_q.extend(resampler.process(&chunk)?);
            }
        }
        while mic_q.len() >= WINDOW {
            let window = mix_window(&mut mic_q, &mut sys_q, WINDOW);
            if !shared.paused.load(Ordering::Relaxed) {
                wav.write(&window)?;
            }
        }
    }

    // Stopping: write what is left, then finalise the header.
    if !mic_q.is_empty() && !shared.paused.load(Ordering::Relaxed) {
        let len = mic_q.len();
        let window = mix_window(&mut mic_q, &mut sys_q, len);
        wav.write(&window)?;
    }
    wav.finish()
}

/// Takes `len` mic samples and up to `len` system samples (silence-padded), sums them and scales
/// the window down proportionally if any sample would exceed ±1. Drops system backlog beyond
/// `MAX_SYSTEM_BACKLOG` so clock drift can't grow without bound.
fn mix_window(mic_q: &mut VecDeque<f32>, sys_q: &mut VecDeque<f32>, len: usize) -> Vec<f32> {
    if sys_q.len() > MAX_SYSTEM_BACKLOG {
        sys_q.drain(..sys_q.len() - MAX_SYSTEM_BACKLOG);
    }
    let take_sys = sys_q.len().min(len);
    let mut out: Vec<f32> = mic_q.drain(..len).collect();
    for (o, s) in out.iter_mut().zip(sys_q.drain(..take_sys)) {
        *o += s;
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 1.0 {
        for o in &mut out {
            *o /= peak;
        }
    }
    out
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

fn store(level: &AtomicU32, value: f32) {
    level.store(value.to_bits(), Ordering::Relaxed);
}

fn chunk_ms(chunk: &[f32], rate: u32) -> u32 {
    (chunk.len() as u64 * 1000 / rate.max(1) as u64) as u32
}

/// Streaming sinc resampler from `from` Hz to 16 kHz (pass-through when already 16 kHz).
struct ToRate {
    resampler: Option<Async<f32>>,
    pending: Vec<f32>,
}

impl ToRate {
    fn new(from: u32) -> Result<Self> {
        let resampler = if from == RATE {
            None
        } else {
            let ratio = RATE as f64 / from as f64;
            let params = SincInterpolationParameters::default();
            Some(
                Async::<f32>::new_sinc(ratio, 1.0, &params, 1024, 1, FixedAsync::Input)
                    .context("creating a resampler")?,
            )
        };
        Ok(Self {
            resampler,
            pending: Vec::new(),
        })
    }

    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        let Some(resampler) = self.resampler.as_mut() else {
            return Ok(input.to_vec());
        };
        self.pending.extend_from_slice(input);
        let mut out = Vec::new();
        loop {
            let need = resampler.input_frames_next();
            if self.pending.len() < need {
                break;
            }
            let block =
                InterleavedSlice::new(&self.pending[..need], 1, need).context("wrapping audio")?;
            out.extend(
                resampler
                    .process(&block, None)
                    .context("resampling")?
                    .take_data(),
            );
            self.pending.drain(..need);
        }
        Ok(out)
    }
}

/// Mic path: soft limit → resample → 80 Hz high-pass → smoothed loudness gain towards −23 LUFS.
struct MicChain {
    to_rate: ToRate,
    high_pass: Biquad,
    meter: EbuR128,
    gain: f32,
}

impl MicChain {
    fn new(rate: u32) -> Result<Self> {
        Ok(Self {
            to_rate: ToRate::new(rate)?,
            high_pass: Biquad::high_pass(80.0, RATE as f32),
            meter: EbuR128::new(1, RATE, Mode::S).context("creating the loudness meter")?,
            gain: 1.0,
        })
    }

    fn process(&mut self, chunk: &[f32]) -> Result<Vec<f32>> {
        // tanh ≈ identity for speech levels (< 3 % off below 0.3) and bounds feedback peaks to ±1.
        let limited: Vec<f32> = chunk.iter().map(|s| s.tanh()).collect();
        let mut samples = self.to_rate.process(&limited)?;
        for s in &mut samples {
            *s = self.high_pass.run(*s);
        }
        self.meter
            .add_frames_f32(&samples)
            .context("measuring loudness")?;
        let loudness = self.meter.loudness_shortterm().unwrap_or(f64::NEG_INFINITY);
        if loudness > -50.0 {
            // Only adapt while someone is speaking, so silence is never boosted into hiss.
            let target = (10f64.powf((TARGET_LUFS - loudness) / 20.0) as f32).clamp(0.5, MAX_GAIN);
            self.gain = self.gain * 0.9 + target * 0.1;
        }
        for s in &mut samples {
            *s *= self.gain;
        }
        Ok(samples)
    }
}

/// RBJ biquad (direct form I).
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl Biquad {
    fn high_pass(cutoff: f32, rate: f32) -> Self {
        let w = 2.0 * std::f32::consts::PI * cutoff / rate;
        let alpha = w.sin() / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        let cos = w.cos();
        let a0 = 1.0 + alpha;
        Self {
            b: [
                (1.0 + cos) / 2.0 / a0,
                -(1.0 + cos) / a0,
                (1.0 + cos) / 2.0 / a0,
            ],
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            x: [0.0; 2],
            y: [0.0; 2],
        }
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes_with_proportional_scaling_and_padding() {
        let mut mic: VecDeque<f32> = vec![0.8; 4].into();
        let mut sys: VecDeque<f32> = vec![0.8, 0.2].into();
        let out = mix_window(&mut mic, &mut sys, 4);
        // 1.6 peak scaled to 1.0; 1.0/1.6 for the mixed pair; missing system samples are silence
        assert!((out[0] - 1.0).abs() < 1e-6);
        assert!((out[1] - 1.0 / 1.6).abs() < 1e-6);
        assert!((out[2] - 0.5).abs() < 1e-6);
        assert!(mic.is_empty() && sys.is_empty());
    }

    #[test]
    fn bounds_system_backlog() {
        let mut mic: VecDeque<f32> = vec![0.0; WINDOW].into();
        let mut sys: VecDeque<f32> = vec![0.1; 5 * WINDOW].into();
        mix_window(&mut mic, &mut sys, WINDOW);
        assert_eq!(sys.len(), MAX_SYSTEM_BACKLOG - WINDOW);
    }

    #[test]
    fn high_pass_removes_rumble_keeps_speech() {
        let amplitude = |freq: f32| {
            let mut f = Biquad::high_pass(80.0, RATE as f32);
            let out: Vec<f32> = (0..RATE as usize)
                .map(|i| f.run((2.0 * std::f32::consts::PI * freq * i as f32 / RATE as f32).sin()))
                .collect();
            out[RATE as usize / 2..]
                .iter()
                .fold(0.0f32, |m, s| m.max(s.abs()))
        };
        assert!(amplitude(20.0) < 0.1, "20 Hz rumble should be cut");
        assert!(amplitude(1000.0) > 0.95, "1 kHz speech band should pass");
    }

    #[test]
    fn resamples_48k_to_16k_in_chunks() {
        let mut r = ToRate::new(48_000).unwrap();
        let mut total = 0;
        for _ in 0..20 {
            total += r.process(&vec![0.25; 4_800]).unwrap().len(); // 20 × 100 ms
        }
        assert!((31_000..=32_100).contains(&total), "{total}"); // ≈ 2 s at 16 kHz, minus filter delay
    }
}
