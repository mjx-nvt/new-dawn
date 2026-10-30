//! How busy the Mac and its owner are, so heavy AI work stays out of the way (D-048, D-050): power,
//! heat, memory pressure and time since the last keyboard or mouse input. Read on demand; every
//! read is cheap (one `pmset` run is the dearest, a few milliseconds).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::kernel::ai::queue::{AiQueue, Lane, Task, Turn};
use crate::kernel::hub::WorkspaceHub;

/// Input within this many seconds: the owner is at the Mac.
const ACTIVE_SECS: f64 = 120.0;
/// Catch-up work waits until the owner has been away this long (unless they allow it while they
/// work).
const AWAY_SECS: f64 = 300.0;
/// Below this charge on battery, catch-up work waits for the charger.
const LOW_BATTERY: u8 = 30;

/// When the Mac is shared with the owner (Settings → AI on this Mac → While you work).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Pace {
    /// Heavy jobs use every core even while the owner works.
    Full,
    /// While the owner works, on battery or when the Mac runs hot: half the CPU threads, utility
    /// priority, the text model unloaded sooner.
    Gentle,
}

/// Registry settings (app-wide); absent = gentle, catch-up only when away.
const PACE_SETTING: &str = "ai_pace";
const BACKGROUND_SETTING: &str = "ai_background_while_working";

/// How AI work shares the Mac (Settings → Models → While you work).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct AiPolicy {
    pub pace: Pace,
    /// Catch-up work (model checks…) may run while the owner works; else only when they're away.
    pub background_while_working: bool,
}

pub async fn policy(hub: &WorkspaceHub) -> Result<AiPolicy> {
    Ok(AiPolicy {
        pace: match hub.app_setting(PACE_SETTING).await?.as_deref() {
            Some("full") => Pace::Full,
            _ => Pace::Gentle,
        },
        background_while_working: hub.app_setting(BACKGROUND_SETTING).await?.as_deref()
            == Some("1"),
    })
}

pub async fn set_policy(hub: &WorkspaceHub, policy: AiPolicy) -> Result<()> {
    let pace = match policy.pace {
        Pace::Full => "full",
        Pace::Gentle => "gentle",
    };
    hub.set_app_setting(PACE_SETTING, pace).await?;
    let background = if policy.background_while_working {
        "1"
    } else {
        "0"
    };
    hub.set_app_setting(BACKGROUND_SETTING, background).await
}

/// The pace in force (gentle when the setting can't be read).
pub async fn pace(app: &AppHandle) -> Pace {
    policy(&app.state::<WorkspaceHub>())
        .await
        .map_or(Pace::Gentle, |p| p.pace)
}

/// A turn in `lane`. Catch-up work (the Background lane) first waits while a recording runs, the
/// Mac is on low battery, hot, short of memory, or in use (unless the owner allows it), showing why
/// in the AI activity list; the list's Run now (`AiQueue::run_now`, also how Cancel lifts the
/// wait) ends that wait for the job (D-050). The Mac's state is read every 30 s, Run now every 2 s.
pub async fn acquire(app: &AppHandle, lane: Lane, task: Task) -> Turn<'_> {
    let queue = app.state::<AiQueue>().inner();
    if lane == Lane::Background {
        let mut held: Option<Held<'_>> = None;
        let mut read_at: Option<std::time::Instant> = None;
        loop {
            if task.id.as_deref().is_some_and(|id| queue.runs_now(id)) {
                break;
            }
            if read_at.is_none_or(|at| at.elapsed() >= std::time::Duration::from_secs(30)) {
                read_at = Some(std::time::Instant::now());
                let reason = if queue.recording() {
                    Some("Waiting until the recording ends")
                } else {
                    let while_working = policy(&app.state::<WorkspaceHub>())
                        .await
                        .is_ok_and(|p| p.background_while_working);
                    Snapshot::now().await.background_blocker(while_working)
                };
                let Some(reason) = reason else {
                    break;
                };
                match &held {
                    Some(h) => queue.change_reason(h.0, reason),
                    None => held = Some(Held(queue.hold_back(task.clone(), reason), queue)),
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    queue.acquire(lane, task).await
}

/// A held-back entry in the activity list, gone when the wait ends or is dropped.
struct Held<'a>(u64, &'a AiQueue);

impl Drop for Held<'_> {
    fn drop(&mut self) {
        self.1.let_go(self.0);
    }
}

/// Heavy work should go easy now: the pace is gentle and the owner works, the Mac is on battery
/// or it runs hot.
pub async fn go_easy(app: &AppHandle) -> bool {
    pace(app).await == Pace::Gentle && Snapshot::now().await.busy()
}

/// Runs `f` on a thread of its own, at utility priority when `gentle`, and waits for it: a batch of
/// Whisper work that yields to the owner's apps without lowering a shared pool thread for good.
pub async fn blocking<T: Send + 'static>(
    gentle: bool,
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name("ai-batch".into())
        .spawn(move || {
            if gentle {
                lower_this_thread();
            }
            let _ = tx.send(f());
        })?;
    Ok(rx.await?)
}

#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub on_battery: bool,
    pub battery: Option<u8>,
    /// `NSProcessInfo.thermalState`: 0 nominal, 1 fair, 2 serious, 3 critical.
    pub thermal: i64,
    /// `kern.memorystatus_vm_pressure_level`: 1 normal, 2 warning, 4 critical.
    pub memory_pressure: i32,
    /// Seconds since the last keyboard, mouse or trackpad input.
    pub idle_secs: f64,
}

