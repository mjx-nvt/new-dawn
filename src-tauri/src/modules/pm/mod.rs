//! Plane's projects (D-073, plan v4 §4.2): beside each of New Dawn's projects (`project`, the
//! dossier of D-054) its Plane side — identifier, description, logo, cover, network, lead and
//! defaults, features, automations — with its states, labels and estimates (`settings`). A
//! project made before 0.11.0, or through the dossier's own path, gets its Plane side the first
//! time the module reads the workspace.

pub mod comments;
pub mod drafts;
pub mod items;
pub mod settings;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::modules::projects::{self, ProjectStatus};

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
pub const IDENTIFIER_MAX: usize = 10;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Network {
    Private,
    Public,
}

/// Cycles, Modules, Views, Pages and Intake, each on or off for a project.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Features {
    pub cycles: bool,
    pub modules: bool,
    pub views: bool,
    pub pages: bool,
    pub intake: bool,
}

/// Plane's automations: archive what's closed after N months, close what's idle after N months
/// into a cancelled state, remind of due dates. 0 months = off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Automations {
    pub archive_in: u32,
    pub close_in: u32,
    pub close_state_id: Option<String>,
    pub due_reminders: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmProject {
    /// The project's id (its dossier's too).
    pub id: String,
    pub name: String,
    /// The dossier's status (D-054).
    pub status: ProjectStatus,
    pub identifier: String,
    pub description: String,
    pub emoji: String,
    pub cover: String,
    pub network: Network,
    /// "me" (the owner) or a person's id.
    pub lead: Option<String>,
    pub default_assignee: Option<String>,
    pub subscribers: Vec<String>,
    /// An IANA zone; empty: the owner's (Formats).
    pub time_zone: String,
    pub features: Features,
    pub automations: Automations,
    pub estimate_id: Option<String>,
    pub favorite: bool,
    pub sort_order: f64,
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    name: String,
    status: ProjectStatus,
    identifier: String,
    description: String,
    emoji: String,
    cover: String,
    network: Network,
    lead: Option<String>,
    default_assignee: Option<String>,
    subscribers_json: String,
    time_zone: String,
    cycles: bool,
    modules: bool,
    views: bool,
    pages: bool,
    intake: bool,
    archive_in: u32,
    close_in: u32,
    close_state_id: Option<String>,
    due_reminders: bool,
    estimate_id: Option<String>,
    favorite: bool,
    sort_order: f64,
    archived_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl From<Row> for PmProject {
    fn from(r: Row) -> Self {
        PmProject {
            id: r.id,
            name: r.name,
            status: r.status,
            identifier: r.identifier,
            description: r.description,
            emoji: r.emoji,
            cover: r.cover,
            network: r.network,
            lead: r.lead,
            default_assignee: r.default_assignee,
            subscribers: serde_json::from_str(&r.subscribers_json).unwrap_or_default(),
            time_zone: r.time_zone,
            features: Features { cycles: r.cycles, modules: r.modules, views: r.views, pages: r.pages, intake: r.intake },
            automations: Automations { archive_in: r.archive_in, close_in: r.close_in, close_state_id: r.close_state_id, due_reminders: r.due_reminders },
            estimate_id: r.estimate_id,
            favorite: r.favorite,
            sort_order: r.sort_order,
            archived_at: r.archived_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

macro_rules! select {
    ($rest:literal) => {
        concat!(
            "SELECT p.id, p.name, p.status, m.identifier, m.description, m.emoji, m.cover, m.network, m.lead, m.default_assignee, \
             m.subscribers_json, m.time_zone, m.cycles, m.modules, m.views, m.pages, m.intake, m.archive_in, m.close_in, \
             m.close_state_id, m.due_reminders, m.estimate_id, m.favorite, m.sort_order, m.archived_at, m.created_at, m.updated_at \
             FROM pm_project m JOIN project p ON p.id = m.project_id ",
            $rest
        )
    };
}

/// "WiredScore certification" → "WC"; "Marina" → "MARIN": the initials of two words or more,
/// else the first five letters or digits; upper case, at most ten.
pub fn identifier_from(name: &str) -> String {
    let words: Vec<String> = crate::kernel::text::fold(name)
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_uppercase)
        .collect();
    let id: String = match words.len() {
        0 => "P".to_owned(),
        1 => words[0].chars().take(5).collect(),
        _ => words.iter().filter_map(|w| w.chars().next()).take(IDENTIFIER_MAX).collect(),
    };
    id
}

/// An identifier as kept: upper case, letters and digits, 1 to 10 of them.
fn clean_identifier(text: &str) -> Result<String> {
    let id = text.trim().to_uppercase();
    if id.is_empty() || id.chars().count() > IDENTIFIER_MAX || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(invalid(format!("An identifier is 1 to {IDENTIFIER_MAX} letters or digits, like VIFC")));
    }
    Ok(id)
}

fn taken(identifier: &str) -> anyhow::Error {
    invalid(format!("Another project uses {identifier} — identifiers are unique"))
}

/// A write that met the identifier's unique index (a race): said as a taken identifier.
fn or_taken(e: sqlx::Error, identifier: &str) -> anyhow::Error {
    if e.as_database_error().is_some_and(|d| d.is_unique_violation()) { taken(identifier) } else { e.into() }
}

async fn identifier_taken(pool: &SqlitePool, id: &str, except: Option<&str>) -> Result<bool> {
    let row: Option<(String,)> = sqlx::query_as("SELECT project_id FROM pm_project WHERE upper(identifier) = ? AND project_id IS NOT ?").bind(id).bind(except).fetch_optional(pool).await?;
    Ok(row.is_some())
}

/// "me" (the owner — the CRM's workspace member, the people memory's "owner") or a live
/// person's id; empty clears.
pub(crate) async fn check_ref(pool: &SqlitePool, who: Option<&str>) -> Result<Option<String>> {
    let Some(who) = who.map(str::trim).filter(|w| !w.is_empty()) else { return Ok(None) };
    if who == "me" {
        return Ok(Some(who.to_owned()));
    }
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM person WHERE id = ? AND deleted_at IS NULL").bind(who).fetch_optional(pool).await?;
    row.map(|(id,)| Some(id)).ok_or_else(|| invalid("That person isn't in the CRM"))
}

/// Gives every project that has no Plane side yet its own: an identifier from its name (made
/// unique with a number) and the default states.
pub async fn ensure(pool: &SqlitePool) -> Result<()> {
    let missing: Vec<(String, String)> = sqlx::query_as("SELECT id, name FROM project WHERE id NOT IN (SELECT project_id FROM pm_project) ORDER BY created_at").fetch_all(pool).await?;
    for (id, name) in missing {
        let base = identifier_from(&name);
        let mut candidate = base.clone();
        let mut n = 1;
        while identifier_taken(pool, &candidate, None).await? {
            n += 1;
            let digits = n.to_string();
            candidate = format!("{}{digits}", base.chars().take(IDENTIFIER_MAX - digits.len()).collect::<String>());
        }
        let mut tx = pool.begin().await?;
        // ON CONFLICT on the project only: an identifier taken meanwhile is an error, not a skip.
        let made = sqlx::query("INSERT INTO pm_project (project_id, identifier) VALUES (?, ?) ON CONFLICT (project_id) DO NOTHING").bind(&id).bind(&candidate).execute(&mut *tx).await?;
        if made.rows_affected() == 1 {
            settings::add_default_states(&mut tx, &id).await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

/// Every project with its Plane side (archived ones too: the list filters), favorites first,
/// then by its order and name.
pub async fn list(pool: &SqlitePool) -> Result<Vec<PmProject>> {
    ensure(pool).await?;
    let rows: Vec<Row> = sqlx::query_as(select!("ORDER BY m.favorite DESC, m.sort_order, p.name COLLATE NOCASE")).fetch_all(pool).await?;
    Ok(rows.into_iter().map(PmProject::from).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PmProject> {
    ensure(pool).await?;
    let row: Option<Row> = sqlx::query_as(select!("WHERE p.id = ?")).bind(id).fetch_optional(pool).await?;
    row.map(PmProject::from).ok_or_else(|| anyhow!(AppError::not_found("No such project")))
}

/// Plane's create form: the project, its Plane side and the default states.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct NewPmProject {
    pub name: String,
    pub identifier: String,
    pub description: String,
    pub emoji: String,
    pub cover: String,
    pub network: Network,
    pub lead: Option<String>,
    pub features: Features,
}

pub async fn create(pool: &SqlitePool, new: NewPmProject) -> Result<PmProject> {
    let identifier = clean_identifier(&new.identifier)?;
    if identifier_taken(pool, &identifier, None).await? {
        return Err(taken(&identifier));
    }
    let lead = check_ref(pool, new.lead.as_deref()).await?;
    let name = projects::clean_name(&new.name)?;
    projects::check_name(pool, &name, None).await?;
    // The project, its Plane side and its states: one transaction.
    let project_id = uuid::Uuid::now_v7().to_string();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO project (id, name) VALUES (?, ?)").bind(&project_id).bind(&name).execute(&mut *tx).await?;
    let f = new.features;
    sqlx::query(
            "INSERT INTO pm_project (project_id, identifier, description, emoji, cover, network, lead, cycles, modules, views, pages, intake, sort_order) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, (SELECT coalesce(max(sort_order), 0) + 1000 FROM pm_project))",
        )
    .bind(&project_id)
    .bind(&identifier)
    .bind(new.description.trim().chars().take(10_000).collect::<String>())
    .bind(new.emoji.trim().chars().take(8).collect::<String>())
    .bind(new.cover.trim().chars().take(200).collect::<String>())
    .bind(new.network)
    .bind(&lead)
    .bind(f.cycles)
    .bind(f.modules)
    .bind(f.views)
    .bind(f.pages)
    .bind(f.intake)
    .execute(&mut *tx)
    .await
    .map_err(|e| or_taken(e, &identifier))?;
    settings::add_default_states(&mut tx, &project_id).await?;
    tx.commit().await?;
    get(pool, &project_id).await
}

/// The settings' edits: a field left out stays; an empty lead or assignee clears it.
#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct PmProjectPatch {
    pub name: Option<String>,
    pub identifier: Option<String>,
    pub description: Option<String>,
    pub emoji: Option<String>,
    pub cover: Option<String>,
    pub network: Option<Network>,
    pub lead: Option<String>,
    pub default_assignee: Option<String>,
    pub subscribers: Option<Vec<String>>,
    pub time_zone: Option<String>,
    pub features: Option<Features>,
    pub automations: Option<Automations>,
    pub favorite: Option<bool>,
    pub sort_order: Option<f64>,
}

/// "me" or a person kept as it is (even one now in Deleted records); a new one checked.
async fn keep_or_check(pool: &SqlitePool, given: &str, now: Option<&str>) -> Result<Option<String>> {
    let given = given.trim();
    if !given.is_empty() && Some(given) == now {
        return Ok(Some(given.to_owned()));
    }
    check_ref(pool, Some(given)).await
}

/// The settings' edits. Everything is checked first; then one transaction writes only what was
/// given, so a merge or a state deleted meanwhile is never written back.
pub async fn update(pool: &SqlitePool, id: &str, patch: PmProjectPatch) -> Result<PmProject> {
    let current = get(pool, id).await?;
    let name = match patch.name.as_deref().map(str::trim).filter(|n| *n != current.name) {
        Some(n) => {
            let n = projects::clean_name(n)?;
            projects::check_name(pool, &n, Some(id)).await?;
            Some(n)
        }
        None => None,
    };
    let identifier = match patch.identifier.as_deref() {
        Some(i) => {
            let i = clean_identifier(i)?;
            if identifier_taken(pool, &i, Some(id)).await? {
                return Err(taken(&i));
            }
            Some(i)
        }
        None => None,
    };
    let lead = match patch.lead.as_deref() {
        Some(l) => Some(keep_or_check(pool, l, current.lead.as_deref()).await?),
        None => None,
    };
    let assignee = match patch.default_assignee.as_deref() {
        Some(a) => Some(keep_or_check(pool, a, current.default_assignee.as_deref()).await?),
        None => None,
    };
    let subscribers = match patch.subscribers {
        Some(subs) => {
            let mut kept: Vec<String> = Vec::new();
            for s in subs {
                if let Some(r) = keep_or_check(pool, &s, current.subscribers.iter().find(|c| **c == s.trim()).map(String::as_str)).await? {
                    if !kept.contains(&r) {
                        kept.push(r);
                    }
                }
            }
            Some(serde_json::to_string(&kept)?)
        }
        None => None,
    };
    let time_zone = match patch.time_zone {
        Some(z) => {
            let z = z.trim().to_owned();
            if z.len() > 64 || z.starts_with('/') || !z.chars().all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c)) {
                return Err(invalid("That isn't a time zone"));
            }
            Some(z)
        }
        None => None,
    };
    if let Some(a) = &patch.automations {
        if a.archive_in > 12 || a.close_in > 12 {
            return Err(invalid("Automations count up to 12 months"));
        }
        if let Some(state) = &a.close_state_id {
            let group: Option<(String,)> = sqlx::query_as("SELECT state_group FROM pm_state WHERE id = ? AND project_id = ?").bind(state).bind(id).fetch_optional(pool).await?;
            if group.map(|(g,)| g) != Some("cancelled".into()) {
                return Err(invalid("Items close into one of the project's cancelled states"));
            }
        }
    }
    let f = patch.features;
    let a = patch.automations;
    let mut tx = pool.begin().await?;
    if let Some(n) = &name {
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE project SET name = ?, updated_at = {NOW} WHERE id = ?"))).bind(n).bind(id).execute(&mut *tx).await?;
    }
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_project SET identifier = coalesce(?, identifier), description = coalesce(?, description), emoji = coalesce(?, emoji), \
         cover = coalesce(?, cover), network = coalesce(?, network), \
         lead = CASE WHEN ? THEN ? ELSE lead END, default_assignee = CASE WHEN ? THEN ? ELSE default_assignee END, \
         subscribers_json = coalesce(?, subscribers_json), time_zone = coalesce(?, time_zone), \
         cycles = coalesce(?, cycles), modules = coalesce(?, modules), views = coalesce(?, views), pages = coalesce(?, pages), intake = coalesce(?, intake), \
         archive_in = coalesce(?, archive_in), close_in = coalesce(?, close_in), close_state_id = CASE WHEN ? THEN ? ELSE close_state_id END, \
         due_reminders = coalesce(?, due_reminders), favorite = coalesce(?, favorite), sort_order = coalesce(?, sort_order), updated_at = {NOW} \
         WHERE project_id = ?"
    )))
    .bind(&identifier)
    .bind(patch.description.map(|d| d.trim().chars().take(10_000).collect::<String>()))
    .bind(patch.emoji.map(|e| e.trim().chars().take(8).collect::<String>()))
    .bind(patch.cover.map(|c| c.trim().chars().take(200).collect::<String>()))
    .bind(patch.network)
    .bind(lead.is_some())
    .bind(lead.flatten())
    .bind(assignee.is_some())
    .bind(assignee.flatten())
    .bind(subscribers)
    .bind(time_zone)
    .bind(f.map(|f| f.cycles))
    .bind(f.map(|f| f.modules))
    .bind(f.map(|f| f.views))
    .bind(f.map(|f| f.pages))
    .bind(f.map(|f| f.intake))
    .bind(a.as_ref().map(|a| a.archive_in))
    .bind(a.as_ref().map(|a| a.close_in))
    .bind(a.is_some())
    .bind(a.as_ref().and_then(|a| a.close_state_id.clone()))
    .bind(a.as_ref().map(|a| a.due_reminders))
    .bind(patch.favorite)
    .bind(patch.sort_order.filter(|o| o.is_finite()))
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| or_taken(e, identifier.as_deref().unwrap_or("that identifier")))?;
    tx.commit().await?;
    get(pool, id).await
}

