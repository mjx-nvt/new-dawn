//! People as dossiers (D-055): what someone said they would do (commitments, taken from each
//! summary's action items that aren't the owner's — the owner's own go to Tasks), when the owner
//! last met them, and the owner's notes. Rule-based like the minutes: the summary already names
//! who does what; nothing is asked of a model here.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile::{self, Owner};
use crate::modules::meetings::summarise::{self, Attendee, SummaryDoc};
use crate::modules::meetings::tasks::{Mine, Who};
use crate::modules::people::{self, Person};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum CommitmentStatus {
    /// From a new summary, waiting for the owner.
    Proposed,
    /// The owner confirmed it: someone owes it.
    Open,
    Kept,
    Dropped,
    /// Not a commitment.
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Commitment {
    pub id: String,
    pub person_id: Option<String>,
    pub person_name: String,
    pub what: String,
    /// The date as said ("" = none).
    pub due_text: String,
    pub meeting_id: Option<String>,
    pub meeting_title: Option<String>,
    pub held_on: Option<String>,
    /// The email it came from instead (D-056), while that email is kept.
    pub email_item_id: Option<String>,
    pub email_subject: Option<String>,
    pub status: CommitmentStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct MetIn {
    pub id: String,
    pub title: String,
    pub held_on: String,
    /// Present (else named among those involved).
    pub attended: bool,
}

/// A person in the People list.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PersonCard {
    pub person: Person,
    /// The newest meeting they attended.
    pub last_contact: Option<MetIn>,
    pub open: u32,
    pub waiting: u32,
}

/// A person's page.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PersonDossier {
    pub person: Person,
    pub notes: String,
    /// The meetings that named them, newest first.
    pub meetings: Vec<MetIn>,
    pub commitments: Vec<Commitment>,
}

macro_rules! commitment_select {
    () => {
        "SELECT c.id, c.person_id, c.person_name, c.what, c.due_text, c.meeting_id, \
         m.title AS meeting_title, m.held_on, c.email_item_id, e.subject AS email_subject, \
         c.status, c.created_at, c.updated_at \
         FROM commitment c LEFT JOIN meeting m ON m.id = c.meeting_id \
         LEFT JOIN email_item e ON e.id = c.email_item_id"
    };
}

/// Owners that aren't anyone's commitment.
const NOBODY: [&str; 6] = [
    "unassigned",
    "not set",
    "everyone",
    "all",
    "tat ca",
    "moi nguoi",
];

/// The person `name` means: by name or another spelling, else the one person the summary's rule
/// calls the same ("Chi" and "Chị Chi"; never a family name alone). None when unsure.
fn match_person<'a>(known: &'a [Person], name: &str) -> Option<&'a Person> {
    people::find(known, name).or_else(|| {
        let mut same = known.iter().filter(|p| {
            summarise::same_person(name, &p.name)
                || p.aliases.iter().any(|a| summarise::same_person(name, a))
        });
        match (same.next(), same.next()) {
            (Some(one), None) => Some(one),
            _ => None,
        }
    })
}

fn source_key(owner: &str, item: &str) -> String {
    format!("{}|{}", people::fold(owner), people::fold(item))
}

