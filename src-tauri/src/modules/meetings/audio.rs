//! Any supported audio file → 16 kHz mono f32 in [-1, 1], ready for VAD and Whisper.

use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result, bail};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, Resampler, SincInterpolationParameters};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::error::AppError;
use crate::kernel::ai::whisper::SAMPLE_RATE;

/// Extensions the import dialog accepts (all decodable by the enabled symphonia features).
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "m4a", "mp4", "aac", "mp3", "wav", "aif", "aiff", "caf", "flac", "ogg",
];

/// Decode → mix to mono → peak-normalise → resample to 16 kHz → clamp.
// ponytail: whole file in memory (mono at the source rate + a 16 kHz copy; ~0.9 GB peak for 1 h at
// 48 kHz). Stream-resample per packet if the recorder starts producing multi-hour files.
pub fn load_16k_mono(path: &Path) -> Result<Vec<f32>> {
    let (mono, rate) = decode_mono(path)?;
    if mono.is_empty() {
        bail!(AppError::invalid("The file contains no audio"));
    }
    let mut mono = normalise(mono);
    if rate as usize != SAMPLE_RATE {
        mono = resample(&mono, rate)?;
    }
    for s in &mut mono {
        *s = s.clamp(-1.0, 1.0);
    }
    Ok(mono)
}

/// Mono samples plus the sample rate **of the decoded stream** (not the container header, which
/// is wrong for HE-AAC m4a files and makes them play at double speed — Meetily's fix).
fn decode_mono(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|_| AppError::invalid("This audio format is not supported"))?;
    let track = format
        .default_track(TrackType::Audio)
        .context("no audio track")?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .context("no audio codec parameters")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|_| AppError::invalid("This audio codec is not supported"))?;

    let mut mono = Vec::new();
    let mut rate = 0;
    let mut interleaved: Vec<f32> = Vec::new();
    while let Some(packet) = format.next_packet().context("reading audio")? {
        if packet.track_id != track_id {
            continue;
        }
        let buffer = match decoder.decode(&packet) {
            Ok(buffer) => buffer,
            Err(SymphoniaError::DecodeError(_)) => continue, // skip a corrupt packet
            Err(err) => return Err(err).context("decoding audio"),
        };
        rate = buffer.spec().rate();
        let channels = buffer.spec().channels().count().max(1);
        interleaved.resize(buffer.samples_interleaved(), 0.0);
        buffer.copy_to_slice_interleaved(&mut interleaved);
        mono.extend(
            interleaved
                .chunks_exact(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
        );
    }
    Ok((mono, rate))
}

/// Peak-normalise to 0.95 and replace NaN/inf with silence.
fn normalise(mut samples: Vec<f32>) -> Vec<f32> {
    for s in &mut samples {
        if !s.is_finite() {
            *s = 0.0;
        }
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let gain = 0.95 / peak;
        for s in &mut samples {
            *s *= gain;
        }
    }
    samples
}

/// Sinc resampling (256 taps, BlackmanHarris2, rubato's automatic cutoff) of the whole clip.
fn resample(samples: &[f32], from_rate: u32) -> Result<Vec<f32>> {
    let ratio = SAMPLE_RATE as f64 / from_rate as f64;
    let mut resampler = Async::<f32>::new_sinc(
        ratio,
        1.0,
        &SincInterpolationParameters::default(),
        1024,
        1,
        FixedAsync::Input,
    )
    .context("creating the resampler")?;
    let input = InterleavedSlice::new(samples, 1, samples.len())
        .context("wrapping audio for the resampler")?;
    let output = resampler
        .process_all(&input, samples.len(), None)
        .context("resampling to 16 kHz")?;
    Ok(output.take_data())
}

#[cfg(test)]
mod tests {
    use super::load_16k_mono;
    use crate::kernel::ai::whisper::SAMPLE_RATE;

    /// macOS `say` writes a 22.05 kHz AIFF; it must come back as ~same-length 16 kHz audio.
    #[test]
    fn decodes_and_resamples_say_output() {
        let path = std::env::temp_dir().join(format!("newdawn-say-{}.aiff", uuid::Uuid::now_v7()));
        let ok = std::process::Command::new("say")
            .args([
                "-o",
                path.to_str().unwrap(),
                "--data-format=BEI16@22050",
                "Testing the audio decoder.",
            ])
            .status()
            .unwrap()
            .success();
        assert!(ok, "say failed");
        let samples = load_16k_mono(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
        assert!((0.8..6.0).contains(&seconds), "{seconds} s");
        assert!(samples.iter().all(|s| (-1.0..=1.0).contains(s)));
        assert!(
            samples.iter().any(|s| s.abs() > 0.5),
            "normalised audio should reach near peak"
        );
    }
}
