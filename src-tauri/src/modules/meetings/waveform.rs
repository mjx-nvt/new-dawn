//! The recording's shape for the player: `BUCKETS` peak values (0–255) over the whole file,
//! computed once from the decoded audio and cached as `waveform.json` beside the recording.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use super::audio;
use crate::error::AppError;
use crate::kernel::ai::whisper::SAMPLE_RATE;

/// Enough for a 3-px bar across any window; the UI merges buckets when narrower.
pub const BUCKETS: usize = 1200;
const FILE: &str = "waveform.json";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Waveform {
    /// One peak (max |sample|, 0–255) per equal slice of the recording.
    pub peaks: Vec<u8>,
    pub duration_ms: u32,
}

/// The cached waveform, or decode the recording once and cache it.
// ponytail: decodes the whole file through the transcription loader (16 kHz mono, a few seconds
// for an hour of AAC) on first open; computing it during transcription would save that wait.
pub async fn get(pool: &SqlitePool, meeting_id: &str) -> Result<Waveform> {
    let meeting = super::get(pool, meeting_id).await?;
    let audio_path = PathBuf::from(
        meeting
            .audio_path
            .ok_or_else(|| AppError::not_found("This meeting has no recording"))?,
    );
    let folder = audio_path
        .parent()
        .context("recording has no folder")?
        .to_path_buf();
    if let Ok(bytes) = tokio::fs::read(folder.join(FILE)).await {
        if let Ok(cached) = serde_json::from_slice::<Waveform>(&bytes) {
            if cached.peaks.len() == BUCKETS {
                return Ok(cached);
            }
        }
    }
    let samples = tokio::task::spawn_blocking(move || audio::load_16k_mono(&audio_path)).await??;
    let waveform = Waveform {
        peaks: peaks(&samples, BUCKETS),
        duration_ms: (samples.len() * 1000 / SAMPLE_RATE) as u32,
    };
    if let Err(err) = write(&folder, &waveform).await {
        tracing::warn!("could not cache {FILE}: {err:#}");
    }
    Ok(waveform)
}

/// Max |sample| per bucket, quantised to 0–255 (samples are already peak-normalised to ±1).
pub fn peaks(samples: &[f32], buckets: usize) -> Vec<u8> {
    if samples.is_empty() {
        return vec![0; buckets];
    }
    (0..buckets)
        .map(|b| {
            let from = b * samples.len() / buckets;
            let to = ((b + 1) * samples.len() / buckets)
                .max(from + 1)
                .min(samples.len());
            let peak = samples[from..to].iter().fold(0f32, |m, s| m.max(s.abs()));
            (peak.clamp(0.0, 1.0) * 255.0).round() as u8
        })
        .collect()
}

/// Temp file, fsync, rename — same as transcript.json.
async fn write(folder: &Path, waveform: &Waveform) -> Result<()> {
    let tmp = folder.join(format!("{FILE}.tmp"));
    let mut file = tokio::fs::File::create(&tmp).await?;
    file.write_all(&serde_json::to_vec(waveform)?).await?;
    file.sync_all().await?;
    tokio::fs::rename(&tmp, folder.join(FILE)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_cover_the_file_and_keep_the_loudest_sample() {
        let mut samples = vec![0.1f32; 1000];
        samples[999] = -1.0; // last bucket holds the loudest (negative) sample
        samples[0] = 0.5;
        let p = peaks(&samples, 10);
        assert_eq!(p.len(), 10);
        assert_eq!(p[0], 128);
        assert_eq!(p[9], 255);
        assert!(p[1..9].iter().all(|&v| v == 26));
        assert_eq!(peaks(&[], 4), vec![0; 4]);
        assert_eq!(peaks(&[0.4], 3).len(), 3); // fewer samples than buckets
    }
}
