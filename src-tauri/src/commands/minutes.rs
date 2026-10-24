use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::export::pdf::{self, ConverterInfo};
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::minutes::{self, ExportOptions, MinutesExport, MinutesFile};

#[tauri::command]
#[specta::specta]
pub async fn export_minutes_docx(
    app: AppHandle,
    slug: String,
    meeting_id: String,
    options: ExportOptions,
) -> Result<MinutesExport, AppError> {
    Ok(minutes::export_docx(&app, &slug, &meeting_id, options).await?)
}

/// PDF of an existing export's DOCX (picks up edits made in Word since; replaces an older PDF).
#[tauri::command]
#[specta::specta]
pub async fn export_minutes_pdf(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    export_id: String,
) -> Result<MinutesExport, AppError> {
    Ok(minutes::export_pdf(&hub, &slug, &export_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_minutes_exports(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<Vec<MinutesExport>, AppError> {
    Ok(minutes::list(&hub.pool(&slug)?, &meeting_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_converters() -> Result<Vec<ConverterInfo>, AppError> {
    Ok(pdf::list())
}

/// Opens an export's DOCX or PDF in its default app, or shows it in Finder (`reveal`).
#[tauri::command]
#[specta::specta]
pub async fn open_minutes_file(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    export_id: String,
    file: MinutesFile,
    reveal: bool,
) -> Result<(), AppError> {
    Ok(minutes::open_file(&hub, &slug, &export_id, file, reveal).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_minutes_export(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    export_id: String,
) -> Result<(), AppError> {
    Ok(minutes::delete_export(&hub, &slug, &export_id).await?)
}
