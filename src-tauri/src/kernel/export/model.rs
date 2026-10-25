//! The data a minutes template is filled with, from the meeting and its summary (D-024).
//! Field names are the template contract in docs/document-export.md §2.2. Numbering prefixes
//! ("3.", "QĐ-", "CV-") belong to the template; every list item carries `n` (1) and `nn` ("01").

use chrono::{DateTime, Duration, FixedOffset, NaiveDate};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde_json::{Value, json};

use crate::modules::meetings::Meeting;
use crate::modules::meetings::summarise::{SectionKind, SummaryDoc};

pub struct MinutesInput<'a> {
    pub meeting: &'a Meeting,
    pub doc: &'a SummaryDoc,
    /// The meeting's local start time.
    pub start: DateTime<FixedOffset>,
    pub meeting_type: Option<&'a str>,
    /// Position of the meeting among those held the same local day (1-based).
    pub day_seq: u32,
    pub transcript: Option<&'a str>,
}

const NOT_SET: &str = "Not set";

pub fn minutes(input: &MinutesInput) -> Value {
    let doc = input.doc;
    let section = |kind| {
        doc.sections
            .iter()
            .find(|s| s.kind == kind)
            .map(|s| plain(&s.markdown))
            .unwrap_or_default()
    };
    let next_focus = if doc.next_focus.is_empty() {
        section(SectionKind::NextSteps)
    } else {
        doc.next_focus.clone()
    };

    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    for a in &doc.attendees {
        let organisation = or(&a.organisation, "Organisation not stated");
        let member = json!({
            "name": a.name,
            "job_title": a.title,
            "attendance": or(&a.attendance, "Present"),
        });
        match groups.iter_mut().find(|(o, _)| *o == organisation) {
            Some((_, members)) => members.push(member),
            None => groups.push((organisation, vec![member])),
        }
    }

    let mut data = json!({
        "title": input.meeting.title,
        "datetime": datetime(input.start, input.meeting.duration_ms),
        "meeting_type": input.meeting_type.unwrap_or("[Meeting type]"),
        "reference": reference(input.start.date_naive(), input.day_seq),
        "summary": section(SectionKind::Overview),
        "next_focus": next_focus,
        "highlights": numbered(doc.highlights.iter().map(|h| json!({ "text": h }))),
        "agenda": numbered(doc.discussion.iter().map(|t| json!({ "topic": t.topic }))),
        "attendee_groups": groups
            .into_iter()
            .map(|(organisation, members)| json!({ "organisation": organisation, "members": members }))
            .collect::<Vec<_>>(),
        "discussion": numbered(doc.discussion.iter().map(|t| json!({
            "topic": t.topic,
            "points": t.points.iter().map(|p| json!({ "text": p })).collect::<Vec<_>>(),
        }))),
        "decisions": numbered(doc.decisions.iter().map(|d| json!({
            "text": d.text,
            "decided_by": d.decided_by,
            "condition": d.condition,
        }))),
        "actions": numbered(doc.action_items.iter().map(|a| json!({
            "item": a.item,
            "owner": a.owner,
            "due": or(&a.due, NOT_SET),
            "status": "Open",
        }))),
        "open_issues": numbered(doc.open_issues.iter().map(|q| json!({
            "issue": q.issue,
            "who": or(&q.who, NOT_SET),
            "by": or(&q.by, NOT_SET),
        }))),
    });
    if let Some(transcript) = input.transcript {
        data["transcript"] = json!(transcript);
    }
    data
}

/// Template data for a made-up meeting (one item in every list, transcript included): used to
/// check a template and to list the fields it can use.
pub fn sample() -> Value {
    use crate::kernel::ai::whisper::Language;
    use crate::modules::meetings::MeetingStatus;
    use crate::modules::meetings::summarise::{
        ActionItem, Attendee, Decision, OpenIssue, Section, Topic,
    };
    let meeting = Meeting {
        id: String::new(),
        meeting_type_id: None,
        title: "Sample meeting".into(),
        held_on: "2026-01-02T03:00:00.000Z".into(),
        language: Language::En,
        audio_path: None,
        duration_ms: Some(3_600_000),
        status: MeetingStatus::Summarised,
        notes_md: None,
        created_at: String::new(),
        updated_at: String::new(),
    };
    let doc = SummaryDoc {
        sections: vec![Section {
            kind: SectionKind::Overview,
            title: "Overview".into(),
            markdown: "Overview.".into(),
            edited: false,
        }],
        attendees: vec![Attendee {
            name: "Name".into(),
            organisation: "Organisation".into(),
            title: "Title".into(),
            attendance: "Present".into(),
        }],
        discussion: vec![Topic {
            topic: "Topic".into(),
            points: vec!["Point".into()],
        }],
        decisions: vec![Decision {
            text: "Decision".into(),
            ..Decision::default()
        }],
        action_items: vec![ActionItem {
            owner: "Owner".into(),
            item: "Task".into(),
            due: "Friday".into(),
        }],
        open_issues: vec![OpenIssue {
            issue: "Question".into(),
            ..OpenIssue::default()
        }],
        next_focus: "Next".into(),
        highlights: vec!["Budget approved at 1.2 bn".into()],
        ..SummaryDoc::default()
    };
    let start = DateTime::parse_from_rfc3339("2026-01-02T10:00:00+07:00").expect("valid date");
    minutes(&MinutesInput {
        meeting: &meeting,
        doc: &doc,
        start,
        meeting_type: Some("Partner call"),
        day_seq: 1,
        transcript: Some("Transcript."),
    })
}

