//! The local API's keys (D-071): a key is shown once and kept only as its SHA-256; a request's
//! key is looked up across the open workspaces, and names the workspace it reads.

use std::io::Read;

use anyhow::{anyhow, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

/// Bytes from the system's random source, as hex.
pub fn random_hex(bytes: usize) -> Result<String> {
    let mut buf = vec![0u8; bytes];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(hex(&buf))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    /// What it's for (Plane's tokens have one; D-084).
    pub description: String,
    /// The key's first characters, to tell keys apart.
    pub prefix: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

/// A key just made: the record, and the key itself — shown this once.
#[derive(Debug, Clone, Serialize, Type)]
pub struct NewApiKey {
    pub key: ApiKey,
    pub secret: String,
}

const COLUMNS: &str = "id, name, description, prefix, expires_at, revoked_at, last_used_at, created_at";

pub async fn list(pool: &SqlitePool) -> Result<Vec<ApiKey>> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM api_key ORDER BY created_at DESC"))).fetch_all(pool).await?)
}

/// A new key, valid for `expires_days` (forever when none). One key opens both of the local
/// API's path families: Twenty's `/rest` and Plane's `/api/v1` (D-084).
pub async fn create(pool: &SqlitePool, name: &str, description: &str, expires_days: Option<u32>) -> Result<NewApiKey> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(anyhow!(AppError::invalid("A key needs a name of at most 100 characters")));
    }
    if description.trim().chars().count() > 500 {
        return Err(anyhow!(AppError::invalid("A description is at most 500 characters")));
    }
    let secret = format!("nd_{}", random_hex(24)?);
    let id = uuid::Uuid::now_v7().to_string();
    // The expiry in the same text as "now", so the lookup compares like with like.
    let expires = expires_days.map(|d| format!("+{} days", d.clamp(1, 3650)));
    sqlx::query("INSERT INTO api_key (id, name, description, prefix, key_hash, expires_at) VALUES (?, ?, ?, ?, ?, CASE WHEN ? IS NULL THEN NULL ELSE strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?) END)")
    .bind(&id)
    .bind(name)
    .bind(description.trim())
    .bind(&secret[..10])
    .bind(sha256_hex(&secret))
    .bind(&expires)
    .bind(&expires)
    .execute(pool)
    .await?;
    let key: ApiKey = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM api_key WHERE id = ?"))).bind(&id).fetch_one(pool).await?;
    Ok(NewApiKey { key, secret })
}

pub async fn revoke(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE api_key SET revoked_at = {NOW}, updated_at = {NOW} WHERE id = ? AND revoked_at IS NULL"))).bind(id).execute(pool).await?;
    Ok(())
}

/// The key in this workspace, when it's valid (not revoked, not expired); its use is noted.
pub async fn check(pool: &SqlitePool, secret: &str) -> Result<Option<String>> {
    let found: Option<(String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id, name FROM api_key WHERE key_hash = ? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > {NOW})")))
        .bind(sha256_hex(secret))
        .fetch_optional(pool)
        .await?;
    let Some((id, name)) = found else { return Ok(None) };
    // Noted at most once a minute: a busy script doesn't write on every read.
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE api_key SET last_used_at = {NOW} WHERE id = ? AND (last_used_at IS NULL OR last_used_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-1 minute'))"))).bind(&id).execute(pool).await?;
    Ok(Some(name))
}

/// The workspace a request's key belongs to, and the key's name.
pub async fn find(hub: &WorkspaceHub, secret: &str) -> Result<Option<(String, String)>> {
    if !secret.starts_with("nd_") {
        return Ok(None);
    }
    for ws in hub.list().await?.into_iter().filter(|w| !w.archived) {
        let Ok(pool) = hub.pool(&ws.slug) else { continue };
        if let Some(name) = check(&pool, secret).await? {
            return Ok(Some((ws.slug, name)));
        }
    }
    Ok(None)
}
