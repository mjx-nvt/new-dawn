use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::error::AppError;
use crate::kernel::registry::{Registry, Workspace, WorkspacePatch};

const CURRENT_WORKSPACE_KEY: &str = "current_workspace";
const ONBOARDING_DONE: &str = "onboarding_done";

static WORKSPACE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/workspace");

/// The only place a SQLite file is opened (rule 1): WAL, foreign keys, then migrations.
pub(crate) async fn open_pool(path: &Path, migrator: &Migrator) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .with_context(|| format!("opening {}", path.display()))?;
    migrator
        .run(&pool)
        .await
        .with_context(|| format!("migrating {}", path.display()))?;
    Ok(pool)
}

/// A consistent copy of an open database (works with WAL and concurrent readers).
pub(crate) async fn vacuum_into(pool: &SqlitePool, path: &Path) -> Result<()> {
    sqlx::query("VACUUM INTO ?")
        .bind(path.display().to_string())
        .execute(pool)
        .await
        .with_context(|| format!("copying a database to {}", path.display()))?;
    Ok(())
}

pub(crate) async fn checkpoint_and_close(pool: SqlitePool) {
    if let Err(err) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
    {
        tracing::warn!("WAL checkpoint failed: {err}");
    }
    pool.close().await;
}

/// Copies a folder tree (files and sub-folders; nothing when `from` doesn't exist).
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(to).with_context(|| format!("creating {}", to.display()))?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), &target)
                .with_context(|| format!("copying {}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Owns the registry and one pool per open workspace. All data access goes through here.
pub struct WorkspaceHub {
    registry: Registry,
    pools: RwLock<HashMap<String, SqlitePool>>,
}

impl WorkspaceHub {
    /// `root` is the app data dir: `~/Library/Application Support/co.majorisdigital.newdawn/`.
    pub async fn open(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
        Ok(Self {
            registry: Registry::open(&root).await?,
            pools: RwLock::default(),
        })
    }

    /// Opens (and migrates) every active workspace. One that fails is logged and left closed —
    /// its screens say so — instead of stopping the app from starting.
    pub async fn open_all_active(&self) -> Result<()> {
        for workspace in self.registry.list().await?.iter().filter(|w| !w.archived) {
            if let Err(err) = self.open_workspace(workspace).await {
                tracing::error!("workspace {} could not be opened: {err:#}", workspace.slug);
            }
        }
        Ok(())
    }

    pub fn pool(&self, slug: &str) -> Result<SqlitePool> {
        let pools = self.pools.read().expect("hub lock poisoned");
        Ok(pools
            .get(slug)
            .cloned()
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "Workspace '{slug}' is not open (if it failed to open at launch, the log in ~/Library/Logs/{} says why)",
                    crate::kernel::storage::LOG_FOLDER
                ))
            })?)
    }

    /// App-wide setting from `registry.db`.
    pub async fn app_setting(&self, key: &str) -> Result<Option<String>> {
        self.registry.setting(key).await
    }

    pub async fn set_app_setting(&self, key: &str, value: &str) -> Result<()> {
        self.registry.set_setting(key, value).await
    }

    /// The first-run setup was finished or skipped.
    pub async fn onboarding_done(&self) -> Result<bool> {
        Ok(self.registry.setting(ONBOARDING_DONE).await?.as_deref() == Some("1"))
    }

    pub async fn set_onboarding_done(&self, done: bool) -> Result<()> {
        self.registry
            .set_setting(ONBOARDING_DONE, if done { "1" } else { "0" })
            .await
    }

    pub async fn workspace(&self, slug: &str) -> Result<Workspace> {
        self.registry.get(slug).await
    }

    pub async fn list(&self) -> Result<Vec<Workspace>> {
        self.registry.list().await
    }

    /// The workspace named by `setting.current_workspace`; if that is unset, missing or archived,
    /// the first active workspace; `None` when there is none.
    pub async fn current_workspace(&self) -> Result<Option<Workspace>> {
        let all = self.registry.list().await?;
        let wanted = self.registry.setting(CURRENT_WORKSPACE_KEY).await?;
        let is_wanted = |w: &&Workspace| !w.archived && Some(&w.slug) == wanted.as_ref();
        Ok(all
            .iter()
            .find(is_wanted)
            .or_else(|| all.iter().find(|w| !w.archived))
            .cloned())
    }

    pub async fn set_current_workspace(&self, slug: &str) -> Result<Workspace> {
        if self.registry.get(slug).await?.archived {
            anyhow::bail!(AppError::invalid(format!("Workspace '{slug}' is archived")));
        }
        self.registry
            .set_setting(CURRENT_WORKSPACE_KEY, slug)
            .await?;
        self.registry.touch_opened(slug).await?;
        self.registry.get(slug).await
    }

    pub async fn create_workspace(&self, name: &str, slug: &str) -> Result<Workspace> {
        let workspace = self.registry.create(name, slug).await?;
        if let Err(err) = self.open_workspace(&workspace).await {
            // Leave no registry row pointing at a database that failed to open.
            self.registry.remove(slug).await?;
            return Err(err);
        }
        Ok(workspace)
    }

    pub async fn update_workspace(&self, slug: &str, patch: WorkspacePatch) -> Result<Workspace> {
        let workspace = self.registry.update(slug, patch).await?;
        if workspace.archived {
            self.close_workspace(slug).await;
        } else {
            self.open_workspace(&workspace).await?;
        }
        Ok(workspace)
    }

    /// Closes the pool, moves the folder to the Trash (recoverable), then forgets the workspace.
    pub async fn delete_workspace(&self, slug: &str) -> Result<()> {
        let workspace = self.registry.get(slug).await?;
        self.close_workspace(slug).await;
        if Path::new(&workspace.folder_path).exists() {
            trash::delete(&workspace.folder_path)
                .with_context(|| format!("moving {} to the Trash", workspace.folder_path))?;
        }
        self.registry.remove(slug).await
    }

    /// Copies `registry.db`, every `workspace.db` and each workspace's `brand/` folder (template,
    /// tokens, logos) into `dir` (`workspaces/<slug>/`). Open databases are copied with VACUUM
    /// INTO; archived workspaces are closed, so their file (and any WAL a failed checkpoint left)
    /// is copied as is.
    pub async fn snapshot_into(&self, dir: &Path) -> Result<()> {
        self.registry.vacuum_into(&dir.join("registry.db")).await?;
        for workspace in self.registry.list().await? {
            let target = dir.join("workspaces").join(&workspace.slug);
            std::fs::create_dir_all(&target)
                .with_context(|| format!("creating {}", target.display()))?;
            let out = target.join("workspace.db");
            let open = self
                .pools
                .read()
                .expect("hub lock poisoned")
                .get(&workspace.slug)
                .cloned();
            match open {
                Some(pool) => vacuum_into(&pool, &out).await?,
                None => {
                    for file in ["workspace.db", "workspace.db-wal"] {
                        let source = Path::new(&workspace.folder_path).join(file);
                        if source.is_file() {
                            std::fs::copy(&source, target.join(file))
                                .with_context(|| format!("copying {}", source.display()))?;
                        }
                    }
                }
            }
            copy_dir(
                &Path::new(&workspace.folder_path).join("brand"),
                &target.join("brand"),
            )?;
        }
        Ok(())
    }

    /// Checkpoints and closes every pool, registry last. Called on app exit.
    pub async fn close_all(&self) {
        let pools: Vec<SqlitePool> = self
            .pools
            .write()
            .expect("hub lock poisoned")
            .drain()
            .map(|(_, p)| p)
            .collect();
        for pool in pools {
            checkpoint_and_close(pool).await;
        }
        self.registry.close().await;
    }

    async fn open_workspace(&self, workspace: &Workspace) -> Result<SqlitePool> {
        if let Some(pool) = self
            .pools
            .read()
            .expect("hub lock poisoned")
            .get(&workspace.slug)
        {
            return Ok(pool.clone());
        }
        let path = Path::new(&workspace.folder_path).join("workspace.db");
        let pool = open_pool(&path, &WORKSPACE_MIGRATOR).await?;
        let existing = {
            let mut pools = self.pools.write().expect("hub lock poisoned");
            match pools.get(&workspace.slug) {
                Some(existing) => existing.clone(),
                None => {
                    pools.insert(workspace.slug.clone(), pool.clone());
                    return Ok(pool);
                }
            }
        };
        // Another caller opened it first; close our duplicate cleanly.
        checkpoint_and_close(pool).await;
        Ok(existing)
    }

    async fn close_workspace(&self, slug: &str) {
        let pool = self.pools.write().expect("hub lock poisoned").remove(slug);
        if let Some(pool) = pool {
            checkpoint_and_close(pool).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceHub;

    /// Create → tables exist → close → reopen (an app restart) → still listed and open.
    #[test]
    fn workspace_survives_restart() {
        let root = std::env::temp_dir().join(format!("newdawn-hub-test-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            let created = hub.create_workspace("VIFC", "vifc").await.unwrap();
            assert!(root.join("workspaces/vifc/workspace.db").exists());
            for dir in ["brand", "recordings", "exports"] {
                assert!(root.join("workspaces/vifc").join(dir).is_dir(), "{dir}");
            }
            assert!(
                hub.create_workspace("Again", "vifc").await.is_err(),
                "duplicate slug"
            );
            hub.close_all().await;

            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.open_all_active().await.unwrap();
            let listed = hub.list().await.unwrap();
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].id, created.id);

            let pool = hub.pool("vifc").unwrap();
            let (tables,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name IN \
                 ('meeting_type', 'document_template', 'meeting', 'transcript_segment', 'summary', \
                  'document_export', 'ai_log', 'setting', 'transcript_fts', 'summary_fts')",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(tables, 10);
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn current_workspace_falls_back() {
        let root = std::env::temp_dir().join(format!("newdawn-hub-test-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            assert!(hub.current_workspace().await.unwrap().is_none());

            hub.create_workspace("Acme", "acme").await.unwrap();
            hub.create_workspace("VIFC", "vifc").await.unwrap();
            // Nothing chosen yet: first by name.
            assert_eq!(hub.current_workspace().await.unwrap().unwrap().slug, "acme");

            hub.set_current_workspace("vifc").await.unwrap();
            let current = hub.current_workspace().await.unwrap().unwrap();
            assert_eq!(current.slug, "vifc");
            assert!(current.last_opened_at.is_some());

            // Archiving the current one falls back; archived ones can't be chosen.
            hub.update_workspace(
                "vifc",
                crate::kernel::registry::WorkspacePatch {
                    name: None,
                    archived: Some(true),
                    owner_title: None,
                },
            )
            .await
            .unwrap();
            assert_eq!(hub.current_workspace().await.unwrap().unwrap().slug, "acme");
            assert!(hub.set_current_workspace("vifc").await.is_err());
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
