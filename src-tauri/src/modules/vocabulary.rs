//! The workspace vocabulary (D-036): terms with the ways Whisper mishears them. It guides Whisper
//! (an initial prompt with names and terms) and corrects each new transcript (mishearing →
//! term, whole words), and it learns from the owner's transcript corrections. Names come from
//! the people memory, which learns from attendee edits and confirmed names.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::modules::people::{self, Person};

#[derive(Debug, Clone, Serialize, Type)]
pub struct Term {
    pub id: String,
    pub term: String,
    pub heard_as: Vec<String>,
    /// "learned" (from transcript corrections) or "manual".
    pub source: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    term: String,
    heard_as_json: String,
    source: String,
    updated_at: String,
}

impl From<Row> for Term {
    fn from(r: Row) -> Self {
        Term {
            id: r.id,
            term: r.term,
            heard_as: serde_json::from_str(&r.heard_as_json).unwrap_or_default(),
            source: r.source,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct TermInput {
    pub id: Option<String>,
    pub term: String,
    pub heard_as: Vec<String>,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Term>> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, term, heard_as_json, source, updated_at FROM vocabulary ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(Term::from).collect())
}

fn clean_list(items: &[String], term: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for h in items {
        let h = people::clean(h, 80);
        if !h.is_empty() && h != term && !out.iter().any(|o| o.eq_ignore_ascii_case(&h)) {
            out.push(h);
        }
    }
    out.truncate(20);
    out
}

/// Adds (no id) or updates a term by hand.
pub async fn save(pool: &SqlitePool, input: TermInput) -> Result<Term> {
    let term = people::clean(&input.term, 80);
    if term.is_empty() {
        bail!(AppError::invalid("A term can't be empty"));
    }
    let heard = serde_json::to_string(&clean_list(&input.heard_as, &term))?;
    let id = match input.id {
        Some(id) => {
            let done = sqlx::query(
                "UPDATE vocabulary SET term = ?, heard_as_json = ?, source = 'manual', \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
            )
            .bind(&term)
            .bind(&heard)
            .bind(&id)
            .execute(pool)
            .await
            .map_err(duplicate)?;
            if done.rows_affected() == 0 {
                bail!(AppError::not_found("No such term"));
            }
            id
        }
        None => {
            let id = Uuid::now_v7().to_string();
            sqlx::query("INSERT INTO vocabulary (id, term, heard_as_json, source) VALUES (?, ?, ?, 'manual')")
                .bind(&id)
                .bind(&term)
                .bind(&heard)
                .execute(pool)
                .await
                .map_err(duplicate)?;
            id
        }
    };
    let row: Row = sqlx::query_as("SELECT id, term, heard_as_json, source, updated_at FROM vocabulary WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;
    Ok(row.into())
}

fn duplicate(e: sqlx::Error) -> anyhow::Error {
    match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => AppError::invalid("That term is already in the vocabulary").into(),
        _ => e.into(),
    }
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM vocabulary WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// Records mishearing → term pairs (from a transcript correction): adds the mishearing to an
/// existing term, or creates a learned term.
pub async fn learn(pool: &SqlitePool, pairs: &[(String, String)]) -> Result<usize> {
    let mut learned = 0;
    for (heard, term) in pairs {
        let row: Option<Row> = sqlx::query_as(
            "SELECT id, term, heard_as_json, source, updated_at FROM vocabulary WHERE term = ? COLLATE NOCASE",
        )
        .bind(term)
        .fetch_optional(pool)
        .await?;
        match row {
            Some(row) => {
                let mut existing: Vec<String> = serde_json::from_str(&row.heard_as_json).unwrap_or_default();
                if existing.iter().any(|h| h.eq_ignore_ascii_case(heard)) {
                    continue;
                }
                existing.push(heard.clone());
                sqlx::query("UPDATE vocabulary SET heard_as_json = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
                    .bind(serde_json::to_string(&clean_list(&existing, &row.term))?)
                    .bind(&row.id)
                    .execute(pool)
                    .await?;
            }
            None => {
                sqlx::query("INSERT INTO vocabulary (id, term, heard_as_json, source) VALUES (?, ?, ?, 'learned')")
                    .bind(Uuid::now_v7().to_string())
                    .bind(term)
                    .bind(serde_json::to_string(&[heard])?)
                    .execute(pool)
                    .await?;
            }
        }
        learned += 1;
    }
    Ok(learned)
}

/// What Whisper is told before the next transcription (the title is a placeholder).
pub async fn preview(pool: &SqlitePool) -> Result<String> {
    let people = people::list(pool).await?;
    let terms = list(pool).await?;
    Ok(whisper_prompt("<the meeting's title>", &people, &terms))
}

/// Whisper's initial prompt: the meeting title, the people (recent first) and the terms, as one
/// plain sentence each, within ~700 characters (Whisper reads about 220 tokens of prompt).
pub fn whisper_prompt(title: &str, people: &[Person], terms: &[Term]) -> String {
    const MAX: usize = 700;
    let safe = |s: &str| s.replace(['\0', '<', '>'], " ").split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = String::new();
    let title = safe(title);
    if !title.is_empty() {
        out.push_str(&format!("Meeting: {title}."));
    }
    let mut add_list = |label: &str, items: Vec<String>| {
        let mut list = Vec::new();
        for item in items {
            let item = safe(&item);
            if item.is_empty() || list.contains(&item) {
                continue;
            }
            let extra = item.chars().count() + 2;
            if out.chars().count() + label.len() + list.iter().map(|s: &String| s.chars().count() + 2).sum::<usize>() + extra > MAX {
                break;
            }
            list.push(item);
        }
        if !list.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&format!("{label}: {}.", list.join(", ")));
        }
    };
    let mut recent: Vec<&Person> = people.iter().collect();
    recent.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    // Group entries ("Richard and Harry", "the CMO team") are not names Whisper should learn.
    let names = recent
        .iter()
        .map(|p| p.name.clone())
        .filter(|n| n.chars().count() <= 40 && !n.contains(" and ") && !n.to_lowercase().contains(" team"))
        .collect();
    add_list("People", names);
    add_list("Terms", terms.iter().map(|t| t.term.clone()).collect());
    out
}

/// Mishearing → right spelling, from the vocabulary and the people memory's other spellings.
pub fn replacements(people: &[Person], terms: &[Term]) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for t in terms {
        pairs.extend(t.heard_as.iter().map(|h| (h.clone(), t.term.clone())));
    }
    for p in people {
        pairs.extend(p.aliases.iter().map(|a| (a.clone(), p.name.clone())));
    }
    pairs.retain(|(from, to)| from.trim().chars().count() >= 3 && from.trim() != to.trim());
    pairs
}

/// Corrects each line with the replacements; returns, per line, what Whisper wrote when the
/// line changed.
pub fn correct_lines(lines: &mut [String], pairs: &[(String, String)]) -> Vec<Option<String>> {
    let before = lines.to_vec();
    people::rename_lines_exact(lines, pairs)
        .into_iter()
        .zip(before)
        .map(|(changed, was)| changed.then_some(was))
        .collect()
}

/// Whisper sometimes repeats its prompt on near-silent audio ("People: …, Terms: …"): such a
/// line is dropped rather than kept as speech.
pub fn echoes_prompt(line: &str, prompt: &str) -> bool {
    let line = people::fold(line);
    let prompt = people::fold(prompt);
    line.chars().count() >= 12
        && (prompt.contains(line.trim_end_matches('.'))
            || ["meeting:", "people:", "terms:"].iter().filter(|k| line.contains(*k)).count() >= 2)
}

fn word(token: &str) -> &str {
    token.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Replacements the owner made in one line: runs of up to four words changed into runs of up to
/// four words, kept only when the new run looks like a name or term (a capital letter, a digit
/// or a letter outside ASCII) — ordinary wording edits are not learned.
pub fn word_diff(old: &str, new: &str) -> Vec<(String, String)> {
    let a: Vec<&str> = old.split_whitespace().map(word).filter(|w| !w.is_empty()).collect();
    let b: Vec<&str> = new.split_whitespace().map(word).filter(|w| !w.is_empty()).collect();
    // LCS table over words.
    let mut lcs = vec![vec![0u16; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut pairs = Vec::new();
    let mut run_a: Vec<&str> = Vec::new();
    let mut run_b: Vec<&str> = Vec::new();
    let flush = |ra: &mut Vec<&str>, rb: &mut Vec<&str>, pairs: &mut Vec<(String, String)>| {
        if (1..=4).contains(&ra.len()) && (1..=4).contains(&rb.len()) {
            let from = ra.join(" ");
            let to = rb.join(" ");
            let looks_like_term = to.chars().any(|c| c.is_uppercase() || c.is_ascii_digit() || (c.is_alphabetic() && !c.is_ascii()));
            // A respelling, not a rewrite: capitalisation alone isn't learned ("the" → "The"), and
            // the two must sound alike — close once folded ("Duke" → "Đức", "Wired Score" →
            // "WiredScore"), not "the client" → "Acme".
            let (f, t) = (people::fold(&from), people::fold(&to));
            let close = crate::modules::meetings::naming::distance(&f.replace(' ', ""), &t.replace(' ', ""))
                * 2
                <= f.chars().count().max(t.chars().count());
            if looks_like_term && from.to_lowercase() != to.to_lowercase() && close && to.chars().any(char::is_alphabetic) {
                pairs.push((from, to));
            }
        }
        ra.clear();
        rb.clear();
    };
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            flush(&mut run_a, &mut run_b, &mut pairs);
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            run_b.push(b[j]);
            j += 1;
        } else {
            run_a.push(a[i]);
            i += 1;
        }
    }
    flush(&mut run_a, &mut run_b, &mut pairs);
    pairs.truncate(3);
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, aliases: &[&str]) -> Person {
        Person {
            id: name.into(),
            name: name.into(),
            title: String::new(),
            organisation: String::new(),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn learns_term_replacements_but_not_wording() {
        assert_eq!(
            word_diff("Email sent to June of Wired Score Singapore.", "Email sent to June of WiredScore Singapore."),
            vec![("Wired Score".to_owned(), "WiredScore".to_owned())]
        );
        assert_eq!(word_diff("So, Duke and I agreed", "So, Đức and I agreed"), vec![("Duke".to_owned(), "Đức".to_owned())]);
        assert!(word_diff("we have to take care of it", "we must take care of it").is_empty(), "plain wording isn't a term");
        assert!(word_diff("same text", "same text").is_empty());
        assert!(word_diff("the meeting started", "The meeting started").is_empty(), "capitalisation isn't learned");
        assert!(word_diff("we told the client today", "we told Acme today").is_empty(), "a rewrite isn't a respelling");
    }

    #[test]
    fn corrects_lines_and_keeps_what_whisper_wrote() {
        let terms = vec![Term {
            id: "1".into(),
            term: "WiredScore".into(),
            heard_as: vec!["Wired Score".into()],
            source: "learned".into(),
            updated_at: String::new(),
        }];
        let people = vec![person("Đức", &["Duke", "Du"])];
        let pairs = replacements(&people, &terms);
        assert!(!pairs.iter().any(|(f, _)| f == "Du"), "too short to replace safely");
        let mut lines = vec!["Duke met Wired Score.".to_owned(), "Nothing here.".to_owned(), "a duke of wired score".to_owned()];
        let raw = correct_lines(&mut lines, &pairs);
        assert_eq!(lines[0], "Đức met WiredScore.");
        assert_eq!(lines[2], "a duke of wired score", "exact spelling only in speech");
        assert_eq!(raw, vec![Some("Duke met Wired Score.".to_owned()), None, None]);
        assert!(echoes_prompt("People: Đức. Terms: WiredScore.", "Meeting: x. People: Đức. Terms: WiredScore."));
        assert!(!echoes_prompt("We met Đức about WiredScore today.", "Meeting: x. People: Đức. Terms: WiredScore."));
        let prompt = whisper_prompt("Weekly <meeting>", &people, &terms);
        assert_eq!(prompt, "Meeting: Weekly meeting. People: Đức. Terms: WiredScore.");
    }
}
