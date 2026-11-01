//! Files (0.10.6): a record's attachments — copied into the workspace's `files/` folder and kept
//! as `attachment` records through the engine, so every object (custom ones too) has them and
//! the owner's files never leave the workspace folder.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use super::query::{self, Actor};
use super::{object, ObjectDef};
use crate::error::AppError;

/// Larger files stay where they are (a link would be the owner's choice; not offered yet).
const MAX_BYTES: u64 = 200 * 1024 * 1024;

/// A file name as the folder keeps it: the base name, odd characters as `_`, at most 120 long.
fn safe_name(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ') { c } else { '_' }).collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_owned();
    let cleaned = if cleaned.is_empty() { "file".to_owned() } else { cleaned };
    cleaned.chars().take(120).collect()
}

fn files_dir(folder: &str) -> PathBuf {
    Path::new(folder).join("files")
}

/// Copies a file into the workspace and makes its attachment record on the object's record.
pub async fn add(pool: &SqlitePool, folder: &str, object_def: &ObjectDef, record_id: &str, source: &Path, actor: &Actor) -> Result<Value> {
    let meta = tokio::fs::metadata(source).await.with_context(|| format!("reading {}", source.display()))?;
    if !meta.is_file() {
        return Err(anyhow!(AppError::invalid("That isn't a file")));
    }
    if meta.len() > MAX_BYTES {
        return Err(anyhow!(AppError::invalid("Files up to 200 MB")));
    }
    let name = source.file_name().and_then(|n| n.to_str()).map(safe_name).unwrap_or_else(|| "file".into());
    let target = fresh_path(folder, &name)?;
    tokio::fs::copy(source, &target).await.with_context(|| format!("copying {} into the workspace", source.display()))?;
    insert(pool, object_def, record_id, &name, &target, actor).await
}

/// A picture pasted or dropped into the editor: only in memory until now.
pub async fn add_bytes(pool: &SqlitePool, folder: &str, object_def: &ObjectDef, record_id: &str, name: &str, bytes: &[u8], actor: &Actor) -> Result<Value> {
    if bytes.is_empty() {
        return Err(anyhow!(AppError::invalid("An empty file")));
    }
    if bytes.len() as u64 > MAX_BYTES {
        return Err(anyhow!(AppError::invalid("Files up to 200 MB")));
    }
    let name = safe_name(name);
    let target = fresh_path(folder, &name)?;
    tokio::fs::write(&target, bytes).await.with_context(|| format!("writing {}", target.display()))?;
    insert(pool, object_def, record_id, &name, &target, actor).await
}

fn fresh_path(folder: &str, name: &str) -> Result<PathBuf> {
    let dir = files_dir(folder);
    std::fs::create_dir_all(&dir).with_context(|| format!("making {}", dir.display()))?;
    Ok(dir.join(format!("{}_{name}", uuid::Uuid::now_v7())))
}

async fn insert(pool: &SqlitePool, object_def: &ObjectDef, record_id: &str, name: &str, path: &Path, actor: &Actor) -> Result<Value> {
    let attachment = object(pool, "attachment").await?;
    if attachment.field(&object_def.meta.name_singular).is_none() {
        return Err(anyhow!(AppError::invalid(format!("Files can't be kept on {} yet", object_def.meta.label_plural.to_lowercase()))));
    }
    let kind = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mut data = json!({ "name": name, "fullPath": path.to_string_lossy(), "type": kind });
    data[object_def.meta.name_singular.as_str()] = json!(record_id);
    if let Some(member) = actor.member_id.as_deref() {
        data["author"] = json!(member);
    }
    query::create(pool, &attachment, &data, actor).await
}

/// The file of an attachment, for Open, Reveal and Save as — only inside the workspace's files
/// folder (an attachment row is an ordinary record; its path is never trusted beyond that).
pub async fn path_of(pool: &SqlitePool, folder: &str, id: &str) -> Result<PathBuf> {
    let attachment = object(pool, "attachment").await?;
    let record = query::get(pool, &attachment, id, false).await?;
    let path = record.get("fullPath").and_then(Value::as_str).filter(|p| !p.is_empty()).ok_or_else(|| anyhow!(AppError::not_found("No such file")))?;
    let path = PathBuf::from(path);
    if !path.starts_with(files_dir(folder)) || path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(anyhow!(AppError::invalid("That file isn't in the workspace's files folder")));
    }
    Ok(path)
}

