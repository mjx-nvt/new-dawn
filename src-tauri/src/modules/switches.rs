//! The two modules a workspace turns on or off — Projects and CRM (D-059). One workspace setting
//! (`modules`, JSON), both on until the owner switches one off. Off hides the module, pauses its
//! agents and skips it on Today; its data stays, and everything returns when it is on again.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

const KEY: &str = "modules";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Modules {
    pub projects: bool,
    pub crm: bool,
}

impl Default for Modules {
    fn default() -> Self {
        Self { projects: true, crm: true }
    }
}

/// Which modules this workspace has on. A missing or unreadable value means both.
pub async fn enabled(pool: &SqlitePool) -> Result<Modules> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM setting WHERE key = ?")
        .bind(KEY)
        .fetch_optional(pool)
        .await?;
    Ok(row
        .and_then(|(v,)| serde_json::from_str(&v).ok())
        .unwrap_or_default())
}

pub async fn set(pool: &SqlitePool, modules: Modules) -> Result<()> {
    sqlx::query(
        "INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(KEY)
    .bind(serde_json::to_string(&modules)?)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_on_by_default_and_a_choice_round_trips() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
            sqlx::query(
                "CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL, \
                 created_at TEXT NOT NULL DEFAULT '', updated_at TEXT NOT NULL DEFAULT '')",
            )
            .execute(&pool)
            .await
            .unwrap();
            assert_eq!(enabled(&pool).await.unwrap(), Modules::default());
            let off = Modules { projects: false, crm: true };
            set(&pool, off).await.unwrap();
            assert_eq!(enabled(&pool).await.unwrap(), off);
            set(&pool, Modules::default()).await.unwrap();
            assert_eq!(enabled(&pool).await.unwrap(), Modules::default());
            // A value that isn't ours (a hand edit, an older build) reads as both on.
            sqlx::query("UPDATE setting SET value = 'x' WHERE key = 'modules'")
                .execute(&pool)
                .await
                .unwrap();
            assert_eq!(enabled(&pool).await.unwrap(), Modules::default());
        });
    }
}
