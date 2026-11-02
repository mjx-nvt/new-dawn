//! Plane's projects and their settings (D-073): each command reads its workspace's pool from the
//! hub and calls `modules::pm`.

use std::path::Path;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::windows;
use crate::modules::pm::bulk::{self, PmBulkAction};
use crate::modules::pm::comments::{self, PmComment, Reaction};
use crate::modules::pm::cycles::{self, CycleDetails, CycleInput, CyclePatch, PmCycle, Share};
use crate::modules::pm::analytics::{self, DayCount, PmOverview};
use crate::modules::pm::transfer::{self, ExportFormat, PmExportReport, PmImportPreview, PmImportSummary};
use crate::modules::pm::home::{self, HomeWidget, MyActivity, PmArchives, PmHit, PmQuickLink, PmRecent, PmSticky, RecentKind};
use crate::modules::pm::intake::{self, IntakeInput, PmIntakeItem};
use crate::modules::pm::modules::{self, ModuleDetails, ModuleInput, ModulePatch, PmModule};
use crate::modules::pm::pages::{self, PageInput, PagePatch, PmPage, PmPageDoc, PmPageVersion};
use crate::modules::pm::display::{self, PmDisplay};
use crate::modules::pm::drafts::{self, PmDraft};
use crate::modules::pm::filters::PmRule;
use crate::modules::pm::views::{self, PmView, PmViewInput, PmViewPatch};
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
pub async fn search_pm_items(hub: State<'_, WorkspaceHub>, slug: String, query: String, except: Option<String>, project_id: Option<String>) -> Result<Vec<PmItemRef>, AppError> {
    Ok(relations::search(&hub.pool(&slug)?, &query, except.as_deref(), project_id.as_deref()).await?)
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

// ── 0.11.4 (D-077): filters and views ──

/// Work items as the filters say — a project's, or the workspace's open projects' (none).
#[tauri::command]
#[specta::specta]
pub async fn query_pm_items(hub: State<'_, WorkspaceHub>, slug: String, project_id: Option<String>, rules: Vec<PmRule>) -> Result<Vec<PmItem>, AppError> {
    Ok(items::query_now(&hub, &slug, project_id.as_deref().filter(|p| !p.is_empty()), &rules).await?)
}

/// A project's views, or the workspace's (none).
#[tauri::command]
#[specta::specta]
pub async fn list_pm_views(hub: State<'_, WorkspaceHub>, slug: String, project_id: Option<String>) -> Result<Vec<PmView>, AppError> {
    Ok(views::list(&hub.pool(&slug)?, project_id.as_deref().filter(|p| !p.is_empty())).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_view(hub: State<'_, WorkspaceHub>, slug: String, input: PmViewInput) -> Result<PmView, AppError> {
    Ok(views::create(&hub.pool(&slug)?, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_view(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: PmViewPatch) -> Result<PmView, AppError> {
    Ok(views::update(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn duplicate_pm_view(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmView, AppError> {
    Ok(views::duplicate(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_view(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(views::delete(&hub.pool(&slug)?, &id).await?)
}

/// Into the sidebar's Favorites, or out.
#[tauri::command]
#[specta::specta]
pub async fn favorite_pm_view(hub: State<'_, WorkspaceHub>, slug: String, id: String, on: bool) -> Result<PmView, AppError> {
    Ok(views::favorite(&hub.pool(&slug)?, &id, on).await?)
}

// ── 0.11.5 (D-078): cycles ──

#[tauri::command]
#[specta::specta]
pub async fn list_pm_cycles(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmCycle>, AppError> {
    Ok(cycles::list(&hub.pool(&slug)?, &project_id, cycles::today()).await?)
}

/// Every open project's active cycles.
#[tauri::command]
#[specta::specta]
pub async fn list_active_pm_cycles(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PmCycle>, AppError> {
    Ok(cycles::active(&hub.pool(&slug)?, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, input: CycleInput) -> Result<PmCycle, AppError> {
    Ok(cycles::create(&hub.pool(&slug)?, input, ME, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: CyclePatch) -> Result<PmCycle, AppError> {
    Ok(cycles::update(&hub.pool(&slug)?, &id, patch, cycles::today()).await?)
}

/// Plane's start now (an upcoming cycle) and complete now (an active one).
#[tauri::command]
#[specta::specta]
pub async fn start_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmCycle, AppError> {
    Ok(cycles::start_now(&hub.pool(&slug)?, &id, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn complete_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmCycle, AppError> {
    Ok(cycles::complete_now(&hub.pool(&slug)?, &id, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_cycle_items(hub: State<'_, WorkspaceHub>, slug: String, id: String, item_ids: Vec<String>) -> Result<PmCycle, AppError> {
    Ok(cycles::add_items(&hub.pool(&slug)?, &id, &item_ids, ME, cycles::today()).await?)
}

/// A completed cycle's open work into another cycle, once; how many moved.
#[tauri::command]
#[specta::specta]
pub async fn transfer_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, id: String, to: String) -> Result<u32, AppError> {
    Ok(cycles::transfer(&hub.pool(&slug)?, &id, &to, ME, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<PmCycle, AppError> {
    Ok(cycles::archive(&hub.pool(&slug)?, &id, archived, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_cycle(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(cycles::delete(&hub.pool(&slug)?, &id, ME, cycles::today()).await?)
}

/// The details panel: burn-down and breakdowns.
#[tauri::command]
#[specta::specta]
pub async fn pm_cycle_details(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<CycleDetails, AppError> {
    Ok(cycles::details(&hub.pool(&slug)?, &id, cycles::today()).await?)
}

// ── 0.11.6 (D-079): modules ──

#[tauri::command]
#[specta::specta]
pub async fn list_pm_modules(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmModule>, AppError> {
    Ok(modules::list(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_module(hub: State<'_, WorkspaceHub>, slug: String, input: ModuleInput) -> Result<PmModule, AppError> {
    Ok(modules::create(&hub.pool(&slug)?, input, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_module(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: ModulePatch) -> Result<PmModule, AppError> {
    Ok(modules::update(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn favorite_pm_module(hub: State<'_, WorkspaceHub>, slug: String, id: String, on: bool) -> Result<PmModule, AppError> {
    Ok(modules::favorite(&hub.pool(&slug)?, &id, on).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_module_items(hub: State<'_, WorkspaceHub>, slug: String, id: String, item_ids: Vec<String>) -> Result<PmModule, AppError> {
    Ok(modules::add_items(&hub.pool(&slug)?, &id, &item_ids, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_pm_module(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<PmModule, AppError> {
    Ok(modules::archive(&hub.pool(&slug)?, &id, archived).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_module(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(modules::delete(&hub.pool(&slug)?, &id, ME).await?)
}

/// The details panel: progress, Needs attention, breakdowns, links.
#[tauri::command]
#[specta::specta]
pub async fn pm_module_details(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<ModuleDetails, AppError> {
    Ok(modules::details(&hub.pool(&slug)?, &id, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_module_link(hub: State<'_, WorkspaceHub>, slug: String, id: String, url: String, title: String) -> Result<PmLink, AppError> {
    Ok(modules::add_link(&hub.pool(&slug)?, &id, &url, &title, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_pm_module_link(hub: State<'_, WorkspaceHub>, slug: String, link_id: String) -> Result<(), AppError> {
    Ok(modules::remove_link(&hub.pool(&slug)?, &link_id).await?)
}

// ── 0.11.7 (D-080): pages ──

#[tauri::command]
#[specta::specta]
pub async fn list_pm_pages(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmPage>, AppError> {
    Ok(pages::list(&hub.pool(&slug)?, &project_id).await?)
}

/// A page with its document.
#[tauri::command]
#[specta::specta]
pub async fn get_pm_page(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmPageDoc, AppError> {
    Ok(pages::get_doc(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_page(hub: State<'_, WorkspaceHub>, slug: String, input: PageInput) -> Result<PmPage, AppError> {
    Ok(pages::create(&hub.pool(&slug)?, input, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_page(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: PagePatch) -> Result<PmPage, AppError> {
    Ok(pages::update(&hub.pool(&slug)?, &id, patch, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_pm_page(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<PmPage, AppError> {
    Ok(pages::archive(&hub.pool(&slug)?, &id, archived).await?)
}

/// An archived page, with its sub-pages and their files.
#[tauri::command]
#[specta::specta]
pub async fn delete_pm_page(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(pages::delete(&hub.pool(&slug)?, &folder, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn favorite_pm_page(hub: State<'_, WorkspaceHub>, slug: String, id: String, on: bool) -> Result<PmPage, AppError> {
    Ok(pages::favorite(&hub.pool(&slug)?, &id, on).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn duplicate_pm_page(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmPage, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(pages::duplicate(&hub.pool(&slug)?, &folder, &id, ME).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_page_versions(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmPageVersion>, AppError> {
    Ok(pages::versions(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn restore_pm_page_version(hub: State<'_, WorkspaceHub>, slug: String, version_id: String) -> Result<PmPage, AppError> {
    Ok(pages::restore(&hub.pool(&slug)?, &version_id, ME).await?)
}

/// A picture for a page, from a file on this Mac.
#[tauri::command]
#[specta::specta]
pub async fn add_pm_page_file(hub: State<'_, WorkspaceHub>, slug: String, page_id: String, path: String) -> Result<ItemFile, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(pages::add_file(&hub.pool(&slug)?, &folder, &page_id, Path::new(&path)).await?)
}

/// A picture pasted into a page, as bytes.
#[tauri::command]
#[specta::specta]
pub async fn add_pm_page_file_bytes(hub: State<'_, WorkspaceHub>, slug: String, page_id: String, name: String, bytes: Vec<u8>) -> Result<ItemFile, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(pages::add_file_bytes(&hub.pool(&slug)?, &folder, &page_id, &name, &bytes).await?)
}

/// A page as Markdown, into the file the save dialog named.
#[tauri::command]
#[specta::specta]
pub async fn export_pm_page_markdown(hub: State<'_, WorkspaceHub>, slug: String, id: String, path: String) -> Result<(), AppError> {
    Ok(pages::export_markdown(&hub.pool(&slug)?, &id, Path::new(&path)).await?)
}

/// The Mac's print sheet for the window asking (Save as PDF is in it).
#[tauri::command]
#[specta::specta]
pub async fn print_window(window: tauri::WebviewWindow) -> Result<(), AppError> {
    window.print().map_err(|e| AppError::from(anyhow::anyhow!(e)))
}

// ── 0.11.8 (D-081): intake ──

#[tauri::command]
#[specta::specta]
pub async fn list_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmIntakeItem>, AppError> {
    Ok(intake::list(&hub.pool(&slug)?, &project_id, cycles::today()).await?)
}

/// How many requests wait in a project's Intake today.
#[tauri::command]
#[specta::specta]
pub async fn pm_intake_count(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<u32, AppError> {
    Ok(intake::open_count(&hub.pool(&slug)?, &project_id, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, input: IntakeInput) -> Result<PmIntakeItem, AppError> {
    Ok(intake::create(&hub.pool(&slug)?, input, ME, cycles::today()).await?)
}

/// Accepted into a state (the project's default when none).
#[tauri::command]
#[specta::specta]
pub async fn accept_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, id: String, state_id: Option<String>) -> Result<PmIntakeItem, AppError> {
    Ok(intake::accept(&hub.pool(&slug)?, &id, state_id.as_deref(), ME, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn decline_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmIntakeItem, AppError> {
    Ok(intake::decline(&hub.pool(&slug)?, &id, ME, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn snooze_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, id: String, until: String) -> Result<PmIntakeItem, AppError> {
    Ok(intake::snooze(&hub.pool(&slug)?, &id, &until, ME, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn duplicate_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, id: String, of: String) -> Result<PmIntakeItem, AppError> {
    Ok(intake::duplicate(&hub.pool(&slug)?, &id, &of, ME, cycles::today()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_intake(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(intake::delete(&hub.pool(&slug)?, &folder, &id, cycles::today()).await?)
}

// ── 0.11.9 (D-082): Home, Your work, navigation, ⌘K ──

#[tauri::command]
#[specta::specta]
pub async fn list_pm_stickies(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PmSticky>, AppError> {
    Ok(home::stickies(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_sticky(hub: State<'_, WorkspaceHub>, slug: String, color: String) -> Result<PmSticky, AppError> {
    Ok(home::add_sticky(&hub.pool(&slug)?, &color).await?)
}

/// A sticky's words (the editor's JSON text) or its colour.
#[tauri::command]
#[specta::specta]
pub async fn update_pm_sticky(hub: State<'_, WorkspaceHub>, slug: String, id: String, description: Option<String>, color: Option<String>) -> Result<PmSticky, AppError> {
    Ok(home::update_sticky(&hub.pool(&slug)?, &id, description.as_deref(), color.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_sticky(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(home::delete_sticky(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_quick_links(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PmQuickLink>, AppError> {
    Ok(home::quick_links(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn add_pm_quick_link(hub: State<'_, WorkspaceHub>, slug: String, title: String, url: String) -> Result<PmQuickLink, AppError> {
    Ok(home::add_quick_link(&hub.pool(&slug)?, &title, &url).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_quick_link(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(home::delete_quick_link(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn pm_home_widgets(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<HomeWidget>, AppError> {
    Ok(home::widgets(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_pm_home_widgets(hub: State<'_, WorkspaceHub>, slug: String, widgets: Vec<HomeWidget>) -> Result<Vec<HomeWidget>, AppError> {
    Ok(home::set_widgets(&hub.pool(&slug)?, &widgets).await?)
}

/// The Projects section's items that are hidden.
#[tauri::command]
#[specta::specta]
pub async fn pm_hidden_navigation(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<String>, AppError> {
    Ok(home::hidden_navigation(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_pm_navigation(hub: State<'_, WorkspaceHub>, slug: String, key: String, hidden: bool) -> Result<Vec<String>, AppError> {
    Ok(home::set_navigation(&hub.pool(&slug)?, &key, hidden).await?)
}

/// Opened now: first in Recents.
#[tauri::command]
#[specta::specta]
pub async fn visit_pm(hub: State<'_, WorkspaceHub>, slug: String, kind: RecentKind, id: String) -> Result<(), AppError> {
    Ok(home::visit(&hub.pool(&slug)?, kind, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_recents(hub: State<'_, WorkspaceHub>, slug: String, kind: Option<RecentKind>) -> Result<Vec<PmRecent>, AppError> {
    Ok(home::recents(&hub.pool(&slug)?, kind, 20).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn pm_archives(hub: State<'_, WorkspaceHub>, slug: String) -> Result<PmArchives, AppError> {
    Ok(home::archives(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn pm_my_activity(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<MyActivity>, AppError> {
    Ok(home::my_activity(&hub.pool(&slug)?, 20).await?)
}

/// ⌘K: projects, cycles, modules and pages by name.
#[tauri::command]
#[specta::specta]
pub async fn search_pm(hub: State<'_, WorkspaceHub>, slug: String, query: String) -> Result<Vec<PmHit>, AppError> {
    Ok(home::search(&hub.pool(&slug)?, &query).await?)
}

// ── 0.11.10 (D-083): analytics, exports, the CSV importer ──

fn day(text: &str) -> Result<chrono::NaiveDate, AppError> {
    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| AppError::invalid("A day is YYYY-MM-DD"))
}

#[tauri::command]
#[specta::specta]
pub async fn pm_overview(hub: State<'_, WorkspaceHub>, slug: String, project_id: Option<String>) -> Result<PmOverview, AppError> {
    Ok(analytics::overview(&hub.pool(&slug)?, project_id.as_deref().filter(|p| !p.is_empty()), cycles::today()).await?)
}

/// Each day from `from` to `to`: the work items made and resolved.
#[tauri::command]
#[specta::specta]
pub async fn pm_created_vs_resolved(hub: State<'_, WorkspaceHub>, slug: String, project_id: Option<String>, from: String, to: String) -> Result<Vec<DayCount>, AppError> {
    Ok(analytics::created_vs_resolved(&hub.pool(&slug)?, project_id.as_deref().filter(|p| !p.is_empty()), day(&from)?, day(&to)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn pm_insights(hub: State<'_, WorkspaceHub>, slug: String, project_id: Option<String>, axis: String) -> Result<Vec<Share>, AppError> {
    Ok(analytics::insights(&hub.pool(&slug)?, project_id.as_deref().filter(|p| !p.is_empty()), &axis).await?)
}

/// Work items as the rules keep them, into the file the save dialog named.
#[tauri::command]
#[specta::specta]
pub async fn export_pm_items(hub: State<'_, WorkspaceHub>, slug: String, project_id: Option<String>, rules: Vec<PmRule>, format: ExportFormat, path: String, label: String) -> Result<PmExportReport, AppError> {
    Ok(transfer::export_query(&hub, &slug, project_id.as_deref().filter(|p| !p.is_empty()), &rules, format, Path::new(&path), &label).await?)
}

/// What a Plane CSV holds, before it comes into a project.
#[tauri::command]
#[specta::specta]
pub async fn preview_pm_import(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, path: String) -> Result<PmImportPreview, AppError> {
    Ok(transfer::preview(&hub.pool(&slug)?, &project_id, Path::new(&path)).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn import_pm_items(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, path: String) -> Result<PmImportSummary, AppError> {
    Ok(transfer::import(&hub.pool(&slug)?, &project_id, Path::new(&path), ME).await?)
}
