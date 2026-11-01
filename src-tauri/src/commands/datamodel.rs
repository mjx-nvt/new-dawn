//! The data model settings (D-060, 0.10.5): custom objects, fields and relations.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records;
use crate::kernel::records::ddl::{self, FieldInput, FieldPatch, ObjectInput, ObjectPatch};
use crate::kernel::records::{FieldMeta, ObjectDef};

#[tauri::command]
#[specta::specta]
pub async fn create_object(hub: State<'_, WorkspaceHub>, slug: String, input: ObjectInput) -> Result<ObjectDef, AppError> {
    Ok(ddl::create_object(&hub.pool(&slug)?, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_object(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: ObjectPatch) -> Result<ObjectDef, AppError> {
    Ok(ddl::update_object(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_object_active(hub: State<'_, WorkspaceHub>, slug: String, id: String, active: bool) -> Result<ObjectDef, AppError> {
    Ok(ddl::set_object_active(&hub.pool(&slug)?, &id, active).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_object(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(ddl::delete_object(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_field(hub: State<'_, WorkspaceHub>, slug: String, object: String, input: FieldInput) -> Result<FieldMeta, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(ddl::create_field(&pool, &object, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_field(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, patch: FieldPatch) -> Result<FieldMeta, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(ddl::update_field(&pool, &object, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_field_active(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, active: bool) -> Result<FieldMeta, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(ddl::set_field_active(&pool, &object, &id, active).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_field(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<(), AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(ddl::delete_field(&pool, &object, &id).await?)
}
