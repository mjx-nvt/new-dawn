//! Plane's projects and their settings (D-073): each command reads its workspace's pool from the
//! hub and calls `modules::pm`.

use std::path::Path;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::windows;
use crate::modules::pm::bulk::{self, PmBulkAction};
use crate::modules::pm::comments::{self, PmComment, Reaction};
use crate::modules::pm::display::{self, PmDisplay};
use crate::modules::pm::drafts::{self, PmDraft};
use crate::modules::pm::inbox::{self, InboxCounts, InboxQuery, Notice};
use crate::modules::pm::items::{self, ItemFile, ItemInput, ItemPatch, PmActivity, PmItem, PmVersion};
use crate::modules::pm::relations::{self, PmItemRef, PmAttachment, PmLink, PmRelation, PmRelationKind};
use crate::modules::pm::settings::{self, EstimateInput, LabelInput, PmEstimate, PmLabel, PmState, StateInput, StatePatch};
use crate::modules::pm::{self, NewPmProject, PmPrefs, PmProject, PmProjectPatch};

#[tauri::command]
#[specta::specta]
pub async fn list_pm_projects(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PmProject>, AppError> {
    Ok(pm::list(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_pm_project(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmProject, AppError> {
    Ok(pm::get(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_project(hub: State<'_, WorkspaceHub>, slug: String, project: NewPmProject) -> Result<PmProject, AppError> {
    Ok(pm::create(&hub.pool(&slug)?, project).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_project(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: PmProjectPatch) -> Result<PmProject, AppError> {
    Ok(pm::update(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_pm_project(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<PmProject, AppError> {
    Ok(pm::archive(&hub.pool(&slug)?, &id, archived).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_states(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmState>, AppError> {
    Ok(settings::states(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_state(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, input: StateInput) -> Result<Vec<PmState>, AppError> {
    Ok(settings::create_state(&hub.pool(&slug)?, &project_id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: StatePatch) -> Result<Vec<PmState>, AppError> {
    Ok(settings::update_state(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_default_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmState>, AppError> {
    Ok(settings::set_default_state(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmState>, AppError> {
    Ok(settings::delete_state(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn move_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String, up: bool) -> Result<Vec<PmState>, AppError> {
    Ok(settings::move_state(&hub.pool(&slug)?, &id, up).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_labels(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::labels(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_label(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, input: LabelInput) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::create_label(&hub.pool(&slug)?, &project_id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_label(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: LabelInput) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::update_label(&hub.pool(&slug)?, &id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_label(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::delete_label(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_estimates(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::estimates(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_estimate(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, input: EstimateInput) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::create_estimate(&hub.pool(&slug)?, &project_id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_estimate(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: EstimateInput) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::update_estimate(&hub.pool(&slug)?, &id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn disable_pm_estimates(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::disable_estimates(&hub.pool(&slug)?, &project_id).await?)
}

/// The owner, who does what the app is asked to.
const ME: &str = "me";

#[tauri::command]
#[specta::specta]
pub async fn list_pm_items(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, archived: bool) -> Result<Vec<PmItem>, AppError> {
    Ok(items::list(&hub.pool(&slug)?, &project_id, archived).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmItem, AppError> {
    Ok(items::get(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_sub_items(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmItem>, AppError> {
    Ok(items::children(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_item(hub: State<'_, WorkspaceHub>, slug: String, input: ItemInput) -> Result<PmItem, AppError> {
    Ok(items::create(&hub.pool(&slug)?, input, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: ItemPatch) -> Result<PmItem, AppError> {
    Ok(items::update(&hub.pool(&slug)?, &id, patch, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<PmItem, AppError> {
    Ok(items::archive(&hub.pool(&slug)?, &id, archived, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(items::delete(&hub.pool(&slug)?, &folder, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String, project_id: Option<String>) -> Result<PmItem, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(items::copy(&hub.pool(&slug)?, &folder, &id, project_id.as_deref(), ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn subscribe_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String, on: bool) -> Result<PmItem, AppError> {
    Ok(items::subscribe(&hub.pool(&slug)?, &id, ME, on).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_activity(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmActivity>, AppError> {
    Ok(items::activities(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_versions(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmVersion>, AppError> {
    Ok(items::versions(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn restore_pm_version(hub: State<'_, WorkspaceHub>, slug: String, version_id: String) -> Result<PmItem, AppError> {
    Ok(items::restore_version(&hub.pool(&slug)?, &version_id, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_comments(hub: State<'_, WorkspaceHub>, slug: String, item_id: String) -> Result<Vec<PmComment>, AppError> {
    Ok(comments::list(&hub.pool(&slug)?, &item_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_comment(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, body: String, parent_id: Option<String>) -> Result<Vec<PmComment>, AppError> {
    Ok(comments::add(&hub.pool(&slug)?, &item_id, &body, parent_id.as_deref(), ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn edit_pm_comment(hub: State<'_, WorkspaceHub>, slug: String, id: String, body: String) -> Result<Vec<PmComment>, AppError> {
    Ok(comments::edit(&hub.pool(&slug)?, &id, &body).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_comment(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmComment>, AppError> {
    Ok(comments::delete(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_pm_reaction(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, comment_id: Option<String>, emoji: String) -> Result<(), AppError> {
    Ok(comments::toggle_reaction(&hub.pool(&slug)?, &item_id, comment_id.as_deref(), &emoji, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_item_reactions(hub: State<'_, WorkspaceHub>, slug: String, item_id: String) -> Result<Vec<Reaction>, AppError> {
    Ok(comments::item_reactions(&hub.pool(&slug)?, &item_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_drafts(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PmDraft>, AppError> {
    Ok(drafts::list(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn save_pm_draft(hub: State<'_, WorkspaceHub>, slug: String, id: Option<String>, input: ItemInput) -> Result<PmDraft, AppError> {
    Ok(drafts::save(&hub.pool(&slug)?, id.as_deref(), input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_pm_draft(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmDraft, AppError> {
    Ok(drafts::copy(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_draft(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(drafts::delete(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn publish_pm_draft(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: ItemInput) -> Result<PmItem, AppError> {
    Ok(drafts::publish(&hub.pool(&slug)?, &id, input, ME).await?)
}

/// A picture picked for an item's description or comment.
#[tauri::command]
#[specta::specta]
pub async fn add_pm_item_file(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, path: String) -> Result<ItemFile, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(items::add_file(&hub.pool(&slug)?, &folder, &item_id, Path::new(&path)).await?)
}

/// A picture pasted, as bytes.
#[tauri::command]
#[specta::specta]
pub async fn add_pm_item_file_bytes(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, name: String, bytes: Vec<u8>) -> Result<ItemFile, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(items::add_file_bytes(&hub.pool(&slug)?, &folder, &item_id, &name, &bytes).await?)
}

/// Opens a work item in a window of its own.
#[tauri::command]
#[specta::specta]
pub async fn open_pm_item_window(app: AppHandle, project_id: String, item_id: String) -> Result<(), AppError> {
    Ok(windows::open_item(&app, &project_id, &item_id)?)
}

// ── 0.11.2 (D-075): relations, links, attachments, votes, bulk changes, the Inbox ──

#[tauri::command]
#[specta::specta]
pub async fn list_pm_relations(hub: State<'_, WorkspaceHub>, slug: String, item_id: String) -> Result<Vec<PmRelation>, AppError> {
    Ok(relations::relations(&hub.pool(&slug)?, &item_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_relation(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, kind: PmRelationKind, other_id: String) -> Result<PmRelation, AppError> {
    Ok(relations::relate(&hub.pool(&slug)?, &item_id, kind, &other_id, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_pm_relation(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(relations::unrelate(&hub.pool(&slug)?, &id, ME).await?)
}

/// Work items to relate to, from every project of the workspace.
#[tauri::command]
#[specta::specta]
pub async fn search_pm_items(hub: State<'_, WorkspaceHub>, slug: String, query: String, except: Option<String>) -> Result<Vec<PmItemRef>, AppError> {
    Ok(relations::search(&hub.pool(&slug)?, &query, except.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_links(hub: State<'_, WorkspaceHub>, slug: String, item_id: String) -> Result<Vec<PmLink>, AppError> {
    Ok(relations::links(&hub.pool(&slug)?, &item_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_link(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, url: String, title: String) -> Result<PmLink, AppError> {
    Ok(relations::add_link(&hub.pool(&slug)?, &item_id, &url, &title, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn edit_pm_link(hub: State<'_, WorkspaceHub>, slug: String, id: String, url: String, title: String) -> Result<PmLink, AppError> {
    Ok(relations::edit_link(&hub.pool(&slug)?, &id, &url, &title, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_pm_link(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(relations::remove_link(&hub.pool(&slug)?, &id, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_attachments(hub: State<'_, WorkspaceHub>, slug: String, item_id: String) -> Result<Vec<PmAttachment>, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(relations::attachments(&hub.pool(&slug)?, &folder, &item_id).await?)
}

/// Files picked or dropped, each up to the owner's limit.
#[tauri::command]
#[specta::specta]
pub async fn add_pm_attachments(hub: State<'_, WorkspaceHub>, slug: String, item_id: String, paths: Vec<String>) -> Result<Vec<PmAttachment>, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    let limit = pm::prefs(&hub).await?.attachment_mb;
    let sources: Vec<&Path> = paths.iter().map(Path::new).collect();
    Ok(relations::attach_all(&hub.pool(&slug)?, &folder, &item_id, &sources, limit, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_pm_attachment(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(relations::detach(&hub.pool(&slug)?, &folder, &id, ME).await?)
}

/// 1 up, -1 down, 0 none.
#[tauri::command]
#[specta::specta]
pub async fn vote_pm_item(hub: State<'_, WorkspaceHub>, slug: String, id: String, vote: i32) -> Result<PmItem, AppError> {
    Ok(items::vote(&hub.pool(&slug)?, &id, vote).await?)
}

/// The same edits on the selected items: every one checked first; how many changed.
#[tauri::command]
#[specta::specta]
pub async fn bulk_update_pm_items(hub: State<'_, WorkspaceHub>, slug: String, ids: Vec<String>, patch: ItemPatch) -> Result<u32, AppError> {
    Ok(bulk::update(&hub.pool(&slug)?, &ids, &patch, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn bulk_pm_items(hub: State<'_, WorkspaceHub>, slug: String, ids: Vec<String>, action: PmBulkAction) -> Result<u32, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(bulk::act(&hub.pool(&slug)?, &folder, &ids, action, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn copy_pm_items(hub: State<'_, WorkspaceHub>, slug: String, ids: Vec<String>, project_id: String) -> Result<u32, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(bulk::copy(&hub.pool(&slug)?, &folder, &ids, &project_id, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_notices(hub: State<'_, WorkspaceHub>, slug: String, query: InboxQuery) -> Result<Vec<Notice>, AppError> {
    let muted = pm::prefs(&hub).await?.muted;
    Ok(inbox::list(&hub.pool(&slug)?, &query, &muted).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn notice_counts(hub: State<'_, WorkspaceHub>, slug: String) -> Result<InboxCounts, AppError> {
    let muted = pm::prefs(&hub).await?.muted;
    Ok(inbox::counts(&hub.pool(&slug)?, &muted).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn mark_notice(hub: State<'_, WorkspaceHub>, slug: String, id: String, read: bool) -> Result<(), AppError> {
    Ok(inbox::mark(&hub.pool(&slug)?, &id, read).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn mark_all_notices(hub: State<'_, WorkspaceHub>, slug: String, mentions: bool) -> Result<(), AppError> {
    let muted = pm::prefs(&hub).await?.muted;
    Ok(inbox::mark_all(&hub.pool(&slug)?, mentions, &muted).await?)
}

/// Until an ISO-8601 time to come, or none to bring it back.
#[tauri::command]
#[specta::specta]
pub async fn snooze_notice(hub: State<'_, WorkspaceHub>, slug: String, id: String, until: Option<String>) -> Result<(), AppError> {
    Ok(inbox::snooze(&hub.pool(&slug)?, &id, until.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_notice(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<(), AppError> {
    Ok(inbox::archive(&hub.pool(&slug)?, &id, archived).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_pm_prefs(hub: State<'_, WorkspaceHub>) -> Result<PmPrefs, AppError> {
    Ok(pm::prefs(&hub).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_pm_prefs(hub: State<'_, WorkspaceHub>, prefs: PmPrefs) -> Result<PmPrefs, AppError> {
    Ok(pm::set_prefs(&hub, prefs).await?)
}

// ── 0.11.3 (D-076): a project's layout and display ──

#[tauri::command]
#[specta::specta]
pub async fn get_pm_display(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<PmDisplay, AppError> {
    Ok(display::get(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_pm_display(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, display: PmDisplay) -> Result<PmDisplay, AppError> {
    Ok(display::set(&hub.pool(&slug)?, &project_id, display).await?)
}
