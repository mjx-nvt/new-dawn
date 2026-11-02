//! Plane's views (D-077): saved filters with a layout and its display — a project's, or the
//! workspace's across its projects; a name, a description, private and locked flags (one owner:
//! labels, not rights). Saved from the Work items tab as it stands, opened back into it, updated,
//! duplicated ("Copy of …"), deleted, and pinned to the sidebar's Favorites. A locked view keeps
//! what it has until it is unlocked.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use super::display::{self, PmDisplay};
use super::filters::{self, PmRule};
use crate::error::AppError;
use crate::kernel::formats::WeekStart;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const NAME_MAX: usize = 255;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmView {
    pub id: String,
    /// None: a workspace view, across the projects.
    pub project_id: Option<String>,
    pub name: String,
    pub description: String,
    pub rules: Vec<PmRule>,
    pub display: PmDisplay,
    pub is_private: bool,
    pub is_locked: bool,
    /// In the sidebar's Favorites.
    pub favorite: bool,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: Option<String>,
    name: String,
    description: String,
    rules_json: String,
    layout: String,
    group_by: String,
    sub_group_by: String,
    order_by: String,
    properties_json: String,
    sub_items: bool,
    empty_groups: bool,
    calendar: String,
    zoom: String,
    is_private: bool,
    is_locked: bool,
    favorite: bool,
    sort_order: f64,
    created_at: String,
    updated_at: String,
}

