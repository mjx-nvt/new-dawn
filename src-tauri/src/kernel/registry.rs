use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use uuid::Uuid;

use crate::error::AppError;
use crate::kernel::hub::{checkpoint_and_close, open_pool, vacuum_into};

static REGISTRY_MIGRATOR: Migrator = sqlx::migrate!("./migrations/registry");

/// Sub-folders every workspace gets next to its workspace.db (docs/plan.md §3).
const WORKSPACE_DIRS: [&str; 3] = ["brand", "recordings", "exports"];

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Workspace {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub folder_path: String,
    pub archived: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// The owner's job title in this workspace ("" = not set); the name is app-wide (D-039).
    pub owner_title: String,
}

#[derive(Debug, Deserialize, Type)]
pub struct WorkspacePatch {
    pub name: Option<String>,
    pub archived: Option<bool>,
    /// "" clears it.
    pub owner_title: Option<String>,
}

/// `registry.db`: the workspace list and app-wide settings.
pub struct Registry {
    pool: SqlitePool,
    root: PathBuf,
}

impl Registry {
    pub async fn open(root: &Path) -> Result<Self> {
        let pool = open_pool(&root.join("registry.db"), &REGISTRY_MIGRATOR).await?;
        Ok(Self {
            pool,
            root: root.to_owned(),
        })
    }

    pub async fn vacuum_into(&self, path: &Path) -> Result<()> {
        vacuum_into(&self.pool, path).await
    }

    pub async fn list(&self) -> Result<Vec<Workspace>> {
        Ok(
            sqlx::query_as("SELECT * FROM workspace ORDER BY name COLLATE NOCASE")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn get(&self, slug: &str) -> Result<Workspace> {
        let found: Option<Workspace> = sqlx::query_as("SELECT * FROM workspace WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;
        Ok(found.ok_or_else(|| AppError::not_found(format!("No workspace '{slug}'")))?)
    }

    /// Validates, creates `workspaces/<slug>/{brand,recordings,exports}` and inserts the row.
    /// The hub then opens (and so creates and migrates) `workspace.db`.
    pub async fn create(&self, name: &str, slug: &str) -> Result<Workspace> {
        let name = name.trim();
        if name.is_empty() {
            bail!(AppError::invalid("Name is required"));
        }
        validate_slug(slug)?;
        let taken: Option<(String,)> = sqlx::query_as("SELECT id FROM workspace WHERE slug = ?")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?;
        if taken.is_some() {
            bail!(AppError::invalid(format!(
                "Slug '{slug}' is already in use"
            )));
        }

        let folder = self.root.join("workspaces").join(slug);
        for dir in WORKSPACE_DIRS {
            let path = folder.join(dir);
            std::fs::create_dir_all(&path)
                .with_context(|| format!("creating {}", path.display()))?;
        }

        sqlx::query("INSERT INTO workspace (id, slug, name, folder_path) VALUES (?, ?, ?, ?)")
            .bind(Uuid::now_v7().to_string())
            .bind(slug)
            .bind(name)
            .bind(folder.to_string_lossy())
            .execute(&self.pool)
            .await?;
        self.get(slug).await
    }

    pub async fn update(&self, slug: &str, patch: WorkspacePatch) -> Result<Workspace> {
        let name = patch.name.as_deref().map(str::trim);
        if name == Some("") {
            bail!(AppError::invalid("Name is required"));
        }
        let owner_title = patch
            .owner_title
            .as_deref()
            .map(|t| crate::kernel::profile::clean(t, 100));
        let done = sqlx::query(
            "UPDATE workspace SET name = coalesce(?, name), archived = coalesce(?, archived), \
             owner_title = coalesce(?, owner_title), \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE slug = ?",
        )
        .bind(name)
        .bind(patch.archived)
        .bind(owner_title)
        .bind(slug)
        .execute(&self.pool)
        .await?;
        if done.rows_affected() == 0 {
            bail!(AppError::not_found(format!("No workspace '{slug}'")));
        }
        self.get(slug).await
    }

    pub async fn setting(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM setting WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(value,)| value))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET \
             value = excluded.value, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn touch_opened(&self, slug: &str) -> Result<()> {
        sqlx::query(
            "UPDATE workspace SET last_opened_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE slug = ?",
        )
        .bind(slug)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove(&self, slug: &str) -> Result<()> {
        sqlx::query("DELETE FROM workspace WHERE slug = ?")
            .bind(slug)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn close(&self) {
        checkpoint_and_close(self.pool.clone()).await;
    }
}

/// Lowercase letters, digits and inner hyphens; 1–40 chars. Used as a folder name.
fn validate_slug(slug: &str) -> Result<()> {
    let ok = (1..=40).contains(&slug.len())
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !slug.starts_with('-')
        && !slug.ends_with('-');
    if !ok {
        bail!(AppError::invalid(
            "Slug must be 1–40 lowercase letters, digits or hyphens"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_slug;

    #[test]
    fn slug_rules() {
        for good in ["vifc", "acme-capital", "a1"] {
            assert!(validate_slug(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "VIFC",
            "-x",
            "x-",
            "a b",
            "../x",
            "x/y",
            &"a".repeat(41),
        ] {
            assert!(validate_slug(bad).is_err(), "{bad}");
        }
    }
}
