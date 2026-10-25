//! Review and clean-up (P3.3, D-025): hand edits flow back into the lists the minutes use,
//! re-runs keep edited sections, transcript corrections, notes, rename and delete.

use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use sqlx::SqlitePool;

use super::summarise::Attendee;
use super::summarise::{
    self, ActionItem, Decision, OpenIssue, SectionKind, SummaryDoc, Topic, not_discussed,
};
use super::{Meeting, MeetingStatus};
use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::people::{self, AttendeeEdit};

/// One top-level bullet (or paragraph) and the bullets nested under it, as plain text.
#[derive(Debug, PartialEq)]
struct Entry {
    text: String,
    children: Vec<String>,
}

fn entries(markdown: &str) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    let mut depth = 0usize;
    let mut buf = String::new();
    let flush = |buf: &mut String, depth: usize, out: &mut Vec<Entry>| {
        let text = buf.trim().to_owned();
        buf.clear();
        let placeholder = [Language::En, Language::Vi]
            .iter()
            .any(|&l| text.trim_end_matches('.') == not_discussed(l).trim_end_matches('.'));
        if text.is_empty() || placeholder {
            return;
        }
        match out.last_mut() {
            Some(parent) if depth > 1 => parent.children.push(text),
            _ => out.push(Entry {
                text,
                children: Vec::new(),
            }),
        }
    };
    for event in Parser::new(markdown) {
        match event {
            Event::Text(t) | Event::Code(t) => buf.push_str(&t),
            Event::SoftBreak | Event::HardBreak => buf.push(' '),
            Event::Start(Tag::List(_)) => {
                flush(&mut buf, depth, &mut out);
                depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                flush(&mut buf, depth, &mut out);
                depth = depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) | Event::End(TagEnd::Item | TagEnd::Paragraph) => {
                flush(&mut buf, depth, &mut out)
            }
            _ => {}
        }
    }
    flush(&mut buf, depth, &mut out);
    out
}

/// Does stored text `old` (may hold Markdown) say the same as parsed bullet text `new`?
fn same(old: &str, new: &str) -> bool {
    let old = entries(old)
        .into_iter()
        .next()
        .map(|e| e.text)
        .unwrap_or_default();
    old.eq_ignore_ascii_case(new.trim())
}

/// "Owner — item — due" (the Action items format); with fewer parts the due date, then the
/// owner, are "Not set" / "Unassigned".
fn action(text: &str) -> ActionItem {
    // "[x] Minh — send — Friday": the tick, then the line (D-038).
    let trimmed = text.trim_start();
    let (done, rest) = match trimmed.get(..3) {
        Some("[x]" | "[X]") => (true, &trimmed[3..]),
        Some("[ ]") => (false, &trimmed[3..]),
        _ => (false, trimmed),
    };
    let mut item = action_line(rest);
    item.done = done;
    item
}

fn action_line(text: &str) -> ActionItem {
    let text = text.replace(" – ", " — ");
    let mut parts: Vec<&str> = text.split(" — ").map(str::trim).collect();
    if parts.len() == 1 {
        // Typed with plain hyphens ("Minh - send agenda - Friday"): only with all three parts,
        // so a hyphen inside a task isn't taken for a separator.
        let hyphens: Vec<&str> = text.split(" - ").map(str::trim).collect();
        if hyphens.len() >= 3 {
            parts = hyphens;
        }
    }
    let make = |owner: &str, item: &str, due: &str| ActionItem {
        owner: owner.to_owned(),
        item: item.to_owned(),
        due: due.to_owned(),
        done: false,
    };
    match parts.as_slice() {
        [owner, item] => make(owner, item, "Not set"),
        [owner, middle @ .., due] if !middle.is_empty() => make(owner, &middle.join(" — "), due),
        _ => make("Unassigned", text.trim(), "Not set"),
    }
}

