use anyhow::Result;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, LogicalPosition, Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

use crate::events::OpenSettingsTab;

pub const MAIN_LABEL: &str = "main";
const SETTINGS_LABEL: &str = "settings";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SettingsTab {
    General,
    Workspaces,
    Models,
    Converters,
    Backup,
    Storage,
    About,
}

impl SettingsTab {
    fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Workspaces => "workspaces",
            Self::Backup => "backup",
            Self::Storage => "storage",
            Self::Models => "models",
            Self::Converters => "converters",
            Self::About => "about",
        }
    }
}

/// Opens the Settings window on `tab`, or brings the existing one forward and switches its tab.
pub fn open_settings(app: &AppHandle, tab: SettingsTab) -> Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        window.show()?;
        window.set_focus()?;
        OpenSettingsTab { tab }.emit_to(app, SETTINGS_LABEL)?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App(format!("settings?tab={}", tab.as_str()).into()),
    )
    .title("Settings")
    // Fixed like System Settings: the panes are laid out for exactly this size.
    .inner_size(780.0, 580.0)
    .resizable(false)
    .maximizable(false)
    .transparent(true)
    .title_bar_style(TitleBarStyle::Overlay)
    .hidden_title(true)
    // tao's y is the title-bar strip's height minus a button's, not the buttons' top: 28
    // centres them in our 52 pt bar (18 left them ~10 pt high). Keep in step with
    // tauri.conf.json (main window).
    .traffic_light_position(LogicalPosition::new(18.0, 28.0))
    .build()?;

    let target = window.clone();
    app.run_on_main_thread(move || super::glass::apply_shell_glass(&target))?;
    Ok(())
}

/// Debug builds: `NEWDAWN_DEV_OPEN=settings:<tab>` opens Settings on that tab at launch,
/// `NEWDAWN_DEV_OPEN=/<route>` sends the main window there — for looking at a screen without
/// clicking. `NEWDAWN_DEV_THEME=dark|light` sets the app's appearance (not the Mac's), and
/// JavaScript written to `<dev data>/dev-eval.js` runs in the main window each time the file
/// changes (scroll, click, type). The app runs as an accessory (no Dock icon, never activated), so a check doesn't take
/// the keyboard from the owner. Its windows can't hide behind others: WebKit stops painting a
/// fully covered window, and a window-only screenshot then shows grey.
#[cfg(debug_assertions)]
pub fn dev_open(app: &AppHandle) -> Result<()> {
    let Ok(target) = std::env::var("NEWDAWN_DEV_OPEN") else {
        return Ok(());
    };
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;
    if let Some(tab) = target.strip_prefix("settings:") {
        open_settings(app, serde_json::from_value(serde_json::json!(tab))?)?;
    } else if let Some(main) = app.get_webview_window(MAIN_LABEL) {
        main.navigate(main.url()?.join(&target)?)?;
    }
    let theme = match std::env::var("NEWDAWN_DEV_THEME").as_deref() {
        Ok("dark") => Some(tauri::Theme::Dark),
        Ok("light") => Some(tauri::Theme::Light),
        _ => None,
    };
    for window in app.webview_windows().values() {
        window.set_theme(theme)?;
    }
    let script = super::storage::data_dir(app)?.join("dev-eval.js");
    let _ = std::fs::remove_file(&script); // never replay a previous session's script
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut seen = None;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let modified = std::fs::metadata(&script).and_then(|m| m.modified()).ok();
            if modified.is_none() || modified == seen {
                continue;
            }
            seen = modified;
            if let (Ok(js), Some(main)) = (
                std::fs::read_to_string(&script),
                handle.get_webview_window(MAIN_LABEL),
            ) {
                let _ = main.eval(js);
            }
        }
    });
    // WebKit stops painting a window that other windows cover; a check runs behind the owner's
    // windows, so keep painting (the SPI Playwright's WebKit uses; skipped if WebKit lacks it).
    #[cfg(target_os = "macos")]
    for window in app.webview_windows().values() {
        window.with_webview(|webview| {
            use objc2::runtime::AnyObject;
            use objc2::{msg_send, sel};
            // SAFETY: on macOS `inner()` is the live WKWebView; the selector is checked first.
            unsafe {
                let view = &*(webview.inner() as *const AnyObject);
                let known: bool =
                    msg_send![view, respondsToSelector: sel!(_setWindowOcclusionDetectionEnabled:)];
                if known {
                    let _: () = msg_send![view, _setWindowOcclusionDetectionEnabled: false];
                }
            }
        })?;
    }
    Ok(())
}

/// Shows and focuses the main window (it is hidden, not closed, when its close button is used).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
