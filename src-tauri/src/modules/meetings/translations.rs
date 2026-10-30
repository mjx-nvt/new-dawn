//! The summary in one more language (D-046), translated on this Mac by `kernel::ai::translate`
//! on request and kept per meeting until the summary changes.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use super::summarise::{self, SummaryDoc};
use crate::error::AppError;
use crate::kernel::ai::translate::{self, SummaryLang};

/// One section of a translated summary.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TranslatedSection {
    pub title: String,
    pub markdown: String,
}

/// What `summary_translation.doc_json` holds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Translated {
    sections: Vec<TranslatedSection>,
    highlights: Vec<String>,
}

/// The summary in one more language, as the Summary tab shows it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct SummaryTranslation {
    pub lang: SummaryLang,
    pub highlights: Vec<String>,
    pub sections: Vec<TranslatedSection>,
    /// Each section's markdown as HTML (the summary's own renderer).
    pub sections_html: Vec<String>,
    /// The summary changed since this was made: translate again.
    pub stale: bool,
    pub translated_at: String,
}

/// Which version of the summary a translation was made from: a fingerprint of the words it
/// translates (titles, highlights, sections). A re-run, an edit or a renamed attendee change it;
/// a tick and the email don't. Kept in `summary_translation.source_key`.
fn source_key(doc: &SummaryDoc) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for section in &doc.sections {
        hash.update(section.title.as_bytes());
        hash.update([0]);
        hash.update(
            section
                .markdown
                .replace("- [x] ", "- [ ] ")
                .replace("- [X] ", "- [ ] ")
                .as_bytes(),
        );
        hash.update([0]);
    }
    for highlight in &doc.highlights {
        hash.update(highlight.as_bytes());
        hash.update([0]);
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The summary in `lang`, if it was translated.
pub async fn get(
    pool: &SqlitePool,
    meeting_id: &str,
    lang: SummaryLang,
) -> Result<Option<SummaryTranslation>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT doc_json, source_key, updated_at FROM summary_translation \
         WHERE meeting_id = ? AND lang = ?",
    )
    .bind(meeting_id)
    .bind(lang.key())
    .fetch_optional(pool)
    .await?;
    let Some((doc_json, source, translated_at)) = row else {
        return Ok(None);
    };
    let current = summarise::view(pool, meeting_id).await?;
    let doc: Translated = serde_json::from_str(&doc_json).unwrap_or_default();
    Ok(Some(SummaryTranslation {
        lang,
        sections_html: doc
            .sections
            .iter()
            .map(|s| super::email::to_html(&s.markdown))
            .collect(),
        highlights: doc.highlights,
        sections: doc.sections,
        stale: current.is_some_and(|v| source_key(&v.doc) != source),
        translated_at,
    }))
}

/// A markdown line split into what stays (list marker, checkbox, heading, quote — and an action
/// item's bold owner, a name: "June" would become the month) and the words.
fn split_marker(line: &str) -> (&str, &str) {
    let rest = line.trim_start();
    let mut at = line.len() - rest.len();
    for marker in ["- [ ] ", "- [x] ", "- [X] "] {
        if let Some(item) = rest.strip_prefix(marker) {
            at += marker.len();
            let owner = item
                .strip_prefix("**")
                .and_then(|r| r.find("**"))
                .map_or(0, |end| end + 4);
            if owner > 0 {
                let words = item[owner..].trim_start_matches([' ', '—', '-']);
                at += item.len() - words.len();
            }
            return (&line[..at], &line[at..]);
        }
    }
    for marker in ["- ", "* ", "+ ", "> "] {
        if rest.starts_with(marker) {
            at += marker.len();
            return (&line[..at], &line[at..]);
        }
    }
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    if hashes > 0 && rest[hashes..].starts_with(' ') {
        return (&line[..at + hashes + 1], &line[at + hashes + 1..]);
    }
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && (rest[digits..].starts_with(". ") || rest[digits..].starts_with(") ")) {
        return (&line[..at + digits + 2], &line[at + digits + 2..]);
    }
    (&line[..at], rest)
}

