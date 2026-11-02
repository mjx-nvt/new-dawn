//! Webhooks (D-071): a record made, changed or deleted is posted to an address on this Mac,
//! signed like Twenty's — `X-Twenty-Webhook-Signature` is the hex HMAC-SHA256 of
//! `{timestamp}:{body}` with the webhook's secret, `X-Twenty-Webhook-Timestamp` the timestamp —
//! with `{ "event", "data", "timestamp" }` as the body; each delivery is logged (the last 50).

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use specta::Type;
use sqlx::SqlitePool;

use super::keys::{hex, random_hex};
use crate::error::AppError;
use crate::kernel::records::events::{self, RecordEvent};
use crate::modules::workflows::run::local_url;

const ACTIONS: [&str; 3] = ["created", "updated", "deleted"];
const TIMEOUT_SECS: u64 = 10;
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// HMAC-SHA256 (RFC 2104) on the `sha2` the app already has.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let (mut inner, mut outer) = (Sha256::new(), Sha256::new());
    inner.update(block.map(|b| b ^ 0x36));
    inner.update(message);
    outer.update(block.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    outer.finalize().into()
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Webhook {
    pub id: String,
    pub url: String,
    pub description: String,
    /// "*.*", "person.*", "opportunity.updated" …
    pub events: Vec<String>,
    /// The signature's key.
    pub secret: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    url: String,
    description: String,
    events_json: String,
    secret: String,
    is_active: bool,
    created_at: String,
}

impl From<Row> for Webhook {
    fn from(r: Row) -> Self {
        Self { id: r.id, url: r.url, description: r.description, events: serde_json::from_str(&r.events_json).unwrap_or_default(), secret: r.secret, is_active: r.is_active, created_at: r.created_at }
    }
}

const COLUMNS: &str = "id, url, description, events_json, secret, is_active, created_at";

fn on() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct WebhookInput {
    #[serde(default)]
    pub id: Option<String>,
    pub url: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default = "on")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Delivery {
    pub id: String,
    pub event: String,
    pub record_id: String,
    pub status_code: Option<u32>,
    pub ok: bool,
    pub error: String,
    pub duration_ms: u32,
    pub created_at: String,
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Webhook>> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM webhook ORDER BY created_at"))).fetch_all(pool).await?;
    Ok(rows.into_iter().map(Webhook::from).collect())
}

/// The events a webhook gets: `<object>.<created|updated|deleted|*>`, `*.*` when none are named.
fn check_events(events: &[String]) -> Result<Vec<String>> {
    let list: Vec<String> = events.iter().map(|e| e.trim().to_owned()).filter(|e| !e.is_empty()).collect();
    if list.is_empty() {
        return Ok(vec!["*.*".into()]);
    }
    for e in &list {
        let ok = e.split_once('.').is_some_and(|(o, a)| !o.is_empty() && (a == "*" || ACTIONS.contains(&a)));
        if !ok {
            return Err(invalid(format!("“{e}” isn't an event — object.created, object.updated, object.deleted, object.* or *.*")));
        }
    }
    Ok(list)
}

pub fn matches(events: &[String], object: &str, action: &str) -> bool {
    events.iter().any(|e| e.split_once('.').is_some_and(|(o, a)| (o == "*" || o == object) && (a == "*" || a == action)))
}

/// A new webhook (a secret made for it) or a changed one; the address must be on this Mac.
pub async fn save(pool: &SqlitePool, input: &WebhookInput) -> Result<Webhook> {
    local_url(&input.url)?;
    let events = json!(check_events(&input.events)?).to_string();
    let id = match input.id.as_deref().filter(|i| !i.is_empty()) {
        Some(id) => {
            let done = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE webhook SET url = ?, description = ?, events_json = ?, is_active = ?, updated_at = {NOW} WHERE id = ?")))
                .bind(input.url.trim())
                .bind(input.description.trim())
                .bind(&events)
                .bind(input.is_active)
                .bind(id)
                .execute(pool)
                .await?;
            if done.rows_affected() == 0 {
                return Err(anyhow!(AppError::not_found("No such webhook")));
            }
            id.to_owned()
        }
        None => {
            let id = uuid::Uuid::now_v7().to_string();
            sqlx::query("INSERT INTO webhook (id, url, description, events_json, secret, is_active) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&id)
                .bind(input.url.trim())
                .bind(input.description.trim())
                .bind(&events)
                .bind(random_hex(24)?)
                .bind(input.is_active)
                .execute(pool)
                .await?;
            id
        }
    };
    let row: Row = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM webhook WHERE id = ?"))).bind(&id).fetch_one(pool).await?;
    Ok(row.into())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM webhook WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn deliveries(pool: &SqlitePool, webhook_id: &str) -> Result<Vec<Delivery>> {
    Ok(sqlx::query_as("SELECT id, event, record_id, status_code, ok, error, duration_ms, created_at FROM webhook_delivery WHERE webhook_id = ? ORDER BY created_at DESC LIMIT 50").bind(webhook_id).fetch_all(pool).await?)
}

/// One client for every delivery: no proxy, no redirect, 10 s.
fn client() -> Result<&'static reqwest::Client> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(c) = CLIENT.get() {
        return Ok(c);
    }
    let made = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(TIMEOUT_SECS)).build()?;
    Ok(CLIENT.get_or_init(|| made))
}

