//! One AI job at a time (CLAUDE.md "Jobs run one at a time"), in three lanes (D-048): what the
//! owner is waiting for first, then the meeting just recorded, then catch-up work. A long job
//! holds its turn between checkpoints and makes way at the next one when a higher lane waits.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use specta::Type;
use tokio::sync::oneshot;

/// Priority order: a lower lane waits for every higher one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    /// The owner is waiting for it (Corrections, Rewrite email, a slogan's translation).
    Interactive,
    /// The meeting just recorded, or a run the owner started (transcription, summary).
    Foreground,
    /// Catch-up work that waits for an idle, cool, charged Mac.
    Background,
}

/// A meeting's long job: the activity list shows its progress and can stop it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Transcription,
    Summary,
}

/// What holds or waits for a turn, for the AI activity list.
#[derive(Debug, Clone, Serialize, Type)]
pub struct Task {
    /// "Transcribing “Weekly sync”", "Applying corrections"…
    pub label: String,
    pub workspace_slug: Option<String>,
    pub meeting_id: Option<String>,
    pub kind: Option<JobKind>,
    /// A catch-up job the activity list can run now (a model check's id).
    pub id: Option<String>,
}

impl Task {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            workspace_slug: None,
            meeting_id: None,
            kind: None,
            id: None,
        }
    }

    pub fn meeting(label: impl Into<String>, slug: &str, meeting_id: &str) -> Self {
        Self {
            workspace_slug: Some(slug.to_owned()),
            meeting_id: Some(meeting_id.to_owned()),
            ..Self::new(label)
        }
    }

    pub fn job(kind: JobKind, label: impl Into<String>, slug: &str, meeting_id: &str) -> Self {
        Self {
            kind: Some(kind),
            ..Self::meeting(label, slug, meeting_id)
        }
    }
}

struct Waiter {
    lane: Lane,
    /// 0 = the front of its lane (a job coming back after making way); else arrival order.
    seq: u64,
    task: Task,
    wake: oneshot::Sender<()>,
}

#[derive(Default)]
struct State {
    holder: Option<(Lane, Task)>,
    waiting: Vec<Waiter>,
    seq: u64,
    /// Catch-up jobs held back before they queue, and why (`resources::acquire`).
    held_back: Vec<(u64, Task, &'static str)>,
    /// Catch-up jobs the owner said to run now, by task id.
    now: HashSet<String>,
    /// Whether a recording runs (set at launch: the recorder lives outside the AI kernel).
    recording: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

#[derive(Default)]
pub struct AiQueue(Mutex<State>);

/// One entry of the AI activity list.
#[derive(Debug, Clone, Serialize, Type)]
pub struct Activity {
    pub lane: Lane,
    pub task: Task,
    pub running: bool,
    /// Why a catch-up job hasn't queued yet (low battery, the owner at the Mac…).
    pub waiting_for: Option<String>,
}

impl AiQueue {
    fn state(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits for a turn in `lane` behind every higher lane and everyone earlier in its own.
    pub async fn acquire(&self, lane: Lane, task: Task) -> Turn<'_> {
        self.wait(lane, task, false).await
    }

    async fn wait(&self, lane: Lane, task: Task, front: bool) -> Turn<'_> {
        let rx = {
            let mut state = self.state();
            if state.holder.is_none() && state.waiting.is_empty() {
                state.holder = Some((lane, task));
                None
            } else {
                Some(enqueue(&mut state, lane, task, front))
            }
        };
        if let Some(rx) = rx {
            self.until_woken(rx).await;
        }
        Turn {
            queue: self,
            lane,
            held: true,
        }
    }

    /// Returns once `rx` is woken: this caller then holds the turn.
    async fn until_woken(&self, rx: oneshot::Receiver<()>) {
        let mut pending = Pending {
            queue: self,
            rx: Some(rx),
        };
        let rx = pending.rx.as_mut().expect("set above");
        // The sender is only dropped after a send (`hand_on`), so an error can't happen; treat
        // it as woken either way.
        let _ = rx.await;
        pending.rx = None;
    }

    /// The turn, only if nobody holds or waits for it now (a live session never waits behind a
    /// job, D-041; the idle unloader never delays one).
    pub fn try_acquire(&self, lane: Lane, task: Task) -> Option<Turn<'_>> {
        let mut state = self.state();
        if state.holder.is_some() || !state.waiting.is_empty() {
            return None;
        }
        state.holder = Some((lane, task));
        Some(Turn {
            queue: self,
            lane,
            held: true,
        })
    }