impl From<Row> for PmView {
    fn from(r: Row) -> Self {
        PmView {
            id: r.id,
            project_id: r.project_id,
            name: r.name,
            description: r.description,
            rules: serde_json::from_str(&r.rules_json).unwrap_or_default(),
            display: PmDisplay {
                layout: r.layout,
                group: r.group_by,
                sub_group: r.sub_group_by,
                order: r.order_by,
                properties: serde_json::from_str(&r.properties_json).unwrap_or_default(),
                sub_items: r.sub_items,
                empty_groups: r.empty_groups,
                calendar: r.calendar,
                zoom: r.zoom,
                ..PmDisplay::default()
            },
            is_private: r.is_private,
            is_locked: r.is_locked,
            favorite: r.favorite,
            sort_order: r.sort_order,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const SELECT: &str = "SELECT v.id, v.project_id, v.name, v.description, v.rules_json, v.layout, v.group_by, v.sub_group_by, v.order_by, v.properties_json, \
     v.sub_items, v.empty_groups, v.calendar, v.zoom, v.is_private, v.is_locked, \
     EXISTS (SELECT 1 FROM favorite f WHERE f.pm_view_id = v.id AND f.deleted_at IS NULL) AS favorite, v.sort_order, v.created_at, v.updated_at FROM pm_view v";

/// A project's views, or the workspace's (none), in their order.
pub async fn list(pool: &SqlitePool, project_id: Option<&str>) -> Result<Vec<PmView>> {
    let rows: Vec<Row> = match project_id {
        Some(p) => sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE v.project_id = ? ORDER BY v.sort_order, v.created_at"))).bind(p).fetch_all(pool).await?,
        None => sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE v.project_id IS NULL ORDER BY v.sort_order, v.created_at"))).fetch_all(pool).await?,
    };
    Ok(rows.into_iter().map(PmView::from).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PmView> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE v.id = ?"))).bind(id).fetch_optional(pool).await?;
    row.map(PmView::from).ok_or_else(|| anyhow!(AppError::not_found("That view is gone")))
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(invalid("A view needs a name"));
    }
    if n.chars().count() > NAME_MAX {
        return Err(invalid(format!("A view's name is at most {NAME_MAX} characters")));
    }
    Ok(n.to_owned())
}

/// A description, trimmed, at most 2,000 characters.
fn clean_description(text: &str) -> String {
    text.trim().chars().take(2000).collect()
}

/// Rules checked by turning them into SQL (unknown fields, operators and days refused).
fn check_rules(rules: Vec<PmRule>) -> Result<Vec<PmRule>> {
    if rules.len() > 50 {
        return Err(invalid("A view has at most 50 filters"));
    }
    filters::sql(&rules, chrono::Local::now().date_naive(), WeekStart::Monday)?;
    Ok(rules)
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct PmViewInput {
    pub project_id: Option<String>,
    pub name: String,
    pub description: String,
    pub rules: Vec<PmRule>,
    pub display: PmDisplay,
    pub is_private: bool,
}

pub async fn create(pool: &SqlitePool, input: PmViewInput) -> Result<PmView> {
    let project = input.project_id.filter(|p| !p.is_empty());
    if let Some(p) = &project {
        super::get(pool, p).await?;
    }
    let (name, rules, d) = (clean_name(&input.name)?, check_rules(input.rules)?, display::check(input.display)?);
    let id = uuid::Uuid::now_v7().to_string();
    // At the end of the list.
    let last: (Option<f64>,) = sqlx::query_as("SELECT max(sort_order) FROM pm_view WHERE project_id IS ?").bind(&project).fetch_one(pool).await?;
    sqlx::query(
        "INSERT INTO pm_view (id, project_id, name, description, rules_json, layout, group_by, sub_group_by, order_by, properties_json, sub_items, empty_groups, calendar, zoom, is_private, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&project)
    .bind(&name)
    .bind(clean_description(&input.description))
    .bind(serde_json::to_string(&rules)?)
    .bind(&d.layout)
    .bind(&d.group)
    .bind(&d.sub_group)
    .bind(&d.order)
    .bind(serde_json::to_string(&d.properties)?)
    .bind(d.sub_items)
    .bind(d.empty_groups)
    .bind(&d.calendar)
    .bind(&d.zoom)
    .bind(input.is_private)
    .bind(last.0.unwrap_or(0.0) + 1000.0)
    .execute(pool)
    .await?;
    get(pool, &id).await
}

/// The edits; a field left out stays. A locked view takes only its unlocking.
#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct PmViewPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub rules: Option<Vec<PmRule>>,
    pub display: Option<PmDisplay>,
    pub is_private: Option<bool>,
    pub is_locked: Option<bool>,
    pub sort_order: Option<f64>,
}

pub async fn update(pool: &SqlitePool, id: &str, patch: PmViewPatch) -> Result<PmView> {
    let view = get(pool, id).await?;
    let changes_more = patch.name.is_some() || patch.description.is_some() || patch.rules.is_some() || patch.display.is_some() || patch.is_private.is_some();
    if view.is_locked && changes_more {
        return Err(invalid(format!("“{}” is locked — unlock it to change it", view.name)));
    }
    let name = patch.name.as_deref().map(clean_name).transpose()?;
    let rules = patch.rules.map(check_rules).transpose()?;
    let d = patch.display.map(display::check).transpose()?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_view SET name = coalesce(?, name), description = coalesce(?, description), rules_json = coalesce(?, rules_json), \
         layout = coalesce(?, layout), group_by = coalesce(?, group_by), sub_group_by = coalesce(?, sub_group_by), order_by = coalesce(?, order_by), \
         properties_json = coalesce(?, properties_json), sub_items = coalesce(?, sub_items), empty_groups = coalesce(?, empty_groups), \
         calendar = coalesce(?, calendar), zoom = coalesce(?, zoom), \
         is_private = coalesce(?, is_private), is_locked = coalesce(?, is_locked), sort_order = coalesce(?, sort_order), updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&name)
    .bind(patch.description.as_deref().map(clean_description))
    .bind(rules.as_ref().map(serde_json::to_string).transpose()?)
    .bind(d.as_ref().map(|d| d.layout.clone()))
    .bind(d.as_ref().map(|d| d.group.clone()))
    .bind(d.as_ref().map(|d| d.sub_group.clone()))
    .bind(d.as_ref().map(|d| d.order.clone()))
    .bind(d.as_ref().map(|d| serde_json::to_string(&d.properties)).transpose()?)
    .bind(d.as_ref().map(|d| d.sub_items))
    .bind(d.as_ref().map(|d| d.empty_groups))
    .bind(d.as_ref().map(|d| d.calendar.clone()))
    .bind(d.as_ref().map(|d| d.zoom.clone()))
    .bind(patch.is_private)
    .bind(patch.is_locked)
    .bind(patch.sort_order.filter(|o| o.is_finite()))
    .bind(id)
    .execute(pool)
    .await?;
    get(pool, id).await
}

/// "Copy of …", unlocked, at the end.
pub async fn duplicate(pool: &SqlitePool, id: &str) -> Result<PmView> {
    let v = get(pool, id).await?;
    let name: String = format!("Copy of {}", v.name).chars().take(NAME_MAX).collect();
    create(pool, PmViewInput { project_id: v.project_id, name, description: v.description, rules: v.rules, display: v.display, is_private: v.is_private }).await
}

/// Gone with its favorite (the key's cascade).
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    get(pool, id).await?;
    sqlx::query("DELETE FROM pm_view WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// Into the sidebar's Favorites, or out.
pub async fn favorite(pool: &SqlitePool, id: &str, on: bool) -> Result<PmView> {
    get(pool, id).await?;
    if on {
        let there: Option<(String,)> = sqlx::query_as("SELECT id FROM favorite WHERE pm_view_id = ? AND deleted_at IS NULL").bind(id).fetch_optional(pool).await?;
        if there.is_none() {
            let (position,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM favorite").fetch_one(pool).await?;
            sqlx::query("INSERT INTO favorite (id, position, workspace_member_id, pm_view_id) VALUES (?, ?, ?, ?)")
                .bind(uuid::Uuid::now_v7().to_string())
                .bind(position)
                .bind(crate::kernel::records::member_id(pool).await?)
                .bind(id)
                .execute(pool)
                .await?;
        }
    } else {
        sqlx::query("DELETE FROM favorite WHERE pm_view_id = ?").bind(id).execute(pool).await?;
    }
    get(pool, id).await
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::navigation;
    use crate::modules::pm::items::{self, ItemInput, ItemPatch};
    use crate::modules::pm::{self, Features, NewPmProject, Network, settings};

    fn rule(field: &str, op: &str, values: &[&str]) -> PmRule {
        PmRule { field: field.into(), op: op.into(), values: values.iter().map(|v| v.to_string()).collect() }
    }

    #[test]
    fn assigned_to_me_due_this_week_not_done_is_a_favorite_view() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-views-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = |name: &str, id: &str| NewPmProject { name: name.into(), identifier: id.into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features::default() };
            let p = pm::create(&pool, new("VIFC Fintech Hub", "VIFC")).await.unwrap();
            let q = pm::create(&pool, new("Sandbox", "SB")).await.unwrap();
            let done = settings::states(&pool, &p.id).await.unwrap().into_iter().find(|s| s.name == "Done").unwrap().id;
            let make = |project: &str, name: &str, due: &str, mine: bool| ItemInput { project_id: project.into(), name: name.into(), target_date: Some(due.into()), assignees: if mine { vec!["me".into()] } else { vec![] }, ..ItemInput::default() };
            // A Wednesday: this week (from Monday) is 2–8 November.
            let today = NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
            let a = items::create(&pool, make(&p.id, "Submit the application", "2026-11-06", true), "me").await.unwrap();
            items::create(&pool, make(&p.id, "Plan the launch", "2026-12-10", true), "me").await.unwrap();
            items::create(&pool, make(&p.id, "Brief the board", "2026-11-05", false), "me").await.unwrap();
            let d = items::create(&pool, make(&p.id, "Draft the rules", "2026-11-05", true), "me").await.unwrap();
            items::update(&pool, &d.id, ItemPatch { state_id: Some(done), ..ItemPatch::default() }, "me").await.unwrap();
            let b = items::create(&pool, make(&q.id, "Pick the first fintechs", "2026-11-02", true), "me").await.unwrap();
            let rules = vec![rule("assignees", "in", &["me"]), rule("due", "is", &["this_week"]), rule("state_group", "not_in", &["completed"])];
            let found = items::query(&pool, None, &rules, today, WeekStart::Monday).await.unwrap();
            let mut keys: Vec<&str> = found.iter().map(|i| i.key.as_str()).collect();
            keys.sort();
            assert_eq!(keys, ["SB-1", "VIFC-1"], "both projects, only what's mine, due this week and open");
            assert_eq!(items::query(&pool, Some(&p.id), &rules, today, WeekStart::Monday).await.unwrap().len(), 1, "one project");
            // A Sunday week (1–7 November) leaves the Monday-week edge as it is here; next week moves on.
            assert_eq!(items::query(&pool, None, &[rule("due", "is", &["next_week"])], today, WeekStart::Monday).await.unwrap().len(), 0);
            assert_eq!(items::query(&pool, None, &[rule("due", "between", &["2026-11-01", "2026-11-30"])], today, WeekStart::Monday).await.unwrap().len(), 4);
            assert_eq!(items::query(&pool, None, &[rule("assignees", "in", &["none"])], today, WeekStart::Monday).await.unwrap().len(), 1, "no assignee");
            assert_eq!(items::query(&pool, None, &[rule("title", "contains", &["the"])], today, WeekStart::Monday).await.unwrap().len(), 5);
            assert!(items::query(&pool, None, &[rule("colour", "in", &["red"])], today, WeekStart::Monday).await.is_err());
            assert!(items::query(&pool, None, &[rule("due", "between", &["this_week"])], today, WeekStart::Monday).await.is_err(), "between needs two days");

            // The workspace view, favorited: the sidebar lists it.
            let input = PmViewInput { project_id: None, name: "Mine this week".into(), description: "Assigned to me, due this week, not Done".into(), rules: rules.clone(), display: PmDisplay { layout: "table".into(), ..PmDisplay::default() }, is_private: false };
            let v = create(&pool, input).await.unwrap();
            assert_eq!((v.rules.len(), v.display.layout.as_str(), v.favorite), (3, "table", false));
            assert!(favorite(&pool, &v.id, true).await.unwrap().favorite);
            let pins = navigation::favorites(&pool).await.unwrap().items;
            assert_eq!(pins.iter().map(|f| (f.object.as_str(), f.label.as_str(), f.view_id.as_deref(), f.record_id.as_deref())).collect::<Vec<_>>(), [("pm-views", "Mine this week", Some(v.id.as_str()), None)]);
            assert_eq!(list(&pool, None).await.unwrap().len(), 1);
            assert!(list(&pool, Some(&p.id)).await.unwrap().is_empty());
            // Locked: only unlocking changes it; a copy is unlocked.
            update(&pool, &v.id, PmViewPatch { is_locked: Some(true), ..PmViewPatch::default() }).await.unwrap();
            assert!(update(&pool, &v.id, PmViewPatch { name: Some("x".into()), ..PmViewPatch::default() }).await.unwrap_err().to_string().contains("locked"));
            let copy = duplicate(&pool, &v.id).await.unwrap();
            assert_eq!((copy.name.as_str(), copy.is_locked, copy.favorite), ("Copy of Mine this week", false, false));
            assert!(create(&pool, PmViewInput { project_id: Some(p.id.clone()), name: "  ".into(), description: String::new(), rules: vec![], display: PmDisplay::default(), is_private: false }).await.is_err());
            // Merged people: the kept one, once.
            for (id, name) in [("m1", "Minh Le"), ("m2", "Minh L.")] {
                sqlx::query("INSERT INTO person (id, name, name_first) VALUES (?, ?, 'Minh')").bind(id).bind(name).execute(&pool).await.unwrap();
            }
            update(&pool, &copy.id, PmViewPatch { rules: Some(vec![rule("assignees", "in", &["m1", "m2"])]), ..PmViewPatch::default() }).await.unwrap();
            crate::modules::people::merge(&pool, "m2", "m1").await.unwrap();
            assert_eq!(get(&pool, &copy.id).await.unwrap().rules, vec![rule("assignees", "in", &["m2"])]);
            // A person deleted for good leaves the view's rules; the others stay as they were.
            sqlx::query("INSERT INTO person (id, name, name_first) VALUES ('p1', 'Lan Pham', 'Lan')").execute(&pool).await.unwrap();
            update(&pool, &copy.id, PmViewPatch { rules: Some(vec![rule("assignees", "in", &["p1", "me"]), rule("title", "contains", &["p1"])]), ..PmViewPatch::default() }).await.unwrap();
            sqlx::query("DELETE FROM person WHERE id = 'p1'").execute(&pool).await.unwrap();
            assert_eq!(get(&pool, &copy.id).await.unwrap().rules, vec![rule("assignees", "in", &["me"]), rule("title", "contains", &["p1"])]);
            // Deleted: its favorite goes with it.
            delete(&pool, &v.id).await.unwrap();
            assert!(navigation::favorites(&pool).await.unwrap().items.is_empty());
            let _ = (a, b);
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