/// A field a template can use: a value, or a list (`{{#name}}…{{/name}}`) with its own fields.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct TemplateField {
    pub name: String,
    pub list: bool,
    pub children: Vec<TemplateField>,
}

/// The fields of `sample()`, alphabetical.
pub fn fields() -> Vec<TemplateField> {
    fn of(value: &Value) -> Vec<TemplateField> {
        let Some(map) = value.as_object() else {
            return Vec::new();
        };
        map.iter()
            .map(|(name, v)| {
                let children = v
                    .as_array()
                    .and_then(|a| a.first())
                    .map(of)
                    .unwrap_or_default();
                TemplateField {
                    name: name.clone(),
                    list: v.is_array(),
                    children,
                }
            })
            .collect()
    }
    of(&sample())
}

fn or(value: &str, fallback: &str) -> String {
    let value = value.trim();
    (if value.is_empty() { fallback } else { value }).to_owned()
}

fn numbered(items: impl Iterator<Item = Value>) -> Vec<Value> {
    items
        .enumerate()
        .map(|(i, mut item)| {
            if let Some(map) = item.as_object_mut() {
                map.insert("n".into(), json!(i + 1));
                map.insert("nn".into(), json!(format!("{:02}", i + 1)));
            }
            item
        })
        .collect()
}

/// `BB-DDMMYYYY-NN` (biên bản + local date + the meeting's number that day).
pub fn reference(date: NaiveDate, day_seq: u32) -> String {
    format!("BB-{}-{day_seq:02}", date.format("%d%m%Y"))
}

/// "Friday, 24/10/2026 — 10:00–11:30" (end time only when the duration is known).
pub fn datetime(start: DateTime<FixedOffset>, duration_ms: Option<u32>) -> String {
    let mut text = start.format("%A, %d/%m/%Y — %H:%M").to_string();
    if let Some(ms) = duration_ms.filter(|&ms| ms > 0) {
        let end = start + Duration::milliseconds(i64::from(ms));
        text.push_str(&end.format("–%H:%M").to_string());
    }
    text
}