    fn release(&self) {
        hand_on(&mut self.state());
    }

    /// Someone in a higher lane than `lane` waits (a blocking batch checks it between stretches).
    pub fn wanted_above(&self, lane: Lane) -> bool {
        self.state().waiting.iter().any(|w| w.lane < lane)
    }

    /// Shows a catch-up job held back by the Mac's state; returns its handle for `change_reason` and
    /// `let_go`.
    pub fn hold_back(&self, task: Task, reason: &'static str) -> u64 {
        let mut state = self.state();
        state.seq += 1;
        let id = state.seq;
        state.held_back.push((id, task, reason));
        id
    }

    pub fn change_reason(&self, held: u64, reason: &'static str) {
        if let Some(entry) = self.state().held_back.iter_mut().find(|h| h.0 == held) {
            entry.2 = reason;
        }
    }

    pub fn let_go(&self, held: u64) {
        self.state().held_back.retain(|h| h.0 != held);
    }

    /// The owner's Run now for a held-back catch-up job: it stops waiting for the Mac (it still
    /// waits its turn behind the other lanes).
    pub fn run_now(&self, id: &str) {
        self.state().now.insert(id.to_owned());
    }

    pub fn runs_now(&self, id: &str) -> bool {
        self.state().now.contains(id)
    }

    /// The job ended: its Run now doesn't carry over to a later run.
    pub fn forget_now(&self, id: &str) {
        self.state().now.remove(id);
    }

    /// How to tell a recording runs (catch-up work waits for it to end).
    pub fn set_recording_check(&self, check: impl Fn() -> bool + Send + Sync + 'static) {
        self.state().recording = Some(Arc::new(check));
    }

    pub fn recording(&self) -> bool {
        let check = self.state().recording.clone();
        check.is_some_and(|recording| recording())
    }

    /// The running job first, then the waiting ones in the order they'll run, then those held back.
    pub fn activity(&self) -> Vec<Activity> {
        let state = self.state();
        let running = state.holder.iter().map(|(lane, task)| Activity {
            lane: *lane,
            task: task.clone(),
            running: true,
            waiting_for: None,
        });
        let waiting = state.waiting.iter().map(|w| Activity {
            lane: w.lane,
            task: w.task.clone(),
            running: false,
            waiting_for: None,
        });
        let held = state.held_back.iter().map(|(_, task, reason)| Activity {
            lane: Lane::Background,
            task: task.clone(),
            running: false,
            waiting_for: Some((*reason).to_owned()),
        });
        running.chain(waiting).chain(held).collect()
    }
}

fn enqueue(state: &mut State, lane: Lane, task: Task, front: bool) -> oneshot::Receiver<()> {
    state.seq += 1;
    let seq = if front { 0 } else { state.seq };
    let (wake, rx) = oneshot::channel();
    state.waiting.push(Waiter {
        lane,
        seq,
        task,
        wake,
    });
    state.waiting.sort_by_key(|w| (w.lane, w.seq));
    rx
}

/// Hands the turn to the first waiter whose future is still there (or to nobody).
fn hand_on(state: &mut State) {
    state.holder = None;
    while !state.waiting.is_empty() {
        let waiter = state.waiting.remove(0);
        state.holder = Some((waiter.lane, waiter.task.clone()));
        if waiter.wake.send(()).is_ok() {
            return;
        }
        state.holder = None; // that waiter gave up; try the next
    }
}

/// A waiting `acquire` that was dropped after being woken still owns the turn: pass it on.
struct Pending<'a> {
    queue: &'a AiQueue,
    rx: Option<oneshot::Receiver<()>>,
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        if let Some(mut rx) = self.rx.take() {
            rx.close(); // a wake from now on fails (and goes to the next waiter); one before is kept
            if rx.try_recv().is_ok() {
                self.queue.release();
            }
        }
    }
}