/// Saves a hand edit to section `index`; for the sections built from lists (Key points,
/// Decisions, Action items, Open questions) the list is rebuilt from the edited bullets, keeping
/// details the bullets don't show (who decided, conditions, who/by) when the text still matches.
/// Returns whether it was an edit: ticking or unticking tasks only isn't (the section stays
/// unedited, so a re-run still writes it fresh; the ticks carry over by `carry_ticks`).
pub fn apply_edit(doc: &mut SummaryDoc, index: usize, markdown: &str) -> Result<bool> {
    let section = doc
        .sections
        .get_mut(index)
        .ok_or_else(|| AppError::invalid("No such section"))?;
    section.markdown = markdown.trim().to_owned();
    let was_edited = section.edited;
    section.edited = true;
    let items = entries(markdown);
    match section.kind {
        SectionKind::KeyPoints => {
            doc.discussion = items
                .into_iter()
                .map(|e| Topic {
                    topic: e.text,
                    points: e.children,
                })
                .collect();
        }
        SectionKind::Decisions => {
            doc.decisions = items
                .into_iter()
                .map(|e| {
                    let old = doc.decisions.iter().find(|d| same(&d.text, &e.text));
                    Decision {
                        decided_by: old.map(|d| d.decided_by.clone()).unwrap_or_default(),
                        condition: old.map(|d| d.condition.clone()).unwrap_or_default(),
                        text: e.text,
                    }
                })
                .collect();
        }
        SectionKind::ActionItems => {
            let tasks: Vec<ActionItem> = items.iter().map(|e| action(&e.text)).collect();
            let ticks_only = tasks.len() == doc.action_items.len()
                && tasks
                    .iter()
                    .zip(&doc.action_items)
                    .all(|(a, b)| same_task(a, b) && a.due.trim() == b.due.trim());
            doc.action_items = tasks;
            if ticks_only {
                section.edited = was_edited;
                return Ok(false);
            }
        }
        SectionKind::OpenQuestions => {
            doc.open_issues = items
                .into_iter()
                .map(|e| {
                    let old = doc.open_issues.iter().find(|q| same(&q.issue, &e.text));
                    OpenIssue {
                        who: old.map(|q| q.who.clone()).unwrap_or_default(),
                        by: old.map(|q| q.by.clone()).unwrap_or_default(),
                        issue: e.text,
                    }
                })
                .collect();
        }
        _ => {}
    }
    Ok(true)
}

fn same_task(a: &ActionItem, b: &ActionItem) -> bool {
    people::fold(&a.owner) == people::fold(&b.owner)
        && people::fold(&a.item) == people::fold(&b.item)
}

/// Ticks the tasks of `doc` that were done in `old` (same owner and task) — after a re-run or a
/// correction that rewrote the list.
pub fn carry_ticks(doc: &mut SummaryDoc, old: &[ActionItem]) {
    let mut changed = false;
    for task in doc.action_items.iter_mut().filter(|t| !t.done) {
        if old.iter().any(|o| o.done && same_task(o, task)) {
            task.done = true;
            changed = true;
        }
    }
    if changed {
        let markdown = super::summarise::action_items_markdown(&doc.action_items);
        for section in doc
            .sections
            .iter_mut()
            .filter(|s| s.kind == SectionKind::ActionItems)
        {
            section.markdown = markdown.clone();
        }
    }
}

/// After a re-run: every section edited in `old` replaces the new one (and so does the list
/// behind it, so the minutes keep the edit too).
pub fn keep_edits(new: &mut SummaryDoc, old: &SummaryDoc) {
    for kept in old.sections.iter().filter(|s| s.edited) {
        let Some(section) = new.sections.iter_mut().find(|s| {
            s.kind == kept.kind && (s.kind != SectionKind::Other || s.title == kept.title)
        }) else {
            // A section the new structure no longer has (an old Next steps): keep it at the end.
            new.sections.push(kept.clone());
            continue;
        };
        *section = kept.clone();
        match kept.kind {
            SectionKind::KeyPoints => new.discussion = old.discussion.clone(),
            SectionKind::Decisions => new.decisions = old.decisions.clone(),
            SectionKind::ActionItems => new.action_items = old.action_items.clone(),
            SectionKind::OpenQuestions => new.open_issues = old.open_issues.clone(),
            _ => {}
        }
    }
}

