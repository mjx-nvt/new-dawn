//! Plane's webhook events (D-084): what the Projects module tells its listeners once a change is
//! written — a project, work item, cycle, module or comment made, changed or deleted. Sent after
//! the transaction commits, so a listener reads what was written. A bounded channel each listener
//! (the webhooks); a full one drops the event with a warning rather than hold a write.

use std::sync::{Mutex, PoisonError};

use sqlx::SqlitePool;
use tokio::sync::mpsc;

/// Plane's event names.
pub const KINDS: [&str; 5] = ["project", "issue", "cycle", "module", "issue_comment"];

/// A work item's field changed: its name, the old and the new value as the activity reads them,
/// and the ids behind them when there are (a state's, a parent's …).
#[derive(Debug, Clone)]
pub struct Change {
    pub field: String,
    pub old: String,
    pub new: String,
    pub old_id: Option<String>,
    pub new_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PmEvent {
    pub pool: SqlitePool,
    /// project · issue · cycle · module · issue_comment
    pub kind: &'static str,
    /// created · updated · deleted
    pub action: &'static str,
    pub id: String,
    pub change: Option<Change>,
    /// "me", an agent's name, an API token's.
    pub actor: String,
}

static SENDERS: Mutex<Vec<mpsc::Sender<PmEvent>>> = Mutex::new(Vec::new());

tokio::task_local! {
    /// Set while an import runs: its rows are made quietly, as Plane's bulk import makes them.
    static QUIET: ();
}

/// Runs `work` without telling anyone of what it changes (an import's rows, links and comments).
pub async fn quietly<F: std::future::Future>(work: F) -> F::Output {
    QUIET.scope((), work).await
}

/// A listener's own channel (1024 events deep).
pub fn subscribe() -> mpsc::Receiver<PmEvent> {
    let (tx, rx) = mpsc::channel(1024);
    SENDERS.lock().unwrap_or_else(PoisonError::into_inner).push(tx);
    rx
}

fn send(event: PmEvent) {
    if QUIET.try_with(|_| ()).is_ok() {
        return;
    }
    let mut senders = SENDERS.lock().unwrap_or_else(PoisonError::into_inner);
    // A listener that's gone (a test's) is forgotten.
    senders.retain(|tx| !tx.is_closed());
    for tx in senders.iter() {
        if let Err(e) = tx.try_send(event.clone()) {
            tracing::warn!("project event dropped: {e}");
        }
    }
}

/// A thing made, changed or deleted.
pub fn emit(pool: &SqlitePool, kind: &'static str, action: &'static str, id: &str, actor: &str) {
    send(PmEvent { pool: pool.clone(), kind, action, id: id.to_owned(), change: None, actor: actor.to_owned() });
}

/// A work item's field changed.
pub fn changed(pool: &SqlitePool, id: &str, change: Change, actor: &str) {
    send(PmEvent { pool: pool.clone(), kind: "issue", action: "updated", id: id.to_owned(), change: Some(change), actor: actor.to_owned() });
}