/// The AI turn; dropping it lets the next job run.
pub struct Turn<'a> {
    queue: &'a AiQueue,
    lane: Lane,
    /// False only while `make_way` waits to get it back.
    held: bool,
}

impl<'a> Turn<'a> {
    /// A higher lane waits: a long job should stop at its next checkpoint and `make_way`.
    pub fn wanted(&self) -> bool {
        self.queue.wanted_above(self.lane)
    }

    /// Lets every waiting higher-lane job run, then continues at the front of this lane. In one
    /// step under the lock: no job of this lane or below can slip in, and if the higher-lane
    /// waiters have all given up the turn comes straight back.
    pub async fn make_way(&mut self) {
        if !self.held || !self.wanted() {
            return;
        }
        let rx = {
            let mut state = self.queue.state();
            let task = state
                .holder
                .take()
                .map_or_else(|| Task::new("AI job"), |(_, task)| task);
            let rx = enqueue(&mut state, self.lane, task, true);
            self.held = false;
            hand_on(&mut state);
            rx
        };
        self.queue.until_woken(rx).await;
        self.held = true;
    }
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        if self.held {
            self.queue.release();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::{AiQueue, Lane, Task};

    fn run<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(f)
    }

    /// Waiters run by lane, then arrival; a job that makes way comes back before later arrivals
    /// of its own lane.
    #[test]
    fn higher_lanes_first_and_a_job_keeps_its_place() {
        run(async {
            let queue = Arc::new(AiQueue::default());
            let order = Arc::new(Mutex::new(Vec::new()));
            let mut first = queue.acquire(Lane::Foreground, Task::new("summary")).await;
            let spawn = |lane, name: &'static str| {
                let (queue, order) = (queue.clone(), order.clone());
                tokio::spawn(async move {
                    let _turn = queue.acquire(lane, Task::new(name)).await;
                    order.lock().unwrap().push(name);
                    tokio::time::sleep(Duration::from_millis(5)).await;
                })
            };
            let jobs = [
                spawn(Lane::Background, "backfill"),
                spawn(Lane::Foreground, "next meeting"),
                spawn(Lane::Interactive, "corrections"),
            ];
            tokio::time::sleep(Duration::from_millis(10)).await;
            assert!(first.wanted(), "an interactive job waits");
            assert_eq!(queue.activity().len(), 4);
            first.make_way().await; // corrections runs, then the summary goes on
            order.lock().unwrap().push("summary");
            assert!(!first.wanted());
            drop(first);
            for job in jobs {
                job.await.unwrap();
            }
            assert_eq!(
                *order.lock().unwrap(),
                ["corrections", "summary", "next meeting", "backfill"]
            );
            assert!(queue.activity().is_empty());
        });
    }

    /// A waiting acquire that's dropped never keeps the turn, woken or not.
    #[test]
    fn a_dropped_wait_passes_the_turn_on() {
        run(async {
            let queue = Arc::new(AiQueue::default());
            let held = queue.acquire(Lane::Foreground, Task::new("job")).await;
            let gone = {
                let queue = queue.clone();
                tokio::spawn(async move {
                    queue
                        .acquire(Lane::Interactive, Task::new("given up"))
                        .await;
                })
            };
            tokio::time::sleep(Duration::from_millis(5)).await;
            gone.abort();
            let _ = gone.await;
            drop(held);
            assert!(
                queue
                    .try_acquire(Lane::Background, Task::new("next"))
                    .is_some()
            );
            // Woken but dropped before it ran: the turn is released all the same.
            let held = queue.acquire(Lane::Foreground, Task::new("job")).await;
            let woken = {
                let queue = queue.clone();
                tokio::spawn(async move {
                    let _turn = queue.acquire(Lane::Interactive, Task::new("woken")).await;
                    std::future::pending::<()>().await;
                })
            };
            tokio::time::sleep(Duration::from_millis(5)).await;
            drop(held); // wakes it, but it doesn't run before the abort (one thread)
            woken.abort();
            let _ = woken.await;
            assert!(
                queue
                    .try_acquire(Lane::Background, Task::new("next"))
                    .is_some()
            );
        });
    }
}
