//! Daily backup of `registry.db` + every `workspace.db` into a folder the user chose (D-026):
//! `<folder>/New Dawn <yyyy-mm-dd>/`, newest 7 kept. "Nightly" = once a day while the app runs
//! (checked at launch and hourly). Each workspace's brand folder (template, logos) is included;
//! recordings and exports are not.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, SecondsFormat, Utc};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;

const FOLDER: &str = "backup_folder";
const LAST_AT: &str = "backup_last_at";
const LAST_ERROR: &str = "backup_last_error";
const PREFIX: &str = "New Dawn ";
/// Marks a backup still being written (left behind only by a quit or crash mid-backup).
const PARTIAL: &str = ".partial-";
const KEEP: usize = 7;
const EVERY: chrono::TimeDelta = chrono::TimeDelta::hours(24);

#[derive(Debug, Clone, Serialize, Type)]
pub struct BackupStatus {
    /// `None`: backups are off.
    pub folder: Option<String>,
    pub last_at: Option<String>,
    pub last_error: Option<String>,
}

pub async fn status(hub: &WorkspaceHub) -> Result<BackupStatus> {
    let get = |key| hub.app_setting(key);
    Ok(BackupStatus {
        folder: get(FOLDER).await?.filter(|f| !f.is_empty()),
        last_at: get(LAST_AT).await?.filter(|v| !v.is_empty()),
        last_error: get(LAST_ERROR).await?.filter(|v| !v.is_empty()),
    })
}

/// `None` turns backups off.
pub async fn set_folder(hub: &WorkspaceHub, folder: Option<&str>) -> Result<BackupStatus> {
    if let Some(folder) = folder {
        let path = Path::new(folder);
        if !path.is_absolute() || !path.is_dir() {
            bail!(AppError::invalid("Choose an existing folder"));
        }
    }
    hub.set_app_setting(FOLDER, folder.unwrap_or_default())
        .await?;
    status(hub).await
}

/// One backup at a time (the hourly check and "Back up now" can meet).
static RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Backs up now into today's folder (replacing an earlier backup from today), then prunes.
pub async fn run(hub: &WorkspaceHub) -> Result<BackupStatus> {
    let _one_at_a_time = RUNNING.lock().await;
    let Some(folder) = status(hub).await?.folder else {
        bail!(AppError::invalid("Choose a backup folder first"));
    };
    let result = write(hub, Path::new(&folder)).await;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    match &result {
        Ok(_) => {
            hub.set_app_setting(LAST_AT, &now).await?;
            hub.set_app_setting(LAST_ERROR, "").await?;
        }
        Err(err) => {
            // What Settings shows: the plain message, without the error code.
            let message = match err.downcast_ref::<AppError>() {
                Some(app) => app.message.clone(),
                None => format!("{err:#}"),
            };
            hub.set_app_setting(LAST_ERROR, &message).await?
        }
    }
    result?;
    status(hub).await
}

async fn write(hub: &WorkspaceHub, dest: &Path) -> Result<PathBuf> {
    if !dest.is_dir() {
        bail!(AppError::not_found(format!(
            "The backup folder {} is missing (an unplugged drive?)",
            dest.display()
        )));
    }
    let name = format!("{PREFIX}{}", Local::now().format("%Y-%m-%d"));
    let target = dest.join(&name);
    // Build beside it, then swap: a failed backup never replaces a good one.
    let partial = dest.join(format!(".{name}{PARTIAL}{}", uuid::Uuid::now_v7()));
    let saved: Result<()> = async {
        std::fs::create_dir_all(&partial)
            .with_context(|| format!("creating {}", partial.display()))?;
        hub.snapshot_into(&partial).await?;
        if is_backup(&target) {
            std::fs::remove_dir_all(&target)
                .with_context(|| format!("replacing {}", target.display()))?;
        }
        std::fs::rename(&partial, &target).with_context(|| format!("saving {}", target.display()))
    }
    .await;
    if let Err(err) = saved {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(err);
    }
    // The backup is good; a failed clean-up is only worth a log line.
    if let Err(err) = prune(dest) {
        tracing::warn!("could not prune old backups: {err:#}");
    }
    Ok(target)
}

