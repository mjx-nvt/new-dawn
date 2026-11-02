//! Plane's Home, Your work and navigation (D-082): the owner's stickies and quick links, the
//! Home's widgets in their order, the Projects section's items shown or hidden, what was opened
//! lately, the archives across projects, the owner's recent activity, and the Projects side of ⌘K.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use super::items;
use super::relations;
use crate::error::AppError;
use crate::kernel::records::types::Json;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
/// A sticky's colours, by name (the stylesheet's).
pub const STICKY_COLORS: [&str; 6] = ["yellow", "green", "blue", "purple", "red", "gray"];
/// The Home's widgets, in their default order.
pub const WIDGETS: [&str; 3] = ["quick_links", "recents", "stickies"];
/// The Projects section's items that can be hidden (the projects themselves always show).
pub const NAV_ITEMS: [&str; 8] = ["home", "your_work", "inbox", "drafts", "views", "analytics", "active_cycles", "archives"];
/// Recents kept.
const RECENTS_KEPT: u32 = 100;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}

// ── Stickies ──

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmSticky {
    pub id: String,
    pub description: Option<Json>,
    pub description_text: String,
    pub color: String,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct StickyRow {
    id: String,
    description_json: Option<String>,
    description_text: String,
    color: String,
    sort_order: f64,
    created_at: String,
    updated_at: String,
}

impl From<StickyRow> for PmSticky {
    fn from(r: StickyRow) -> Self {
        PmSticky { id: r.id, description: r.description_json.and_then(|j| serde_json::from_str(&j).ok()).map(Json), description_text: r.description_text, color: r.color, sort_order: r.sort_order, created_at: r.created_at, updated_at: r.updated_at }
    }
}

/// The stickies, newest first (Plane's).
pub async fn stickies(pool: &SqlitePool) -> Result<Vec<PmSticky>> {
    let rows: Vec<StickyRow> = sqlx::query_as("SELECT id, description_json, description_text, color, sort_order, created_at, updated_at FROM pm_sticky ORDER BY sort_order DESC, created_at DESC").fetch_all(pool).await?;
    Ok(rows.into_iter().map(PmSticky::from).collect())
}

fn check_color(color: &str) -> Result<&str> {
    STICKY_COLORS.iter().find(|c| **c == color).copied().ok_or_else(|| invalid("That isn't a sticky's colour"))
}

/// A new sticky, first; empty until written.
pub async fn add_sticky(pool: &SqlitePool, color: &str) -> Result<PmSticky> {
    let color = check_color(color)?;
    let sid = id();
    let (last,): (Option<f64>,) = sqlx::query_as("SELECT max(sort_order) FROM pm_sticky").fetch_one(pool).await?;
    sqlx::query("INSERT INTO pm_sticky (id, color, sort_order) VALUES (?, ?, ?)").bind(&sid).bind(color).bind(last.unwrap_or(0.0) + 1000.0).execute(pool).await?;
    sticky(pool, &sid).await
}

async fn sticky(pool: &SqlitePool, sid: &str) -> Result<PmSticky> {
    let row: Option<StickyRow> = sqlx::query_as("SELECT id, description_json, description_text, color, sort_order, created_at, updated_at FROM pm_sticky WHERE id = ?").bind(sid).fetch_optional(pool).await?;
    row.map(PmSticky::from).ok_or_else(|| anyhow!(AppError::not_found("That sticky is gone")))
}

/// Its words (the editor's JSON text) or its colour.
pub async fn update_sticky(pool: &SqlitePool, sid: &str, description: Option<&str>, color: Option<&str>) -> Result<PmSticky> {
    sticky(pool, sid).await?;
    let color = color.map(check_color).transpose()?;
    let doc = description.map(|d| items::clean_description(Some(d))).transpose()?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_sticky SET description_json = CASE WHEN ? THEN ? ELSE description_json END, description_text = CASE WHEN ? THEN ? ELSE description_text END, \
         color = coalesce(?, color), updated_at = {NOW} WHERE id = ?"
    )))
    .bind(doc.is_some())
    .bind(doc.as_ref().and_then(|(j, _)| j.clone()))
    .bind(doc.is_some())
    .bind(doc.as_ref().map(|(_, t)| t.clone()))
    .bind(color)
    .bind(sid)
    .execute(pool)
    .await?;
    sticky(pool, sid).await
}

