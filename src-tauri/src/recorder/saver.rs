//! Incremental 16 kHz mono 16-bit WAV. The header sizes are patched at every checkpoint (every
//! 2 s), so a file left behind by a crash is already playable; `repair` fixes the rest at launch.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

pub const RATE: u32 = 16_000;
const HEADER_LEN: u64 = 44;
const CHECKPOINT_EVERY: Duration = Duration::from_secs(2);

pub struct WavWriter {
    file: BufWriter<File>,
    samples: u64,
    last_checkpoint: Instant,
}

impl WavWriter {
    pub fn create(path: &Path) -> Result<Self> {
        let mut file = BufWriter::new(File::create(path).with_context(|| format!("creating {}", path.display()))?);
        file.write_all(&header(0))?;
        Ok(Self { file, samples: 0, last_checkpoint: Instant::now() })
    }

    /// Appends samples (clamped to ±1, written as i16) and checkpoints every 2 s.
    pub fn write(&mut self, samples: &[f32]) -> Result<()> {
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            self.file.write_all(&v.to_le_bytes())?;
        }
        self.samples += samples.len() as u64;
        if self.last_checkpoint.elapsed() >= CHECKPOINT_EVERY {
            self.checkpoint()?;
        }
        Ok(())
    }

    /// Flushes the data, rewrites the header with the current size and syncs to disk.
    fn checkpoint(&mut self) -> Result<()> {
        self.file.flush()?;
        let file = self.file.get_mut();
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&header(self.samples * 2))?;
        file.seek(SeekFrom::End(0))?;
        file.sync_data()?;
        self.last_checkpoint = Instant::now();
        Ok(())
    }

    /// Final checkpoint; returns the number of samples written.
    pub fn finish(mut self) -> Result<u64> {
        self.checkpoint()?;
        Ok(self.samples)
    }
}

/// Fixes the header of a WAV written by `WavWriter` from the real file length (after a crash).
/// Returns the number of samples in the file.
pub fn repair(path: &Path) -> Result<u64> {
    let mut file = OpenOptions::new().read(true).write(true).open(path).with_context(|| format!("opening {}", path.display()))?;
    let len = file.metadata()?.len();
    if len < HEADER_LEN {
        bail!("{} is too short to be a recording", path.display());
    }
    let data = (len - HEADER_LEN) & !1; // whole 16-bit samples only
    file.set_len(HEADER_LEN + data)?;
    file.write_all(&header(data))?;
    file.sync_all()?;
    Ok(data / 2)
}

pub fn duration_ms(samples: u64) -> u32 {
    (samples * 1000 / RATE as u64) as u32
}

fn header(data_bytes: u64) -> [u8; 44] {
    let data = data_bytes.min(u32::MAX as u64 - 36) as u32;
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36 + data).to_le_bytes());
    h[8..12].copy_from_slice(b"WAVE");
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    h[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
    h[22..24].copy_from_slice(&1u16.to_le_bytes()); // mono
    h[24..28].copy_from_slice(&RATE.to_le_bytes());
    h[28..32].copy_from_slice(&(RATE * 2).to_le_bytes()); // byte rate
    h[32..34].copy_from_slice(&2u16.to_le_bytes()); // block align
    h[34..36].copy_from_slice(&16u16.to_le_bytes()); // bits per sample
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data.to_le_bytes());
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::meetings::audio::load_16k_mono;

    fn tone(seconds: f32) -> Vec<f32> {
        (0..(seconds * RATE as f32) as usize).map(|i| (i as f32 * 0.05).sin() * 0.5).collect()
    }

    #[test]
    fn writes_a_playable_wav() {
        let path = std::env::temp_dir().join(format!("newdawn-wav-{}.wav", uuid::Uuid::now_v7()));
        let mut w = WavWriter::create(&path).unwrap();
        w.write(&tone(1.5)).unwrap();
        assert_eq!(w.finish().unwrap(), 24_000);
        let decoded = load_16k_mono(&path).unwrap();
        assert_eq!(decoded.len(), 24_000);
        std::fs::remove_file(path).unwrap();
    }

    /// A crash leaves a stale header and maybe half a sample; `repair` makes it whole again.
    #[test]
    fn repairs_a_crashed_recording() {
        let path = std::env::temp_dir().join(format!("newdawn-wav-{}.wav", uuid::Uuid::now_v7()));
        {
            let mut w = WavWriter::create(&path).unwrap();
            w.write(&tone(1.0)).unwrap();
            w.file.flush().unwrap(); // data on disk, header still says 0 bytes; then "crash"
        }
        let mut f = OpenOptions::new().append(true).open(&path).unwrap();
        f.write_all(&[0x7f]).unwrap(); // a torn final sample
        drop(f);
        assert_eq!(repair(&path).unwrap(), 16_000);
        assert_eq!(load_16k_mono(&path).unwrap().len(), 16_000);
        assert_eq!(duration_ms(16_000), 1_000);
        std::fs::remove_file(path).unwrap();
    }
}
