//! Plane's modules (D-079): a project's larger pieces of work — a name unique in the project, a
//! description, optional dates, Plane's six statuses, a lead and members ("me" or people), links;
//! an item in several modules; archived when completed or cancelled. Their details: progress,
//! the work by state group, what needs attention (open items overdue or blocked), breakdowns by
//! state, assignee, label and priority.

use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::cycles::Share;
use super::relations::{self, PmLink};
use super::items;
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const NAME_MAX: usize = 255;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum ModuleStatus {
    Backlog,
    Planned,
    InProgress,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmModule {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub status: ModuleStatus,
    pub lead: Option<String>,
    pub members: Vec<String>,
    pub archived_at: Option<String>,
    /// In the sidebar's Favorites.
    pub is_favorite: bool,
    pub sort_order: f64,
    /// Its work items, completed, cancelled.
    pub items: u32,
    pub done: u32,
    pub cancelled: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: String,
    name: String,
    description: String,
    start_date: Option<String>,
    target_date: Option<String>,
    status: ModuleStatus,
    lead: Option<String>,
    members: Option<String>,
    archived_at: Option<String>,
    is_favorite: bool,
    sort_order: f64,
    items: u32,
    done: u32,
    cancelled: u32,
    created_at: String,
    updated_at: String,
}

impl From<Row> for PmModule {
    fn from(r: Row) -> Self {
        PmModule {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            start_date: r.start_date,
            target_date: r.target_date,
            status: r.status,
            lead: r.lead,
            members: r.members.map(|m| m.split('\u{1f}').filter(|s| !s.is_empty()).map(str::to_owned).collect()).unwrap_or_default(),
            archived_at: r.archived_at,
            is_favorite: r.is_favorite,
            sort_order: r.sort_order,
            items: r.items,
            done: r.done,
            cancelled: r.cancelled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const SELECT: &str = "SELECT d.id, d.project_id, d.name, d.description, d.start_date, d.target_date, d.status, d.lead, \
     (SELECT group_concat(who, char(31)) FROM pm_module_member mm WHERE mm.module_id = d.id) AS members, d.archived_at, \
     EXISTS (SELECT 1 FROM favorite f WHERE f.pm_module_id = d.id AND f.deleted_at IS NULL) AS is_favorite, d.sort_order, \
     (SELECT count(*) FROM pm_module_item x JOIN pm_item i ON i.id = x.item_id WHERE x.module_id = d.id AND i.archived_at IS NULL) AS items, \
     (SELECT count(*) FROM pm_module_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id WHERE x.module_id = d.id AND i.archived_at IS NULL AND s.state_group = 'completed') AS done, \
     (SELECT count(*) FROM pm_module_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id WHERE x.module_id = d.id AND i.archived_at IS NULL AND s.state_group = 'cancelled') AS cancelled, \
     d.created_at, d.updated_at FROM pm_module d";

pub async fn list(pool: &SqlitePool, project_id: &str) -> Result<Vec<PmModule>> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE d.project_id = ? ORDER BY is_favorite DESC, d.sort_order, d.created_at"))).bind(project_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(PmModule::from).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PmModule> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE d.id = ?"))).bind(id).fetch_optional(pool).await?;
    row.map(PmModule::from).ok_or_else(|| anyhow!(AppError::not_found("That module is gone")))
}

/// One line, no control characters (an item reads its modules' names joined by them).
fn clean_name(name: &str) -> Result<String> {
    let n = name.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
    if n.is_empty() {
        return Err(invalid("A module needs a name"));
    }
    if n.chars().count() > NAME_MAX {
        return Err(invalid(format!("A module's name is at most {NAME_MAX} characters")));
    }
    Ok(n)
}

/// A name once a project, compared in full Unicode (the index folds ASCII only).
async fn check_name(pool: &SqlitePool, project_id: &str, name: &str, except: Option<&str>) -> Result<()> {
    let names: Vec<(String,)> = sqlx::query_as("SELECT name FROM pm_module WHERE project_id = ? AND id <> coalesce(?, '')").bind(project_id).bind(except).fetch_all(pool).await?;
    if names.iter().any(|(n,)| super::settings::same(n, name)) {
        return Err(invalid(format!("The project has a module “{name}” already")));
    }
    Ok(())
}

