use tauri::AppHandle;

use crate::error::AppError;
use crate::kernel::brand::tokens::{self, BrandPatch, BrandView};

#[tauri::command]
#[specta::specta]
pub async fn get_brand(app: AppHandle, slug: String) -> Result<BrandView, AppError> {
    Ok(tokens::get(&app, &slug).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_brand(
    app: AppHandle,
    slug: String,
    patch: BrandPatch,
) -> Result<BrandView, AppError> {
    Ok(tokens::update(&app, &slug, patch).await?)
}

/// A picked SVG/PNG becomes the logo (`on_dark`: the variant for dark backgrounds); `None` clears.
#[tauri::command]
#[specta::specta]
pub async fn set_brand_logo(
    app: AppHandle,
    slug: String,
    path: Option<String>,
    on_dark: bool,
) -> Result<BrandView, AppError> {
    Ok(tokens::set_logo(&app, &slug, path.as_deref(), on_dark).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn reset_brand(app: AppHandle, slug: String) -> Result<BrandView, AppError> {
    Ok(tokens::reset(&app, &slug).await?)
}

/// The English slogan in Vietnamese, by the Translation agent (the UI saves it).
#[tauri::command]
#[specta::specta]
pub async fn translate_slogan(
    app: AppHandle,
    slug: String,
    text: String,
) -> Result<String, AppError> {
    Ok(crate::modules::brand::translate_slogan(&app, &slug, &text).await?)
}