impl Snapshot {
    pub async fn now() -> Self {
        let (on_battery, battery) = power().await;
        Self {
            on_battery,
            battery,
            thermal: thermal_state(),
            memory_pressure: memory_pressure(),
            idle_secs: idle_secs(),
        }
    }

    /// The owner is at the Mac, it's on battery, or it runs hot.
    pub fn busy(&self) -> bool {
        self.idle_secs < ACTIVE_SECS || self.on_battery || self.thermal >= 2
    }

    /// Why catch-up work must wait now, in words for the AI activity list; None = it may run.
    pub fn background_blocker(&self, while_working: bool) -> Option<&'static str> {
        if self.on_battery && self.battery.is_some_and(|b| b < LOW_BATTERY) {
            Some("Waiting for the charger (battery below 30 %)")
        } else if self.thermal >= 2 {
            Some("Waiting for the Mac to cool down")
        } else if self.memory_pressure > 1 {
            Some("Waiting for free memory")
        } else if !while_working && self.idle_secs < AWAY_SECS {
            Some("Waiting until you're away from the Mac")
        } else {
            None
        }
    }
}

/// On battery, and the charge (`pmset -g batt`: "Now drawing from 'Battery Power'" … "57%;").
async fn power() -> (bool, Option<u8>) {
    match tokio::process::Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .output()
        .await
    {
        Ok(out) => parse_power(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => (false, None),
    }
}

fn parse_power(text: &str) -> (bool, Option<u8>) {
    let on_battery = text.contains("'Battery Power'");
    let battery = text
        .split(|c: char| c.is_whitespace() || c == ';')
        .find_map(|w| w.strip_suffix('%')?.parse().ok());
    (on_battery, battery)
}

fn memory_pressure() -> i32 {
    let mut level: libc::c_int = 1;
    let mut size = std::mem::size_of::<libc::c_int>();
    // SAFETY: the name is NUL-terminated and `level`/`size` describe a writable c_int.
    let ok = unsafe {
        libc::sysctlbyname(
            c"kern.memorystatus_vm_pressure_level".as_ptr(),
            (&mut level as *mut libc::c_int).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 { level } else { 1 }
}

fn thermal_state() -> i64 {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    // SAFETY: +[NSProcessInfo processInfo] is a shared object that lives for the process;
    // -thermalState returns an NSInteger.
    unsafe {
        let info: *mut AnyObject = msg_send![class!(NSProcessInfo), processInfo];
        if info.is_null() {
            return 0;
        }
        let state: isize = msg_send![&*info, thermalState];
        state as i64
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state: i32, event_type: u32) -> f64;
}

fn idle_secs() -> f64 {
    const COMBINED_SESSION_STATE: i32 = 0;
    const ANY_INPUT: u32 = u32::MAX;
    // SAFETY: a pure query with constant arguments.
    unsafe { CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT) }
}

unsafe extern "C" {
    fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
}

/// Runs the calling thread at utility priority (macOS QoS): the scheduler favours the owner's
/// apps and moves this work to the efficiency cores when they're busy.
fn lower_this_thread() {
    const QOS_CLASS_UTILITY: u32 = 0x11;
    // SAFETY: affects only the calling thread; a failure leaves its priority unchanged.
    unsafe {
        pthread_set_qos_class_self_np(QOS_CLASS_UTILITY, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::{Snapshot, parse_power};

    fn calm() -> Snapshot {
        Snapshot {
            on_battery: false,
            battery: Some(100),
            thermal: 0,
            memory_pressure: 1,
            idle_secs: 900.0,
        }
    }

    #[test]
    fn reads_pmset() {
        let ac = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=23658595)\t100%; charged; 0:00 remaining present: true\n";
        assert_eq!(parse_power(ac), (false, Some(100)));
        let battery = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t27%; discharging; 1:02 remaining present: true\n";
        assert_eq!(parse_power(battery), (true, Some(27)));
        assert_eq!(parse_power(""), (false, None));
    }

    #[test]
    fn busy_when_working_on_battery_or_hot() {
        assert!(!calm().busy());
        assert!(
            Snapshot {
                idle_secs: 10.0,
                ..calm()
            }
            .busy()
        );
        assert!(
            Snapshot {
                on_battery: true,
                ..calm()
            }
            .busy()
        );
        assert!(
            Snapshot {
                thermal: 2,
                ..calm()
            }
            .busy()
        );
    }

    #[test]
    fn catch_up_waits_for_the_reason_that_matters_first() {
        assert_eq!(calm().background_blocker(false), None);
        let working = Snapshot {
            idle_secs: 10.0,
            ..calm()
        };
        assert!(working.background_blocker(false).unwrap().contains("away"));
        assert_eq!(working.background_blocker(true), None);
        let low = Snapshot {
            on_battery: true,
            battery: Some(20),
            ..calm()
        };
        assert!(low.background_blocker(true).unwrap().contains("charger"));
        let hot = Snapshot {
            thermal: 2,
            ..calm()
        };
        assert!(hot.background_blocker(true).unwrap().contains("cool"));
        let tight = Snapshot {
            memory_pressure: 2,
            ..calm()
        };
        assert!(tight.background_blocker(true).unwrap().contains("memory"));
    }

    /// The live readings on this Mac (whatever they are) are in range.
    #[test]
    fn reads_this_mac() {
        let now = tauri::async_runtime::block_on(Snapshot::now());
        assert!((0..=3).contains(&now.thermal));
        assert!([1, 2, 4].contains(&now.memory_pressure));
        assert!(now.idle_secs >= 0.0);
    }
}
