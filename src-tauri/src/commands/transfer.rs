//! Import and export (D-065, 0.10.8): the spreadsheet import's steps and a view's CSV export.

use std::path::Path;

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::transfer::{self, ColumnMap, ExportReport, ImportFile, ImportReport, Preview, Table, TransferJob};
use crate::kernel::records::{self, Actor, Query};

/// Reads a CSV / XLSX / XLS file and proposes how its columns map to the object's fields.
#[tauri::command]
#[specta::specta]
pub async fn read_import_file(hub: State<'_, WorkspaceHub>, slug: String, object: String, path: String) -> Result<ImportFile, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &object).await?;
    Ok(transfer::open(&pool, &def, Path::new(&path)).await?)
}

/// Every row checked with the mapping: what would be created, updated, or is in error.
#[tauri::command]
#[specta::specta]
pub async fn validate_import(hub: State<'_, WorkspaceHub>, slug: String, object: String, table: Table, mapping: Vec<ColumnMap>, identifier: Option<String>) -> Result<Preview, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &object).await?;
    Ok(transfer::validate(&pool, &def, &table, &mapping, identifier.as_deref()).await?)
}

/// Writes the rows without errors; the report says what happened to each.
#[tauri::command]
#[specta::specta]
pub async fn run_import(hub: State<'_, WorkspaceHub>, slug: String, object: String, file_name: String, table: Table, mapping: Vec<ColumnMap>, identifier: Option<String>) -> Result<ImportReport, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &object).await?;
    let actor = Actor::import(records::member_id(&pool).await?, &file_name);
    Ok(transfer::run(&pool, &def, &file_name, &table, &mapping, identifier.as_deref(), &actor).await?)
}

/// A view's records — its filters, sorts and visible fields — as a CSV file where asked.
#[tauri::command]
#[specta::specta]
pub async fn export_view(hub: State<'_, WorkspaceHub>, slug: String, object: String, view_name: String, query: Query, fields: Vec<String>, path: String) -> Result<ExportReport, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &object).await?;
    Ok(transfer::export_view(&pool, &def, &view_name, query, &fields, Path::new(&path)).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_transfer_jobs(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<TransferJob>, AppError> {
    Ok(transfer::jobs(&hub.pool(&slug)?).await?)
}