/// Markdown → plain text for a template field: paragraphs and list items on their own lines,
/// bullets as "• ", emphasis dropped.
pub fn plain(markdown: &str) -> String {
    let mut out = String::new();
    for event in Parser::new(markdown) {
        match event {
            Event::Text(text) | Event::Code(text) => out.push_str(&text),
            Event::SoftBreak => out.push(' '),
            Event::HardBreak
            | Event::Start(Tag::List(_))
            | Event::End(TagEnd::Paragraph | TagEnd::Item) => out.push('\n'),
            Event::Start(Tag::Item) => out.push_str("• "),
            _ => {}
        }
    }
    out.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::ai::whisper::Language;
    use crate::modules::meetings::MeetingStatus;
    use crate::modules::meetings::summarise::{
        ActionItem, Attendee, Decision, OpenIssue, Section, Topic,
    };

    fn meeting() -> Meeting {
        Meeting {
            id: "m1".into(),
            meeting_type_id: None,
            title: "Quarterly partner review".into(),
            held_on: "2026-10-24T03:00:00.000Z".into(),
            language: Language::En,
            audio_path: None,
            duration_ms: Some(90 * 60_000),
            status: MeetingStatus::Summarised,
            notes_md: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn doc() -> SummaryDoc {
        let attendee = |name: &str, org: &str, title: &str| Attendee {
            name: name.into(),
            organisation: org.into(),
            title: title.into(),
            attendance: String::new(),
        };
        SummaryDoc {
            sections: vec![
                Section {
                    kind: SectionKind::Overview,
                    title: "Overview".into(),
                    markdown: "We met **Acme**.\n\nBoth sides agreed.".into(),
                    edited: false,
                },
                Section {
                    kind: SectionKind::NextSteps,
                    title: "Next steps".into(),
                    markdown: "- Draft agenda".into(),
                    edited: false,
                },
            ],
            attendees: vec![
                attendee("Tien", "VIFC", "Head of Hub"),
                attendee("Sarah", "Acme", "Partner"),
                attendee("Minh", "VIFC", ""),
            ],
            discussion: vec![
                Topic {
                    topic: "Pipeline".into(),
                    points: vec!["Twelve deals".into()],
                },
                Topic {
                    topic: "Delegation".into(),
                    points: vec![],
                },
            ],
            decisions: vec![Decision {
                text: "City hall".into(),
                ..Decision::default()
            }],
            action_items: (1..=3)
                .map(|i| ActionItem {
                    owner: format!("Owner {i}"),
                    item: format!("Task {i}"),
                    due: String::new(),
                })
                .collect(),
            open_issues: vec![OpenIssue {
                issue: "Observer?".into(),
                who: "Management".into(),
                by: String::new(),
            }],
            next_focus: String::new(),
            highlights: Vec::new(),
            generated_at: String::new(),
        }
    }

    fn input<'a>(meeting: &'a Meeting, doc: &'a SummaryDoc) -> MinutesInput<'a> {
        MinutesInput {
            meeting,
            doc,
            start: DateTime::parse_from_rfc3339("2026-10-24T10:00:00+07:00").unwrap(),
            meeting_type: None,
            day_seq: 2,
            transcript: None,
        }
    }

    #[test]
    fn maps_the_summary_onto_the_template_fields() {
        let (meeting, doc) = (meeting(), doc());
        let data = minutes(&input(&meeting, &doc));
        assert_eq!(data["datetime"], "Saturday, 24/10/2026 — 10:00–11:30");
        assert_eq!(data["reference"], "BB-24102026-02");
        assert_eq!(data["meeting_type"], "[Meeting type]");
        assert_eq!(data["summary"], "We met Acme.\nBoth sides agreed.");
        assert_eq!(
            data["next_focus"], "• Draft agenda",
            "falls back to Next steps"
        );
        let groups = data["attendee_groups"].as_array().unwrap();
        assert_eq!(groups.len(), 2, "grouped by organisation, first-seen order");
        assert_eq!(groups[0]["members"][1]["name"], "Minh");
        assert_eq!(groups[0]["members"][1]["attendance"], "Present");
        assert_eq!(data["agenda"][1]["n"], 2);
        assert_eq!(data["discussion"][0]["points"][0]["text"], "Twelve deals");
        assert_eq!(data["decisions"][0]["nn"], "01");
        assert_eq!(data["actions"].as_array().unwrap().len(), 3);
        assert_eq!(data["actions"][2]["due"], "Not set");
        assert_eq!(data["open_issues"][0]["by"], "Not set");
        assert!(data.get("transcript").is_none());
    }

    #[test]
    fn markdown_to_plain_text() {
        assert_eq!(
            plain("- **Pipeline**\n  - Twelve deals\n- `x` and *y*"),
            "• Pipeline\n• Twelve deals\n• x and y"
        );
        assert_eq!(plain("one\ntwo"), "one two");
    }

    #[test]
    fn end_time_only_with_a_duration() {
        let start = DateTime::parse_from_rfc3339("2026-10-23T09:05:00+07:00").unwrap();
        assert_eq!(datetime(start, None), "Friday, 23/10/2026 — 09:05");
    }

    /// The real VIFC template, filled from a summary — the end-to-end check of the field contract.
    #[test]
    fn fills_the_vifc_template_from_a_summary() {
        let (meeting, doc) = (meeting(), doc());
        let data = minutes(&input(&meeting, &doc));
        let template = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../brands/vifc/templates/meeting-minutes/template.docx");
        let out = std::env::temp_dir().join(format!("newdawn-model-{}.docx", uuid::Uuid::now_v7()));
        super::super::docx::render(&template, &data, &out).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(
            &mut zip::ZipArchive::new(std::fs::File::open(&out).unwrap())
                .unwrap()
                .by_name("word/document.xml")
                .unwrap(),
            &mut xml,
        )
        .unwrap();
        if let Some(keep) = std::env::var_os("NEWDAWN_KEEP_SAMPLE") {
            std::fs::copy(&out, keep).unwrap();
        }
        std::fs::remove_file(out).unwrap();
        assert!(!xml.contains("{{"), "every placeholder filled");
        for text in [
            "3.2",
            "Delegation",
            "QĐ-01",
            "CV-03",
            "Task 3",
            "BB-24102026-02",
        ] {
            assert!(xml.contains(text), "{text}");
        }
    }
}
