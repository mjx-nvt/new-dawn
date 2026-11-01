//! The record engine (D-060): Twenty's data model in SQLite. Objects and fields are metadata
//! (`object_meta`, `field_meta`); every object is a real table; the standard objects are declared
//! in `standard.rs` and synced at every open; custom objects and fields are run-time DDL (0.10.5).
//! Everything the CRM module shows goes through `query`.

pub mod query;
pub mod search;
pub mod standard;
pub mod timeline;
pub mod types;
pub mod views;

use anyhow::{anyhow, Result};
use sqlx::SqlitePool;

pub use query::{Actor, Page, Query};
pub use search::Hit;
pub use types::{FieldMeta, FieldRow, ObjectDef, ObjectMeta};

use crate::error::AppError;

const OBJECT_COLUMNS: &str = "id, name_singular, name_plural, label_singular, label_plural, icon, description, table_name, \
     is_custom, is_active, is_system, label_identifier_field_id, image_identifier_field_id, position";

const FIELD_COLUMNS: &str = "id, object_id, name, label, type, column_name, options_json, default_json, is_custom, is_active, \
     is_nullable, is_unique, is_system, position, description, icon";

/// Every object, system ones included, in sidebar order.
pub async fn objects(pool: &SqlitePool) -> Result<Vec<ObjectMeta>> {
    let all: Vec<ObjectMeta> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {OBJECT_COLUMNS} FROM object_meta ORDER BY position, label_plural")))
        .fetch_all(pool)
        .await?;
    // A table name the engine won't put in SQL (never from our declarations; the DDL layer checks
    // custom ones before they land here).
    Ok(all.into_iter().filter(|m| types::valid_ident(&m.table_name)).collect())
}

/// An object by its singular name, plural name or id, with its fields in order.
pub async fn object(pool: &SqlitePool, name: &str) -> Result<ObjectDef> {
    let meta: Option<ObjectMeta> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {OBJECT_COLUMNS} FROM object_meta WHERE name_singular = ? OR name_plural = ? OR id = ?"
    )))
    .bind(name)
    .bind(name)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    let meta = meta
        .filter(|m| types::valid_ident(&m.table_name))
        .ok_or_else(|| anyhow!(AppError::not_found(format!("No object named {name}"))))?;
    let fields = fields(pool, &meta.id).await?;
    Ok(ObjectDef { meta, fields })
}

pub async fn fields(pool: &SqlitePool, object_id: &str) -> Result<Vec<FieldMeta>> {
    let rows: Vec<FieldRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {FIELD_COLUMNS} FROM field_meta WHERE object_id = ? ORDER BY position, name")))
        .bind(object_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().filter_map(FieldRow::into_meta).collect())
}

/// The one workspace member: the owner.
pub async fn member_id(pool: &SqlitePool) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM workspace_member WHERE deleted_at IS NULL ORDER BY created_at LIMIT 1").fetch_optional(pool).await?;
    Ok(row.map(|(id,)| id))
}

/// The actor for what the owner does by hand in this workspace.
pub async fn owner_actor(pool: &SqlitePool, owner_name: &str) -> Result<Actor> {
    Ok(Actor::manual(member_id(pool).await?, owner_name))
}

/// A person's name split the way Twenty keeps it: the last word is the last name; one word is a
/// first name.
pub fn split_name(name: &str) -> (String, String) {
    let name = name.trim();
    match name.rsplit_once(' ') {
        Some((first, last)) => (first.trim().to_owned(), last.trim().to_owned()),
        None => (name.to_owned(), String::new()),
    }
}

/// Run at every workspace open, after the migrations: the standard metadata, the owner as the
/// workspace member, the people memory's names and organisations carried into Twenty's shape,
/// and the search index for tables that predate it. Idempotent.
pub async fn prepare(pool: &SqlitePool, owner_name: &str) -> Result<()> {
    standard::sync(pool).await?;
    let (first, last) = split_name(owner_name);
    match member_id(pool).await? {
        None => {
            sqlx::query("INSERT INTO workspace_member (id, name_first, name_last) VALUES (?, ?, ?)")
                .bind(uuid::Uuid::now_v7().to_string())
                .bind(&first)
                .bind(&last)
                .execute(pool)
                .await?;
        }
        // The owner renamed themselves in Settings: the member follows.
        Some(id) if !owner_name.trim().is_empty() => {
            sqlx::query(
                "UPDATE workspace_member SET name_first = ?1, name_last = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE id = ?3 AND (name_first <> ?1 OR name_last <> ?2)",
            )
            .bind(&first)
            .bind(&last)
            .bind(id)
            .execute(pool)
            .await?;
        }
        Some(_) => {}
    }
    let actor = Actor::system("New Dawn");
    // Names: one string → first and last.
    let unsplit: Vec<(String, String)> = sqlx::query_as("SELECT id, name FROM person WHERE name_first = '' AND name_last = '' AND name <> ''")
        .fetch_all(pool)
        .await?;
    for (id, name) in unsplit {
        let (first, last) = split_name(&name);
        sqlx::query("UPDATE person SET name_first = ?, name_last = ? WHERE id = ?")
            .bind(first)
            .bind(last)
            .bind(id)
            .execute(pool)
            .await?;
    }
    // Organisations: the text becomes a company and a link.
    let unlinked: Vec<(String, String)> = sqlx::query_as("SELECT id, organisation FROM person WHERE company_id IS NULL AND trim(organisation) <> ''")
        .fetch_all(pool)
        .await?;
    if !unlinked.is_empty() {
        let company = object(pool, "company").await?;
        let mut known: Vec<(String, String)> = sqlx::query_as("SELECT id, name FROM company WHERE deleted_at IS NULL").fetch_all(pool).await?;
        for (person_id, organisation) in unlinked {
            let folded = crate::kernel::text::fold(&organisation);
            let company_id = match known.iter().find(|(_, n)| crate::kernel::text::fold(n) == folded) {
                Some((id, _)) => id.clone(),
                None => {
                    let record = query::create(pool, &company, &serde_json::json!({ "name": organisation.trim() }), &actor).await?;
                    let id = record.get("id").and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
                    known.push((id.clone(), organisation.trim().to_owned()));
                    id
                }
            };
            sqlx::query("UPDATE person SET company_id = ? WHERE id = ?").bind(company_id).bind(person_id).execute(pool).await?;
        }
    }
    // The search index: what other writers added or removed since the last open.
    for meta in objects(pool).await? {
        if meta.is_system || !meta.is_active {
            continue;
        }
        let obj = object(pool, &meta.id).await?;
        views::ensure_index(pool, &obj).await?;
        search::sync(pool, &obj).await?;
    }
    Ok(())
}

/// Indexes one record afresh, for a writer outside the engine (the people memory, Tasks).
pub async fn reindex(pool: &SqlitePool, object_name: &str, id: &str) -> Result<()> {
    let obj = object(pool, object_name).await?;
    search::reindex(pool, &obj, id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_splits_at_its_last_space() {
        assert_eq!(split_name("Harry Do"), ("Harry".into(), "Do".into()));
        assert_eq!(split_name("Nguyen Van Tien"), ("Nguyen Van".into(), "Tien".into()));
        assert_eq!(split_name("Duc"), ("Duc".into(), String::new()));
        assert_eq!(split_name("  Mr. Tom "), ("Mr.".into(), "Tom".into()));
    }
}