/// Optional days, YYYY-MM-DD (years 1900 to 2199), the start not after the target.
fn clean_dates(start: Option<&str>, target: Option<&str>) -> Result<(Option<String>, Option<String>)> {
    let (s, t) = (items::clean_day(start)?, items::clean_day(target)?);
    if let (Some(s), Some(t)) = (&s, &t) {
        if s > t {
            return Err(invalid("A module's target date comes on or after its start"));
        }
    }
    Ok((s, t))
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct ModuleInput {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub status: ModuleStatus,
    pub lead: Option<String>,
    pub members: Vec<String>,
}

pub async fn create(pool: &SqlitePool, input: ModuleInput, actor: &str) -> Result<PmModule> {
    super::get(pool, &input.project_id).await?;
    let name = clean_name(&input.name)?;
    check_name(pool, &input.project_id, &name, None).await?;
    let (start, target) = clean_dates(input.start_date.as_deref(), input.target_date.as_deref())?;
    let lead = super::check_ref(pool, input.lead.as_deref()).await?;
    let members = items::check_people(pool, &input.members, &[]).await?;
    let id = uuid::Uuid::now_v7().to_string();
    let (last,): (Option<f64>,) = sqlx::query_as("SELECT max(sort_order) FROM pm_module WHERE project_id = ?").bind(&input.project_id).fetch_one(pool).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO pm_module (id, project_id, name, description, start_date, target_date, status, lead, sort_order, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&input.project_id)
        .bind(&name)
        .bind(input.description.trim().chars().take(2000).collect::<String>())
        .bind(&start)
        .bind(&target)
        .bind(input.status)
        .bind(&lead)
        .bind(last.unwrap_or(0.0) + 1000.0)
        .bind(actor)
        .execute(&mut *tx)
        .await
        .map_err(taken)?;
    for w in &members {
        sqlx::query("INSERT INTO pm_module_member (module_id, who) VALUES (?, ?)").bind(&id).bind(w).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    get(pool, &id).await
}

/// A name taken meanwhile (the index's ASCII folding), said plainly.
fn taken(e: sqlx::Error) -> anyhow::Error {
    match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => invalid("The project has a module by that name already"),
        _ => e.into(),
    }
}

/// The edits; a field left out stays (an empty date clears it; members replace).
#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct ModulePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub status: Option<ModuleStatus>,
    pub lead: Option<String>,
    pub members: Option<Vec<String>>,
    pub sort_order: Option<f64>,
}