/// Saves the attendees as the user corrected them (minutes and email follow) and, with
/// `remember`, teaches the people memory so the next meeting gets them right.
pub async fn update_attendees(
    pool: &SqlitePool,
    workspace: &str,
    meeting_id: &str,
    attendees: &[AttendeeEdit],
    remember: bool,
) -> Result<summarise::SummaryView> {
    let meeting = super::get(pool, meeting_id).await?;
    if meeting.status == MeetingStatus::Summarising {
        bail!(AppError::invalid(
            "A summary is being written — edit it when it's done"
        ));
    }
    let mut current = summarise::view(pool, meeting_id)
        .await?
        .ok_or_else(|| AppError::not_found("This meeting has no summary yet"))?;
    current.doc.attendees = attendees
        .iter()
        .filter(|a| !a.name.trim().is_empty())
        .map(Attendee::from)
        .collect();
    if remember {
        people::remember(pool, attendees).await?;
    }
    // Every spelling the user corrected, and every known spelling of these people, becomes the
    // right name across the summary; the email and the minutes are rebuilt from it (D-032).
    let people = people::list(pool).await?;
    // ponytail: a heard_as like "Chair" renamed to a person also rewrites "the Chair decided"
    // in prose; acceptable until it bites (then: skip pairs whose `from` isn't a name).
    let mut pairs: Vec<(String, String)> = attendees
        .iter()
        .filter(|a| a.changed && !a.name.trim().is_empty())
        .filter_map(|a| {
            a.heard_as
                .as_ref()
                .map(|h| (people::clean(h, 200), people::clean(&a.name, 200)))
        })
        .collect();
    for a in attendees.iter().filter(|a| !a.name.trim().is_empty()) {
        if let Some(p) = people::find(&people, &a.name) {
            pairs.extend(
                p.aliases
                    .iter()
                    .map(|alias| (alias.clone(), p.name.clone())),
            );
        }
    }
    let renamed = people::rename_in_doc(&mut current.doc, &pairs);
    if renamed > 0 {
        tracing::info!("attendee edit renamed {renamed} mention(s) in meeting {meeting_id}");
    }
    let mut email = current.email_md.clone();
    if people::rename_in_text(&mut email, &pairs) > 0 {
        super::save_email(pool, meeting_id, &current.email_subject, &email).await?;
    }
    summarise::save_doc(pool, workspace, &meeting, &current.doc).await?;
    summarise::view(pool, meeting_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("summary vanished while saving"))
}

fn busy(meeting: &Meeting) -> bool {
    matches!(
        meeting.status,
        MeetingStatus::Recording | MeetingStatus::Transcribing | MeetingStatus::Summarising
    )
}

/// Corrects one transcript line; the next summary run reads the corrected text.
pub async fn update_segment(
    pool: &SqlitePool,
    meeting_id: &str,
    seq: u32,
    text: &str,
) -> Result<()> {
    let text = text.trim();
    if text.is_empty() {
        bail!(AppError::invalid("A transcript line can't be empty"));
    }
    if super::get(pool, meeting_id).await?.status == MeetingStatus::Transcribing {
        bail!(AppError::invalid(
            "The meeting is being transcribed — edit it when it's done"
        ));
    }
    let old: Option<(String,)> =
        sqlx::query_as("SELECT text FROM transcript_segment WHERE meeting_id = ? AND seq = ?")
            .bind(meeting_id)
            .bind(seq)
            .fetch_optional(pool)
            .await?;
    let Some((old,)) = old else {
        bail!(AppError::not_found("No such transcript line"));
    };
    sqlx::query(
        // A hand edit wins: the vocabulary's undo slot is cleared so Undo can't discard it.
        "UPDATE transcript_segment SET text = ?, raw_text = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE meeting_id = ? AND seq = ?",
    )
    .bind(text)
    .bind(meeting_id)
    .bind(seq)
    .execute(pool)
    .await?;
    // A corrected name or term teaches the vocabulary, so the next transcript gets it right.
    let pairs = crate::modules::vocabulary::word_diff(&old, text);
    if !pairs.is_empty() {
        let learned = crate::modules::vocabulary::learn(pool, &pairs).await?;
        tracing::info!("transcript edit taught the vocabulary {learned} term(s)");
    }
    Ok(())
}

