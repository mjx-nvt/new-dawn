use tauri::AppHandle;

use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::export::model::{self, TemplateField};
use crate::modules::meetings::template::{self, MeetingTypeView, TemplateCheck, TemplatePreview};

/// The workspace's meeting types with their templates (a first one is made if there are none).
#[tauri::command]
#[specta::specta]
pub async fn list_meeting_type_templates(
    app: AppHandle,
    slug: String,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::list(&app, &slug).await?)
}

/// A new type; `template` = a picked .docx, else a copy of the default type's template.
#[tauri::command]
#[specta::specta]
pub async fn create_meeting_type(
    app: AppHandle,
    slug: String,
    name: String,
    language: Language,
    template: Option<String>,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::create(&app, &slug, &name, language, template.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_meeting_type(
    app: AppHandle,
    slug: String,
    id: String,
    name: String,
    language: Language,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::update(&app, &slug, &id, &name, language).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_default_meeting_type(
    app: AppHandle,
    slug: String,
    id: String,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::set_default(&app, &slug, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_meeting_type(
    app: AppHandle,
    slug: String,
    id: String,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::delete(&app, &slug, &id).await?)
}

/// Checks a .docx the user picked, without using it.
#[tauri::command]
#[specta::specta]
pub async fn check_minutes_template(path: String) -> Result<TemplateCheck, AppError> {
    Ok(template::check_path(&path).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn replace_type_template(
    app: AppHandle,
    slug: String,
    id: String,
    path: String,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::replace(&app, &slug, &id, &path).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn reset_type_template(
    app: AppHandle,
    slug: String,
    id: String,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::reset(&app, &slug, &id).await?)
}

/// Opens the type's template in Word, or shows it in Finder.
#[tauri::command]
#[specta::specta]
pub async fn open_type_template(
    app: AppHandle,
    slug: String,
    id: String,
    reveal: bool,
) -> Result<(), AppError> {
    Ok(template::open(&app, &slug, &id, reveal).await?)
}

/// Every field a minutes template can use.
#[tauri::command]
#[specta::specta]
pub fn minutes_template_fields() -> Result<Vec<TemplateField>, AppError> {
    Ok(model::fields())
}

/// Makes the minutes template from the brand again: the default type's (the old file goes to
/// the Trash), or the first type's.
#[tauri::command]
#[specta::specta]
pub async fn generate_minutes_template(
    app: AppHandle,
    slug: String,
) -> Result<Vec<MeetingTypeView>, AppError> {
    Ok(template::generate(&app, &slug).await?)
}

/// A picture of the default type's template filled with a made-up meeting.
#[tauri::command]
#[specta::specta]
pub async fn preview_minutes_template(
    app: AppHandle,
    slug: String,
) -> Result<TemplatePreview, AppError> {
    Ok(template::preview(&app, &slug).await?)
}