pub async fn update(pool: &SqlitePool, id: &str, patch: ModulePatch) -> Result<PmModule> {
    let m = get(pool, id).await?;
    if m.archived_at.is_some() {
        return Err(invalid("An archived module is read-only — restore it to change it"));
    }
    let name = match patch.name.as_deref() {
        Some(n) => {
            let n = clean_name(n)?;
            check_name(pool, &m.project_id, &n, Some(id)).await?;
            Some(n)
        }
        None => None,
    };
    let dates = if patch.start_date.is_some() || patch.target_date.is_some() {
        Some(clean_dates(patch.start_date.as_deref().or(m.start_date.as_deref()), patch.target_date.as_deref().or(m.target_date.as_deref()))?)
    } else {
        None
    };
    let lead = match patch.lead.as_deref() {
        Some(l) if Some(l) == m.lead.as_deref() => None,
        Some(l) => Some(super::check_ref(pool, Some(l)).await?),
        None => None,
    };
    let members = match &patch.members {
        // A member deleted from the CRM since stays; a new one must be live.
        Some(who) => Some(items::check_people(pool, who, &m.members).await?),
        None => None,
    };
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_module SET name = coalesce(?, name), description = coalesce(?, description), \
         start_date = CASE WHEN ? THEN ? ELSE start_date END, target_date = CASE WHEN ? THEN ? ELSE target_date END, \
         status = coalesce(?, status), lead = CASE WHEN ? THEN ? ELSE lead END, sort_order = coalesce(?, sort_order), updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&name)
    .bind(patch.description.as_deref().map(|d| d.trim().chars().take(2000).collect::<String>()))
    .bind(dates.is_some())
    .bind(dates.as_ref().and_then(|(s, _)| s.clone()))
    .bind(dates.is_some())
    .bind(dates.as_ref().and_then(|(_, t)| t.clone()))
    .bind(patch.status)
    .bind(lead.is_some())
    .bind(lead.clone().flatten())
    .bind(patch.sort_order.filter(|o| o.is_finite()))
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(taken)?;
    if let Some(who) = &members {
        sqlx::query("DELETE FROM pm_module_member WHERE module_id = ?").bind(id).execute(&mut *tx).await?;
        for w in who {
            sqlx::query("INSERT INTO pm_module_member (module_id, who) VALUES (?, ?)").bind(id).bind(w).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    get(pool, id).await
}

/// Archived (completed or cancelled only) or back.
pub async fn archive(pool: &SqlitePool, id: &str, on: bool) -> Result<PmModule> {
    let m = get(pool, id).await?;
    if on && !matches!(m.status, ModuleStatus::Completed | ModuleStatus::Cancelled) {
        return Err(invalid("Only a completed or cancelled module can be archived"));
    }
    let sql = if on { format!("UPDATE pm_module SET archived_at = {NOW}, updated_at = {NOW} WHERE id = ?") } else { format!("UPDATE pm_module SET archived_at = NULL, updated_at = {NOW} WHERE id = ?") };
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(pool).await?;
    // Archiving unstars (Plane's).
    if on {
        super::views::pin(pool, super::views::Pin::Module, id, false).await?;
    }
    get(pool, id).await
}

/// Deleted; its work items stay — each item's activity says so.
pub async fn delete(pool: &SqlitePool, id: &str, actor: &str) -> Result<()> {
    let m = get(pool, id).await?;
    let mut tx = pool.begin().await?;
    let held: Vec<(String,)> = sqlx::query_as("SELECT item_id FROM pm_module_item WHERE module_id = ?").bind(id).fetch_all(&mut *tx).await?;
    for (item,) in &held {
        items::activity(&mut tx, item, "updated", "module", &m.name, "", (None, None), actor).await?;
    }
    sqlx::query("DELETE FROM pm_module WHERE id = ?").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Into the sidebar's Favorites, or out.
pub async fn favorite(pool: &SqlitePool, id: &str, on: bool) -> Result<PmModule> {
    if on && get(pool, id).await?.archived_at.is_some() {
        return Err(invalid("An archived module is read-only — restore it to star it"));
    }
    super::views::pin(pool, super::views::Pin::Module, id, on).await?;
    get(pool, id).await
}

/// The modules an item is put in, checked: the project's, existing; one it isn't in yet not
/// archived, and none it leaves archived (an archived module is read-only). Once each.
pub(crate) async fn check(pool: &SqlitePool, project_id: &str, wanted: &[String], held: &[String]) -> Result<Vec<String>> {
    for left in held.iter().filter(|h| !wanted.iter().any(|w| w.trim() == h.as_str())) {
        let m = get(pool, left).await?;
        if m.archived_at.is_some() {
            return Err(invalid(format!("“{}” is archived — restore it to take work out", m.name)));
        }
    }
    let mut kept: Vec<String> = Vec::new();
    for id in wanted.iter().map(|w| w.trim()).filter(|w| !w.is_empty()) {
        if kept.iter().any(|k| k == id) {
            continue;
        }
        if !held.iter().any(|h| h == id) {
            let m = get(pool, id).await?;
            if m.project_id != project_id {
                return Err(invalid(format!("“{}” is another project's module", m.name)));
            }
            if m.archived_at.is_some() {
                return Err(invalid(format!("“{}” is archived — restore it first", m.name)));
            }
        }
        kept.push(id.to_owned());
    }
    Ok(kept)
}

/// An item's modules made `wanted` (checked), an activity row a module joined or left, in the
/// caller's transaction.
pub(crate) async fn set_modules_in(tx: &mut Transaction<'_, Sqlite>, item: &str, wanted: &[String], actor: &str) -> Result<()> {
    let now: Vec<(String, String)> = sqlx::query_as("SELECT d.id, d.name FROM pm_module_item x JOIN pm_module d ON d.id = x.module_id WHERE x.item_id = ?").bind(item).fetch_all(&mut **tx).await?;
    for (id, name) in now.iter().filter(|(i, _)| !wanted.contains(i)) {
        sqlx::query("DELETE FROM pm_module_item WHERE module_id = ? AND item_id = ?").bind(id).bind(item).execute(&mut **tx).await?;
        items::activity(tx, item, "updated", "module", name, "", (None, None), actor).await?;
    }
    for id in wanted.iter().filter(|w| !now.iter().any(|(i, _)| i == *w)) {
        let (name,): (String,) = sqlx::query_as("SELECT name FROM pm_module WHERE id = ?").bind(id).fetch_optional(&mut **tx).await?.ok_or_else(|| anyhow!(AppError::not_found("That module is gone")))?;
        sqlx::query("INSERT INTO pm_module_item (module_id, item_id) VALUES (?, ?)").bind(id).bind(item).execute(&mut **tx).await?;
        items::activity(tx, item, "updated", "module", "", &name, (None, None), actor).await?;
    }
    Ok(())
}

/// Puts items in a module (they keep their others), in one transaction.
pub async fn add_items(pool: &SqlitePool, id: &str, item_ids: &[String], actor: &str) -> Result<PmModule> {
    let mut sets = Vec::with_capacity(item_ids.len());
    for item in item_ids {
        let it = items::get(pool, item).await?;
        if it.archived_at.is_some() {
            return Err(invalid(format!("{} is archived — restore it first", it.key)));
        }
        let held: Vec<String> = it.modules.iter().map(|m| m.id.clone()).collect();
        let mut wanted = held.clone();
        wanted.push(id.to_owned());
        sets.push((it.id, check(pool, &it.project_id, &wanted, &held).await?));
    }
    let mut tx = pool.begin().await?;
    for (item, wanted) in &sets {
        set_modules_in(&mut tx, item, wanted, actor).await?;
    }
    tx.commit().await?;
    get(pool, id).await
}

/// What needs attention in a module: an open item overdue, or blocked by an open one.
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Attention {
    #[sqlx(flatten)]
    pub item: relations::PmItemRef,
    pub overdue: bool,
    pub blocked: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ModuleDetails {
    /// Work items by state group (Backlog … Cancelled), in that order.
    pub groups: Vec<Share>,
    pub attention: Vec<Attention>,
    pub states: Vec<Share>,
    pub assignees: Vec<Share>,
    pub labels: Vec<Share>,
    pub priorities: Vec<Share>,
    pub links: Vec<PmLink>,
}

pub async fn details(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<ModuleDetails> {
    get(pool, id).await?;
    let read = |sql: String| async move {
        let rows: Vec<(String, String, u32, u32)> = sqlx::query_as(sqlx::AssertSqlSafe(sql)).bind(id).fetch_all(pool).await?;
        Ok::<_, anyhow::Error>(rows.into_iter().map(|(key, label, total, done)| Share { key, label, total, done }).collect::<Vec<_>>())
    };
    let from = "FROM pm_module_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id";
    let done = "sum(s.state_group = 'completed')";
    let groups = read(format!(
        "SELECT s.state_group, s.state_group, count(*), {done} {from} WHERE x.module_id = ? AND i.archived_at IS NULL GROUP BY 1 \
         ORDER BY CASE s.state_group WHEN 'backlog' THEN 0 WHEN 'unstarted' THEN 1 WHEN 'started' THEN 2 WHEN 'completed' THEN 3 ELSE 4 END"
    ))
    .await?;
    let states = read(format!("SELECT s.id, s.name, count(*), {done} {from} WHERE x.module_id = ? AND i.archived_at IS NULL GROUP BY s.id ORDER BY s.sequence")).await?;
    let assignees = read(format!(
        "SELECT coalesce(a.who, 'none'), CASE WHEN a.who IS NULL THEN 'No assignee' WHEN a.who = 'me' THEN 'You' ELSE coalesce(trim(p.name_first || ' ' || coalesce(p.name_last, '')), 'Someone') END, count(*), {done} \
         {from} LEFT JOIN pm_item_assignee a ON a.item_id = i.id LEFT JOIN person p ON p.id = a.who WHERE x.module_id = ? AND i.archived_at IS NULL GROUP BY 1 ORDER BY 3 DESC"
    ))
    .await?;
    let labels = read(format!(
        "SELECT coalesce(l.id, 'none'), coalesce(l.name, 'No label'), count(*), {done} {from} LEFT JOIN pm_item_label il ON il.item_id = i.id \
         LEFT JOIN pm_label l ON l.id = il.label_id WHERE x.module_id = ? AND i.archived_at IS NULL GROUP BY 1 ORDER BY 3 DESC"
    ))
    .await?;
    let priorities = read(format!(
        "SELECT i.priority, i.priority, count(*), {done} {from} WHERE x.module_id = ? AND i.archived_at IS NULL GROUP BY 1 \
         ORDER BY CASE i.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 WHEN 'low' THEN 3 ELSE 4 END"
    ))
    .await?;
    // Needs attention: open items past their due date, or blocked by an item still open (a
    // Blocking is stored as the other item's Blocked by).
    let attention: Vec<Attention> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT * FROM (SELECT {}, o.sequence AS seq, coalesce(o.target_date < ?, 0) AS overdue, \
         EXISTS (SELECT 1 FROM pm_relation r JOIN pm_item b ON b.id = r.related_id JOIN pm_state bs ON bs.id = b.state_id \
                 WHERE r.item_id = o.id AND r.kind = 'blocked_by' AND bs.state_group NOT IN ('completed', 'cancelled')) AS blocked \
         FROM pm_module_item x JOIN pm_item o ON o.id = x.item_id JOIN pm_project m ON m.project_id = o.project_id JOIN pm_state s ON s.id = o.state_id \
         WHERE x.module_id = ? AND o.archived_at IS NULL AND s.state_group NOT IN ('completed', 'cancelled')) WHERE overdue OR blocked ORDER BY seq",
        relations::REF
    )))
    .bind(today.to_string())
    .bind(id)
    .fetch_all(pool)
    .await?;
    let links = module_links(pool, id).await?;
    Ok(ModuleDetails { groups, attention, states, assignees, labels, priorities, links })
}

// ── Links ──

/// A module's links, read as an item's (`item_id` is the module's id).
pub async fn module_links(pool: &SqlitePool, id: &str) -> Result<Vec<PmLink>> {
    Ok(sqlx::query_as("SELECT id, module_id AS item_id, url, title, created_by, created_at FROM pm_module_link WHERE module_id = ? ORDER BY created_at, id").bind(id).fetch_all(pool).await?)
}

/// An archived module's links stay as they are.
async fn writable(pool: &SqlitePool, id: &str) -> Result<()> {
    if get(pool, id).await?.archived_at.is_some() {
        return Err(invalid("An archived module is read-only — restore it to change it"));
    }
    Ok(())
}

pub async fn add_link(pool: &SqlitePool, id: &str, url: &str, title: &str, actor: &str) -> Result<PmLink> {
    writable(pool, id).await?;
    let (url, title) = (relations::clean_url(url)?, relations::clean_title(title)?);
    let lid = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO pm_module_link (id, module_id, url, title, created_by) VALUES (?, ?, ?, ?, ?)")
        .bind(&lid)
        .bind(id)
        .bind(&url)
        .bind(&title)
        .bind(actor)
        .execute(pool)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(d) if d.is_unique_violation() => invalid("That link is on this module already"),
            _ => e.into(),
        })?;
    module_links(pool, id).await?.into_iter().find(|l| l.id == lid).ok_or_else(|| anyhow!("the link just made is gone"))
}