/// Puts back what Whisper wrote for a line the vocabulary corrected (nothing is learned).
pub async fn restore_segment(pool: &SqlitePool, meeting_id: &str, seq: u32) -> Result<()> {
    if super::get(pool, meeting_id).await?.status == MeetingStatus::Transcribing {
        bail!(AppError::invalid(
            "The meeting is being transcribed — edit it when it's done"
        ));
    }
    let done = sqlx::query(
        "UPDATE transcript_segment SET text = raw_text, raw_text = NULL, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE meeting_id = ? AND seq = ? AND raw_text IS NOT NULL",
    )
    .bind(meeting_id)
    .bind(seq)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found(
            "This line wasn't corrected by the vocabulary"
        ));
    }
    Ok(())
}

pub async fn set_notes(pool: &SqlitePool, meeting_id: &str, notes: &str) -> Result<Meeting> {
    sqlx::query(
        "UPDATE meeting SET notes_md = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(Some(notes.trim()).filter(|n| !n.is_empty()))
    .bind(meeting_id)
    .execute(pool)
    .await?;
    super::get(pool, meeting_id).await
}

/// New title; the summary's email subject follows.
pub async fn rename(
    hub: &WorkspaceHub,
    slug: &str,
    meeting_id: &str,
    title: &str,
) -> Result<Meeting> {
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        bail!(AppError::invalid("A title needs 1–200 characters"));
    }
    let pool = hub.pool(slug)?;
    sqlx::query("UPDATE meeting SET title = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(title)
        .bind(meeting_id)
        .execute(&pool)
        .await?;
    let meeting = super::get(&pool, meeting_id).await?;
    if meeting.status != MeetingStatus::Summarising {
        if let Some(view) = summarise::view(&pool, meeting_id).await? {
            let workspace = hub.workspace(slug).await?;
            summarise::save_doc(&pool, &workspace.name, &meeting, &view.doc).await?;
        }
    }
    Ok(meeting)
}

/// Moves a file or folder to the Trash off the async workers (Finder can take a second).
pub async fn to_trash(path: PathBuf) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        trash::delete(&path)
            .map_err(|e| anyhow::anyhow!("moving {} to the Trash: {e}", path.display()))
    })
    .await?
}

