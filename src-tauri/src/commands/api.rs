//! The local API (D-071, 0.10.14): on/off, keys, webhooks and their deliveries.

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::api::keys::{self, ApiKey, NewApiKey};
use crate::modules::api::webhooks::{self, Delivery, HookFormat, Webhook, WebhookInput};
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

/// A new key (a Twenty key, or a Plane token with its description): shown this once.
#[tauri::command]
#[specta::specta]
pub async fn create_api_key(hub: State<'_, WorkspaceHub>, slug: String, name: String, description: String, expires_days: Option<u32>) -> Result<NewApiKey, AppError> {
    Ok(keys::create(&hub.pool(&slug)?, &name, &description, expires_days).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn revoke_api_key(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(keys::revoke(&hub.pool(&slug)?, &id).await?)
}

/// Twenty's webhooks (Workspace → CRM) or Plane's (Workspace → Projects).
#[tauri::command]
#[specta::specta]
pub async fn list_webhooks(hub: State<'_, WorkspaceHub>, slug: String, format: HookFormat) -> Result<Vec<Webhook>, AppError> {
    Ok(webhooks::list(&hub.pool(&slug)?, format).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn save_webhook(hub: State<'_, WorkspaceHub>, slug: String, input: WebhookInput) -> Result<Webhook, AppError> {
    Ok(webhooks::save(&hub.pool(&slug)?, &input).await?)
}

/// A new secret for a webhook (Plane's "Re-generate key"): the old one stops at once.
#[tauri::command]
#[specta::specta]
pub async fn regenerate_webhook_secret(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Webhook, AppError> {
    Ok(webhooks::regenerate(&hub.pool(&slug)?, &id).await?)
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
