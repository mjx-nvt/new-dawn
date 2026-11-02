//! Bulk operations (D-075, Plane's, from the List layout): the selected items checked first — one
//! that breaks a rule is named with the rule and nothing is written — then each changed as it
//! would be alone, with its own activity (ten items' priority, ten activities).

use anyhow::{Result, anyhow};
use serde::Deserialize;
use specta::Type;
use sqlx::SqlitePool;

use super::items::{self, ItemPatch};
use super::settings::StateGroup;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum PmBulkAction {
    Subscribe,
    Archive,
    Delete,
}

/// An item's refusal, named: "VIFC-3: The start date comes before the due date".
fn named(key: &str, err: anyhow::Error) -> anyhow::Error {
    match err.downcast::<AppError>() {
        Ok(e) => anyhow!(AppError::new(&e.code, format!("{key}: {}", e.message))),
        Err(e) => e,
    }
}

/// The selection, each item once, in its order.
fn unique(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// The same edits on every item; how many changed. Only what Plane's bar sets (the cycle too,
/// D-078; modules, D-079, which it adds to: an item keeps the others) — not a title, a
/// description, a parent or an order.
pub async fn update(pool: &SqlitePool, ids: &[String], patch: &ItemPatch, actor: &str) -> Result<u32> {
    let patch = ItemPatch {
        state_id: patch.state_id.clone(),
        priority: patch.priority,
        assignees: patch.assignees.clone(),
        labels: patch.labels.clone(),
        start_date: patch.start_date.clone(),
        target_date: patch.target_date.clone(),
        estimate_point_id: patch.estimate_point_id.clone(),
        cycle_id: patch.cycle_id.clone(),
        modules: patch.modules.clone(),
        ..ItemPatch::default()
    };
    let ids = unique(ids);
    let mut plans = Vec::with_capacity(ids.len());
    for id in &ids {
        let mut patch = patch.clone();
        if let (Some(add), Ok(item)) = (&patch.modules, items::get(pool, id).await) {
            let mut all: Vec<String> = item.modules.into_iter().map(|m| m.id).collect();
            all.extend(add.iter().cloned());
            patch.modules = Some(all);
        }
        let plan = items::plan(pool, id, patch).await;
        let key = match &plan {
            Ok(p) => p.item.key.clone(),
            // Gone meanwhile: named by its id.
            Err(_) => items::get(pool, id).await.map(|i| i.key).unwrap_or_else(|_| id.clone()),
        };
        plans.push(plan.map_err(|e| named(&key, e))?);
    }
    let mut changed = 0;
    for plan in plans {
        if items::write(pool, plan, actor, true).await? {
            changed += 1;
        }
    }
    Ok(changed)
}

/// Subscribe, archive (only when every item is completed or cancelled) or delete (sub-items
/// with their parents).
pub async fn act(pool: &SqlitePool, folder: &str, ids: &[String], action: PmBulkAction, actor: &str) -> Result<u32> {
    let ids = unique(ids);
    let mut all = Vec::with_capacity(ids.len());
    for id in &ids {
        all.push(items::get(pool, id).await?);
    }
    let mut done = 0;
    match action {
        PmBulkAction::Subscribe => {
            for i in &all {
                items::subscribe(pool, &i.id, "me", true).await?;
                done += 1;
            }
        }
        PmBulkAction::Archive => {
            let open: Vec<&str> = all.iter().filter(|i| !matches!(i.state_group, StateGroup::Completed | StateGroup::Cancelled)).map(|i| i.key.as_str()).collect();
            if !open.is_empty() {
                return Err(anyhow!(AppError::invalid(format!(
                    "Only completed or cancelled work items can be archived — {} {} not; nothing was archived",
                    open.join(", "),
                    if open.len() == 1 { "is" } else { "are" }
                ))));
            }
            for i in &all {
                items::archive(pool, &i.id, true, actor).await?;
                done += 1;
            }
        }
        PmBulkAction::Delete => {
            for i in &all {
                // A sub-item may have gone with its parent already.
                if items::get(pool, &i.id).await.is_ok() {
                    items::delete(pool, folder, &i.id).await?;
                }
                done += 1;
            }
        }
    }
    Ok(done)
}

/// Copies of the items in a project (theirs or another), in their order.
pub async fn copy(pool: &SqlitePool, folder: &str, ids: &[String], project_id: &str, actor: &str) -> Result<u32> {
    let project = super::get(pool, project_id).await?;
    if project.archived_at.is_some() {
        return Err(anyhow!(AppError::invalid(format!("{} is archived — restore it first", project.name))));
    }
    let ids = unique(ids);
    let mut all = Vec::with_capacity(ids.len());
    for id in &ids {
        all.push(items::get(pool, id).await?);
    }
    all.sort_by(|a, b| a.sort_order.total_cmp(&b.sort_order));
    for i in &all {
        items::copy(pool, folder, &i.id, Some(project_id), actor).await?;
    }
    Ok(all.len() as u32)
}
