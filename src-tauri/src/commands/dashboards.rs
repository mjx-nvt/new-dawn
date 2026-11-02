//! Dashboards (D-067, 0.10.10): a dashboard's own layout, its copy, and the chart query every
//! chart widget (on a dashboard or a record page) runs.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::charts::{self, ChartConfig, ChartData};
use crate::kernel::records::layouts::{self, Layout, LayoutInput};
use crate::kernel::records::types::Json;
use crate::kernel::records;

#[tauri::command]
#[specta::specta]
pub async fn dashboard_layout(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Layout, AppError> {
    Ok(layouts::dashboard_layout(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn save_dashboard_layout(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: LayoutInput) -> Result<Layout, AppError> {
    Ok(layouts::save_dashboard(&hub.pool(&slug)?, &id, input).await?)
}

/// Twenty's "Duplicate dashboard": the record and its layout.
#[tauri::command]
#[specta::specta]
pub async fn duplicate_dashboard(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Json, AppError> {
    let actor = records::actor_of(&hub, &slug).await?;
    Ok(Json(layouts::duplicate_dashboard(&hub.pool(&slug)?, &id, &actor).await?))
}

/// The categories and series of a chart.
#[tauri::command]
#[specta::specta]
pub async fn chart_data(hub: State<'_, WorkspaceHub>, slug: String, config: ChartConfig) -> Result<ChartData, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &config.source).await?;
    Ok(charts::chart_data(&pool, &def, &config).await?)
}
