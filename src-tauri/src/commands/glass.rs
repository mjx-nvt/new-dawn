use crate::error::AppError;
use crate::kernel::glass::{self, GlassStatus};

#[tauri::command]
#[specta::specta]
pub fn glass_status() -> Result<GlassStatus, AppError> {
    Ok(glass::status())
}