/// Copies an attachment's file to where the owner asked (Save as…).
pub async fn save_as(pool: &SqlitePool, folder: &str, id: &str, to: &Path) -> Result<()> {
    let from = path_of(pool, folder, id).await?;
    tokio::fs::copy(&from, to).await.with_context(|| format!("saving {} as {}", from.display(), to.display()))?;
    Ok(())
}

/// Soft delete through the engine (Deleted records, restore); the file stays in the folder.
pub async fn remove(pool: &SqlitePool, id: &str, actor: &Actor) -> Result<()> {
    query::delete(pool, &object(pool, "attachment").await?, id, actor).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    #[test]
    fn names_are_kept_safe() {
        assert_eq!(safe_name("Q3 deck (final).pdf"), "Q3 deck _final_.pdf");
        assert_eq!(safe_name("../../etc/passwd"), "_.._etc_passwd", "no path separators, no leading dot");
        assert_eq!(safe_name(""), "file");
    }

    /// A file dropped on a company is copied into the workspace's files folder and listed as
    /// an attachment of that company; a pasted picture too; a soft delete hides it.
    #[test]
    fn a_file_lands_in_the_workspace_and_on_the_record() {
        let root = std::env::temp_dir().join(format!("newdawn-files-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            let workspace = hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::system("test");
            let company = object(&pool, "company").await.unwrap();
            let keppel = query::create(&pool, &company, &serde_json::json!({ "name": "Keppel Land" }), &actor).await.unwrap();
            let id = keppel["id"].as_str().unwrap();
            let source = root.join("Term sheet.txt");
            std::fs::write(&source, b"draft").unwrap();
            let made = add(&pool, &workspace.folder_path, &company, id, &source, &actor).await.unwrap();
            let stored = PathBuf::from(made["fullPath"].as_str().unwrap());
            assert!(stored.starts_with(Path::new(&workspace.folder_path).join("files")));
            assert_eq!(std::fs::read(&stored).unwrap(), b"draft");
            assert_eq!(made["type"], "txt");
            assert_eq!(made["company"]["label"], "Keppel Land");
            let picture = add_bytes(&pool, &workspace.folder_path, &company, id, "shot.png", &[0x89, b'P', b'N', b'G'], &actor).await.unwrap();
            assert_eq!(picture["type"], "png");
            let listed = super::super::linked::attachments(&pool, &company, id).await.unwrap();
            assert_eq!(listed.len(), 2);
            assert_eq!(path_of(&pool, &workspace.folder_path, listed[0].id.as_str()).await.unwrap(), PathBuf::from(&listed[0].full_path));
            let copy = root.join("copy.txt");
            save_as(&pool, &workspace.folder_path, made["id"].as_str().unwrap(), &copy).await.unwrap();
            assert_eq!(std::fs::read(&copy).unwrap(), b"draft");
            // A row made by hand that points outside the folder serves nothing.
            let attachment = object(&pool, "attachment").await.unwrap();
            let rogue = query::create(&pool, &attachment, &serde_json::json!({ "name": "passwd", "fullPath": "/etc/passwd", "company": id }), &actor).await.unwrap();
            assert!(path_of(&pool, &workspace.folder_path, rogue["id"].as_str().unwrap()).await.is_err());
            remove(&pool, made["id"].as_str().unwrap(), &actor).await.unwrap();
            assert_eq!(super::super::linked::attachments(&pool, &company, id).await.unwrap().len(), 2, "the picture and the rogue row stay listed");
            assert!(stored.exists(), "the file stays until a delete for good");
            assert!(add(&pool, &workspace.folder_path, &company, id, &root, &actor).await.is_err(), "a folder isn't a file");
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
