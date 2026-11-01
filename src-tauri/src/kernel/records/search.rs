//! The record engine's search index (D-060): one FTS5 table over every object's text, kept up to
//! date by the engine on each write, so `/` finds a company by a word of its tagline.

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use specta::Type;
use sqlx::SqlitePool;

use super::types::{FieldType, ObjectDef};

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Hit {
    pub object: String,
    pub record_id: String,
}

/// The words of a record the index should see: its text fields, composite text parts and lists.
pub fn text_of(object: &ObjectDef, record: &Value) -> String {
    let mut words = Vec::new();
    for field in object.fields.iter().filter(|f| f.is_active && f.r#type.is_searchable()) {
        let Some(value) = record.get(&field.name) else { continue };
        if field.r#type == FieldType::Select {
            let label = value.as_str().and_then(|v| field.options.select.iter().find(|o| o.value == v)).map(|o| o.label.clone());
            words.extend(label);
            continue;
        }
        if field.r#type == FieldType::MultiSelect {
            if let Some(values) = value.as_array() {
                for v in values.iter().filter_map(Value::as_str) {
                    words.extend(field.options.select.iter().find(|o| o.value == v).map(|o| o.label.clone()));
                }
            }
            continue;
        }
        collect(value, &mut words);
    }
    words.retain(|w| !w.trim().is_empty());
    fold_d(&words.join(" "))
}

fn collect(value: &Value, words: &mut Vec<String>) {
    match value {
        Value::String(s) => words.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| collect(v, words)),
        Value::Object(map) => map.values().for_each(|v| collect(v, words)),
        _ => {}
    }
}

pub async fn index(pool: &SqlitePool, object: &ObjectDef, record: &Value) -> Result<()> {
    let Some(id) = record.get("id").and_then(Value::as_str) else { return Ok(()) };
    remove(pool, &object.meta.name_singular, id).await?;
    let text = text_of(object, record);
    if text.is_empty() {
        return Ok(());
    }
    sqlx::query("INSERT INTO search_index (object, record_id, text) VALUES (?, ?, ?)")
        .bind(&object.meta.name_singular)
        .bind(id)
        .bind(text)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn remove(pool: &SqlitePool, object: &str, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM search_index WHERE object = ? AND record_id = ?")
        .bind(object)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// đ / Đ are their own code points: `remove_diacritics` leaves them, so both sides fold them (D-022).
fn fold_d(text: &str) -> String {
    text.replace('đ', "d").replace('Đ', "D")
}

/// An FTS5 query from what the owner typed: each word as a prefix, all required.
pub fn match_query(text: &str) -> String {
    fold_d(text)
        .split_whitespace()
        .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

pub async fn find(pool: &SqlitePool, text: &str, limit: u32) -> Result<Vec<Hit>> {
    let query = match_query(text);
    if query.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as("SELECT object, record_id FROM search_index WHERE search_index MATCH ? ORDER BY rank LIMIT ?")
        .bind(query)
        .bind(limit)
        .fetch_all(pool)
        .await?)
}

/// Indexes the live records the index lacks and drops the rows of records gone or deleted —
/// writers outside the engine (the people memory, Tasks) don't index. Run at every open.
pub async fn sync(pool: &SqlitePool, object: &ObjectDef) -> Result<()> {
    let missing: Vec<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT id FROM {} WHERE deleted_at IS NULL AND id NOT IN (SELECT record_id FROM search_index WHERE object = ?)",
        object.meta.table_name
    )))
    .bind(&object.meta.name_singular)
    .fetch_all(pool)
    .await?;
    for (id,) in missing {
        reindex(pool, object, &id).await?;
    }
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DELETE FROM search_index WHERE object = ? AND record_id NOT IN (SELECT id FROM {} WHERE deleted_at IS NULL)",
        object.meta.table_name
    )))
    .bind(&object.meta.name_singular)
    .execute(pool)
    .await?;
    Ok(())
}

/// Indexes one record afresh (a writer outside the engine just changed it).
pub async fn reindex(pool: &SqlitePool, object: &ObjectDef, id: &str) -> Result<()> {
    match super::query::get(pool, object, id, false).await {
        Ok(record) => index(pool, object, &record).await,
        Err(_) => remove(pool, &object.meta.name_singular, id).await,
    }
}
