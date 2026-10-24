//! macOS notifications (Notification Center). Best effort: a failure is logged, never raised.

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub fn show(app: &AppHandle, title: &str, body: &str) {
    if let Err(err) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("could not show a notification: {err}");
    }
}

/// 754_000 → "12:34"; 3_723_000 → "1:02:03".
pub fn clock(ms: u32) -> String {
    let total = ms / 1000;
    let (h, m, s) = (total / 3600, total % 3600 / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_durations() {
        assert_eq!(super::clock(754_000), "12:34");
        assert_eq!(super::clock(3_723_000), "1:02:03");
    }
}
