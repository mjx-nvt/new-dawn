//! Apple Mail as the connected account (D-066, 0.10.9): a record's email threads, read on
//! demand through Apple Events — Mail must be running and allowed in Automation; New Dawn
//! fetches nothing and sends nothing (D-056's way) — and kept as `message_thread`, `message`
//! and `message_participant` rows between refreshes; the blocklist; the mail settings; a
//! record's addresses; the people a thread names that the CRM doesn't know yet.

use std::collections::HashMap;
use std::process::Stdio;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use sqlx::SqlitePool;
use tokio::process::Command;

use crate::error::AppError;
use crate::kernel::records::{self, query, Actor, ObjectDef};
use crate::modules::crm::{self, CalendarEvent};

/// Inbox messages kept per address (the newest), sent messages looked through, how far back
/// Mail is asked, a snippet's length.
const PER_HANDLE: usize = 100;
const SENT_SCAN: u32 = 200;
const DAYS_BACK: u32 = 365;
const SNIPPET: usize = 240;
/// One conversation with Mail at a time (two records opened together would otherwise run two
/// scripts against it).
static MAIL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
/// A record's threads are read again after this long, or on Refresh.
const STALE_SECS: i64 = 600;
const RS: char = '\u{1e}';
const FS: char = '\u{1f}';

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

// ---- settings and the blocklist ------------------------------------------------------------

/// Twenty's mailbox settings, for this workspace.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MailSettings {
    /// deactivated · sent_received · sent_only — whom the Emails widget proposes as people.
    pub auto_create: String,
    /// metadata · subject · everything — what is read of a message.
    pub visibility: String,
}

async fn setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM setting WHERE key = ?").bind(key).fetch_optional(pool).await?;
    Ok(row.map(|(v,)| v))
}

async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query("INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn settings(pool: &SqlitePool) -> Result<MailSettings> {
    Ok(MailSettings {
        auto_create: setting(pool, "mail:auto_create").await?.unwrap_or_else(|| "sent_received".into()),
        visibility: setting(pool, "mail:visibility").await?.unwrap_or_else(|| "subject".into()),
    })
}

