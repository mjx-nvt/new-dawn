use tokio::sync::{Mutex, MutexGuard};

/// One AI job at a time (CLAUDE.md "Jobs run one at a time"): hold the guard for the whole job.
#[derive(Default)]
pub struct AiQueue(Mutex<()>);

impl AiQueue {
    pub async fn acquire(&self) -> MutexGuard<'_, ()> {
        self.0.lock().await
    }

    /// The turn, only if nobody holds it now (a live session never waits behind a job, D-041).
    pub fn try_acquire(&self) -> Option<MutexGuard<'_, ()>> {
        self.0.try_lock().ok()
    }
}
