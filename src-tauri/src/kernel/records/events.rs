//! Record events (D-068): what the engine tells the rest of the app when a record is made,
//! changed or deleted — a bounded channel for each listener (the workflows, the webhooks of
//! D-071). The engine never calls a module itself.

use std::sync::{Mutex, PoisonError};

use serde_json::Value;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

/// A record made, changed or deleted, with the pool it lives in.
#[derive(Debug, Clone)]
pub struct RecordEvent {
    pub pool: SqlitePool,
    /// The object's singular name.
    pub object: String,
    /// created · updated · deleted
    pub event: String,
    pub id: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    /// The actor's source (MANUAL · WORKFLOW · IMPORT …): a workflow's own writes fire nothing.
    pub source: String,
}

static SENDERS: Mutex<Vec<mpsc::Sender<RecordEvent>>> = Mutex::new(Vec::new());

/// A listener's own channel (1024 events deep).
pub fn subscribe() -> mpsc::Receiver<RecordEvent> {
    let (tx, rx) = mpsc::channel(1024);
    SENDERS.lock().unwrap_or_else(PoisonError::into_inner).push(tx);
    rx
}

/// Tells whoever listens; nothing happens when no one does, and a full channel drops the event
/// with a warning rather than blocking the engine.
// ponytail: a workflow's own writes and an import's rows aren't announced — a run would only
// skip the first, and a large import would overrun the bound of 1024; a batch event per import
// is the upgrade when a workflow should answer imports (D-068).
pub fn emit(event: RecordEvent) {
    if matches!(event.source.as_str(), "WORKFLOW" | "IMPORT") {
        return;
    }
    let mut senders = SENDERS.lock().unwrap_or_else(PoisonError::into_inner);
    // A listener that's gone (a test's) is forgotten.
    senders.retain(|tx| !tx.is_closed());
    for tx in senders.iter() {
        if let Err(e) = tx.try_send(event.clone()) {
            tracing::warn!("record event dropped: {e}");
        }
    }
}