/// Moves the meeting's recording folder to the Trash, then deletes the meeting (its transcript,
/// summary and export rows go with it; exported files stay in `exports/`). `job_running`: a
/// transcription or summary job is registered for it (its status may not show that yet).
pub async fn delete_meeting(
    hub: &WorkspaceHub,
    slug: &str,
    meeting_id: &str,
    job_running: bool,
) -> Result<()> {
    let pool = hub.pool(slug)?;
    let meeting = super::get(&pool, meeting_id).await?;
    if busy(&meeting) || job_running {
        bail!(AppError::invalid(
            "Wait until recording, transcription or the summary has finished"
        ));
    }
    let recordings = Path::new(&hub.workspace(slug).await?.folder_path).join("recordings");
    // The id must be one plain name, so the folder can only be recordings/<id>.
    let single = matches!(
        Path::new(&meeting.id)
            .components()
            .collect::<Vec<_>>()
            .as_slice(),
        [Component::Normal(_)]
    );
    let folder = recordings.join(&meeting.id);
    if single && folder.is_dir() {
        to_trash(folder).await?;
    } else if let Some(audio) = meeting.audio_path.as_deref() {
        // Older layouts: only a file that really lives under recordings/.
        let inside = match (Path::new(audio).canonicalize(), recordings.canonicalize()) {
            (Ok(audio), Ok(recordings)) => audio.is_file() && audio.starts_with(recordings),
            _ => false,
        };
        if inside {
            to_trash(PathBuf::from(audio)).await?;
        }
    }
    super::delete(&pool, meeting_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::meetings::summarise::{Section, default_structure};

    fn doc() -> SummaryDoc {
        let sections = default_structure(Language::En)
            .into_iter()
            .map(|spec| Section {
                kind: spec.kind,
                title: spec.title,
                markdown: String::new(),
                edited: false,
            })
            .collect();
        SummaryDoc {
            sections,
            decisions: vec![Decision {
                text: "Host at city hall".into(),
                decided_by: "Chair".into(),
                condition: "If free".into(),
            }],
            open_issues: vec![OpenIssue {
                issue: "Invite the observer?".into(),
                who: "Management".into(),
                by: "Friday".into(),
            }],
            ..SummaryDoc::default()
        }
    }

    #[test]
    fn reads_bullets_with_nesting_and_skips_placeholders() {
        assert_eq!(
            entries(
                "- **Pipeline**\n  - Twelve deals\n  - Four at *term sheet*\n- Budget\n\nNot discussed."
            ),
            vec![
                Entry {
                    text: "Pipeline".into(),
                    children: vec!["Twelve deals".into(), "Four at term sheet".into()]
                },
                Entry {
                    text: "Budget".into(),
                    children: vec![]
                },
            ]
        );
        assert_eq!(entries("A paragraph\nwrapped.").len(), 1);
    }

    #[test]
    fn edits_rebuild_the_lists_and_keep_details() {
        let mut d = doc();
        apply_edit(&mut d, 1, "- **Pipeline**\n  - Twelve deals").unwrap();
        assert_eq!(d.discussion[0].topic, "Pipeline");
        assert_eq!(d.discussion[0].points, vec!["Twelve deals"]);
        assert!(d.sections[1].edited);

        apply_edit(&mut d, 2, "- host at city hall\n- Sign by Friday").unwrap();
        assert_eq!(
            d.decisions[0].decided_by, "Chair",
            "kept: same text, case aside"
        );
        assert_eq!(d.decisions[1].decided_by, "");

        apply_edit(&mut d, 3, "- **Minh** — Send the agenda — *Friday*\n- Sarah – Book the room\n- Call the bank\n- Tien — Brief A — then B — Monday").unwrap();
        assert_eq!(d.action_items[0].owner, "Minh");
        assert_eq!(d.action_items[0].due, "Friday");
        assert_eq!(d.action_items[1].due, "Not set");
        assert_eq!(d.action_items[2].owner, "Unassigned");
        assert_eq!(d.action_items[2].item, "Call the bank");
        assert_eq!(d.action_items[3].item, "Brief A — then B");
        apply_edit(
            &mut d,
            3,
            "- Minh - send agenda - Friday\n- Re-check the e-mail draft",
        )
        .unwrap();
        assert_eq!(d.action_items[0].owner, "Minh");
        assert_eq!(
            d.action_items[1].owner, "Unassigned",
            "a lone hyphen isn't a separator"
        );

        d.decisions[0].text = "Approve **$2m** budget".into();
        d.decisions[0].decided_by = "Board".into();
        apply_edit(&mut d, 2, "- Approve $2m budget\n\nNot discussed").unwrap();
        assert_eq!(d.decisions.len(), 1, "a placeholder isn't a decision");
        assert_eq!(
            d.decisions[0].decided_by, "Board",
            "matched despite the old Markdown"
        );

        apply_edit(&mut d, 4, "- Invite the observer?\n- New question").unwrap();
        assert_eq!(d.open_issues[0].who, "Management");
        assert_eq!(d.open_issues.len(), 2);

        apply_edit(
            &mut d,
            3,
            "- [x] **Minh** — send agenda — *Friday*\n- [ ] Book the room",
        )
        .unwrap();
        assert!(
            d.action_items[0].done && !d.action_items[1].done,
            "ticks survive an edit"
        );
        assert_eq!(d.action_items[0].item, "send agenda");

        // Ticking alone isn't an edit; the tick survives a fresh list that has the same task.
        let task = || {
            vec![ActionItem {
                owner: "Minh".into(),
                item: "send agenda".into(),
                due: "Friday".into(),
                done: false,
            }]
        };
        let mut t = doc();
        t.action_items = task();
        assert!(
            !apply_edit(&mut t, 3, "- [x] **Minh** — send agenda — *Friday*").unwrap(),
            "a tick isn't an edit"
        );
        assert!(!t.sections[3].edited && t.action_items[0].done);
        let mut fresh = doc();
        fresh.action_items = task();
        carry_ticks(&mut fresh, &t.action_items);
        assert!(fresh.action_items[0].done && fresh.sections[3].markdown.starts_with("- [x]"));

        assert!(apply_edit(&mut d, 99, "x").is_err());
    }

    #[test]
    fn rerun_keeps_edited_sections_and_their_lists() {
        let mut old = doc();
        apply_edit(&mut old, 2, "- Mine").unwrap();
        let mut new = doc();
        new.sections[0].markdown = "Fresh overview".into();
        new.decisions = vec![];
        keep_edits(&mut new, &old);
        assert_eq!(new.sections[2].markdown, "- Mine");
        assert!(new.sections[2].edited);
        assert_eq!(new.decisions[0].text, "Mine");
        assert_eq!(
            new.sections[0].markdown, "Fresh overview",
            "unedited sections are rewritten"
        );
    }

    #[test]
    fn corrects_segments_and_deletes_meetings() {
        use crate::kernel::ai::whisper::Segment;
        let root = std::env::temp_dir().join(format!("newdawn-review-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("Test", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            // Audio outside recordings/ (and no recordings/m1 folder): nothing goes to the Trash,
            // so the test never touches the user's real Trash.
            let outside = root.join("elsewhere.wav");
            std::fs::write(&outside, "x").unwrap();
            super::super::create(
                &pool,
                super::super::NewMeeting {
                    id: "m1",
                    meeting_type_id: None,
                    title: "Call",
                    held_at_unix: 0,
                    language: Language::En,
                    audio_path: &outside.display().to_string(),
                    status: MeetingStatus::Recorded,
                },
            )
            .await
            .unwrap();
            let seg = Segment {
                start_ms: 0,
                end_ms: 1000,
                text: "helo".into(),
            };
            super::super::save_transcript(&pool, "m1", &[seg], &[], 1000)
                .await
                .unwrap();
            update_segment(&pool, "m1", 0, " hello Acme ")
                .await
                .unwrap();
            assert!(update_segment(&pool, "m1", 0, "  ").await.is_err());
            assert!(update_segment(&pool, "m1", 7, "x").await.is_err());
            let hits = super::super::search(&pool, "acme").await.unwrap();
            assert_eq!(hits.len(), 1, "the FTS index follows the correction");

            let m = set_notes(&pool, "m1", "  Chair: Tien  ").await.unwrap();
            assert_eq!(m.notes_md.as_deref(), Some("Chair: Tien"));
            assert!(rename(&hub, "test", "m1", "   ").await.is_err());
            assert_eq!(
                rename(&hub, "test", "m1", "Acme call").await.unwrap().title,
                "Acme call"
            );

            assert!(
                delete_meeting(&hub, "test", "m1", true).await.is_err(),
                "a job is running"
            );
            delete_meeting(&hub, "test", "m1", false).await.unwrap();
            assert!(
                outside.exists(),
                "a file outside recordings/ is never trashed"
            );
            assert!(super::super::get(&pool, "m1").await.is_err());
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
