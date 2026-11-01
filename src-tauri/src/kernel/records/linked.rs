//! What hangs off a record beside its fields (D-062): the tasks and notes that target it
//! (Twenty's task_target and note_target) and its attachments. Bodies and uploads come with the
//! notes release; here they are listed and made.

use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::{json, Value};
use specta::Type;
use sqlx::SqlitePool;

use super::query::{self, Actor};
use super::timeline::snake;
use super::types::ObjectDef;
use crate::error::AppError;

/// task → (task, taskTarget); note → (note, noteTarget).
fn kind_of(kind: &str) -> Result<(&'static str, &'static str)> {
    match kind {
        "task" => Ok(("task", "taskTarget")),
        "note" => Ok(("note", "noteTarget")),
        _ => Err(anyhow!(AppError::invalid("Linked records are tasks or notes"))),
    }
}

/// `<object>_id` on a junction or attachment table, when the object has one there (the standard
/// record objects do; custom objects get theirs with the data model settings).
async fn column_on(pool: &SqlitePool, table: &'static str, object: &ObjectDef) -> Result<Option<String>> {
    let column = format!("{}_id", snake(&object.meta.name_singular));
    let rows: Vec<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT name FROM pragma_table_info('{table}')"))).fetch_all(pool).await?;
    Ok(rows.into_iter().any(|(n,)| n == column).then_some(column))
}

/// The tasks or notes that target a record, newest first, as the engine returns them.
pub async fn list(pool: &SqlitePool, object: &ObjectDef, record_id: &str, kind: &str) -> Result<Vec<Value>> {
    let (name, _) = kind_of(kind)?;
    let table = if name == "task" { "task_target" } else { "note_target" };
    let Some(column) = column_on(pool, table, object).await? else { return Ok(Vec::new()) };
    let target = super::object(pool, name).await?;
    let ids: Vec<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT x.id FROM {name} x JOIN {table} t ON t.{name}_id = x.id WHERE t.{column} = ? AND x.deleted_at IS NULL ORDER BY x.created_at DESC"
    )))
    .bind(record_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(ids.len());
    for (id,) in ids {
        out.push(query::get(pool, &target, &id, false).await?);
    }
    Ok(out)
}

/// A new task or note that targets the record: made, then linked, by the owner.
pub async fn add(pool: &SqlitePool, object: &ObjectDef, record_id: &str, kind: &str, title: &str, actor: &Actor) -> Result<Value> {
    let (name, junction) = kind_of(kind)?;
    let junction = super::object(pool, junction).await?;
    if junction.field(&object.meta.name_singular).is_none() {
        return Err(anyhow!(AppError::invalid(format!("A {name} can't be linked to {} yet", object.meta.label_plural.to_lowercase()))));
    }
    let title = title.trim();
    if title.is_empty() {
        return Err(anyhow!(AppError::invalid(format!("A {name} needs a title"))));
    }
    let target = super::object(pool, name).await?;
    let data = if name == "task" { json!({ "title": title, "status": "TODO" }) } else { json!({ "title": title }) };
    let made = query::create(pool, &target, &data, actor).await?;
    let id = made.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    let mut link = serde_json::Map::new();
    link.insert(name.to_owned(), Value::String(id));
    link.insert(object.meta.name_singular.clone(), Value::String(record_id.to_owned()));
    query::create(pool, &junction, &Value::Object(link), actor).await?;
    Ok(made)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub full_path: String,
    pub r#type: String,
    pub created_at: String,
}

/// A record's files, newest first.
pub async fn attachments(pool: &SqlitePool, object: &ObjectDef, record_id: &str) -> Result<Vec<Attachment>> {
    let Some(column) = column_on(pool, "attachment", object).await? else { return Ok(Vec::new()) };
    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT id, name, full_path, type, created_at FROM attachment WHERE {column} = ? AND deleted_at IS NULL ORDER BY created_at DESC"
    )))
    .bind(record_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id, name, full_path, r#type, created_at)| Attachment { id, name, full_path, r#type, created_at }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    /// A task added from a company's page targets the company and lists there; a note too.
    #[test]
    fn a_task_added_on_a_record_targets_it() {
        let root = std::env::temp_dir().join(format!("newdawn-linked-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::system("test");
            let company = super::super::object(&pool, "company").await.unwrap();
            let keppel = query::create(&pool, &company, &json!({ "name": "Keppel Land" }), &actor).await.unwrap();
            let id = keppel["id"].as_str().unwrap();
            assert!(list(&pool, &company, id, "task").await.unwrap().is_empty());
            let task = add(&pool, &company, id, "task", "  Send the proposal ", &actor).await.unwrap();
            assert_eq!(task["title"], "Send the proposal");
            let tasks = list(&pool, &company, id, "task").await.unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0]["id"], task["id"]);
            add(&pool, &company, id, "note", "Call notes", &actor).await.unwrap();
            assert_eq!(list(&pool, &company, id, "note").await.unwrap().len(), 1);
            assert!(add(&pool, &company, id, "task", "   ", &actor).await.is_err(), "a title");
            assert!(add(&pool, &company, id, "email", "x", &actor).await.is_err(), "tasks or notes");
            assert!(attachments(&pool, &company, id).await.unwrap().is_empty());
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