/// Translates the summary (titles, highlights, every line of the sections, markdown kept) into
/// `lang` and keeps it.
pub async fn translate(
    pool: &SqlitePool,
    meeting_id: &str,
    lang: SummaryLang,
) -> Result<SummaryTranslation> {
    let view = summarise::view(pool, meeting_id)
        .await?
        .ok_or_else(|| AppError::not_found("This meeting has no summary yet"))?;
    let doc: &SummaryDoc = &view.doc;
    // Everything with words, in one list: titles, highlights, then each section's lines.
    let mut texts: Vec<String> = doc.sections.iter().map(|s| s.title.clone()).collect();
    texts.extend(doc.highlights.iter().cloned());
    let lines: Vec<Vec<(&str, &str)>> = doc
        .sections
        .iter()
        .map(|s| s.markdown.lines().map(split_marker).collect())
        .collect();
    for (_, words) in lines.iter().flatten() {
        texts.push((*words).to_owned());
    }
    let mut out = translate::translate(
        pool,
        Some(meeting_id),
        "summary_translation",
        "en",
        lang.code(),
        &texts,
    )
    .await?
    .into_iter();
    let titles: Vec<String> = out.by_ref().take(doc.sections.len()).collect();
    let highlights: Vec<String> = out.by_ref().take(doc.highlights.len()).collect();
    let sections = titles
        .into_iter()
        .zip(&lines)
        .map(|(title, section)| TranslatedSection {
            // A title alone comes back in lower case ("vue d'ensemble").
            title: title
                .chars()
                .take(1)
                .flat_map(char::to_uppercase)
                .chain(title.chars().skip(1))
                .collect(),
            markdown: section
                .iter()
                .map(|(marker, _)| format!("{marker}{}", out.next().unwrap_or_default()))
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .collect();
    let json = serde_json::to_string(&Translated {
        sections,
        highlights,
    })?;
    sqlx::query(
        "INSERT INTO summary_translation (id, meeting_id, lang, doc_json, source_key) \
         VALUES (?, ?, ?, ?, ?) \
         ON CONFLICT (meeting_id, lang) DO UPDATE SET doc_json = excluded.doc_json, \
         source_key = excluded.source_key, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(super::new_id())
    .bind(meeting_id)
    .bind(lang.key())
    .bind(&json)
    .bind(source_key(&view.doc))
    .execute(pool)
    .await?;
    get(pool, meeting_id, lang)
        .await?
        .ok_or_else(|| anyhow::anyhow!("the translation wasn't saved"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_markdown_markers_out_of_the_translation() {
        assert_eq!(
            split_marker("- [ ] **June** — send it"),
            ("- [ ] **June** — ", "send it")
        );
        assert_eq!(
            split_marker("- [x] **Minh** - done"),
            ("- [x] **Minh** - ", "done")
        );
        assert_eq!(
            split_marker("- [ ] Unowned task"),
            ("- [ ] ", "Unowned task")
        );
        assert_eq!(
            split_marker("- **Budget**: agreed"),
            ("- ", "**Budget**: agreed")
        );
        assert_eq!(split_marker("  - a point"), ("  - ", "a point"));
        assert_eq!(split_marker("## Heading"), ("## ", "Heading"));
        assert_eq!(split_marker("12. Twelfth"), ("12. ", "Twelfth"));
        assert_eq!(split_marker("#hashtag stays"), ("", "#hashtag stays"));
        assert_eq!(split_marker("Plain text."), ("", "Plain text."));
        assert_eq!(split_marker(""), ("", ""));
    }

    /// The whole path with Apple's models (macOS 26, the languages on this Mac):
    /// `cargo test summary_translation_on_this_mac -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn summary_translation_on_this_mac() {
        use crate::modules::meetings::{eval, save_summary};
        tauri::async_runtime::block_on(async {
            let setup = eval::setup(None).await;
            let pool = &setup.pool;
            let doc: SummaryDoc = serde_json::from_value(serde_json::json!({
                "sections": [
                    {"kind": "overview", "title": "Overview", "markdown": "The team agreed to open the second entry point before the visit."},
                    {"kind": "action_items", "title": "Action items", "markdown": "- [ ] **Ryan** — Send the basement layouts to Stu.\n- [x] **June** — Share the slides."}
                ],
                "decisions": [], "action_items": [],
                "highlights": ["A Gold rating is expected."],
                "generated_at": "2026-10-29T03:00:00.000Z"
            }))
            .unwrap();
            let json = serde_json::to_string(&doc).unwrap();
            save_summary(pool, "m1", &json, "", "", None, false, None)
                .await
                .unwrap();
            assert!(get(pool, "m1", SummaryLang::Fr).await.unwrap().is_none());
            let fr = translate(pool, "m1", SummaryLang::Fr).await.unwrap();
            eprintln!("{:#?}", fr.sections);
            assert_eq!((fr.sections.len(), fr.highlights.len()), (2, 1));
            assert!(
                fr.sections[1].markdown.starts_with("- [ ] **Ryan** — "),
                "{}",
                fr.sections[1].markdown
            );
            assert!(
                fr.sections[1].markdown.contains("**June** — "),
                "a name, not the month"
            );
            assert!(
                fr.sections[0].title.starts_with('V'),
                "{}",
                fr.sections[0].title
            );
            assert!(fr.sections[1].markdown.contains("\n- [x] "));
            assert!(fr.sections_html[1].contains("checkbox") && !fr.stale);
            // A tick leaves it current; a changed word (a renamed person) makes it out of date.
            let mut ticked = doc.clone();
            ticked.sections[1].markdown = ticked.sections[1]
                .markdown
                .replace("- [ ] **Ryan**", "- [x] **Ryan**");
            let json = serde_json::to_string(&ticked).unwrap();
            save_summary(pool, "m1", &json, "", "", None, false, None)
                .await
                .unwrap();
            assert!(
                !get(pool, "m1", SummaryLang::Fr)
                    .await
                    .unwrap()
                    .unwrap()
                    .stale
            );
            let mut newer = doc.clone();
            newer.sections[1].markdown = newer.sections[1].markdown.replace("Ryan", "Ryan Do");
            let json = serde_json::to_string(&newer).unwrap();
            save_summary(pool, "m1", &json, "", "", None, false, None)
                .await
                .unwrap();
            assert!(
                get(pool, "m1", SummaryLang::Fr)
                    .await
                    .unwrap()
                    .unwrap()
                    .stale
            );
            assert!(!translate(pool, "m1", SummaryLang::Fr).await.unwrap().stale);
            setup.finish().await;
        });
    }
}
