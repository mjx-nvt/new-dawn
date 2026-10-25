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

/// Shows and focuses the main window (it is hidden, not closed, when its close button is used).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
