//! What starts a run (D-068): a record event from the engine's channel, a schedule's time, or
//! the owner's hand (`super::start_manual`). One task listens to the channel; one ticks every
//! minute over the open workspaces — the schedules due (one run however many ticks were
//! missed, so a launch catches up without a flood) and the delays over.

use anyhow::Result;
use chrono::{DateTime, Local, SecondsFormat, Utc};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::{run, Trigger};
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::events::{self, RecordEvent};

/// A record made, changed or deleted: every active RECORD trigger that watches it starts a run.
/// A workflow's own writes start nothing (no loops).
pub async fn on_record(pool: &SqlitePool, event: &RecordEvent) -> Result<u32> {
    if event.source == "WORKFLOW" {
        return Ok(0);
    }
    // A workflow in Deleted records answers nothing until it's restored.
    let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT t.workflow_id, t.version_id, t.settings_json FROM workflow_automated_trigger t JOIN workflow w ON w.id = t.workflow_id AND w.deleted_at IS NULL WHERE t.kind = 'RECORD'").fetch_all(pool).await?;
    let mut started = 0;
    for (workflow_id, version_id, settings) in rows {
        let settings: Value = serde_json::from_str(&settings).unwrap_or(Value::Null);
        if settings.get("object").and_then(Value::as_str) != Some(event.object.as_str()) {
            continue;
        }
        let wanted = settings.get("event").and_then(Value::as_str).unwrap_or("");
        let fits = match event.event.as_str() {
            "created" => matches!(wanted, "CREATED" | "CREATED_OR_UPDATED"),
            "updated" => {
                let watched: Vec<&str> = settings.get("fields").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
                let changed = watched.is_empty() || watched.iter().any(|f| event.before.as_ref().and_then(|b| b.get(f)) != event.after.as_ref().and_then(|a| a.get(f)));
                matches!(wanted, "UPDATED" | "CREATED_OR_UPDATED") && changed
            }
            "deleted" => wanted == "DELETED",
            _ => false,
        };
        if !fits {
            continue;
        }
        let record = event.after.clone().or_else(|| event.before.clone()).unwrap_or(Value::Null);
        let trigger = json!({ "record": record, "before": event.before, "event": event.event });
        // One workflow's trouble never stops the others'.
        match run::start(pool, &workflow_id, &version_id, trigger).await {
            Ok(run_id) => {
                if let Err(e) = run::execute(pool, &run_id).await {
                    tracing::warn!("workflow run {run_id} on {} {}: {e:#}", event.object, event.id);
                }
                started += 1;
            }
            Err(e) => tracing::warn!("workflow {workflow_id} on {} {}: {e:#}", event.object, event.id),
        }
    }
    Ok(started)
}

/// A workflow's webhook trigger (D-071): a run of the active version whose trigger is a
/// webhook, the request's JSON as its trigger data; the run goes on by itself. None when the
/// workflow has no such trigger (or is in Deleted records).
pub async fn on_webhook(pool: &SqlitePool, workflow_id: &str, body: Value) -> Result<Option<String>> {
    let version: Option<(String,)> = sqlx::query_as("SELECT t.version_id FROM workflow_automated_trigger t JOIN workflow w ON w.id = t.workflow_id AND w.deleted_at IS NULL WHERE t.kind = 'WEBHOOK' AND t.workflow_id = ?")
        .bind(workflow_id)
        .fetch_optional(pool)
        .await?;
    let Some((version_id,)) = version else { return Ok(None) };
    let run_id = run::start(pool, workflow_id, &version_id, body).await?;
    let (pool, run) = (pool.clone(), run_id.clone());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run::execute(&pool, &run).await {
            tracing::warn!("webhook run {run}: {e:#}");
        }
    });
    Ok(Some(run_id))
}

/// A minute's tick for one workspace: the runs a quit or a crash stopped mid-way (untouched
/// since before this launch) are marked failed — Retry goes on from their step, rather than
/// running a half-done step twice by itself; the schedules due start a run and move on; the
/// delays over go on. How many runs were started or resumed.
pub async fn tick(pool: &SqlitePool, launched: DateTime<Utc>) -> Result<u32> {
    sqlx::query(
        "UPDATE workflow_run SET status = 'FAILED', error = 'Stopped when New Dawn quit — Retry goes on from this step', \
         ended_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE status IN ('RUNNING', 'NOT_STARTED') AND updated_at < ?",
    )
    .bind(launched.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
    .execute(pool)
    .await?;
    let now = Utc::now();
    let due: Vec<(String, String, String, String)> = sqlx::query_as("SELECT t.id, t.workflow_id, t.version_id, t.settings_json FROM workflow_automated_trigger t JOIN workflow w ON w.id = t.workflow_id AND w.deleted_at IS NULL WHERE t.kind = 'SCHEDULE' AND (t.next_at IS NULL OR t.next_at <= ?)")
        .bind(now.to_rfc3339_opts(SecondsFormat::Secs, true))
        .fetch_all(pool)
        .await?;
    let mut n = 0;
    for (id, workflow_id, version_id, settings) in due {
        let t: Trigger = serde_json::from_str::<Value>(&settings).ok().map(|v| Trigger { kind: "SCHEDULE".into(), every: v.get("every").and_then(Value::as_u64).unwrap_or(0) as u32, unit: v.get("unit").and_then(Value::as_str).unwrap_or("").to_owned(), cron: v.get("cron").and_then(Value::as_str).unwrap_or("").to_owned(), ..Default::default() }).unwrap_or_default();
        let next = super::next_time(&t, Local::now()).map(|d| d.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Secs, true));
        // The slot is taken by moving its time on: a tick that overlaps this one finds it gone.
        let taken = sqlx::query("UPDATE workflow_automated_trigger SET next_at = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ? AND (next_at IS NULL OR next_at <= ?)")
            .bind(next)
            .bind(&id)
            .bind(now.to_rfc3339_opts(SecondsFormat::Secs, true))
            .execute(pool)
            .await?;
        if taken.rows_affected() == 0 {
            continue;
        }
        match run::start(pool, &workflow_id, &version_id, json!({ "scheduled_at": now.to_rfc3339_opts(SecondsFormat::Secs, true) })).await {
            Ok(run_id) => {
                if let Err(e) = run::execute(pool, &run_id).await {
                    tracing::warn!("scheduled run {run_id}: {e:#}");
                }
                n += 1;
            }
            Err(e) => tracing::warn!("scheduled workflow {workflow_id}: {e:#}"),
        }
    }
    n += run::resume_due(pool, now).await?;
    Ok(n)
}

/// The listener on the engine's record events, and the minute's tick over every workspace.
pub fn spawn(app: &AppHandle) {
    super::set_app(app);
    let launched = Utc::now();
    {
        let mut events = events::subscribe();
        tauri::async_runtime::spawn(async move {
            // Each event's runs go on their own task: a slow step (an AI turn behind a meeting's
            // summary, an HTTP call) never holds the next event up.
            while let Some(event) = events.recv().await {
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = on_record(&event.pool, &event).await {
                        tracing::warn!("record trigger: {e:#}");
                    }
                });
            }
        });
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        loop {
            let hub = handle.state::<WorkspaceHub>();
            if let Ok(workspaces) = hub.list().await {
                for ws in workspaces {
                    if let Ok(pool) = hub.pool(&ws.slug) {
                        // On its own task, like the events: one slow run never holds the minute.
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = tick(&pool, launched).await {
                                tracing::warn!("workflow tick for {}: {e:#}", ws.slug);
                            }
                        });
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