/// Only folders this code made: "New Dawn YYYY-MM-DD" holding a registry.db.
fn is_backup(path: &Path) -> bool {
    let named = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix(PREFIX))
        .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok());
    named && path.join("registry.db").is_file()
}

/// Keeps the newest `KEEP` backups (by the date in the name) and removes leftovers of
/// interrupted ones (runs under the `RUNNING` lock, so none is in progress).
fn prune(dest: &Path) -> Result<()> {
    let entries: Vec<PathBuf> = std::fs::read_dir(dest)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    for leftover in entries.iter().filter(|p| {
        p.is_dir()
            && p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&format!(".{PREFIX}")) && n.contains(PARTIAL))
    }) {
        std::fs::remove_dir_all(leftover)
            .with_context(|| format!("removing {}", leftover.display()))?;
    }
    let mut backups: Vec<PathBuf> = entries.into_iter().filter(|p| is_backup(p)).collect();
    backups.sort();
    let excess = backups.len().saturating_sub(KEEP);
    for old in &backups[..excess] {
        std::fs::remove_dir_all(old).with_context(|| format!("removing {}", old.display()))?;
    }
    Ok(())
}

/// Runs a backup when one is due (folder set, last one over 24 h ago). Errors are recorded in
/// the status for Settings, never shown as a dialog.
async fn run_if_due(hub: &WorkspaceHub) {
    let Ok(current) = status(hub).await else {
        return;
    };
    if current.folder.is_none() {
        return;
    }
    let due = current
        .last_at
        .and_then(|at| DateTime::parse_from_rfc3339(&at).ok())
        .is_none_or(|at| Utc::now() - at.with_timezone(&Utc) >= EVERY);
    if due {
        if let Err(err) = run(hub).await {
            tracing::warn!("backup failed: {err:#}");
        }
    }
}

/// Checks a minute after launch, then hourly.
pub fn spawn_scheduler(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(60)).await;
        loop {
            run_if_due(&app.state::<WorkspaceHub>()).await;
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backs_up_and_keeps_seven() {
        let root = std::env::temp_dir().join(format!("newdawn-backup-{}", uuid::Uuid::now_v7()));
        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.join("data")).await.unwrap();
            let ws = hub.create_workspace("VIFC", "vifc").await.unwrap();
            std::fs::write(Path::new(&ws.folder_path).join("brand/tokens.json"), "{}").unwrap();
            assert!(run(&hub).await.is_err(), "off until a folder is chosen");
            assert!(set_folder(&hub, Some("relative/path")).await.is_err());
            set_folder(&hub, Some(&dest.display().to_string()))
                .await
                .unwrap();

            // Eight older backups and a stranger's folder.
            for day in 1..=8 {
                let old = dest.join(format!("New Dawn 2026-01-0{day}"));
                std::fs::create_dir_all(&old).unwrap();
                std::fs::write(old.join("registry.db"), "").unwrap();
            }
            std::fs::create_dir_all(dest.join("New Dawn notes")).unwrap();
            std::fs::create_dir_all(dest.join(".New Dawn 2026-01-09.partial-x")).unwrap();

            let done = run(&hub).await.unwrap();
            assert!(done.last_at.is_some() && done.last_error.is_none());
            let today = dest.join(format!("New Dawn {}", Local::now().format("%Y-%m-%d")));
            assert!(today.join("registry.db").is_file());
            assert!(today.join("workspaces/vifc/workspace.db").is_file());
            assert!(today.join("workspaces/vifc/brand/tokens.json").is_file());
            run(&hub).await.unwrap(); // same day: replaced, not duplicated

            let mut kept: Vec<String> = std::fs::read_dir(&dest)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            kept.sort();
            assert_eq!(
                kept.len(),
                8,
                "7 backups + the folder that isn't ours: {kept:?}"
            );
            assert!(kept.contains(&"New Dawn notes".to_owned()));
            assert!(
                !kept.iter().any(|n| n.contains(".partial-")),
                "leftovers removed"
            );
            assert!(
                !kept.contains(&"New Dawn 2026-01-02".to_owned()),
                "oldest pruned"
            );
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