pub async fn delete_sticky(pool: &SqlitePool, sid: &str) -> Result<()> {
    sqlx::query("DELETE FROM pm_sticky WHERE id = ?").bind(sid).execute(pool).await?;
    Ok(())
}

// ── Quick links ──

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmQuickLink {
    pub id: String,
    pub title: String,
    pub url: String,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn quick_links(pool: &SqlitePool) -> Result<Vec<PmQuickLink>> {
    Ok(sqlx::query_as("SELECT id, title, url, sort_order, created_at, updated_at FROM pm_quick_link ORDER BY sort_order, created_at").fetch_all(pool).await?)
}

fn taken(e: sqlx::Error) -> anyhow::Error {
    match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => invalid("That address is a quick link already"),
        _ => e.into(),
    }
}

pub async fn add_quick_link(pool: &SqlitePool, title: &str, url: &str) -> Result<PmQuickLink> {
    let (url, title) = (relations::clean_url(url)?, relations::clean_title(title)?);
    let lid = id();
    let (last,): (Option<f64>,) = sqlx::query_as("SELECT max(sort_order) FROM pm_quick_link").fetch_one(pool).await?;
    sqlx::query("INSERT INTO pm_quick_link (id, title, url, sort_order) VALUES (?, ?, ?, ?)").bind(&lid).bind(&title).bind(&url).bind(last.unwrap_or(0.0) + 1000.0).execute(pool).await.map_err(taken)?;
    quick_links(pool).await?.into_iter().find(|l| l.id == lid).ok_or_else(|| anyhow!("the link just made is gone"))
}

pub async fn delete_quick_link(pool: &SqlitePool, lid: &str) -> Result<()> {
    sqlx::query("DELETE FROM pm_quick_link WHERE id = ?").bind(lid).execute(pool).await?;
    Ok(())
}

// ── Widgets and navigation ──

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct HomeWidget {
    pub key: String,
    pub is_enabled: bool,
}

/// The Home's widgets in their order — each one, on or off (none set: all on, Plane's order).
pub async fn widgets(pool: &SqlitePool) -> Result<Vec<HomeWidget>> {
    let rows: Vec<(String, bool)> = sqlx::query_as("SELECT key, is_enabled FROM pm_home_widget ORDER BY position, key").fetch_all(pool).await?;
    let mut out: Vec<HomeWidget> = rows.into_iter().filter(|(k, _)| WIDGETS.contains(&k.as_str())).map(|(key, is_enabled)| HomeWidget { key, is_enabled }).collect();
    for w in WIDGETS {
        if !out.iter().any(|x| x.key == w) {
            out.push(HomeWidget { key: w.into(), is_enabled: true });
        }
    }
    Ok(out)
}

/// The widgets as given: their order and whether each shows.
pub async fn set_widgets(pool: &SqlitePool, widgets: &[HomeWidget]) -> Result<Vec<HomeWidget>> {
    if widgets.iter().any(|w| !WIDGETS.contains(&w.key.as_str())) {
        return Err(invalid("That isn't a widget of the Home"));
    }
    let mut tx = pool.begin().await?;
    for (n, w) in widgets.iter().enumerate() {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO pm_home_widget (key, is_enabled, position) VALUES (?, ?, ?) ON CONFLICT (key) DO UPDATE SET is_enabled = excluded.is_enabled, position = excluded.position, updated_at = {NOW}"
        )))
        .bind(&w.key)
        .bind(w.is_enabled)
        .bind(n as f64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    self::widgets(pool).await
}

/// The Projects section's items that are hidden (Customize navigation).
pub async fn hidden_navigation(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT key FROM pm_navigation WHERE is_hidden = 1").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(k,)| k).filter(|k| NAV_ITEMS.contains(&k.as_str())).collect())
}

