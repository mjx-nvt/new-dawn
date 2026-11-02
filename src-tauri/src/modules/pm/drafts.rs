//! Drafts (D-074): a work item not made yet — the create form's fields, kept when its window
//! closes with text in it; edited, copied, moved to another project, deleted, or made.

use anyhow::{Result, anyhow};
use serde::Serialize;
use specta::Type;
use sqlx::SqlitePool;

use super::items::{self, ItemInput, PmItem};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmDraft {
    pub id: String,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub input: ItemInput,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct DraftRow {
    id: String,
    project_id: Option<String>,
    project_name: Option<String>,
    data_json: String,
    created_at: String,
    updated_at: String,
}

/// A draft's form, read over an empty one: a field the form gained since the draft was kept takes
/// its default, so a draft never goes blank because the form grew.
fn read_input(json: &str) -> ItemInput {
    let mut form = serde_json::to_value(ItemInput::default()).unwrap_or_default();
    if let (Some(form), Ok(serde_json::Value::Object(kept))) = (form.as_object_mut(), serde_json::from_str::<serde_json::Value>(json)) {
        form.extend(kept);
    }
    serde_json::from_value(form).unwrap_or_default()
}

/// Every draft of the workspace, newest first.
pub async fn list(pool: &SqlitePool) -> Result<Vec<PmDraft>> {
    let rows: Vec<DraftRow> = sqlx::query_as(
        "SELECT d.id, d.project_id, p.name AS project_name, d.data_json, d.created_at, d.updated_at FROM pm_draft d LEFT JOIN project p ON p.id = d.project_id ORDER BY d.updated_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let mut input = read_input(&r.data_json);
            input.project_id = r.project_id.clone().unwrap_or_default();
            PmDraft { id: r.id, project_id: r.project_id, project_name: r.project_name, input, created_at: r.created_at, updated_at: r.updated_at }
        })
        .collect())
}

async fn get(pool: &SqlitePool, id: &str) -> Result<PmDraft> {
    list(pool).await?.into_iter().find(|d| d.id == id).ok_or_else(|| anyhow!(AppError::not_found("That draft is gone")))
}

/// Keeps a draft (a new one when `id` is none); its project may be none (moved later).
pub async fn save(pool: &SqlitePool, id: Option<&str>, input: ItemInput) -> Result<PmDraft> {
    let project = Some(input.project_id.clone()).filter(|p| !p.is_empty());
    if let Some(p) = &project {
        super::get(pool, p).await?;
    }
    let data = serde_json::to_string(&input)?;
    let id = match id.filter(|i| !i.is_empty()) {
        Some(i) => {
            let done = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_draft SET project_id = ?, data_json = ?, updated_at = {NOW} WHERE id = ?"))).bind(&project).bind(&data).bind(i).execute(pool).await?;
            if done.rows_affected() == 0 {
                return Err(anyhow!(AppError::not_found("That draft is gone")));
            }
            i.to_owned()
        }
        None => {
            let i = uuid::Uuid::now_v7().to_string();
            sqlx::query("INSERT INTO pm_draft (id, project_id, data_json) VALUES (?, ?, ?)").bind(&i).bind(&project).bind(&data).execute(pool).await?;
            i
        }
    };
    get(pool, &id).await
}

pub async fn copy(pool: &SqlitePool, id: &str) -> Result<PmDraft> {
    let d = get(pool, id).await?;
    save(pool, None, d.input).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM pm_draft WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// What no longer holds in a draft (it waited while people, labels or states changed) is left
/// out rather than refused.
async fn sanitize(pool: &SqlitePool, mut input: ItemInput) -> Result<ItemInput> {
    let project = input.project_id.clone();
    let mut people = Vec::new();
    for w in input.assignees {
        if w == "me" || sqlx::query_as::<_, (String,)>("SELECT id FROM person WHERE id = ? AND deleted_at IS NULL").bind(&w).fetch_optional(pool).await?.is_some() {
            people.push(w);
        }
    }
    input.assignees = people;
    let mut labels = Vec::new();
    for l in input.labels {
        if sqlx::query_as::<_, (String,)>("SELECT id FROM pm_label WHERE id = ? AND project_id = ?").bind(&l).bind(&project).fetch_optional(pool).await?.is_some() {
            labels.push(l);
        }
    }
    input.labels = labels;
    if let Some(s) = input.state_id.clone() {
        let ok = sqlx::query_as::<_, (String,)>("SELECT id FROM pm_state WHERE id = ? AND project_id = ? AND state_group <> 'triage'").bind(&s).bind(&project).fetch_optional(pool).await?.is_some();
        input.state_id = ok.then_some(s);
    }
    if let Some(e) = input.estimate_point_id.clone() {
        let ok = sqlx::query_as::<_, (String,)>("SELECT ep.id FROM pm_estimate_point ep JOIN pm_project m ON m.estimate_id = ep.estimate_id WHERE ep.id = ? AND m.project_id = ?").bind(&e).bind(&project).fetch_optional(pool).await?.is_some();
        input.estimate_point_id = ok.then_some(e);
    }
    if let Some(p) = input.parent_id.clone() {
        let ok = sqlx::query_as::<_, (String,)>("SELECT id FROM pm_item WHERE id = ? AND project_id = ? AND archived_at IS NULL").bind(&p).bind(&project).fetch_optional(pool).await?.is_some();
        input.parent_id = ok.then_some(p);
    }
    Ok(input)
}

/// Makes the work item the draft holds (as it now reads) and lets the draft go.
pub async fn publish(pool: &SqlitePool, id: &str, input: ItemInput, actor: &str) -> Result<PmItem> {
    get(pool, id).await?;
    let input = sanitize(pool, input).await?;
    let made = items::create(pool, input, actor).await?;
    delete(pool, id).await?;
    Ok(made)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_draft_kept_before_the_form_grew_still_reads() {
        let input = super::read_input(r#"{"project_id":"p","name":"Sandbox rules","assignees":["me"]}"#);
        assert_eq!((input.name.as_str(), input.assignees.len(), input.labels.len()), ("Sandbox rules", 1, 0));
    }
}
