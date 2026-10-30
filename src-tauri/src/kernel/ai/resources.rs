//! How busy the Mac and its owner are, so heavy AI work stays out of the way (D-048): power, heat
//! and time since the last keyboard or mouse input. Read on demand; every read is cheap (one
//! `pmset` run is the dearest, a few milliseconds).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::kernel::hub::WorkspaceHub;

/// Input within this many seconds: the owner is at the Mac.
const ACTIVE_SECS: f64 = 120.0;

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

/// Registry setting (app-wide); absent = gentle.
const PACE_SETTING: &str = "ai_pace";

pub async fn read_pace(hub: &WorkspaceHub) -> Result<Pace> {
    Ok(match hub.app_setting(PACE_SETTING).await?.as_deref() {
        Some("full") => Pace::Full,
        _ => Pace::Gentle,
    })
}

pub async fn set_pace(hub: &WorkspaceHub, pace: Pace) -> Result<()> {
    let value = match pace {
        Pace::Full => "full",
        Pace::Gentle => "gentle",
    };
    hub.set_app_setting(PACE_SETTING, value).await
}

/// The pace in force (gentle when the setting can't be read).
pub async fn pace(app: &AppHandle) -> Pace {
    read_pace(&app.state::<WorkspaceHub>())
        .await
        .unwrap_or(Pace::Gentle)
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
    /// `NSProcessInfo.thermalState`: 0 nominal, 1 fair, 2 serious, 3 critical.
    pub thermal: i64,
    /// Seconds since the last keyboard, mouse or trackpad input.
    pub idle_secs: f64,
}

impl Snapshot {
    pub async fn now() -> Self {
        Self {
            on_battery: on_battery().await,
            thermal: thermal_state(),
            idle_secs: idle_secs(),
        }
    }

    /// The owner is at the Mac, it's on battery, or it runs hot.
    pub fn busy(&self) -> bool {
        self.idle_secs < ACTIVE_SECS || self.on_battery || self.thermal >= 2
    }
}

/// On battery (`pmset -g batt`: "Now drawing from 'Battery Power'").
async fn on_battery() -> bool {
    tokio::process::Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .output()
        .await
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("'Battery Power'"))
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
    use super::Snapshot;

    #[test]
    fn busy_when_working_on_battery_or_hot() {
        let away = Snapshot {
            on_battery: false,
            thermal: 0,
            idle_secs: 900.0,
        };
        assert!(!away.busy());
        assert!(
            Snapshot {
                idle_secs: 10.0,
                ..away
            }
            .busy()
        );
        assert!(
            Snapshot {
                on_battery: true,
                ..away
            }
            .busy()
        );
        assert!(Snapshot { thermal: 2, ..away }.busy());
    }

    /// The live readings on this Mac (whatever they are) are in range.
    #[test]
    fn reads_this_mac() {
        let now = tauri::async_runtime::block_on(Snapshot::now());
        assert!((0..=3).contains(&now.thermal));
        assert!(now.idle_secs >= 0.0);
    }
}
