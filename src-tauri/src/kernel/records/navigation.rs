//! The sidebar's CRM section and ⌘K (D-064, 0.10.7): favorites in folders (a record or a view,
//! personal pins), the Workspace section's object order and hidden objects, every view for
//! "go to", and the records opened lately.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Column, Row, SqlitePool};

use super::timeline::snake;
use super::types::{valid_ident, ObjectDef, ObjectMeta};
use super::{member_id, object, objects};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
/// Recents kept per workspace.
const RECENTS_KEPT: i64 = 100;

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct FavoriteFolder {
    pub id: String,
    pub name: String,
    pub position: f64,
}

/// A pinned record or view, with what the sidebar shows for it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct Favorite {
    pub id: String,
    pub folder_id: Option<String>,
    pub position: f64,
    /// The object's plural name (the route), and its label.
    pub object: String,
    pub object_label: String,
    pub record_id: Option<String>,
    pub view_id: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Favorites {
    pub folders: Vec<FavoriteFolder>,
    pub items: Vec<Favorite>,
}

/// What to pin: a record (its object by singular / plural name or id) or a view.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct FavoriteTarget {
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub record_id: Option<String>,
    #[serde(default)]
    pub view_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct NavItem {
    pub object_id: String,
    /// The object's plural name (the route).
    pub object: String,
    pub label: String,
    pub icon: String,
    pub position: f64,
    pub is_hidden: bool,
    pub folder: String,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct NavItemInput {
    pub object_id: String,
    pub position: f64,
    pub is_hidden: bool,
    #[serde(default)]
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct ViewEntry {
    pub id: String,
    pub name: String,
    pub r#type: String,
    /// The object's plural name and label.
    pub object: String,
    pub object_label: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Recent {
    pub object: String,
    pub object_label: String,
    pub record_id: String,
    pub label: String,
    pub visited_at: String,
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// The record objects a favorite or the sidebar can point at: by their `<object>_id` column.
async fn record_objects(pool: &SqlitePool) -> Result<Vec<ObjectMeta>> {
    Ok(objects(pool).await?.into_iter().filter(|o| !o.is_system).collect())
}

/// The objects' definitions, each loaded once as the pins and the recents need them.
#[derive(Default)]
struct Defs(HashMap<String, ObjectDef>);

impl Defs {
    async fn get(&mut self, pool: &SqlitePool, meta: &ObjectMeta) -> Result<&ObjectDef> {
        if !self.0.contains_key(&meta.id) {
            self.0.insert(meta.id.clone(), object(pool, &meta.id).await?);
        }
        Ok(&self.0[&meta.id])
    }
}

/// A record's label, or None when it is gone or deleted.
async fn label_of(pool: &SqlitePool, def: &ObjectDef, record_id: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {} FROM {} t WHERE t.id = ? AND t.deleted_at IS NULL", def.label_sql("t"), def.meta.table_name)))
        .bind(record_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(l,)| l))
}

// ---- favorites ----------------------------------------------------------------------------

/// The folders and the pins, in order; a pin whose record or view is gone is left out.
pub async fn favorites(pool: &SqlitePool) -> Result<Favorites> {
    let folders: Vec<FavoriteFolder> = sqlx::query_as("SELECT id, name, position FROM favorite_folder ORDER BY position, name").fetch_all(pool).await?;
    let metas = record_objects(pool).await?;
    let by_column: HashMap<String, &ObjectMeta> = metas.iter().map(|m| (format!("{}_id", snake(&m.name_singular)), m)).collect();
    let rows = sqlx::query("SELECT * FROM favorite WHERE deleted_at IS NULL ORDER BY position, created_at").fetch_all(pool).await?;
    let mut items = Vec::with_capacity(rows.len());
    let mut defs = Defs::default();
    for row in rows {
        let id: String = row.try_get("id")?;
        let folder_id: Option<String> = row.try_get("folder_id")?;
        let position: f64 = row.try_get("position")?;
        let view_id: Option<String> = row.try_get("view_id")?;
        if let Some(view_id) = view_id {
            let view: Option<(String, String)> = sqlx::query_as("SELECT v.name, o.id FROM view v JOIN object_meta o ON o.id = v.object_id WHERE v.id = ?").bind(&view_id).fetch_optional(pool).await?;
            let Some((name, object_id)) = view else { continue };
            let Some(meta) = metas.iter().find(|m| m.id == object_id) else { continue };
            items.push(Favorite { id, folder_id, position, object: meta.name_plural.clone(), object_label: meta.label_plural.clone(), record_id: None, view_id: Some(view_id), label: name });
            continue;
        }
        // The one `<object>_id` column that is set names the record.
        let mut found = None;
        for column in row.columns() {
            let name = column.name();
            let Some(meta) = by_column.get(name) else { continue };
            if let Ok(Some(record_id)) = row.try_get::<Option<String>, _>(name) {
                found = Some((*meta, record_id));
                break;
            }
        }
        let Some((meta, record_id)) = found else { continue };
        let Some(label) = label_of(pool, defs.get(pool, meta).await?, &record_id).await? else { continue };
        items.push(Favorite { id, folder_id, position, object: meta.name_plural.clone(), object_label: meta.label_plural.clone(), record_id: Some(record_id), view_id: None, label });
    }
    Ok(Favorites { folders, items })
}

/// Pins a record or a view (once) for the owner; returns the favorite's id.
pub async fn add_favorite(pool: &SqlitePool, target: FavoriteTarget) -> Result<String> {
    let (position,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM favorite").fetch_one(pool).await?;
    let member_id = member_id(pool).await?;
    let id = uuid::Uuid::now_v7().to_string();
    if let Some(view_id) = target.view_id.filter(|v| !v.is_empty()) {
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM favorite WHERE view_id = ? AND deleted_at IS NULL").bind(&view_id).fetch_optional(pool).await?;
        if let Some((id,)) = existing {
            return Ok(id);
        }
        sqlx::query("INSERT INTO favorite (id, position, workspace_member_id, view_id) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(position)
            .bind(&member_id)
            .bind(&view_id)
            .execute(pool)
            .await?;
        return Ok(id);
    }
    let (Some(object_name), Some(record_id)) = (target.object, target.record_id) else {
        return Err(invalid("A favorite is a record or a view"));
    };
    let def = object(pool, &object_name).await?;
    if def.meta.is_system {
        return Err(invalid("A system object's rows aren't pinned"));
    }
    let column = format!("{}_id", snake(&def.meta.name_singular));
    if !valid_ident(&column) {
        return Err(invalid("The object's link column isn't a column name"));
    }
    let existing: Option<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id FROM favorite WHERE {column} = ? AND deleted_at IS NULL")))
        .bind(&record_id)
        .fetch_optional(pool)
        .await?;
    if let Some((id,)) = existing {
        return Ok(id);
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO favorite (id, position, workspace_member_id, {column}) VALUES (?, ?, ?, ?)")))
        .bind(&id)
        .bind(position)
        .bind(&member_id)
        .bind(&record_id)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn remove_favorite(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM favorite WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// Into a folder (or the root) at a position — a drag in the sidebar.
pub async fn move_favorite(pool: &SqlitePool, id: &str, folder_id: Option<&str>, position: f64) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE favorite SET folder_id = ?, position = ?, updated_at = {NOW} WHERE id = ?")))
        .bind(folder_id.filter(|f| !f.is_empty()))
        .bind(position)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_folder(pool: &SqlitePool, name: &str) -> Result<FavoriteFolder> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid("A folder needs a name"));
    }
    let (position,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM favorite_folder").fetch_one(pool).await?;
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO favorite_folder (id, name, position) VALUES (?, ?, ?)").bind(&id).bind(name).bind(position).execute(pool).await?;
    Ok(FavoriteFolder { id, name: name.to_owned(), position })
}

pub async fn rename_folder(pool: &SqlitePool, id: &str, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid("A folder needs a name"));
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE favorite_folder SET name = ?, updated_at = {NOW} WHERE id = ?"))).bind(name).bind(id).execute(pool).await?;
    Ok(())
}

/// The folder goes; its favorites come back to the root (the foreign key sets them null).
pub async fn delete_folder(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM favorite_folder WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

// ---- the Workspace section ----------------------------------------------------------------

/// Every record object's place in the sidebar; an object without a row yet (a new custom one)
/// gets one at the end — the only time this read writes.
pub async fn navigation(pool: &SqlitePool) -> Result<Vec<NavItem>> {
    let missing: Vec<(String,)> = sqlx::query_as("SELECT id FROM object_meta WHERE is_system = 0 AND id NOT IN (SELECT object_id FROM navigation_item) ORDER BY position, label_plural")
        .fetch_all(pool)
        .await?;
    for (object_id,) in missing {
        sqlx::query("INSERT OR IGNORE INTO navigation_item (id, object_id, position) VALUES (?, ?, (SELECT coalesce(max(position), 0.0) + 1.0 FROM navigation_item))")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(&object_id)
            .execute(pool)
            .await?;
    }
    let rows: Vec<(String, String, String, String, f64, bool, String)> = sqlx::query_as(
        "SELECT n.object_id, o.name_plural, o.label_plural, o.icon, n.position, n.is_hidden, n.folder FROM navigation_item n \
         JOIN object_meta o ON o.id = n.object_id WHERE o.is_system = 0 AND o.is_active = 1 ORDER BY n.position, o.position",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(object_id, object, label, icon, position, is_hidden, folder)| NavItem { object_id, object, label, icon, position, is_hidden, folder })
        .collect())
}

/// The customise mode's Save: positions, hidden, folders.
pub async fn set_navigation(pool: &SqlitePool, items: &[NavItemInput]) -> Result<()> {
    let mut tx = pool.begin().await?;
    for item in items {
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE navigation_item SET position = ?, is_hidden = ?, folder = ?, updated_at = {NOW} WHERE object_id = ?")))
            .bind(item.position)
            .bind(item.is_hidden)
            .bind(item.folder.trim())
            .bind(&item.object_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Every view of every record object, for ⌘K's "go to".
pub async fn all_views(pool: &SqlitePool) -> Result<Vec<ViewEntry>> {
    Ok(sqlx::query_as(
        "SELECT v.id, v.name, v.type, o.name_plural AS object, o.label_plural AS object_label FROM view v \
         JOIN object_meta o ON o.id = v.object_id WHERE o.is_system = 0 AND o.is_active = 1 ORDER BY o.position, v.position",
    )
    .fetch_all(pool)
    .await?)
}

// ---- recents ------------------------------------------------------------------------------

/// A record page was opened: the visit, newest first; the table keeps the last hundred.
pub async fn note_visit(pool: &SqlitePool, object_name: &str, record_id: &str) -> Result<()> {
    let def = object(pool, object_name).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO recent_visit (id, object, record_id) VALUES (?, ?, ?) \
         ON CONFLICT (object, record_id) DO UPDATE SET visited_at = {NOW}, updated_at = {NOW}"
    )))
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(&def.meta.name_singular)
    .bind(record_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM recent_visit WHERE id IN (SELECT id FROM recent_visit ORDER BY visited_at DESC LIMIT -1 OFFSET ?)")
        .bind(RECENTS_KEPT)
        .execute(pool)
        .await?;
    Ok(())
}

/// A record deleted for good, or a whole object dropped (`record_id` None): its visits go too,
/// and the assistant's threads on it are kept, detached from it (D-070).
pub async fn forget_visits(pool: &SqlitePool, object_name: &str, record_id: Option<&str>) -> Result<()> {
    match record_id {
        Some(id) => {
            sqlx::query("DELETE FROM recent_visit WHERE object = ? AND record_id = ?").bind(object_name).bind(id).execute(pool).await?;
            sqlx::query("UPDATE crm_chat_thread SET record_object = '', record_id = '' WHERE record_object = ? AND record_id = ?").bind(object_name).bind(id).execute(pool).await?;
        }
        None => {
            sqlx::query("DELETE FROM recent_visit WHERE object = ?").bind(object_name).execute(pool).await?;
            sqlx::query("UPDATE crm_chat_thread SET record_object = '', record_id = '' WHERE record_object = ?").bind(object_name).execute(pool).await?;
        }
    };
    Ok(())
}

/// The records opened lately that still exist, newest first.
pub async fn recents(pool: &SqlitePool, limit: u32) -> Result<Vec<Recent>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT object, record_id, visited_at FROM recent_visit ORDER BY visited_at DESC LIMIT ?")
        .bind(limit.clamp(1, 50) as i64 * 2)
        .fetch_all(pool)
        .await?;
    let metas = record_objects(pool).await?;
    let mut defs = Defs::default();
    let mut out = Vec::new();
    for (object_name, record_id, visited_at) in rows {
        let Some(meta) = metas.iter().find(|m| m.name_singular == object_name && m.is_active) else { continue };
        let Some(label) = label_of(pool, defs.get(pool, meta).await?, &record_id).await? else { continue };
        out.push(Recent { object: meta.name_plural.clone(), object_label: meta.label_plural.clone(), record_id, label, visited_at });
        if out.len() as u32 >= limit {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::query::{self, Actor};
    use serde_json::json;

    /// A record and a view pinned (once each), moved into a folder, the folder gone — the pins
    /// back at the root; the Workspace section lists every record object; a visit is a recent.
    #[test]
    fn favorites_folders_navigation_and_recents() {
        let root = std::env::temp_dir().join(format!("newdawn-nav-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::system("test");
            let company = object(&pool, "company").await.unwrap();
            let keppel = query::create(&pool, &company, &json!({ "name": "Keppel Land" }), &actor).await.unwrap();
            let id = keppel["id"].as_str().unwrap().to_owned();
            let pin = add_favorite(&pool, FavoriteTarget { object: Some("companies".into()), record_id: Some(id.clone()), view_id: None }).await.unwrap();
            assert_eq!(add_favorite(&pool, FavoriteTarget { object: Some("company".into()), record_id: Some(id.clone()), view_id: None }).await.unwrap(), pin, "pinned once");
            let index = super::super::views::ensure_index(&pool, &company).await.unwrap();
            let view_pin = add_favorite(&pool, FavoriteTarget { object: None, record_id: None, view_id: Some(index.clone()) }).await.unwrap();
            let all = favorites(&pool).await.unwrap();
            assert_eq!(all.items.len(), 2);
            assert_eq!(all.items[0].label, "Keppel Land");
            assert_eq!(all.items[1].label, "All Companies");
            assert_eq!(all.items[1].object, "companies");

            let folder = create_folder(&pool, " Deals ").await.unwrap();
            assert_eq!(folder.name, "Deals");
            move_favorite(&pool, &view_pin, Some(&folder.id), 0.5).await.unwrap();
            let all = favorites(&pool).await.unwrap();
            assert_eq!(all.folders.len(), 1);
            assert_eq!(all.items[0].folder_id.as_deref(), Some(folder.id.as_str()));
            assert!(create_folder(&pool, "  ").await.is_err());
            delete_folder(&pool, &folder.id).await.unwrap();
            let all = favorites(&pool).await.unwrap();
            assert!(all.folders.is_empty());
            assert!(all.items.iter().all(|f| f.folder_id.is_none()), "back at the root");
            remove_favorite(&pool, &pin).await.unwrap();
            query::delete(&pool, &company, &id, &actor).await.unwrap();
            assert_eq!(favorites(&pool).await.unwrap().items.len(), 1, "a deleted record's pin is left out");

            let items = navigation(&pool).await.unwrap();
            let names: Vec<&str> = items.iter().map(|i| i.object.as_str()).collect();
            assert_eq!(names, ["people", "companies", "opportunities", "notes", "tasks", "dashboards", "workflows", "workflowRuns"]);
            let mut edited: Vec<NavItemInput> = items.iter().map(|i| NavItemInput { object_id: i.object_id.clone(), position: i.position, is_hidden: i.object == "notes", folder: if i.object == "tasks" { "Work".into() } else { String::new() } }).collect();
            edited[0].position = 10.0;
            set_navigation(&pool, &edited).await.unwrap();
            let items = navigation(&pool).await.unwrap();
            assert_eq!(items.last().unwrap().object, "people");
            assert!(items.iter().find(|i| i.object == "notes").unwrap().is_hidden);
            assert_eq!(items.iter().find(|i| i.object == "tasks").unwrap().folder, "Work");
            assert!(all_views(&pool).await.unwrap().iter().any(|v| v.name == "All Companies" && v.object == "companies"));

            query::restore(&pool, &company, &id, &actor).await.unwrap();
            note_visit(&pool, "companies", &id).await.unwrap();
            note_visit(&pool, "company", &id).await.unwrap();
            let recent = recents(&pool, 8).await.unwrap();
            assert_eq!(recent.len(), 1, "one visit per record");
            assert_eq!(recent[0].label, "Keppel Land");
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
