//! The local API (D-071, 0.10.14): on/off, keys, webhooks and their deliveries.

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::api::keys::{self, ApiKey, NewApiKey};
use crate::modules::api::webhooks::{self, Delivery, Webhook, WebhookInput};
use crate::modules::api::{self, ApiStatus};

#[tauri::command]
#[specta::specta]
pub async fn api_status(app: AppHandle) -> Result<ApiStatus, AppError> {
    Ok(api::status(&app).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_api_enabled(app: AppHandle, on: bool) -> Result<ApiStatus, AppError> {
    Ok(api::set_enabled(&app, on).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_api_keys(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<ApiKey>, AppError> {
    Ok(keys::list(&hub.pool(&slug)?).await?)
}

/// A new key: shown this once.
#[tauri::command]
#[specta::specta]
pub async fn create_api_key(hub: State<'_, WorkspaceHub>, slug: String, name: String, expires_days: Option<u32>) -> Result<NewApiKey, AppError> {
    Ok(keys::create(&hub.pool(&slug)?, &name, expires_days).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn revoke_api_key(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(keys::revoke(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_webhooks(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<Webhook>, AppError> {
    Ok(webhooks::list(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn save_webhook(hub: State<'_, WorkspaceHub>, slug: String, input: WebhookInput) -> Result<Webhook, AppError> {
    Ok(webhooks::save(&hub.pool(&slug)?, &input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_webhook(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(webhooks::delete(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn webhook_deliveries(hub: State<'_, WorkspaceHub>, slug: String, webhook_id: String) -> Result<Vec<Delivery>, AppError> {
    Ok(webhooks::deliveries(&hub.pool(&slug)?, &webhook_id).await?)
}