pub async fn set_navigation(pool: &SqlitePool, key: &str, hidden: bool) -> Result<Vec<String>> {
    if !NAV_ITEMS.contains(&key) {
        return Err(invalid("That isn't an item of the Projects section"));
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO pm_navigation (key, is_hidden) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET is_hidden = excluded.is_hidden, updated_at = {NOW}")))
        .bind(key)
        .bind(hidden)
        .execute(pool)
        .await?;
    hidden_navigation(pool).await
}

// ── Recents ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum RecentKind {
    Item,
    Page,
    Project,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmRecent {
    pub kind: RecentKind,
    pub ref_id: String,
    pub project_id: String,
    /// "VIFC-12 Draft the rules", a page's name, a project's.
    pub label: String,
    pub project_name: String,
    pub visited_at: String,
}

/// Opened now (an item, a page or a project): first in Recents. What's gone is skipped.
pub async fn visit(pool: &SqlitePool, kind: RecentKind, ref_id: &str) -> Result<()> {
    let project: Option<(String,)> = match kind {
        RecentKind::Item => sqlx::query_as("SELECT project_id FROM pm_item WHERE id = ?").bind(ref_id).fetch_optional(pool).await?,
        RecentKind::Page => sqlx::query_as("SELECT project_id FROM pm_page WHERE id = ?").bind(ref_id).fetch_optional(pool).await?,
        RecentKind::Project => sqlx::query_as("SELECT project_id FROM pm_project WHERE project_id = ?").bind(ref_id).fetch_optional(pool).await?,
    };
    let Some((project_id,)) = project else { return Ok(()) };
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO pm_recent (kind, ref_id, project_id) VALUES (?, ?, ?) ON CONFLICT (kind, ref_id) DO UPDATE SET visited_at = {NOW}, updated_at = {NOW}"
    )))
    .bind(kind)
    .bind(ref_id)
    .bind(&project_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM pm_recent WHERE rowid NOT IN (SELECT rowid FROM pm_recent ORDER BY visited_at DESC LIMIT ?)").bind(RECENTS_KEPT).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// What was opened lately, newest first: all, or one kind. Archived and Intake's requests are
/// left out, and an archived project's.
pub async fn recents(pool: &SqlitePool, kind: Option<RecentKind>, limit: u32) -> Result<Vec<PmRecent>> {
    Ok(sqlx::query_as(
        "SELECT r.kind, r.ref_id, r.project_id, \
         CASE r.kind WHEN 'item' THEN m.identifier || '-' || i.sequence || ' ' || i.name WHEN 'page' THEN CASE WHEN g.name = '' THEN 'Untitled' ELSE g.name END ELSE p.name END AS label, \
         p.name AS project_name, r.visited_at \
         FROM pm_recent r JOIN pm_project m ON m.project_id = r.project_id JOIN project p ON p.id = r.project_id \
         LEFT JOIN pm_item i ON r.kind = 'item' AND i.id = r.ref_id LEFT JOIN pm_state s ON s.id = i.state_id \
         LEFT JOIN pm_page g ON r.kind = 'page' AND g.id = r.ref_id \
         WHERE m.archived_at IS NULL AND (? IS NULL OR r.kind = ?) \
         AND (r.kind <> 'item' OR (i.id IS NOT NULL AND i.archived_at IS NULL AND s.state_group <> 'triage')) \
         AND (r.kind <> 'page' OR (g.id IS NOT NULL AND g.archived_at IS NULL AND m.pages = 1)) \
         ORDER BY r.visited_at DESC LIMIT ?",
    )
    .bind(kind)
    .bind(kind)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

// ── Archives ──

/// What's archived across the projects (Plane's Archives): projects, and the work items, cycles,
/// modules and pages of the open ones.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PmArchives {
    pub projects: Vec<PmArchived>,
    pub items: Vec<PmArchived>,
    pub cycles: Vec<PmArchived>,
    pub modules: Vec<PmArchived>,
    pub pages: Vec<PmArchived>,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmArchived {
    pub id: String,
    pub project_id: String,
    pub label: String,
    pub project_name: String,
    pub archived_at: String,
}

pub async fn archives(pool: &SqlitePool) -> Result<PmArchives> {
    let read = |sql: &'static str| async move { Ok::<_, anyhow::Error>(sqlx::query_as::<_, PmArchived>(sql).fetch_all(pool).await?) };
    Ok(PmArchives {
        projects: read("SELECT m.project_id AS id, m.project_id, p.name AS label, p.name AS project_name, m.archived_at FROM pm_project m JOIN project p ON p.id = m.project_id WHERE m.archived_at IS NOT NULL ORDER BY m.archived_at DESC").await?,
        items: read(
            "SELECT i.id, i.project_id, m.identifier || '-' || i.sequence || ' ' || i.name AS label, p.name AS project_name, i.archived_at FROM pm_item i \
             JOIN pm_project m ON m.project_id = i.project_id JOIN project p ON p.id = i.project_id WHERE i.archived_at IS NOT NULL AND m.archived_at IS NULL ORDER BY i.archived_at DESC LIMIT 500",
        )
        .await?,
        cycles: read("SELECT c.id, c.project_id, c.name AS label, p.name AS project_name, c.archived_at FROM pm_cycle c JOIN pm_project m ON m.project_id = c.project_id JOIN project p ON p.id = c.project_id WHERE c.archived_at IS NOT NULL AND m.archived_at IS NULL AND m.cycles = 1 ORDER BY c.archived_at DESC").await?,
        modules: read("SELECT d.id, d.project_id, d.name AS label, p.name AS project_name, d.archived_at FROM pm_module d JOIN pm_project m ON m.project_id = d.project_id JOIN project p ON p.id = d.project_id WHERE d.archived_at IS NOT NULL AND m.archived_at IS NULL AND m.modules = 1 ORDER BY d.archived_at DESC").await?,
        pages: read(
            "SELECT g.id, g.project_id, CASE WHEN g.name = '' THEN 'Untitled' ELSE g.name END AS label, p.name AS project_name, g.archived_at FROM pm_page g \
             JOIN pm_project m ON m.project_id = g.project_id JOIN project p ON p.id = g.project_id \
             WHERE g.archived_at IS NOT NULL AND m.archived_at IS NULL AND m.pages = 1 AND (g.parent_id IS NULL OR g.parent_id NOT IN (SELECT id FROM pm_page WHERE archived_at IS NOT NULL)) ORDER BY g.archived_at DESC",
        )
        .await?,
    })
}

// ── Your work: the owner's recent activity ──

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct MyActivity {
    pub id: String,
    pub item_id: String,
    pub project_id: String,
    pub key: String,
    pub name: String,
    pub kind: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub actor: String,
    pub created_at: String,
}

/// What the owner did lately on work items of open projects, newest first.
pub async fn my_activity(pool: &SqlitePool, limit: u32) -> Result<Vec<MyActivity>> {
    Ok(sqlx::query_as(
        "SELECT a.id, a.item_id, i.project_id, m.identifier || '-' || i.sequence AS key, i.name, a.kind, a.field, a.old_value, a.new_value, a.actor, a.created_at \
         FROM pm_activity a JOIN pm_item i ON i.id = a.item_id JOIN pm_project m ON m.project_id = i.project_id JOIN pm_state s ON s.id = i.state_id \
         WHERE a.actor = 'me' AND m.archived_at IS NULL AND s.state_group <> 'triage' ORDER BY a.created_at DESC, a.id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

// ── ⌘K ──

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmHit {
    /// "project", "cycle", "module" or "page".
    pub kind: String,
    pub id: String,
    pub project_id: String,
    pub label: String,
    pub project_name: String,
}

/// Projects, cycles, modules and pages whose name has the words (work items by key and title
/// are the relation search's), a few of each, open projects only, nothing archived.
pub async fn search(pool: &SqlitePool, query: &str) -> Result<Vec<PmHit>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    Ok(sqlx::query_as(
        "SELECT * FROM (SELECT 'project' AS kind, m.project_id AS id, m.project_id, p.name AS label, p.name AS project_name FROM pm_project m JOIN project p ON p.id = m.project_id \
           WHERE m.archived_at IS NULL AND (p.name LIKE ?1 ESCAPE '\\' OR m.identifier LIKE ?1 ESCAPE '\\') ORDER BY m.updated_at DESC LIMIT 5) \
         UNION ALL SELECT * FROM (SELECT 'cycle', c.id, c.project_id, c.name, p.name FROM pm_cycle c JOIN pm_project m ON m.project_id = c.project_id JOIN project p ON p.id = c.project_id \
           WHERE m.archived_at IS NULL AND c.archived_at IS NULL AND m.cycles = 1 AND c.name LIKE ?1 ESCAPE '\\' ORDER BY c.updated_at DESC LIMIT 5) \
         UNION ALL SELECT * FROM (SELECT 'module', d.id, d.project_id, d.name, p.name FROM pm_module d JOIN pm_project m ON m.project_id = d.project_id JOIN project p ON p.id = d.project_id \
           WHERE m.archived_at IS NULL AND d.archived_at IS NULL AND m.modules = 1 AND d.name LIKE ?1 ESCAPE '\\' ORDER BY d.updated_at DESC LIMIT 5) \
         UNION ALL SELECT * FROM (SELECT 'page', g.id, g.project_id, CASE WHEN g.name = '' THEN 'Untitled' ELSE g.name END, p.name FROM pm_page g JOIN pm_project m ON m.project_id = g.project_id JOIN project p ON p.id = g.project_id \
           WHERE m.archived_at IS NULL AND g.archived_at IS NULL AND m.pages = 1 AND g.name LIKE ?1 ESCAPE '\\' ORDER BY g.updated_at DESC LIMIT 5)",
    )
    .bind(&like)
    .fetch_all(pool)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::items::ItemInput;
    use crate::modules::pm::{self, Features, NewPmProject, Network};

    #[test]
    fn the_home_keeps_its_stickies_links_widgets_and_recents() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-home-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let p = pm::create(&pool, NewPmProject { name: "VIFC Fintech Hub".into(), identifier: "VIFC".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features { cycles: true, pages: true, ..Features::default() } })
                .await
                .unwrap();

            // Stickies: newest first, a colour of the list, words kept.
            let first = add_sticky(&pool, "yellow").await.unwrap();
            let second = add_sticky(&pool, "blue").await.unwrap();
            assert!(add_sticky(&pool, "teal").await.is_err());
            let doc = serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Call Minh" }] }] }).to_string();
            assert_eq!(update_sticky(&pool, &first.id, Some(&doc), Some("green")).await.unwrap().description_text, "Call Minh");
            assert_eq!(stickies(&pool).await.unwrap().iter().map(|s| s.id.clone()).collect::<Vec<_>>(), [second.id.clone(), first.id.clone()]);
            delete_sticky(&pool, &second.id).await.unwrap();

            // Quick links: an address once.
            add_quick_link(&pool, "Sandbox rules", "vifc.gov.vn/sandbox").await.unwrap();
            assert!(add_quick_link(&pool, "", &format!("{}://vifc.gov.vn/sandbox", "https")).await.is_err());

            // Widgets: all on by default; reordered and one off.
            assert_eq!(widgets(&pool).await.unwrap().iter().map(|w| w.key.as_str()).collect::<Vec<_>>(), WIDGETS);
            let set = set_widgets(&pool, &[HomeWidget { key: "stickies".into(), is_enabled: true }, HomeWidget { key: "recents".into(), is_enabled: false }, HomeWidget { key: "quick_links".into(), is_enabled: true }]).await.unwrap();
            assert_eq!((set[0].key.as_str(), set[1].is_enabled), ("stickies", false));
            assert_eq!(set_navigation(&pool, "drafts", true).await.unwrap(), ["drafts"]);
            assert!(set_navigation(&pool, "people", true).await.is_err());

            // Recents: newest first; an archived item drops out; ⌘K finds a cycle.
            let a = items::create(&pool, ItemInput { project_id: p.id.clone(), name: "Draft the rules".into(), ..ItemInput::default() }, "me").await.unwrap();
            visit(&pool, RecentKind::Item, &a.id).await.unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3));
            visit(&pool, RecentKind::Project, &p.id).await.unwrap();
            let seen = recents(&pool, None, 20).await.unwrap();
            assert_eq!(seen.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(), ["VIFC Fintech Hub", "VIFC-1 Draft the rules"]);
            assert_eq!(recents(&pool, Some(RecentKind::Item), 20).await.unwrap().len(), 1);
            super::super::cycles::create(&pool, super::super::cycles::CycleInput { project_id: p.id.clone(), name: "Sprint 1".into(), description: String::new(), start_date: None, end_date: None }, "me", super::super::cycles::today()).await.unwrap();
            assert_eq!(search(&pool, "sprint").await.unwrap().iter().map(|h| h.kind.as_str()).collect::<Vec<_>>(), ["cycle"]);
            assert_eq!(my_activity(&pool, 20).await.unwrap().len(), 1, "made it");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