/// The summary's action items that are someone else's become proposed commitments (once per item);
/// the meeting's waiting ones the summary no longer has go. Items that are the owner's (they went
/// to Tasks, or the rules say so — the same rules as Tasks), done already, or nobody's are skipped.
/// One transaction per meeting. Without the owner's name nothing can be told apart: nothing is
/// proposed. Returns how many were added.
pub async fn follow_doc(
    pool: &SqlitePool,
    meeting_id: &str,
    doc: &SummaryDoc,
    owner: Option<&Owner>,
    known: &[Person],
) -> Result<u32> {
    let Some(owner) = owner else { return Ok(0) };
    let who = Who::new(owner, known, &doc.attendees);
    let mut tx = pool.begin().await?;
    let (mut added, mut keys) = (0, Vec::new());
    for item in &doc.action_items {
        let name = item.owner.split_whitespace().collect::<Vec<_>>().join(" ");
        let what = item.item.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty()
            || what.is_empty()
            || item.done
            || item.task_id.is_some()
            || NOBODY.contains(&people::fold(&name).as_str())
            || who.sort(item) == Mine::Yes
        {
            continue;
        }
        let due = item.due.trim();
        let due = if people::fold(due) == "not set" {
            ""
        } else {
            due
        };
        let key = source_key(&name, &what);
        let done = sqlx::query(
            "INSERT OR IGNORE INTO commitment (id, person_id, person_name, what, due_text, meeting_id, source_key) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(match_person(known, &name).map(|p| p.id.clone()))
        .bind(&name)
        .bind(&what)
        .bind(due)
        .bind(meeting_id)
        .bind(&key)
        .execute(&mut *tx)
        .await?;
        added += done.rows_affected() as u32;
        keys.push(key);
    }
    // A summary that no longer has an item (re-run, edited) takes its waiting commitment with it.
    sqlx::query(
        "DELETE FROM commitment WHERE meeting_id = ? AND status = 'proposed' \
         AND source_key NOT IN (SELECT value FROM json_each(?))",
    )
    .bind(meeting_id)
    .bind(serde_json::to_string(&keys)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(added)
}

/// A commitment the Email reading agent found (D-056): proposed, linked to the email instead of a
/// meeting, once per email and item. Returns whether it was added.
pub async fn add_from_email(
    pool: &SqlitePool,
    email_item_id: &str,
    who: &str,
    what: &str,
    due: &str,
    known: &[Person],
    owner: Option<&Owner>,
) -> Result<bool> {
    let name = who.split_whitespace().collect::<Vec<_>>().join(" ");
    let what = what.split_whitespace().collect::<Vec<_>>().join(" ");
    // The owner's own promise (an email they wrote) is theirs, not a commitment to them.
    let owners = owner.is_some_and(|o| summarise::same_person(&name, &o.name));
    if name.is_empty()
        || what.is_empty()
        || owners
        || NOBODY.contains(&people::fold(&name).as_str())
    {
        return Ok(false);
    }
    let done = sqlx::query(
        "INSERT OR IGNORE INTO commitment (id, person_id, person_name, what, due_text, email_item_id, source_key) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(match_person(known, &name).map(|p| p.id.clone()))
    .bind(&name)
    .bind(&what)
    .bind(due.trim())
    .bind(email_item_id)
    .bind(format!("email|{}", source_key(&name, &what)))
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// One meeting's commitments from its stored summary (after a summary or an edit of it).
pub async fn sync(hub: &WorkspaceHub, slug: &str, meeting_id: &str) -> Result<u32> {
    let pool = hub.pool(slug)?;
    let Some(summary) = crate::modules::meetings::summary(&pool, meeting_id).await? else {
        return Ok(0);
    };
    let doc: SummaryDoc =
        serde_json::from_str(&summary.sections_json).context("reading the stored summary")?;
    let owner = profile::owner(hub, slug).await?;
    let known = people::list(&pool).await?;
    follow_doc(&pool, meeting_id, &doc, owner.as_ref(), &known).await
}

/// "Look through earlier meetings": every summary's commitments (what the owner already decided
/// isn't proposed again). Returns how many were added.
pub async fn collect(hub: &WorkspaceHub, slug: &str) -> Result<u32> {
    let pool = hub.pool(slug)?;
    let owner = profile::owner(hub, slug).await?;
    let known = people::list(&pool).await?;
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT meeting_id, sections_json FROM summary")
            .fetch_all(&pool)
            .await?;
    let mut added = 0;
    for (meeting_id, json) in rows {
        let Ok(doc) = serde_json::from_str::<SummaryDoc>(&json) else {
            continue;
        };
        added += follow_doc(&pool, &meeting_id, &doc, owner.as_ref(), &known).await?;
    }
    Ok(added)
}

/// After a summary is written or edited: its commitments, in the background (no model, a moment).
pub fn follow(app: &AppHandle, slug: &str, meeting_id: &str) {
    let (app, slug, meeting_id) = (app.clone(), slug.to_owned(), meeting_id.to_owned());
    tauri::async_runtime::spawn(async move {
        if let Err(err) = sync(&app.state::<WorkspaceHub>(), &slug, &meeting_id).await {
            tracing::warn!("commitments of {meeting_id}: {err:#}");
        }
    });
}

/// A name corrected in a meeting (naming check, attendee edit, corrections): its commitments
/// follow, like its tasks (`tasks::rename`) — the name, the item's text, the key, and the person
/// once the memory knows them.
pub async fn rename(pool: &SqlitePool, meeting_id: &str, pairs: &[(String, String)]) -> Result<()> {
    if pairs.is_empty() {
        return Ok(());
    }
    let known = people::list(pool).await?;
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, person_name, what, person_id FROM commitment WHERE meeting_id = ?",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await?;
    for (id, mut name, mut what, person_id) in rows {
        let changed =
            people::rename_in_text(&mut name, pairs) + people::rename_in_text(&mut what, pairs);
        if changed == 0 && person_id.is_some() {
            continue;
        }
        let person_id = person_id.or_else(|| match_person(&known, &name).map(|p| p.id.clone()));
        // OR IGNORE: if the corrected item is already there, the older row keeps its spelling.
        sqlx::query(
            "UPDATE OR IGNORE commitment SET person_name = ?, what = ?, person_id = ?, source_key = ?, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(&name)
        .bind(&what)
        .bind(person_id)
        .bind(source_key(&name, &what))
        .bind(&id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// The people of the memory that the summaries' attendees name: each one's meetings, newest
/// first. Reads only the attendees of each summary; `only` = one person's page.
async fn attendance(
    pool: &SqlitePool,
    known: &[Person],
    only: Option<&Person>,
) -> Result<HashMap<String, Vec<MetIn>>> {
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT m.id, m.title, m.held_on, json_extract(s.sections_json, '$.attendees') \
         FROM summary s JOIN meeting m ON m.id = s.meeting_id ORDER BY m.held_on DESC",
    )
    .fetch_all(pool)
    .await?;
    let mut met: HashMap<String, Vec<MetIn>> = HashMap::new();
    for (id, title, held_on, attendees) in rows {
        let Some(attendees) =
            attendees.and_then(|a| serde_json::from_str::<Vec<Attendee>>(&a).ok())
        else {
            continue;
        };
        for a in &attendees {
            let person = match only {
                Some(p) => match_person(known, &a.name).filter(|m| m.id == p.id),
                None => match_person(known, &a.name),
            };
            let Some(p) = person else { continue };
            let list = met.entry(p.id.clone()).or_default();
            match list.last_mut() {
                Some(m) if m.id == id => m.attended |= a.attending,
                _ => list.push(MetIn {
                    id: id.clone(),
                    title: title.clone(),
                    held_on: held_on.clone(),
                    attended: a.attending,
                }),
            }
        }
    }
    Ok(met)
}

/// The People list: everyone in the people memory but the owner, with when the owner last met them
/// and their commitments open or waiting.
pub async fn people(hub: &WorkspaceHub, slug: &str) -> Result<Vec<PersonCard>> {
    let pool = hub.pool(slug)?;
    let owner = profile::owner(hub, slug).await?;
    let mut known = people::list(&pool).await?;
    if let Some(me) = owner
        .as_ref()
        .and_then(|o| people::find(&known, &o.name))
        .map(|p| p.id.clone())
    {
        known.retain(|p| p.id != me);
    }
    let met = attendance(&pool, &known, None).await?;
    let counts: Vec<(String, u32, u32)> = sqlx::query_as(
        "SELECT person_id, coalesce(sum(status = 'open'), 0), coalesce(sum(status = 'proposed'), 0) \
         FROM commitment WHERE person_id IS NOT NULL GROUP BY person_id",
    )
    .fetch_all(&pool)
    .await?;
    let counts: HashMap<String, (u32, u32)> =
        counts.into_iter().map(|(id, o, w)| (id, (o, w))).collect();
    Ok(known
        .into_iter()
        .map(|person| {
            let (open, waiting) = counts.get(&person.id).copied().unwrap_or_default();
            PersonCard {
                last_contact: met
                    .get(&person.id)
                    .and_then(|m| m.iter().find(|m| m.attended).cloned()),
                open,
                waiting,
                person,
            }
        })
        .collect())
}

/// Commitments waiting for the owner, across everyone (named people and others), newest first.
pub async fn waiting(pool: &SqlitePool) -> Result<Vec<Commitment>> {
    Ok(sqlx::query_as(concat!(
        commitment_select!(),
        " WHERE c.status = 'proposed' ORDER BY c.created_at DESC"
    ))
    .fetch_all(pool)
    .await?)
}

/// Confirmed commitments of someone the people memory doesn't know (an organisation, a name not
/// remembered, a person since deleted): open first, then the newest decided.
pub async fn unlinked(pool: &SqlitePool) -> Result<Vec<Commitment>> {
    Ok(sqlx::query_as(concat!(
        commitment_select!(),
        " WHERE c.person_id IS NULL AND c.status IN ('open', 'kept', 'dropped') \
          ORDER BY c.status <> 'open', c.updated_at DESC"
    ))
    .fetch_all(pool)
    .await?)
}

/// The newest dismissed, for Undo.
pub async fn dismissed(pool: &SqlitePool) -> Result<Vec<Commitment>> {
    Ok(sqlx::query_as(concat!(
        commitment_select!(),
        " WHERE c.status = 'dismissed' ORDER BY c.updated_at DESC LIMIT 20"
    ))
    .fetch_all(pool)
    .await?)
}

pub async fn dossier(pool: &SqlitePool, person_id: &str) -> Result<PersonDossier> {
    let known = people::list(pool).await?;
    let person = known
        .iter()
        .find(|p| p.id == person_id)
        .cloned()
        .ok_or_else(|| AppError::not_found("No such person"))?;
    let (notes,): (String,) = sqlx::query_as("SELECT notes FROM person WHERE id = ?")
        .bind(person_id)
        .fetch_one(pool)
        .await?;
    let meetings = attendance(pool, &known, Some(&person))
        .await?
        .remove(person_id)
        .unwrap_or_default();
    let commitments = sqlx::query_as(concat!(
        commitment_select!(),
        " WHERE c.person_id = ? ORDER BY c.created_at DESC"
    ))
    .bind(person_id)
    .fetch_all(pool)
    .await?;
    Ok(PersonDossier {
        person,
        notes,
        meetings,
        commitments,
    })
}

/// The moves a commitment can make: waiting → confirmed or dismissed; confirmed → kept or dropped;
/// back to confirmed from kept, dropped or dismissed (Undo); dismissed → waiting again.
fn allowed_from(to: CommitmentStatus) -> &'static [&'static str] {
    match to {
        CommitmentStatus::Open => &["proposed", "kept", "dropped", "dismissed"],
        CommitmentStatus::Dismissed => &["proposed"],
        CommitmentStatus::Kept | CommitmentStatus::Dropped => &["open"],
        CommitmentStatus::Proposed => &["dismissed"],
    }
}

pub async fn set_status(
    pool: &SqlitePool,
    id: &str,
    status: CommitmentStatus,
) -> Result<Commitment> {
    let done = sqlx::query(
        "UPDATE commitment SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ? AND status IN (SELECT value FROM json_each(?))",
    )
    .bind(status)
    .bind(id)
    .bind(serde_json::to_string(allowed_from(status))?)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("That commitment changed or is gone"));
    }
    Ok(
        sqlx::query_as(concat!(commitment_select!(), " WHERE c.id = ?"))
            .bind(id)
            .fetch_one(pool)
            .await?,
    )
}

pub async fn set_notes(pool: &SqlitePool, person_id: &str, notes: &str) -> Result<()> {
    let notes = notes.trim();
    if notes.chars().count() > 20_000 {
        bail!(AppError::invalid("Notes are at most 20,000 characters"));
    }
    let done = sqlx::query(
        "UPDATE person SET notes = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(notes)
    .bind(person_id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("No such person"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(id: &str, name: &str) -> Person {
        Person {
            id: id.into(),
            name: name.into(),
            title: String::new(),
            organisation: String::new(),
            aliases: Vec::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn a_name_finds_one_person_or_none() {
        let known = [
            person("p1", "Ms. Chi"),
            person("p2", "Nguyễn Thị Lan"),
            person("p3", "Lê Lan"),
        ];
        assert_eq!(
            match_person(&known, "Chi").map(|p| p.id.as_str()),
            Some("p1")
        );
        assert!(
            match_person(&known, "Nguyen").is_none(),
            "a family name alone is nobody"
        );
        assert!(
            match_person(&known, "Lan").is_none(),
            "two people called Lan: unsure"
        );
        assert_eq!(
            match_person(&known, "Nguyễn Thị Lan").map(|p| p.id.as_str()),
            Some("p2")
        );
    }

    #[test]
    fn statuses_move_only_forward_or_back_by_undo() {
        assert!(allowed_from(CommitmentStatus::Kept).contains(&"open"));
        assert!(!allowed_from(CommitmentStatus::Kept).contains(&"dismissed"));
        assert!(
            !allowed_from(CommitmentStatus::Kept).contains(&"proposed"),
            "confirm first"
        );
        assert!(
            allowed_from(CommitmentStatus::Open).contains(&"dismissed"),
            "undo a dismiss"
        );
        assert_eq!(
            source_key(" Mr  Phuc", "Ultrasound report"),
            "mr phuc|ultrasound report"
        );
    }
}