pub async fn remove_link(pool: &SqlitePool, link: &str) -> Result<()> {
    if let Some((module,)) = sqlx::query_as::<_, (String,)>("SELECT module_id FROM pm_module_link WHERE id = ?").bind(link).fetch_optional(pool).await? {
        writable(pool, &module).await?;
    }
    sqlx::query("DELETE FROM pm_module_link WHERE id = ?").bind(link).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::navigation;
    use crate::modules::pm::filters::PmRule;
    use crate::modules::pm::items::{ItemInput, ItemPatch};
    use crate::modules::pm::relations::PmRelationKind;
    use crate::modules::pm::{self, Features, NewPmProject, Network, cycles, settings};
    use chrono::Duration;

    #[test]
    fn a_module_lists_its_overdue_and_blocked_items_under_needs_attention() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-modules-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = |name: &str, id: &str| NewPmProject { name: name.into(), identifier: id.into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features { modules: true, ..Features::default() } };
            let p = pm::create(&pool, new("VIFC Fintech Hub", "VIFC")).await.unwrap();
            let other = pm::create(&pool, new("Sandbox", "SB")).await.unwrap();
            let state = |name: &'static str| {
                let pool = pool.clone();
                let project = p.id.clone();
                async move { settings::states(&pool, &project).await.unwrap().into_iter().find(|s| s.name == name).unwrap().id }
            };
            let today = cycles::today();
            let day = |n: i64| (today + Duration::days(n)).to_string();
            let input = |name: &str| ModuleInput { project_id: p.id.clone(), name: name.into(), description: String::new(), start_date: Some(day(-10)), target_date: Some(day(20)), status: ModuleStatus::InProgress, lead: Some("me".into()), members: vec!["me".into(), "me".into()] };

            let m = create(&pool, input("Sandbox licence"), "me").await.unwrap();
            assert_eq!((m.members.len(), m.lead.as_deref(), m.items), (1, Some("me"), 0));
            assert!(create(&pool, input("SANDBOX LICENCE"), "me").await.unwrap_err().to_string().contains("already"), "a name once a project, any case");
            assert!(create(&pool, ModuleInput { start_date: Some(day(5)), target_date: Some(day(1)), ..input("Backwards") }, "me").await.is_err());

            // Three items: one overdue, one blocked by an open item, one fine; the blocker isn't in it.
            let make = |name: &str, due: Option<String>| ItemInput { project_id: p.id.clone(), name: name.into(), target_date: due, ..ItemInput::default() };
            let late = items::create(&pool, make("Draft the rules", Some(day(-2))), "me").await.unwrap();
            let stuck = items::create(&pool, make("Pick the pilots", Some(day(9))), "me").await.unwrap();
            let fine = items::create(&pool, ItemInput { modules: vec![m.id.clone()], ..make("Book the room", None) }, "me").await.unwrap();
            let blocker = items::create(&pool, make("Legal review", None), "me").await.unwrap();
            relations::relate(&pool, &stuck.id, PmRelationKind::BlockedBy, &blocker.id, "me").await.unwrap();
            assert_eq!(fine.modules.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Sandbox licence"], "made in the module");
            assert_eq!(add_items(&pool, &m.id, &[late.id.clone(), stuck.id.clone(), fine.id.clone()], "me").await.unwrap().items, 3);
            let elsewhere = items::create(&pool, ItemInput { project_id: other.id.clone(), name: "Not here".into(), ..ItemInput::default() }, "me").await.unwrap();
            assert!(add_items(&pool, &m.id, &[elsewhere.id], "me").await.is_err(), "another project's item");

            let d = details(&pool, &m.id, today).await.unwrap();
            let flagged: Vec<(&str, bool, bool)> = d.attention.iter().map(|a| (a.item.name.as_str(), a.overdue, a.blocked)).collect();
            assert_eq!(flagged, [("Draft the rules", true, false), ("Pick the pilots", false, true)]);
            assert_eq!(d.groups.iter().map(|g| g.total).sum::<u32>(), 3);
            assert_eq!(d.priorities.iter().map(|g| g.total).sum::<u32>(), 3);

            // The blocker done: only the overdue item is left; the late one done: none.
            let done = state("Done").await;
            items::update(&pool, &blocker.id, ItemPatch { state_id: Some(done.clone()), ..ItemPatch::default() }, "me").await.unwrap();
            assert_eq!(details(&pool, &m.id, today).await.unwrap().attention.len(), 1);
            items::update(&pool, &late.id, ItemPatch { state_id: Some(done.clone()), ..ItemPatch::default() }, "me").await.unwrap();
            assert!(details(&pool, &m.id, today).await.unwrap().attention.is_empty());
            assert_eq!(get(&pool, &m.id).await.unwrap().done, 1);

            // An item in two modules; out of one from the item; the module filter; activity.
            let second = create(&pool, ModuleInput { status: ModuleStatus::Planned, ..input("Pilot cohort") }, "me").await.unwrap();
            let both = items::update(&pool, &stuck.id, ItemPatch { modules: Some(vec![m.id.clone(), second.id.clone()]), ..ItemPatch::default() }, "me").await.unwrap();
            assert_eq!(both.modules.len(), 2);
            let rule = |id: &str| PmRule { field: "module".into(), op: "in".into(), values: vec![id.into()] };
            assert_eq!(items::query(&pool, Some(&p.id), &[rule(&second.id)], today, crate::kernel::formats::WeekStart::Monday).await.unwrap().len(), 1);
            items::update(&pool, &stuck.id, ItemPatch { modules: Some(vec![second.id.clone()]), ..ItemPatch::default() }, "me").await.unwrap();
            assert_eq!(get(&pool, &m.id).await.unwrap().items, 2);
            let moved: Vec<(String, String)> = sqlx::query_as("SELECT old_value, new_value FROM pm_activity WHERE item_id = ? AND field = 'module' ORDER BY created_at, id").bind(&stuck.id).fetch_all(&pool).await.unwrap();
            assert_eq!(moved, [("".into(), "Sandbox licence".into()), ("".into(), "Pilot cohort".into()), ("Sandbox licence".into(), "".into())]);

            // Dates moved (the timeline's drag); a link; starred into the sidebar.
            let moved = update(&pool, &m.id, ModulePatch { start_date: Some(day(-3)), target_date: Some(day(30)), ..ModulePatch::default() }).await.unwrap();
            assert_eq!((moved.start_date, moved.target_date), (Some(day(-3)), Some(day(30))));
            assert!(update(&pool, &m.id, ModulePatch { name: Some("pilot COHORT".into()), ..ModulePatch::default() }).await.is_err());
            add_link(&pool, &m.id, "example.com/spec", "Spec", "me").await.unwrap();
            assert!(add_link(&pool, &m.id, &format!("{}://example.com/spec", "https"), "", "me").await.is_err(), "the same address once");
            assert!(favorite(&pool, &m.id, true).await.unwrap().is_favorite);
            let pins = navigation::favorites(&pool).await.unwrap().items;
            assert_eq!(pins.iter().map(|f| (f.object.as_str(), f.label.as_str())).collect::<Vec<_>>(), [("pm-modules", "Sandbox licence")]);

            // Archived only when completed or cancelled — which unstars it; read-only then.
            assert!(archive(&pool, &m.id, true).await.is_err());
            update(&pool, &m.id, ModulePatch { status: Some(ModuleStatus::Completed), ..ModulePatch::default() }).await.unwrap();
            let archived = archive(&pool, &m.id, true).await.unwrap();
            assert!(archived.archived_at.is_some() && !archived.is_favorite);
            assert!(update(&pool, &m.id, ModulePatch { name: Some("x".into()), ..ModulePatch::default() }).await.is_err());
            assert!(items::update(&pool, &blocker.id, ItemPatch { modules: Some(vec![m.id.clone()]), ..ItemPatch::default() }, "me").await.is_err(), "nothing joins an archived module");
            let kept = items::update(&pool, &late.id, ItemPatch { modules: Some(vec![m.id.clone(), second.id.clone()]), ..ItemPatch::default() }, "me").await.unwrap();
            assert_eq!(kept.modules.len(), 2, "an item stays in an archived module it was in");
            assert!(items::update(&pool, &late.id, ItemPatch { modules: Some(vec![second.id.clone()]), ..ItemPatch::default() }, "me").await.is_err(), "nor leaves it");
            let copy = items::copy(&pool, "", &late.id, None, "me").await.unwrap();
            assert_eq!(copy.modules.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Pilot cohort"], "a copy joins the open modules only");
            assert!(add_link(&pool, &m.id, "example.com/later", "", "me").await.is_err() && favorite(&pool, &m.id, true).await.is_err());

            // Deleted: its items stay, each told.
            delete(&pool, &second.id, "me").await.unwrap();
            assert!(items::get(&pool, &stuck.id).await.unwrap().modules.is_empty());
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
