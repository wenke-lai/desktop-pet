use tauri::{menu::{Menu, MenuItem, PredefinedMenuItem}, tray::TrayIconBuilder, App, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use crate::AppState;

pub struct TrayLabel(pub MenuItem<tauri::Wry>);
pub fn build(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let label = MenuItem::with_id(app, "label", "Debug Pet", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Pets Folder", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Reload Content", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "Next Pet", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&label, &separator, &open, &reload, &next, &exit])?;
    let mut tray = TrayIconBuilder::with_id("pet-tray").menu(&menu).tooltip("Desktop Pet").show_menu_on_left_click(true).on_menu_event(|app, event| {
        match event.id.as_ref() {
            "open" => {
                let path = app.state::<AppState>().root.join("pets");
                if let Err(e) = app.opener().open_path(path.to_string_lossy().into_owned(), None::<&str>) { log::error!("Open Pets Folder: {e}"); }
            }
            "reload" | "next" => { if let Err(e) = app.emit_to("main", "tray-action", event.id.as_ref()) { log::error!("Tray event: {e}"); } }
            "exit" => {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = crate::persist_position(&window, &app.state::<AppState>()) { log::error!("Save position: {e}"); }
                }
                app.exit(0);
            }
            _ => {}
        }
    });
    if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
    tray.build(app)?;
    app.manage(TrayLabel(label));
    Ok(())
}