pub async fn set_settings(pool: &SqlitePool, s: &MailSettings) -> Result<()> {
    if !matches!(s.auto_create.as_str(), "deactivated" | "sent_received" | "sent_only") {
        return Err(invalid("Contact auto-creation: Deactivated, Sent & Received or Sent only"));
    }
    if !matches!(s.visibility.as_str(), "metadata" | "subject" | "everything") {
        return Err(invalid("Visibility: Metadata only, Subject + metadata or Share everything"));
    }
    let before = settings(pool).await?;
    set_setting(pool, "mail:auto_create", &s.auto_create).await?;
    set_setting(pool, "mail:visibility", &s.visibility).await?;
    // Another visibility: what was kept has more or less than it should — read again.
    if before.visibility != s.visibility {
        sqlx::query("DELETE FROM setting WHERE key LIKE 'mail:refreshed:%'").execute(pool).await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Blocked {
    pub id: String,
    pub handle: String,
    pub created_at: String,
}

pub async fn blocklist(pool: &SqlitePool) -> Result<Vec<Blocked>> {
    Ok(sqlx::query_as("SELECT id, handle, created_at FROM blocklist ORDER BY handle").fetch_all(pool).await?)
}

/// An address (`name@example.com`) or a domain (`@example.com`).
pub async fn block(pool: &SqlitePool, handle: &str) -> Result<Blocked> {
    let handle = handle.trim().to_lowercase();
    if !handle.contains('@') || handle.ends_with('@') || handle.contains(char::is_whitespace) {
        return Err(invalid("An address (name@example.com) or a domain (@example.com)"));
    }
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO blocklist (id, handle) VALUES (?, ?) ON CONFLICT (handle) DO NOTHING").bind(&id).bind(&handle).execute(pool).await?;
    // What was kept of them goes too (Twenty deletes a blocked sender's messages): the cache is
    // whole threads, so it is emptied and read again from Mail at the next open.
    sqlx::query("DELETE FROM message_thread").execute(pool).await?;
    sqlx::query("DELETE FROM setting WHERE key LIKE 'mail:refreshed:%'").execute(pool).await?;
    let row: Blocked = sqlx::query_as("SELECT id, handle, created_at FROM blocklist WHERE handle = ?").bind(&handle).fetch_one(pool).await?;
    Ok(row)
}

pub async fn unblock(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM blocklist WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

fn is_blocked(blocked: &[String], handle: &str) -> bool {
    blocked.iter().any(|b| if b.starts_with('@') { handle.ends_with(b.as_str()) } else { handle == b })
}

// ---- a record's addresses ----------------------------------------------------------------

fn host_of(url: &str) -> String {
    let lower = url.trim().to_lowercase();
    let no_scheme = lower.split_once("://").map_or(lower.as_str(), |(_, rest)| rest);
    no_scheme.strip_prefix("www.").unwrap_or(no_scheme).trim_end_matches('/').split('/').next().unwrap_or("").to_owned()
}

fn push_emails(out: &mut Vec<String>, primary: &str, additional_json: &str) {
    let primary = primary.trim().to_lowercase();
    if primary.contains('@') {
        out.push(primary);
    }
    if let Ok(Value::Array(more)) = serde_json::from_str::<Value>(additional_json) {
        out.extend(more.iter().filter_map(Value::as_str).map(|s| s.trim().to_lowercase()).filter(|s| s.contains('@')));
    }
}

async fn company_handles(pool: &SqlitePool, company_id: &str, out: &mut Vec<String>) -> Result<()> {
    let people: Vec<(String, String)> = sqlx::query_as("SELECT emails_primary, emails_additional FROM person WHERE company_id = ? AND deleted_at IS NULL").bind(company_id).fetch_all(pool).await?;
    for (primary, additional) in people {
        push_emails(out, &primary, &additional);
    }
    let domain: Option<(String,)> = sqlx::query_as("SELECT domain_url FROM company WHERE id = ?").bind(company_id).fetch_optional(pool).await?;
    if let Some((url,)) = domain {
        let host = host_of(&url);
        if host.contains('.') {
            out.push(format!("@{host}"));
        }
    }
    Ok(())
}

/// What a record is reached by: a person's addresses; a company's people's addresses and its
/// `@domain`; an opportunity's company's. Lower case, each once.
pub async fn handles_of(pool: &SqlitePool, object: &ObjectDef, id: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    match object.meta.name_singular.as_str() {
        "person" => {
            let row: Option<(String, String)> = sqlx::query_as("SELECT emails_primary, emails_additional FROM person WHERE id = ?").bind(id).fetch_optional(pool).await?;
            if let Some((primary, additional)) = row {
                push_emails(&mut out, &primary, &additional);
            }
        }
        "company" => company_handles(pool, id, &mut out).await?,
        "opportunity" => {
            let row: Option<(Option<String>,)> = sqlx::query_as("SELECT company_id FROM opportunity WHERE id = ?").bind(id).fetch_optional(pool).await?;
            if let Some((Some(company),)) = row {
                company_handles(pool, &company, &mut out).await?;
            }
        }
        _ => {}
    }
    out.sort();
    out.dedup();
    Ok(out)
}

// ---- Mail --------------------------------------------------------------------------------

/// How Mail answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum MailState {
    Ok,
    NotRunning,
    NoPermission,
    Failed,
}

/// Mail's answer for a record: first the owner's own addresses (every account's), then the
/// inbox messages from each address since the cut-off, then the sent messages to any of them —
/// through Apple Events, each property of a whole list in one event (never one event a
/// message, which a large mailbox turns into minutes). The addresses travel as arguments, never
/// inside the script; the first arguments are the visibility (0 metadata · 1 subject · 2 the
/// first words too), the sent messages looked through, and the days back. ASCII 30 ends a row,
/// ASCII 31 a field; both are taken out of every value. The inbox rows carry no recipients
/// (that would be an event a message); the sent rows do, so "whom you wrote to" is known.
const SCRIPT: &str = r#"on clean(v)
    set t to v as string
    repeat with code in {30, 31}
        set AppleScript's text item delimiters to (ASCII character code)
        set parts to text items of t
        set AppleScript's text item delimiters to ""
        set t to parts as string
    end repeat
    return t
end clean
on joined(l)
    set AppleScript's text item delimiters to ","
    set t to l as string
    set AppleScript's text item delimiters to ""
    return t
end joined
on addresses(rs)
    set out to {}
    repeat with r in rs
        try
            set end of out to ((address of r) as string)
        end try
    end repeat
    return my joined(out)
end addresses
on matches(addrs, handles)
    repeat with h in handles
        if addrs contains (h as string) then return true
    end repeat
    return false
end matches
on isot(d)
    try
        return (d as «class isot» as string)
    on error
        return ""
    end try
end isot
on run argv
    if application "Mail" is not running then return "NOT_RUNNING"
    set mode to item 1 of argv
    set sentScan to (item 2 of argv) as integer
    set daysBack to (item 3 of argv) as integer
    set handles to items 4 thru -1 of argv
    set rs to (ASCII character 30)
    set fs to (ASCII character 31)
    set cutoff to (current date) - (daysBack * days)
    set out to ""
    with timeout of 80 seconds
        tell application "Mail"
            set mine to {}
            try
                repeat with a in accounts
                    set mine to mine & (email addresses of a)
                end repeat
            end try
            set out to "ME" & fs & my clean(my joined(mine)) & rs
            repeat with h in handles
                try
                    set found to (messages of inbox whose sender contains (h as string) and date received > cutoff)
                    if (count of found) > 0 then
                        set ids to message id of found
                        set snds to sender of found
                        set dts to date received of found
                        if mode is "0" then
                            set subs to {}
                        else
                            set subs to subject of found
                        end if
                        if mode is "2" and (count of found) <= 100 then
                            set bods to content of found
                        else
                            set bods to {}
                        end if
                        repeat with k from 1 to count of found
                            set sb to ""
                            if (count of subs) >= k then set sb to item k of subs
                            set bd to ""
                            if (count of bods) >= k then set bd to item k of bods
                            if (length of bd) > 400 then set bd to text 1 thru 400 of bd
                            set out to out & my clean(item k of ids) & fs & my clean(sb) & fs & my clean(item k of snds) & fs & my isot(item k of dts) & fs & "" & fs & "" & fs & "in" & fs & my clean(bd) & rs
                        end repeat
                    end if
                end try
            end repeat
            try
                set sentList to (messages of sent mailbox whose date sent > cutoff)
                set n to count of sentList
                if n > sentScan then set n to sentScan
                if n > 0 then
                    set sids to message id of sentList
                    set ssnds to sender of sentList
                    set sdts to date sent of sentList
                    if mode is "0" then
                        set ssubs to {}
                    else
                        set ssubs to subject of sentList
                    end if
                    repeat with k from 1 to n
                        set m to item k of sentList
                        set t to ""
                        set c to ""
                        try
                            set t to my addresses(to recipients of m)
                        end try
                        try
                            set c to my addresses(cc recipients of m)
                        end try
                        if my matches(t & "," & c, handles) then
                            set sb to ""
                            if (count of ssubs) >= k then set sb to item k of ssubs
                            set bd to ""
                            if mode is "2" then
                                try
                                    set bd to content of m
                                    if (length of bd) > 400 then set bd to text 1 thru 400 of bd
                                end try
                            end if
                            set out to out & my clean(item k of sids) & fs & my clean(sb) & fs & my clean(item k of ssnds) & fs & my isot(item k of sdts) & fs & my clean(t) & fs & my clean(c) & fs & "out" & fs & my clean(bd) & rs
                        end if
                    end repeat
                end if
            end try
        end tell
    end timeout
    return out
end run"#;

/// What Mail answered: the owner's own addresses, and the messages.
#[derive(Default)]
struct Answer {
    owner: Vec<String>,
    messages: Vec<Fetched>,
}

struct Fetched {
    message_id: String,
    subject: String,
    sender_name: String,
    sender_handle: String,
    received_at: String,
    to: Vec<String>,
    cc: Vec<String>,
    direction: String,
    snippet: String,
}

fn split_sender(s: &str) -> (String, String) {
    match s.rfind('<') {
        Some(start) => (s[..start].trim().trim_matches('"').to_owned(), s[start + 1..].trim_end().trim_end_matches('>').trim().to_lowercase()),
        None => (String::new(), s.trim().to_lowercase()),
    }
}

fn address_list(s: &str) -> Vec<String> {
    s.split(',').map(|a| a.trim().to_lowercase()).filter(|a| a.contains('@')).collect()
}

fn snippet_of(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(SNIPPET).collect()
}

fn parse_rows(text: &str) -> Answer {
    let mut answer = Answer::default();
    for r in text.split(RS).filter(|r| !r.trim().is_empty()) {
        let f: Vec<&str> = r.split(FS).collect();
        if f.first() == Some(&"ME") {
            answer.owner = address_list(f.get(1).unwrap_or(&""));
            continue;
        }
        if f.len() < 8 {
            continue;
        }
        let message_id = f[0].trim().trim_start_matches('<').trim_end_matches('>').to_owned();
        if message_id.is_empty() {
            continue;
        }
        let (sender_name, sender_handle) = split_sender(f[2]);
        answer.messages.push(Fetched {
            message_id,
            subject: f[1].trim().chars().take(500).collect(),
            sender_name,
            sender_handle,
            received_at: f[3].trim().to_owned(),
            to: address_list(f[4]),
            cc: address_list(f[5]),
            direction: if f[6].trim() == "out" { "out".into() } else { "in".into() },
            snippet: snippet_of(f[7]),
        });
    }
    // The newest of each address first; the inbox keeps a hundred an address.
    answer.messages.sort_by(|a, b| b.received_at.cmp(&a.received_at));
    let mut per_sender: HashMap<String, usize> = HashMap::new();
    answer.messages.retain(|m| {
        if m.direction == "out" {
            return true;
        }
        let seen = per_sender.entry(m.sender_handle.clone()).or_insert(0);
        *seen += 1;
        *seen <= PER_HANDLE
    });
    answer
}

/// Mail's answer for these addresses: the state, what it answered, and a word for the owner
/// when Mail couldn't be asked. An address at a domain also asked for is left out (the domain
/// covers it); one script runs at a time.
async fn read_mail(handles: &[String], visibility: &str) -> Result<(MailState, Answer, String)> {
    let mode = match visibility {
        "metadata" => "0",
        "everything" => "2",
        _ => "1",
    };
    let domains: Vec<&String> = handles.iter().filter(|h| h.starts_with('@')).collect();
    let asked: Vec<&String> = handles.iter().filter(|h| h.starts_with('@') || !domains.iter().any(|d| h.ends_with(d.as_str()))).collect();
    let _one_at_a_time = MAIL.lock().await;
    let mut cmd = Command::new("/usr/bin/osascript");
    cmd.arg("-e").arg(SCRIPT).arg(mode).arg(SENT_SCAN.to_string()).arg(DAYS_BACK.to_string());
    for h in asked {
        cmd.arg(h);
    }
    let run = cmd.stdin(Stdio::null()).kill_on_drop(true).output();
    let output = match tokio::time::timeout(std::time::Duration::from_secs(90), run).await {
        Ok(output) => output.context("running osascript")?,
        Err(_) => return Ok((MailState::Failed, Answer::default(), "Mail didn't answer in time — try again".into())),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("-1743") {
            return Ok((MailState::NoPermission, Answer::default(), "New Dawn may not read from Mail yet — allow it in System Settings → Privacy & Security → Automation".into()));
        }
        tracing::warn!("reading threads from Mail: {}", stderr.trim());
        return Ok((MailState::Failed, Answer::default(), format!("Mail couldn't answer: {}", stderr.trim().chars().take(200).collect::<String>())));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.trim_end_matches('\n');
    if text == "NOT_RUNNING" {
        return Ok((MailState::NotRunning, Answer::default(), "Open Mail to read the threads — New Dawn never launches it".into()));
    }
    Ok((MailState::Ok, parse_rows(text), String::new()))
}

/// A thread's key: the subject without Re:, Fwd: and their kin, lower case, with the domains
/// of the other side (a "Meeting" with Keppel Land and one with Marina Center are two threads);
/// a message without a subject is a thread of its own. A late cc from a new domain starts a
/// new thread — the price of threads without Mail's own thread ids.
fn thread_key(subject: &str, message_id: &str, participants: &[String], owner_domains: &[String]) -> String {
    let mut s = subject.trim().to_lowercase();
    loop {
        let before = s.len();
        for prefix in ["re:", "fwd:", "fw:", "aw:", "tr:", "wg:", "sv:", "vs:", "ref:"] {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest.trim_start().to_owned();
            }
        }
        if s.len() == before {
            break;
        }
    }
    let subject: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if subject.is_empty() {
        return format!("msg:{message_id}");
    }
    let mut domains: Vec<&str> = participants.iter().filter_map(|h| h.split('@').nth(1)).filter(|d| !owner_domains.iter().any(|o| o == d)).collect();
    domains.sort_unstable();
    domains.dedup();
    format!("{subject}|{}", domains.join(","))
}

/// Keeps what Mail answered in one transaction: threads by key, messages by their Message-ID,
/// participants afresh, the thread's roll-up once; messages touching a blocked address are
/// left out.
async fn store(pool: &SqlitePool, answer: &Answer, blocked: &[String]) -> Result<()> {
    let mut owner_domains: Vec<String> = answer.owner.iter().filter_map(|h| h.split('@').nth(1)).map(str::to_owned).collect();
    owner_domains.extend(answer.messages.iter().filter(|m| m.direction == "out").filter_map(|m| m.sender_handle.split('@').nth(1)).map(str::to_owned));
    owner_domains.sort_unstable();
    owner_domains.dedup();
    let mut tx = pool.begin().await?;
    let mut touched: Vec<String> = Vec::new();
    for f in &answer.messages {
        if is_blocked(blocked, &f.sender_handle) || f.to.iter().chain(f.cc.iter()).any(|h| is_blocked(blocked, h)) {
            continue;
        }
        let people: Vec<String> = std::iter::once(&f.sender_handle).chain(f.to.iter()).chain(f.cc.iter()).cloned().collect();
        let key = thread_key(&f.subject, &f.message_id, &people, &owner_domains);
        let thread: Option<(String,)> = sqlx::query_as("SELECT id FROM message_thread WHERE key = ?").bind(&key).fetch_optional(&mut *tx).await?;
        let thread_id = match thread {
            Some((id,)) => id,
            None => {
                let id = uuid::Uuid::now_v7().to_string();
                sqlx::query("INSERT INTO message_thread (id, key, subject) VALUES (?, ?, ?)").bind(&id).bind(&key).bind(&f.subject).execute(&mut *tx).await?;
                id
            }
        };
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM message WHERE message_id = ?").bind(&f.message_id).fetch_optional(&mut *tx).await?;
        let id = existing.map(|(id,)| id).unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
        sqlx::query(
            "INSERT INTO message (id, thread_id, message_id, subject, sender_handle, sender_name, received_at, direction, mailbox, snippet) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (message_id) DO UPDATE SET subject = excluded.subject, sender_handle = excluded.sender_handle, sender_name = excluded.sender_name, \
             received_at = excluded.received_at, direction = excluded.direction, snippet = excluded.snippet, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
        )
        .bind(&id)
        .bind(&thread_id)
        .bind(&f.message_id)
        .bind(&f.subject)
        .bind(&f.sender_handle)
        .bind(&f.sender_name)
        .bind(&f.received_at)
        .bind(&f.direction)
        .bind(if f.direction == "out" { "Sent" } else { "Inbox" })
        .bind(&f.snippet)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM message_participant WHERE message_id = ?").bind(&id).execute(&mut *tx).await?;
        let mut parts: Vec<(&str, &str, &str)> = vec![("from", f.sender_handle.as_str(), f.sender_name.as_str())];
        parts.extend(f.to.iter().map(|h| ("to", h.as_str(), "")));
        parts.extend(f.cc.iter().map(|h| ("cc", h.as_str(), "")));
        for (role, handle, name) in parts.into_iter().filter(|(_, h, _)| h.contains('@')) {
            sqlx::query("INSERT INTO message_participant (id, message_id, role, handle, display_name) VALUES (?, ?, ?, ?, ?)")
                .bind(uuid::Uuid::now_v7().to_string())
                .bind(&id)
                .bind(role)
                .bind(handle)
                .bind(name)
                .execute(&mut *tx)
                .await?;
        }
        if !touched.contains(&thread_id) {
            touched.push(thread_id);
        }
    }
    for thread_id in touched {
        let names: Vec<(String, String)> = sqlx::query_as(
            "SELECT p.handle, max(p.display_name) FROM message_participant p JOIN message m ON m.id = p.message_id WHERE m.thread_id = ? GROUP BY p.handle ORDER BY p.handle",
        )
        .bind(&thread_id)
        .fetch_all(&mut *tx)
        .await?;
        let participants = Value::Array(names.into_iter().map(|(handle, name)| json!({ "handle": handle, "name": name })).collect());
        sqlx::query(
            "UPDATE message_thread SET subject = CASE WHEN subject = '' THEN coalesce((SELECT subject FROM message WHERE thread_id = ? AND subject <> '' LIMIT 1), '') ELSE subject END, \
             last_message_at = (SELECT coalesce(max(received_at), '') FROM message WHERE thread_id = ?), \
             message_count = (SELECT count(*) FROM message WHERE thread_id = ?), participants_json = ?, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(&thread_id)
        .bind(&thread_id)
        .bind(&thread_id)
        .bind(participants.to_string())
        .bind(&thread_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ---- the record's threads ----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Type)]
pub struct RecordRef {
    /// The object's plural name (the route) and the record.
    pub object: String,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Participant {
    pub handle: String,
    pub name: String,
    pub record: Option<RecordRef>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct EmailThread {
    pub id: String,
    pub subject: String,
    pub last_message_at: String,
    pub count: u32,
    pub participants: Vec<Participant>,
    pub snippet: String,
    /// The newest message's Message-ID — what Open in Mail opens.
    pub latest_message_id: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct EmailThreads {
    pub threads: Vec<EmailThread>,
    /// People on these threads the CRM doesn't know, per Contact auto-creation.
    pub unknown: Vec<Participant>,
    pub refreshed_at: Option<String>,
    pub mail_state: MailState,
    /// A word for the owner when Mail couldn't be asked.
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct EmailMessage {
    pub id: String,
    pub message_id: String,
    pub subject: String,
    pub sender_handle: String,
    pub sender_name: String,
    pub received_at: String,
    pub direction: String,
    pub snippet: String,
}

/// Who an address belongs to: a person by their emails, else their company by its domain.
struct Directory {
    people: HashMap<String, (String, String)>,
    companies: HashMap<String, (String, String)>,
}

impl Directory {
    async fn load(pool: &SqlitePool) -> Result<Self> {
        let mut people = HashMap::new();
        let rows: Vec<(String, String, String, String)> = sqlx::query_as("SELECT id, name, emails_primary, emails_additional FROM person WHERE deleted_at IS NULL").fetch_all(pool).await?;
        for (id, name, primary, additional) in rows {
            let mut handles = Vec::new();
            push_emails(&mut handles, &primary, &additional);
            for h in handles {
                people.entry(h).or_insert((id.clone(), name.clone()));
            }
        }
        let mut companies = HashMap::new();
        let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT id, name, domain_url FROM company WHERE deleted_at IS NULL AND domain_url <> ''").fetch_all(pool).await?;
        for (id, name, url) in rows {
            let host = host_of(&url);
            if host.contains('.') {
                companies.entry(host).or_insert((id, name));
            }
        }
        Ok(Self { people, companies })
    }

    fn record_of(&self, handle: &str) -> Option<RecordRef> {
        if let Some((id, label)) = self.people.get(handle) {
            return Some(RecordRef { object: "people".into(), id: id.clone(), label: label.clone() });
        }
        let domain = handle.split('@').nth(1)?;
        self.companies.get(domain).map(|(id, label)| RecordRef { object: "companies".into(), id: id.clone(), label: label.clone() })
    }
}

/// The threads on which any of the record's addresses took part, newest first — read from Mail
/// again when asked or when the last read is older than ten minutes.
pub async fn threads(pool: &SqlitePool, object: &ObjectDef, id: &str, refresh: bool) -> Result<EmailThreads> {
    let handles = handles_of(pool, object, id).await?;
    if handles.is_empty() {
        return Ok(EmailThreads { threads: Vec::new(), unknown: Vec::new(), refreshed_at: None, mail_state: MailState::Ok, note: "No email address on this record yet".into() });
    }
    let settings = settings(pool).await?;
    let blocked: Vec<String> = blocklist(pool).await?.into_iter().map(|b| b.handle).collect();
    let key = format!("mail:refreshed:{}:{id}", object.meta.name_singular);
    let mut refreshed_at = setting(pool, &key).await?;
    let stale = refreshed_at
        .as_deref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .is_none_or(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_seconds() > STALE_SECS);
    let (mut mail_state, mut note) = (MailState::Ok, String::new());
    if refresh || stale {
        let (state, answer, word) = read_mail(&handles, &settings.visibility).await?;
        mail_state = state;
        note = word;
        if state == MailState::Ok {
            store(pool, &answer, &blocked).await?;
            if !answer.owner.is_empty() {
                set_setting(pool, "mail:owner", &answer.owner.join(",")).await?;
            }
            let now = chrono::Utc::now().to_rfc3339();
            set_setting(pool, &key, &now).await?;
            refreshed_at = Some(now);
        }
    }
    // The threads the addresses are on: exact addresses, and anyone at a domain.
    let (exact, domains): (Vec<&String>, Vec<&String>) = handles.iter().partition(|h| !h.starts_with('@'));
    let mut clauses: Vec<String> = Vec::new();
    if !exact.is_empty() {
        clauses.push(format!("p.handle IN ({})", vec!["?"; exact.len()].join(", ")));
    }
    clauses.extend(domains.iter().map(|_| "p.handle LIKE ?".to_owned()));
    let sql = format!(
        "SELECT t.id, t.subject, t.last_message_at, t.message_count, t.participants_json, \
         coalesce((SELECT snippet FROM message m2 WHERE m2.thread_id = t.id ORDER BY m2.received_at DESC LIMIT 1), '') AS snippet, \
         coalesce((SELECT message_id FROM message m2 WHERE m2.thread_id = t.id ORDER BY m2.received_at DESC LIMIT 1), '') AS latest \
         FROM message_thread t WHERE t.id IN (SELECT m.thread_id FROM message m JOIN message_participant p ON p.message_id = m.id WHERE {}) \
         ORDER BY t.last_message_at DESC LIMIT 50",
        clauses.join(" OR ")
    );
    let mut q = sqlx::query_as::<_, (String, String, String, i64, String, String, String)>(sqlx::AssertSqlSafe(sql));
    for h in &exact {
        q = q.bind((*h).clone());
    }
    for d in &domains {
        q = q.bind(format!("%{d}"));
    }
    let rows = q.fetch_all(pool).await?;
    let directory = Directory::load(pool).await?;
    // The owner's own addresses: Mail's accounts (kept at the last read), and whoever sent mail.
    let owner: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT sender_handle FROM message WHERE direction = 'out'").fetch_all(pool).await?;
    let mut owner: Vec<String> = owner.into_iter().map(|(h,)| h).collect();
    owner.extend(address_list(&setting(pool, "mail:owner").await?.unwrap_or_default()));
    let written_to: Vec<(String,)> = sqlx::query_as("SELECT DISTINCT p.handle FROM message_participant p JOIN message m ON m.id = p.message_id WHERE m.direction = 'out' AND p.role <> 'from'").fetch_all(pool).await?;
    let written_to: Vec<String> = written_to.into_iter().map(|(h,)| h).collect();
    let mut unknown: Vec<Participant> = Vec::new();
    let threads = rows
        .into_iter()
        .map(|(id, subject, last_message_at, count, participants_json, snippet, latest)| {
            let participants: Vec<Participant> = serde_json::from_str::<Vec<Value>>(&participants_json)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|p| {
                    let handle = p.get("handle")?.as_str()?.to_lowercase();
                    let name = p.get("name").and_then(Value::as_str).unwrap_or("").to_owned();
                    let record = directory.record_of(&handle);
                    Some(Participant { handle, name, record })
                })
                .collect();
            for p in participants.iter().filter(|p| p.record.as_ref().is_none_or(|r| r.object != "people")) {
                let proposed = match settings.auto_create.as_str() {
                    "deactivated" => false,
                    "sent_only" => written_to.contains(&p.handle),
                    _ => true,
                };
                if proposed && !owner.contains(&p.handle) && !is_blocked(&blocked, &p.handle) && !unknown.iter().any(|u| u.handle == p.handle) {
                    unknown.push(p.clone());
                }
            }
            EmailThread { id, subject, last_message_at, count: count.clamp(0, i64::from(u32::MAX)) as u32, participants, snippet, latest_message_id: latest }
        })
        .collect();
    Ok(EmailThreads { threads, unknown, refreshed_at, mail_state, note })
}

/// A thread's messages, newest first.
pub async fn messages(pool: &SqlitePool, thread_id: &str) -> Result<Vec<EmailMessage>> {
    Ok(sqlx::query_as("SELECT id, message_id, subject, sender_handle, sender_name, received_at, direction, snippet FROM message WHERE thread_id = ? ORDER BY received_at DESC")
        .bind(thread_id)
        .fetch_all(pool)
        .await?)
}

/// A person the CRM didn't know, added from a thread: their name (or the address's local part)
/// and address, and their company when one owns the domain.
pub async fn add_person(pool: &SqlitePool, handle: &str, name: &str, actor: &Actor) -> Result<Value> {
    let handle = handle.trim().to_lowercase();
    if !handle.contains('@') {
        return Err(invalid("An email address"));
    }
    let person = records::object(pool, "person").await?;
    let name = if name.trim().is_empty() { handle.split('@').next().unwrap_or("").replace(['.', '_'], " ") } else { name.trim().to_owned() };
    let (first, last) = records::split_name(&name);
    let mut data = json!({ "name": { "firstName": first, "lastName": last }, "emails": { "primaryEmail": handle } });
    let directory = Directory::load(pool).await?;
    if let Some(domain) = handle.split('@').nth(1) {
        if let Some((company_id, _)) = directory.companies.get(domain) {
            data["company"] = Value::String(company_id.clone());
        }
    }
    query::create(pool, &person, &data, actor).await
}

/// A record's calendar: a person's meetings; a company's people's; an opportunity's company's.
pub async fn calendar(pool: &SqlitePool, object: &ObjectDef, id: &str) -> Result<Vec<CalendarEvent>> {
    let people: Vec<String> = match object.meta.name_singular.as_str() {
        "person" => vec![id.to_owned()],
        "company" => sqlx::query_as::<_, (String,)>("SELECT id FROM person WHERE company_id = ? AND deleted_at IS NULL").bind(id).fetch_all(pool).await?.into_iter().map(|(i,)| i).collect(),
        "opportunity" => {
            let row: Option<(Option<String>,)> = sqlx::query_as("SELECT company_id FROM opportunity WHERE id = ?").bind(id).fetch_optional(pool).await?;
            match row {
                Some((Some(company),)) => sqlx::query_as::<_, (String,)>("SELECT id FROM person WHERE company_id = ? AND deleted_at IS NULL").bind(company).fetch_all(pool).await?.into_iter().map(|(i,)| i).collect(),
                _ => Vec::new(),
            }
        }
        _ => Vec::new(),
    };
    if people.is_empty() {
        return Ok(Vec::new());
    }
    crm::calendar(pool, &people).await
}

/// Whether Mail is running on this Mac (the Accounts pane's status; nothing is launched).
pub async fn mail_running() -> bool {
    Command::new("/usr/bin/pgrep").arg("-x").arg("Mail").stdin(Stdio::null()).output().await.map(|o| o.status.success()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    #[test]
    fn subjects_senders_and_blocks_read_right() {
        let people = vec!["marina@keppel.com".to_owned(), "me@majoris.co".to_owned(), "duc@keppel.com".to_owned()];
        let mine = vec!["majoris.co".to_owned()];
        assert_eq!(thread_key("Re: RE: Fwd: Lease terms  ", "x", &people, &mine), "lease terms|keppel.com");
        assert_eq!(thread_key("Meeting", "x", &["a@marina.vn".to_owned()], &mine), "meeting|marina.vn", "the same subject with another company is another thread");
        assert_eq!(thread_key("", "m9@x", &people, &mine), "msg:m9@x", "no subject: a thread of its own");
        assert_eq!(split_sender("Marina Tran <Marina@Keppel.com>"), ("Marina Tran".into(), "marina@keppel.com".into()));
        assert_eq!(split_sender("duc@keppel.com"), (String::new(), "duc@keppel.com".into()));
        assert!(is_blocked(&["@spam.example".into()], "x@spam.example"));
        assert!(is_blocked(&["bob@x.com".into()], "bob@x.com"));
        assert!(!is_blocked(&["@spam.example".into()], "x@fine.example"));
        assert_eq!(host_of(concat!("https", "://www.Keppel.com/about")), "keppel.com");
        let answer = parse_rows(&format!("ME{FS}Me@Majoris.co,me2@majoris.co{RS}<a1@x>{FS}Re: Lease{FS}Marina Tran <marina@keppel.com>{FS}2026-11-02T10:00:00{FS}me@majoris.co{FS}{FS}in{FS}Hi   there{RS}<a0@x>{FS}Older{FS}marina@keppel.com{FS}2026-10-02T10:00:00{FS}{FS}{FS}in{FS}{RS}"));
        assert_eq!(answer.owner, ["me@majoris.co", "me2@majoris.co"]);
        assert_eq!(answer.messages.len(), 2);
        assert_eq!(answer.messages[0].message_id, "a1@x", "newest first");
        assert_eq!(answer.messages[0].snippet, "Hi there");
    }

    /// What Mail answered is kept as threads and participants; a person's threads come back
    /// newest first with the people linked; an unknown address is proposed (or not, per the
    /// setting, or when blocked); adding them makes a person on the company of their domain.
    #[test]
    fn threads_are_kept_linked_and_proposed() {
        let root = std::env::temp_dir().join(format!("newdawn-mail-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::system("test");
            let company = records::object(&pool, "company").await.unwrap();
            let keppel = query::create(&pool, &company, &json!({ "name": "Keppel Land", "domainName": { "primaryLinkUrl": "keppel.com" } }), &actor).await.unwrap();
            let person = records::object(&pool, "person").await.unwrap();
            let marina = query::create(&pool, &person, &json!({ "name": { "firstName": "Marina", "lastName": "Tran" }, "emails": { "primaryEmail": "marina@keppel.com", "additionalEmails": ["mt@gmail.example"] }, "company": keppel["id"] }), &actor).await.unwrap();
            let marina_id = marina["id"].as_str().unwrap().to_owned();
            assert_eq!(handles_of(&pool, &person, &marina_id).await.unwrap(), ["marina@keppel.com", "mt@gmail.example"]);
            let mut company_handles = handles_of(&pool, &company, keppel["id"].as_str().unwrap()).await.unwrap();
            company_handles.sort();
            assert_eq!(company_handles, ["@keppel.com", "marina@keppel.com", "mt@gmail.example"]);
            let fetched = vec![
                Fetched { message_id: "m1@x".into(), subject: "Lease terms".into(), sender_name: "Marina Tran".into(), sender_handle: "marina@keppel.com".into(), received_at: "2026-11-01T09:00:00".into(), to: vec!["me@majoris.co".into()], cc: vec!["duc@keppel.com".into()], direction: "in".into(), snippet: "Please find the terms".into() },
                Fetched { message_id: "m2@x".into(), subject: "Re: Lease terms".into(), sender_name: "Tien".into(), sender_handle: "me@majoris.co".into(), received_at: "2026-11-02T09:00:00".into(), to: vec!["marina@keppel.com".into()], cc: vec![], direction: "out".into(), snippet: "Thanks, agreed".into() },
                Fetched { message_id: "m3@x".into(), subject: "Newsletter".into(), sender_name: "".into(), sender_handle: "news@spam.example".into(), received_at: "2026-11-02T10:00:00".into(), to: vec!["mt@gmail.example".into()], cc: vec![], direction: "in".into(), snippet: "Buy".into() },
            ];
            block(&pool, "@spam.example").await.unwrap();
            let blocked: Vec<String> = blocklist(&pool).await.unwrap().into_iter().map(|b| b.handle).collect();
            store(&pool, &Answer { owner: vec!["me@majoris.co".into()], messages: fetched }, &blocked).await.unwrap();
            set_setting(&pool, "mail:owner", "me@majoris.co").await.unwrap();
            let (threads,): (i64,) = sqlx::query_as("SELECT count(*) FROM message_thread").fetch_one(&pool).await.unwrap();
            assert_eq!(threads, 1, "the two lease messages share a thread; the blocked newsletter is left out");
            // Fresh enough: no Mail read (there is none in a test), the rows come from the cache.
            set_setting(&pool, &format!("mail:refreshed:person:{marina_id}"), &chrono::Utc::now().to_rfc3339()).await.unwrap();
            let got = threads_for_test(&pool, &person, &marina_id).await;
            assert_eq!(got.threads.len(), 1);
            let t = &got.threads[0];
            assert_eq!((t.subject.as_str(), t.count, t.latest_message_id.as_str()), ("Lease terms", 2, "m2@x"));
            assert_eq!(t.snippet, "Thanks, agreed");
            let marina_part = t.participants.iter().find(|p| p.handle == "marina@keppel.com").unwrap();
            assert_eq!(marina_part.record.as_ref().map(|r| r.label.as_str()), Some("Marina Tran"));
            let duc = t.participants.iter().find(|p| p.handle == "duc@keppel.com").unwrap();
            assert_eq!(duc.record.as_ref().map(|r| (r.object.as_str(), r.label.as_str())), Some(("companies", "Keppel Land")), "an unknown address at a known domain points at the company");
            assert_eq!(got.unknown.iter().map(|u| u.handle.as_str()).collect::<Vec<_>>(), ["duc@keppel.com"], "the owner's own address is never proposed");
            set_settings(&pool, &MailSettings { auto_create: "sent_only".into(), visibility: "subject".into() }).await.unwrap();
            assert!(threads_for_test(&pool, &person, &marina_id).await.unknown.is_empty(), "nothing was sent to Duc");
            set_settings(&pool, &MailSettings { auto_create: "deactivated".into(), visibility: "everything".into() }).await.unwrap();
            assert!(threads_for_test(&pool, &person, &marina_id).await.unknown.is_empty());
            assert!(set_settings(&pool, &MailSettings { auto_create: "maybe".into(), visibility: "subject".into() }).await.is_err());
            let added = add_person(&pool, "duc@keppel.com", "", &actor).await.unwrap();
            assert_eq!(added["name"]["firstName"], "duc");
            assert_eq!(added["company"]["label"], "Keppel Land");
            assert_eq!(messages(&pool, &t.id).await.unwrap().len(), 2);
            let company_threads = threads_for_test(&pool, &company, keppel["id"].as_str().unwrap()).await;
            assert_eq!(company_threads.threads.len(), 1, "the company sees its people's and its domain's threads");
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// `threads` without a Mail read: the refresh stamp is fresh, so only the cache is read.
    async fn threads_for_test(pool: &SqlitePool, object: &ObjectDef, id: &str) -> EmailThreads {
        set_setting(pool, &format!("mail:refreshed:{}:{id}", object.meta.name_singular), &chrono::Utc::now().to_rfc3339()).await.unwrap();
        threads(pool, object, id, false).await.unwrap()
    }
}