/// Deletes a project with everything in it — its dossier, its Plane side, its items' files.
pub async fn delete(pool: &SqlitePool, folder: &str, id: &str) -> Result<()> {
    let dirs = items::delete_project_files(pool, folder, id).await?;
    projects::delete(pool, id).await?;
    for d in dirs {
        let _ = std::fs::remove_dir_all(d);
    }
    Ok(())
}

/// Archives a project (it leaves the list, keeps everything) or brings it back.
pub async fn archive(pool: &SqlitePool, id: &str, archived: bool) -> Result<PmProject> {
    get(pool, id).await?;
    let sql = if archived { format!("UPDATE pm_project SET archived_at = coalesce(archived_at, {NOW}), updated_at = {NOW} WHERE project_id = ?") } else { format!("UPDATE pm_project SET archived_at = NULL, updated_at = {NOW} WHERE project_id = ?") };
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(pool).await?;
    get(pool, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    #[test]
    fn identifiers_come_from_names() {
        assert_eq!(identifier_from("WiredScore certification"), "WC");
        assert_eq!(identifier_from("Marina"), "MARIN");
        assert_eq!(identifier_from("Tòa nhà Đông Á — giai đoạn 2"), "TNDAGD2");
        assert_eq!(identifier_from("   "), "P");
        assert_eq!(identifier_from("Đà Nẵng"), "DN");
        assert!(clean_identifier("vifc").is_ok_and(|i| i == "VIFC"));
        assert!(clean_identifier("TOO-LONG").is_err() && clean_identifier("ELEVENCHARS").is_err() && clean_identifier("").is_err());
    }

    #[test]
    fn projects_keep_unique_identifiers_and_their_states() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            // A dossier made before 0.11.0 gets its Plane side when the module reads.
            sqlx::query("INSERT INTO project (id, name) VALUES ('old', 'WiredScore certification')").execute(&pool).await.unwrap();
            let all = list(&pool).await.unwrap();
            assert_eq!((all.len(), all[0].identifier.as_str()), (1, "WC"));
            assert_eq!(settings::states(&pool, &all[0].id).await.unwrap().len(), 5);
            let new = |name: &str, id: &str| NewPmProject {
                name: name.into(),
                identifier: id.into(),
                description: "The hub".into(),
                emoji: "🏙".into(),
                cover: "dawn".into(),
                network: Network::Private,
                lead: Some("me".into()),
                features: Features { cycles: true, intake: true, ..Features::default() },
            };
            let made = create(&pool, new("VIFC Fintech Hub", "vifc")).await.unwrap();
            // A subscriber now in Deleted records doesn't block the others' edits.
            sqlx::query("INSERT INTO person (id, name, name_first) VALUES ('p1', 'Lan Pham', 'Lan')").execute(&pool).await.unwrap();
            update(&pool, &made.id, PmProjectPatch { subscribers: Some(vec!["p1".into()]), ..PmProjectPatch::default() }).await.unwrap();
            sqlx::query("UPDATE person SET deleted_at = '2026-11-02T00:00:00.000Z' WHERE id = 'p1'").execute(&pool).await.unwrap();
            let both = update(&pool, &made.id, PmProjectPatch { subscribers: Some(vec!["p1".into(), "me".into()]), ..PmProjectPatch::default() }).await.unwrap();
            assert_eq!(both.subscribers, ["p1", "me"]);
            assert!(update(&pool, &made.id, PmProjectPatch { lead: Some("p1".into()), ..PmProjectPatch::default() }).await.is_err(), "a new lead must be live");
            assert_eq!(update(&pool, &made.id, PmProjectPatch { lead: Some(String::new()), ..PmProjectPatch::default() }).await.unwrap().lead, None, "empty clears");
            assert_eq!((made.identifier.as_str(), made.features.cycles, made.features.intake, made.features.modules, made.lead.as_deref()), ("VIFC", true, true, false, Some("me")));
            let twice = create(&pool, new("Another hub", "VIFC")).await.unwrap_err().to_string();
            assert!(twice.contains("Another project uses VIFC"), "{twice}");
            assert_eq!(list(&pool).await.unwrap().len(), 2, "nothing left of the refused one");
            assert!(create(&pool, new("Third project", "TP")).await.is_ok());
            assert!(create(&pool, NewPmProject { lead: Some("nobody".into()), ..new("No such lead", "NSL") }).await.is_err());
            // Settings: a renamed identifier stays unique; features, automations, favorite.
            let refused = update(&pool, &made.id, PmProjectPatch { name: Some("Renamed".into()), identifier: Some("wc".into()), ..PmProjectPatch::default() }).await.unwrap_err().to_string();
            assert!(refused.contains("Another project uses WC"), "{refused}");
            assert_eq!(get(&pool, &made.id).await.unwrap().name, "VIFC Fintech Hub", "nothing written when one field is refused");
            let cancelled = settings::states(&pool, &made.id).await.unwrap().into_iter().find(|s| s.group == settings::StateGroup::Cancelled).unwrap();
            let changed = update(
                &pool,
                &made.id,
                PmProjectPatch {
                    name: Some("VIFC Hub".into()),
                    automations: Some(Automations { archive_in: 3, close_in: 6, close_state_id: Some(cancelled.id.clone()), due_reminders: true }),
                    favorite: Some(true),
                    subscribers: Some(vec!["me".into(), "me".into()]),
                    ..PmProjectPatch::default()
                },
            )
            .await
            .unwrap();
            assert_eq!((changed.name.as_str(), changed.automations.close_in, changed.favorite, changed.subscribers.len()), ("VIFC Hub", 6, true, 1));
            let backlog = settings::states(&pool, &made.id).await.unwrap().remove(0);
            assert!(update(&pool, &made.id, PmProjectPatch { automations: Some(Automations { archive_in: 0, close_in: 1, close_state_id: Some(backlog.id), due_reminders: false }), ..PmProjectPatch::default() }).await.is_err(), "only into a cancelled state");
            assert_eq!(list(&pool).await.unwrap()[0].id, made.id, "favorites first");
            assert!(archive(&pool, &made.id, true).await.unwrap().archived_at.is_some());
            projects::delete(&pool, &made.id).await.unwrap();
            let (states,): (i64,) = sqlx::query_as("SELECT count(*) FROM pm_state WHERE project_id = ?").bind(&made.id).fetch_one(&pool).await.unwrap();
            assert_eq!(states, 0, "its Plane side goes with it");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
