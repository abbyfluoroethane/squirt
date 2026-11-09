//! The macOS menu bar. Squirt's own items send their id to the page as a
//! `menu` event (the page does the work); standard items (copy, hide,
//! quit...) are the system's.

use tauri::menu::{AboutMetadataBuilder, Menu, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Runtime};

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, label: &str, accel: Option<&str>| {
        let b = MenuItemBuilder::with_id(id, label);
        match accel {
            Some(a) => b.accelerator(a).build(app),
            None => b.build(app),
        }
    };
    let sep = || PredefinedMenuItem::separator(app);

    let about = AboutMetadataBuilder::new()
        .name(Some("Squirt"))
        .version(Some(app.package_info().version.to_string()))
        .copyright(Some("MIT license"))
        .build();

    let app_menu = SubmenuBuilder::new(app, "Squirt")
        .item(&PredefinedMenuItem::about(app, Some("About Squirt"), Some(about))?)
        .item(&sep()?)
        .item(&item("settings", "Settings…", Some("CmdOrCtrl+,"))?)
        .item(&item("setup", "Run Setup Again…", None)?)
        .item(&sep()?)
        .item(&PredefinedMenuItem::services(app, None)?)
        .item(&sep()?)
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .item(&sep()?)
        .item(&PredefinedMenuItem::quit(app, None)?)
        .build()?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&item("add-folder", "Add Folder to Library…", Some("CmdOrCtrl+O"))?)
        .item(&item("rescan", "Rescan Library", None)?)
        .item(&sep()?)
        .item(&PredefinedMenuItem::close_window(app, None)?)
        .build()?;

    // Needed for copy and paste in text fields.
    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .item(&sep()?)
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item("view-collection", "Collection", Some("CmdOrCtrl+1"))?)
        .item(&item("view-device", "Device", Some("CmdOrCtrl+2"))?)
        .item(&item("view-social", "Social", Some("CmdOrCtrl+3"))?)
        .item(&sep()?)
        .item(&item("now-playing", "Now Playing", Some("CmdOrCtrl+Shift+F"))?)
        .item(&sep()?)
        .item(&PredefinedMenuItem::fullscreen(app, None)?)
        .build()?;

    // Space (play/pause) stays a page shortcut: as a menu shortcut it would
    // also fire while typing in a text field.
    let controls = SubmenuBuilder::new(app, "Controls")
        .item(&item("play-pause", "Play/Pause", None)?)
        .item(&item("next", "Next", Some("CmdOrCtrl+Right"))?)
        .item(&item("previous", "Previous", Some("CmdOrCtrl+Left"))?)
        .item(&sep()?)
        .item(&item("volume-up", "Increase Volume", Some("CmdOrCtrl+Up"))?)
        .item(&item("volume-down", "Decrease Volume", Some("CmdOrCtrl+Down"))?)
        .item(&sep()?)
        .item(&item("go-to-current", "Go to Current Song", Some("CmdOrCtrl+L"))?)
        .build()?;

    let window = SubmenuBuilder::new(app, "Window")
        .item(&PredefinedMenuItem::minimize(app, None)?)
        .item(&PredefinedMenuItem::maximize(app, Some("Zoom"))?)
        .build()?;

    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &controls, &window])
}

/// Forwards Squirt's own menu items to the page.
pub fn forward<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let _ = app.emit("menu", id);
}
