//! Keeps the data folder from growing without end (D-031). The only thing that grows is audio:
//! a 16 kHz WAV is 115 MB an hour and an imported file keeps its size. So: once a transcript is
//! saved the original becomes a 48 kbps AAC (`afconvert`, which macOS ships; ~20 MB an hour)
//! and the original goes; recordings of summarised meetings are trashed after the retention
//! period (default 90 days, 0 = keep forever); `ai_log` rows older than 90 days are pruned.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;

pub const RETENTION_SETTING: &str = "retention_days";
pub const DEFAULT_RETENTION_DAYS: u32 = 90;
const AI_LOG_DAYS: i64 = 90;
/// The compressed recording. Originals are `audio.<ext>` (what was recorded or imported).
const COMPRESSED: &str = "recording.m4a";
const BITRATE: &str = "48000";

/// The log folder's name in `~/Library/Logs`; a debug build keeps its own (see `data_dir`).
pub const LOG_FOLDER: &str = if cfg!(debug_assertions) {
    "co.majorisdigital.newdawn.dev"
} else {
    "co.majorisdigital.newdawn"
};

/// The app's data folder. A debug build (`pnpm tauri dev`, the tests) keeps its own
/// `co.majorisdigital.newdawn.dev`, so developing never touches the installed app's data (D-044).
pub fn data_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    if !cfg!(debug_assertions) {
        return Ok(dir);
    }
    let mut name = dir.file_name().unwrap_or_default().to_owned();
    name.push(".dev");
    Ok(dir.with_file_name(name))
}

/// The log folder: `~/Library/Logs/co.majorisdigital.newdawn` (`….dev` in a debug build).
pub fn log_dir(home: &Path) -> PathBuf {
    home.join("Library/Logs").join(LOG_FOLDER)
}

/// `path` is an original (not yet compressed) recording.
pub fn is_original(path: &Path) -> bool {
    path.file_stem().and_then(|s| s.to_str()) == Some("audio")
}

/// Converts `original` to `recording.m4a` beside it (mono AAC, 48 kbps) and deletes the
/// original. Returns the new path. Already-compressed files come back unchanged.
/// One encode at a time: "Compress now" and the compression after a transcript could otherwise
/// write the same temp file (and two afconverts would fight for the CPU anyway).
static COMPRESSING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn compress(original: &Path) -> Result<PathBuf> {
    if !is_original(original) {
        return Ok(original.to_owned());
    }
    let _one_at_a_time = COMPRESSING.lock().await;
    let folder = original.parent().context("recording folder")?;
    let target = folder.join(COMPRESSED);
    if !original.is_file() {
        // Compressed meanwhile by whoever held the lock before us.
        if target.is_file() {
            return Ok(target);
        }
        bail!("{} is gone", original.display());
    }
    let tmp = folder.join("recording.tmp.m4a");
    let output = tokio::process::Command::new("/usr/bin/afconvert")
        .args([
            "-f", "m4af", "-d", "aac", "-b", BITRATE, "-q", "127", "--mix",
        ])
        .arg(original)
        .arg(&tmp)
        .output()
        .await
        .context("running afconvert")?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&tmp);
        bail!(
            "afconvert failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let size = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    if size < 4096 {
        let _ = std::fs::remove_file(&tmp);
        bail!("afconvert wrote an empty file");
    }
    // Durable before the original goes: a power cut can't leave a half-written replacement.
    std::fs::File::open(&tmp)?.sync_all()?;
    std::fs::rename(&tmp, &target).with_context(|| format!("saving {}", target.display()))?;
    if let Err(err) = std::fs::remove_file(original) {
        tracing::warn!("compressed, but the original stayed: {err}");
    }
    Ok(target)
}

/// Compresses a meeting's recording after its transcript is saved and points the meeting at
/// the new file. Not fatal: the original is still playable and transcribable.
pub async fn compress_meeting(
    pool: &SqlitePool,
    meeting_id: &str,
    audio_path: &Path,
) -> Result<()> {
    let new_path = compress(audio_path).await?;
    if new_path != audio_path {
        set_audio_path(pool, meeting_id, Some(&new_path.display().to_string())).await?;
    }
    Ok(())
}

