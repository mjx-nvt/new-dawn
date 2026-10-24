//! Review and clean-up (P3.3, D-025): hand edits flow back into the lists the minutes use,
//! re-runs keep edited sections, transcript corrections, notes, rename and delete.

use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use sqlx::SqlitePool;

use super::summarise::{
    self, ActionItem, Decision, OpenIssue, SectionKind, SummaryDoc, Topic, not_discussed,
};
use super::{Meeting, MeetingStatus};
use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::hub::WorkspaceHub;

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
pub fn apply_edit(doc: &mut SummaryDoc, index: usize, markdown: &str) -> Result<()> {
    let section = doc
        .sections
        .get_mut(index)
        .ok_or_else(|| AppError::invalid("No such section"))?;
    section.markdown = markdown.trim().to_owned();
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
            doc.action_items = items.iter().map(|e| action(&e.text)).collect();
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
    Ok(())
}

/// After a re-run: every section edited in `old` replaces the new one (and so does the list
/// behind it, so the minutes keep the edit too).
pub fn keep_edits(new: &mut SummaryDoc, old: &SummaryDoc) {
    for kept in old.sections.iter().filter(|s| s.edited) {
        let Some(section) = new.sections.iter_mut().find(|s| {
            s.kind == kept.kind && (s.kind != SectionKind::Other || s.title == kept.title)
        }) else {
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
    let done = sqlx::query(
        "UPDATE transcript_segment SET text = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE meeting_id = ? AND seq = ?",
    )
    .bind(text)
    .bind(meeting_id)
    .bind(seq)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("No such transcript line"));
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
pub(super) async fn to_trash(path: PathBuf) -> Result<()> {
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

        apply_edit(&mut d, 5, "- Invite the observer?\n- New question").unwrap();
        assert_eq!(d.open_issues[0].who, "Management");
        assert_eq!(d.open_issues.len(), 2);

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
            super::super::save_transcript(&pool, "m1", &[seg], 1000)
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