/// Posts one event to a webhook — an address on this Mac only, no proxy, no redirect, 10 s —
/// signed with its secret, and logs the delivery (keeping the last 50).
pub async fn deliver(pool: &SqlitePool, hook: &Webhook, event: &str, record: &Value) -> Result<bool> {
    let timestamp = Utc::now().timestamp_millis().to_string();
    let body = json!({ "event": event, "data": record, "timestamp": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true) }).to_string();
    let signature = hex(&hmac_sha256(hook.secret.as_bytes(), format!("{timestamp}:{body}").as_bytes()));
    let started = Instant::now();
    let sent = async {
        let url = local_url(&hook.url)?;
        let response = client()?
            .post(url)
            .header("content-type", "application/json")
            .header("X-Twenty-Webhook-Signature", &signature)
            .header("X-Twenty-Webhook-Timestamp", &timestamp)
            .body(body)
            .send()
            .await?;
        Ok::<u16, anyhow::Error>(response.status().as_u16())
    }
    .await;
    let (status, ok, error) = match sent {
        Ok(code) if (200..300).contains(&code) => (Some(u32::from(code)), true, String::new()),
        Ok(code) => (Some(u32::from(code)), false, format!("The address answered {code}")),
        Err(e) => (None, false, format!("{e:#}")),
    };
    let record_id = record.get("id").and_then(Value::as_str).unwrap_or("");
    sqlx::query("INSERT INTO webhook_delivery (id, webhook_id, event, record_id, status_code, ok, error, duration_ms) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(&hook.id)
        .bind(event)
        .bind(record_id)
        .bind(status)
        .bind(ok)
        .bind(&error)
        .bind(u32::try_from(started.elapsed().as_millis()).unwrap_or(u32::MAX))
        .execute(pool)
        .await?;
    // The last 50 a webhook (a NULL-safe form: fewer than 50 deletes nothing).
    sqlx::query("DELETE FROM webhook_delivery WHERE webhook_id = ?1 AND created_at < (SELECT created_at FROM webhook_delivery WHERE webhook_id = ?1 ORDER BY created_at DESC LIMIT 1 OFFSET 49)").bind(&hook.id).execute(pool).await?;
    Ok(ok)
}

