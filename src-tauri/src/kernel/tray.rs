use tauri::image::Image;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

/// Menu-bar status item: Record (start/stop), Open New Dawn, Quit.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let record = MenuItemBuilder::with_id("record", "Record / Stop").build(app)?;
    let open = MenuItemBuilder::with_id("open-app", "Open New Dawn").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit New Dawn").build(app)?;
    let menu = Menu::with_items(app, &[&record, &PredefinedMenuItem::separator(app)?, &open, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../../icons/tray.png"))?)
        .icon_as_template(true) // macOS tints it for light/dark menu bars
        .tooltip("New Dawn")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .build(app)?;
    Ok(())
}
