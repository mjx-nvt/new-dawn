use tokio::sync::{Mutex, MutexGuard};

/// One AI job at a time (CLAUDE.md "Jobs run one at a time"): hold the guard for the whole job.
#[derive(Default)]
pub struct AiQueue(Mutex<()>);

impl AiQueue {
    pub async fn acquire(&self) -> MutexGuard<'_, ()> {
        self.0.lock().await
    }
}
