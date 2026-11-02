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

/// The time each record was last touched — a change on its timeline, a note or a task attached
/// to it — by id (D-070: the assistant's "no activity for N days").
pub async fn last_activity(pool: &SqlitePool, object: &ObjectDef, ids: &[String]) -> Result<std::collections::HashMap<String, String>> {
    let mut out: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let marks = vec!["?"; ids.len()].join(", ");
    let column = format!("{}_id", snake(&object.meta.name_singular));
    let mut sources = Vec::new();
    if let Some(col) = target_column(pool, object).await? {
        sources.push(format!("SELECT {col} AS rid, max(happens_at) AS at FROM timeline_activity WHERE {col} IN ({marks}) GROUP BY {col}"));
    }
    for junction in ["note_target", "task_target"] {
        // The column comes from the metadata and is checked against the table before it's used.
        let has: Option<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT name FROM pragma_table_info('{junction}') WHERE name = ?"))).bind(&column).fetch_optional(pool).await?;
        if has.is_some() {
            sources.push(format!("SELECT {column} AS rid, max(created_at) AS at FROM {junction} WHERE {column} IN ({marks}) GROUP BY {column}"));
        }
    }
    for sql in sources {
        let mut q = sqlx::query_as::<_, (String, String)>(sqlx::AssertSqlSafe(sql));
        for id in ids {
            q = q.bind(id);
        }
        for (rid, at) in q.fetch_all(pool).await? {
            if out.get(&rid).is_none_or(|known| known < &at) {
                out.insert(rid, at);
            }
        }
    }
    Ok(out)
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