async fn set_audio_path(pool: &SqlitePool, meeting_id: &str, path: Option<&str>) -> Result<()> {
    sqlx::query(
        "UPDATE meeting SET audio_path = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(path)
    .bind(meeting_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn retention_days(hub: &WorkspaceHub) -> Result<u32> {
    Ok(hub
        .app_setting(RETENTION_SETTING)
        .await?
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS))
}

pub async fn set_retention_days(hub: &WorkspaceHub, days: u32) -> Result<()> {
    if ![0, 30, 90, 180, 365].contains(&days) {
        bail!(AppError::invalid(
            "Choose 30, 90, 180 or 365 days, or 0 to keep forever"
        ));
    }
    hub.set_app_setting(RETENTION_SETTING, &days.to_string())
        .await
}

#[derive(sqlx::FromRow)]
struct Expired {
    id: String,
    audio_path: String,
}

/// Trashes the recordings of summarised meetings whose summary is older than `days` (the
/// transcript, summary and minutes stay). Returns how many.
pub async fn trash_old_recordings(hub: &WorkspaceHub, slug: &str, days: u32) -> Result<u32> {
    if days == 0 {
        return Ok(0);
    }
    let pool = hub.pool(slug)?;
    // The folder must be reachable (an unplugged drive must not strip the pointers).
    let folder = PathBuf::from(hub.workspace(slug).await?.folder_path)
        .join("recordings")
        .canonicalize()
        .context("the recordings folder isn't reachable")?;
    let rows: Vec<Expired> = sqlx::query_as(
        "SELECT m.id, m.audio_path FROM meeting m JOIN summary s ON s.meeting_id = m.id \
         WHERE m.status = 'summarised' AND m.audio_path IS NOT NULL \
         AND s.updated_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)",
    )
    .bind(format!("-{days} days"))
    .fetch_all(&pool)
    .await?;
    let mut count = 0;
    for row in rows {
        let path = PathBuf::from(&row.audio_path);
        match path.canonicalize() {
            // Only files that really live in this workspace's recordings folder.
            Ok(real) if real.starts_with(&folder) && real.is_file() => {
                crate::modules::meetings::review::to_trash(real).await?;
            }
            Ok(_) => {
                tracing::warn!(
                    "{} is outside the recordings folder; left alone",
                    path.display()
                );
                continue;
            }
            Err(_) => {} // already gone: just clear the pointer
        }
        set_audio_path(&pool, &row.id, None).await?;
        count += 1;
    }
    Ok(count)
}

/// Compresses every original recording of meetings that aren't being worked on. Returns how
/// many were compressed.
pub async fn compress_all(hub: &WorkspaceHub, slug: &str) -> Result<u32> {
    let pool = hub.pool(slug)?;
    let rows: Vec<Expired> = sqlx::query_as(
        "SELECT id, audio_path FROM meeting WHERE audio_path IS NOT NULL \
         AND status IN ('recorded', 'transcribed', 'summarised', 'failed')",
    )
    .fetch_all(&pool)
    .await?;
    let mut count = 0;
    for row in rows {
        let path = PathBuf::from(&row.audio_path);
        if path.is_file() && is_original(&path) {
            // One undecodable file must not block the rest.
            match compress_meeting(&pool, &row.id, &path).await {
                Ok(()) => count += 1,
                Err(err) => tracing::warn!("could not compress {}: {err:#}", path.display()),
            }
        }
    }
    Ok(count)
}

pub async fn prune_ai_log(pool: &SqlitePool) -> Result<u64> {
    let done = sqlx::query(
        "DELETE FROM ai_log WHERE created_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)",
    )
    .bind(format!("-{AI_LOG_DAYS} days"))
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

/// Sizes are bytes as f64: the bindings export refuses u64.
#[derive(Debug, Clone, Serialize, Type)]
pub struct WorkspaceStorage {
    pub slug: String,
    pub name: String,
    pub recordings_bytes: f64,
    /// Recordings still in their original (uncompressed) form.
    pub originals: u32,
    pub exports_bytes: f64,
    pub database_bytes: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct StorageReport {
    pub workspaces: Vec<WorkspaceStorage>,
    pub models_bytes: f64,
    pub logs_bytes: f64,
    pub retention_days: u32,
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

fn count_originals(recordings: &Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(recordings) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| {
            std::fs::read_dir(e.path())
                .map(|d| d.flatten().any(|f| is_original(&f.path())))
                .unwrap_or(false)
        })
        .count() as u32
}

pub async fn report(app: &AppHandle) -> Result<StorageReport> {
    let hub = app.state::<WorkspaceHub>();
    let mut workspaces = Vec::new();
    for w in hub.list().await?.into_iter().filter(|w| !w.archived) {
        let folder = PathBuf::from(&w.folder_path);
        workspaces.push(WorkspaceStorage {
            slug: w.slug,
            name: w.name,
            recordings_bytes: dir_size(&folder.join("recordings")) as f64,
            originals: count_originals(&folder.join("recordings")),
            exports_bytes: dir_size(&folder.join("exports")) as f64,
            database_bytes: ["workspace.db", "workspace.db-wal"]
                .iter()
                .map(|f| {
                    std::fs::metadata(folder.join(f))
                        .map(|m| m.len())
                        .unwrap_or(0)
                })
                .sum::<u64>() as f64,
        });
    }
    let logs = std::env::var_os("HOME")
        .map(|h| dir_size(&log_dir(Path::new(&h))))
        .unwrap_or(0);
    Ok(StorageReport {
        workspaces,
        models_bytes: dir_size(&data_dir(app)?.join("models")) as f64,
        logs_bytes: logs as f64,
        retention_days: retention_days(&hub).await?,
    })
}

/// Retention and log pruning for every open workspace; errors are logged, never fatal.
async fn housekeeping(app: &AppHandle) {
    let hub = app.state::<WorkspaceHub>();
    let Ok(days) = retention_days(&hub).await else {
        return;
    };
    let Ok(list) = hub.list().await else { return };
    for w in list.into_iter().filter(|w| !w.archived) {
        if let Ok(pool) = hub.pool(&w.slug) {
            if let Err(err) = prune_ai_log(&pool).await {
                tracing::warn!("ai_log prune failed for {}: {err:#}", w.slug);
            }
        }
        match trash_old_recordings(&hub, &w.slug, days).await {
            Ok(n) if n > 0 => tracing::info!(
                "{n} recording(s) of {} past {days} days moved to the Trash",
                w.slug
            ),
            Ok(_) => {}
            Err(err) => tracing::warn!("retention failed for {}: {err:#}", w.slug),
        }
    }
}

/// Two minutes after launch, then every 24 h of wall-clock time (checked hourly, so a sleeping
/// Mac doesn't drift the schedule).
pub fn spawn_scheduler(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        let mut last = std::time::SystemTime::UNIX_EPOCH;
        loop {
            let due = std::time::SystemTime::now()
                .duration_since(last)
                .map(|d| d >= Duration::from_secs(86_400))
                .unwrap_or(true);
            if due {
                housekeeping(&app).await;
                last = std::time::SystemTime::now();
            }
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compresses_a_wav_and_removes_the_original() {
        let dir = std::env::temp_dir().join(format!("newdawn-storage-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        // One second of 16 kHz silence as WAV.
        let samples = vec![0i16; 16_000];
        let mut bytes = Vec::new();
        let data_len = (samples.len() * 2) as u32;
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&16_000u32.to_le_bytes());
        bytes.extend_from_slice(&32_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        let wav = dir.join("audio.wav");
        std::fs::write(&wav, bytes).unwrap();
        let out = tauri::async_runtime::block_on(compress(&wav)).unwrap();
        assert_eq!(out, dir.join("recording.m4a"));
        assert!(out.is_file() && !wav.exists());
        assert!(!is_original(&out));
        assert_eq!(
            tauri::async_runtime::block_on(compress(&out)).unwrap(),
            out,
            "left alone"
        );
        assert_eq!(count_originals(&dir.parent().unwrap().join("nope")), 0);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
