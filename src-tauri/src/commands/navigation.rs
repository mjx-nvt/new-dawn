//! The sidebar's CRM section and ⌘K (D-064, 0.10.7): favorites, folders, the Workspace
//! section's order, every view, recents.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::navigation::{self, FavoriteFolder, FavoriteTarget, Favorites, NavItem, NavItemInput, Recent, ViewEntry};

#[tauri::command]
#[specta::specta]
pub async fn list_favorites(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Favorites, AppError> {
    Ok(navigation::favorites(&hub.pool(&slug)?).await?)
}

/// Pins a record or a view; returns the favorite's id.
#[tauri::command]
#[specta::specta]
pub async fn add_favorite(hub: State<'_, WorkspaceHub>, slug: String, target: FavoriteTarget) -> Result<String, AppError> {
    Ok(navigation::add_favorite(&hub.pool(&slug)?, target).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_favorite(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(navigation::remove_favorite(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn move_favorite(hub: State<'_, WorkspaceHub>, slug: String, id: String, folder_id: Option<String>, position: f64) -> Result<(), AppError> {
    Ok(navigation::move_favorite(&hub.pool(&slug)?, &id, folder_id.as_deref(), position).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_favorite_folder(hub: State<'_, WorkspaceHub>, slug: String, name: String) -> Result<FavoriteFolder, AppError> {
    Ok(navigation::create_folder(&hub.pool(&slug)?, &name).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn rename_favorite_folder(hub: State<'_, WorkspaceHub>, slug: String, id: String, name: String) -> Result<(), AppError> {
    Ok(navigation::rename_folder(&hub.pool(&slug)?, &id, &name).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_favorite_folder(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(navigation::delete_folder(&hub.pool(&slug)?, &id).await?)
}

/// The Workspace section: every record object's place, hidden ones included.
#[tauri::command]
#[specta::specta]
pub async fn list_navigation(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<NavItem>, AppError> {
    Ok(navigation::navigation(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_navigation(hub: State<'_, WorkspaceHub>, slug: String, items: Vec<NavItemInput>) -> Result<(), AppError> {
    Ok(navigation::set_navigation(&hub.pool(&slug)?, &items).await?)
}

/// Every view of every record object, for ⌘K.
#[tauri::command]
#[specta::specta]
pub async fn list_all_views(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<ViewEntry>, AppError> {
    Ok(navigation::all_views(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn note_visit(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<(), AppError> {
    Ok(navigation::note_visit(&hub.pool(&slug)?, &object, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_recents(hub: State<'_, WorkspaceHub>, slug: String, limit: u32) -> Result<Vec<Recent>, AppError> {
    Ok(navigation::recents(&hub.pool(&slug)?, limit).await?)
}
