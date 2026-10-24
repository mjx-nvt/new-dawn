use tauri::menu::{Menu, MenuEvent, MenuItem, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};
use tauri_specta::Event;

use crate::events::MenuAction;
use crate::kernel::windows::{self, SettingsTab};

fn item(app: &AppHandle, id: &str, text: &str, accelerator: Option<&str>, enabled: bool) -> tauri::Result<MenuItem<Wry>> {
    let builder = MenuItemBuilder::with_id(id, text).enabled(enabled);
    match accelerator {
        Some(keys) => builder.accelerator(keys).build(app),
        None => builder.build(app),
    }
}

/// The native menu bar. Items for features that don't exist yet are disabled, not hidden.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let app_menu = SubmenuBuilder::new(app, "New Dawn")
        .about(None)
        .separator()
        .item(&item(app, "settings", "Settings…", Some("CmdOrCtrl+,"), true)?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&item(app, "new-recording", "New Recording", Some("CmdOrCtrl+R"), true)?)
        .item(&item(app, "import-audio", "Import Audio…", Some("CmdOrCtrl+Shift+I"), true)?)
        .separator()
        .close_window()
        .build()?;

    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item(app, "toggle-sidebar", "Toggle Sidebar", Some("CmdOrCtrl+Alt+S"), true)?)
        .separator()
        .fullscreen()
        .build()?;

    let workspace = SubmenuBuilder::new(app, "Workspace")
        .item(&item(app, "switch-workspace", "Switch…", None, true)?)
        .item(&item(app, "new-workspace", "New Workspace…", None, true)?)
        .separator()
        .item(&item(app, "tenant-setup", "Tenant Setup…", None, true)?)
        .build()?;

    let window = SubmenuBuilder::new(app, "Window").minimize().maximize().build()?;
    window.set_as_windows_menu_for_nsapp()?;

    let help = SubmenuBuilder::new(app, "Help")
        .item(&item(app, "reveal-data", "Reveal Data Folder in Finder", None, true)?)
        .build()?;
    help.set_as_help_menu_for_nsapp()?;

    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &workspace, &window, &help])
}

/// Handles menu-bar and tray items. Anything the UI owns is forwarded as a typed event.
pub fn on_event(app: &AppHandle, event: MenuEvent) {
    let result = match event.id().as_ref() {
        "settings" => windows::open_settings(app, SettingsTab::General),
        "new-workspace" | "tenant-setup" => forward(app, MenuAction::OpenTenantSetup),
        "switch-workspace" => forward(app, MenuAction::SwitchWorkspace),
        "toggle-sidebar" => forward(app, MenuAction::ToggleSidebar),
        "import-audio" => forward(app, MenuAction::ImportAudio),
        "new-recording" | "record" => forward(app, MenuAction::ToggleRecording),
        "reveal-data" => reveal_data_folder(app),
        "open-app" => {
            windows::show_main(app);
            Ok(())
        }
        "quit" => {
            app.exit(0); // runs RunEvent::Exit, which checkpoints and closes the databases
            Ok(())
        }
        _ => Ok(()),
    };
    if let Err(err) = result {
        tracing::warn!("menu action {} failed: {err:#}", event.id().as_ref());
    }
}

fn forward(app: &AppHandle, action: MenuAction) -> anyhow::Result<()> {
    windows::show_main(app);
    action.emit(app)?;
    Ok(())
}

fn reveal_data_folder(app: &AppHandle) -> anyhow::Result<()> {
    let dir = app.path().app_data_dir()?;
    tauri_plugin_opener::reveal_item_in_dir(dir)?;
    Ok(())
}