/// A record event: every active webhook of its workspace that wants it gets it. How many were sent.
pub async fn on_record(event: &RecordEvent) -> Result<u32> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {COLUMNS} FROM webhook WHERE is_active = 1"))).fetch_all(&event.pool).await?;
    let record = event.after.clone().or_else(|| event.before.clone()).unwrap_or(Value::Null);
    let name = format!("{}.{}", event.object, event.event);
    let mut sent = 0;
    for hook in rows.into_iter().map(Webhook::from).filter(|h| matches(&h.events, &event.object, &event.event)) {
        if let Err(e) = deliver(&event.pool, &hook, &name, &record).await {
            tracing::warn!("webhook {}: {e:#}", hook.id);
        }
        sent += 1;
    }
    Ok(sent)
}

/// The listener: each event's deliveries go on their own task, never holding the next one.
pub fn spawn() {
    let mut events = events::subscribe();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = on_record(&event).await {
                    tracing::warn!("webhooks: {e:#}");
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_rfc_4231() {
        assert_eq!(hex(&hmac_sha256(&[0x0b; 20], b"Hi There")), "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
        assert_eq!(hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")), "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
        assert_eq!(hex(&hmac_sha256(&[0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First")), "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54");
    }

    /// A record event reaches a listener on this Mac, signed with the webhook's secret; the
    /// delivery is logged; an address on the network is refused when the webhook is saved.
    #[test]
    fn a_delivery_is_signed_and_logged() {
        use std::io::{Read, Write};
        tauri::async_runtime::block_on(async {
            let root = std::env::temp_dir().join(format!("newdawn-webhooks-{}", uuid::Uuid::now_v7()));
            let hub = crate::kernel::hub::WorkspaceHub::open(root).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let got = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut raw = Vec::new();
                let mut buf = [0u8; 8192];
                loop {
                    let n = stream.read(&mut buf).unwrap();
                    raw.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&raw).to_string();
                    if let Some((head, body)) = text.split_once("\r\n\r\n") {
                        let length = head.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap())).unwrap_or(0);
                        if body.len() >= length {
                            stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").unwrap();
                            return text;
                        }
                    }
                }
            });
            let away = concat!("https", "://example.com/hook");
            assert!(save(&pool, &WebhookInput { id: None, url: away.into(), description: String::new(), events: vec![], is_active: true }).await.is_err());
            let hook = save(&pool, &WebhookInput { id: None, url: format!("http://127.0.0.1:{port}/hook"), description: "test".into(), events: vec!["person.*".into()], is_active: true }).await.unwrap();
            assert!(deliver(&pool, &hook, "person.updated", &json!({ "id": "p1", "jobTitle": "CFO" })).await.unwrap());
            let request = got.join().unwrap();
            let (head, body) = request.split_once("\r\n\r\n").unwrap();
            let header = |name: &str| head.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_owned())).unwrap();
            let timestamp = header("x-twenty-webhook-timestamp");
            assert_eq!(header("x-twenty-webhook-signature"), hex(&hmac_sha256(hook.secret.as_bytes(), format!("{timestamp}:{body}").as_bytes())));
            let sent: Value = serde_json::from_str(body).unwrap();
            assert_eq!((sent["event"].as_str(), sent["data"]["jobTitle"].as_str()), (Some("person.updated"), Some("CFO")));
            let log = deliveries(&pool, &hook.id).await.unwrap();
            assert_eq!((log.len(), log[0].ok, log[0].status_code, log[0].record_id.as_str()), (1, true, Some(200), "p1"));
        });
    }

    #[test]
    fn events_match_by_object_and_action() {
        let events = vec!["person.created".to_owned(), "opportunity.*".to_owned()];
        assert!(matches(&events, "person", "created") && !matches(&events, "person", "updated"));
        assert!(matches(&events, "opportunity", "deleted"));
        assert!(matches(&["*.*".to_owned()], "company", "updated"));
        assert!(check_events(&["person.moved".to_owned()]).is_err());
        assert_eq!(check_events(&[]).unwrap(), ["*.*"]);
    }
}
