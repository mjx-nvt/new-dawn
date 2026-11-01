//! Timeline activities (D-060): what happened to a record — created, each change with its diff,
//! deleted, restored — shown on its record page and read by the agents.

use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;

use super::query::Actor;
use super::types::ObjectDef;

/// The column of `timeline_activity` that points at this object's records, when it has one
/// (the standard record-bearing objects; custom objects add theirs when they are made).
pub async fn target_column(pool: &SqlitePool, object: &ObjectDef) -> Result<Option<String>> {
    if object.meta.is_system {
        return Ok(None); // a workspace member's or a target's changes aren't a record's history
    }
    let column = format!("{}_id", snake(&object.meta.name_singular));
    let rows: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_table_info('timeline_activity')")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().any(|(n,)| n == column).then_some(column))
}

/// Writes one activity on a record: `event` is created · updated · deleted · restored, and
/// `properties` carries the diff for an update (`{field: {before, after}}`).
pub async fn record(pool: &SqlitePool, object: &ObjectDef, record_id: &str, event: &str, properties: Value, actor: &Actor) -> Result<()> {
    let Some(column) = target_column(pool, object).await? else { return Ok(()) };
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO timeline_activity (id, name, properties_json, workspace_member_id, {column}) VALUES (?, ?, ?, ?, ?)"
    )))
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(format!("{}.{event}", object.meta.name_singular))
    .bind(properties.to_string())
    .bind(actor.member_id.as_deref())
    .bind(record_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Forgets a record's activities (a permanent delete).
pub async fn forget(pool: &SqlitePool, object: &ObjectDef, record_id: &str) -> Result<()> {
    let Some(column) = target_column(pool, object).await? else { return Ok(()) };
    sqlx::query(sqlx::AssertSqlSafe(format!("DELETE FROM timeline_activity WHERE {column} = ?")))
        .bind(record_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// camelCase → snake_case (`workspaceMember` → `workspace_member`).
pub fn snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
