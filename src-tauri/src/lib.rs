mod codex;
mod tray;
mod window;

use pet_core::{content::{self, Catalog}, geometry::Point, manifest::RenderConfig, settings::Settings};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{Manager, State, WebviewWindow};

pub struct Session { catalog: Catalog, settings: Settings, render: Option<RenderConfig> }
pub struct AppState { root: PathBuf, session: Mutex<Session> }
#[derive(Deserialize)]
struct Project { name: String }
#[derive(Serialize)]
struct Snapshot { catalog: Catalog, settings: Settings, content_root: String }
fn snapshot(state: &AppState) -> Result<Snapshot, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    Ok(Snapshot { catalog: session.catalog.clone(), settings: session.settings.clone(), content_root: state.root.to_string_lossy().into_owned() })
}
#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> { snapshot(&state) }
#[tauri::command]
async fn reload_content(app: tauri::AppHandle) -> Result<Snapshot, String> {
    let root = app.state::<AppState>().root.clone();
    let catalog = tauri::async_runtime::spawn_blocking(move || content::scan(&root.join("pets"))).await.map_err(|e| e.to_string())??;
    let state = app.state::<AppState>();
    {
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        session.catalog = catalog;
        session.settings = Settings::load(&state.root.join("settings.json"));
    }
    snapshot(&state)
}
#[tauri::command]
fn activate_pet(id: Option<String>, window: WebviewWindow, state: State<'_, AppState>, app: tauri::AppHandle) -> Result<window::DesktopSample, String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let (render, name) = match id.as_ref() {
        Some(id) => {
            let pet = session.catalog.pets.iter().find(|p| &p.definition.id == id).ok_or("Pet was removed; reload content")?;
            (pet.definition.render.clone(), pet.definition.display_name.clone())
        }
        None => (window::debug_render(), "Debug Pet".into()),
    };
    let anchor = match &session.render { Some(previous) => Some(window::anchor(&window, previous)?), None => session.settings.last_position };
    window::place(&window, &render, session.settings.scale, anchor)?;
    window.set_always_on_top(session.settings.always_on_top).map_err(|e| e.to_string())?;
    window.set_title(&format!("Desktop Pet — {name}")).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    session.settings.active_pet = id;
    session.settings.last_position = Some(window::anchor(&window, &render)?);
    session.render = Some(render);
    session.settings.save(&state.root.join("settings.json"))?;
    app.state::<tray::TrayLabel>().0.set_text(&name).map_err(|e| e.to_string())?;
    log::info!("Active pet changed: {name}");
    window::sample(&window)
}
#[tauri::command]
fn desktop_sample(window: WebviewWindow) -> Result<window::DesktopSample, String> { window::sample(&window) }
#[tauri::command]
fn set_cursor_passthrough(ignore: bool, window: WebviewWindow) -> Result<(), String> { window.set_ignore_cursor_events(ignore).map_err(|e| e.to_string()) }
#[tauri::command]
fn move_pet(anchor: Point, window: WebviewWindow, state: State<'_, AppState>) -> Result<window::DesktopSample, String> {
    if !anchor.x.is_finite() || !anchor.y.is_finite() { return Err("Invalid desktop coordinates".into()); }
    let session = state.session.lock().map_err(|e| e.to_string())?;
    let render = session.render.as_ref().ok_or("No active pet")?;
    window::place(&window, render, session.settings.scale, Some(anchor))?;
    window::sample(&window)
}
#[tauri::command]
fn settle_window(window: WebviewWindow, state: State<'_, AppState>) -> Result<window::DesktopSample, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    if let Some(render) = &session.render {
        window::place(&window, render, session.settings.scale, Some(window::anchor(&window, render)?))?;
    }
    window::sample(&window)
}
fn persist_position(window: &WebviewWindow, state: &AppState) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    if let Some(render) = &session.render { session.settings.last_position = Some(window::anchor(window, render)?); }
    session.settings.save(&state.root.join("settings.json"))
}
#[tauri::command]
fn save_position(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> { persist_position(&window, &state) }
#[tauri::command]
async fn get_codex_usage(state: State<'_, codex::UsageCache>) -> Result<codex::UsageSnapshot, String> { state.read().await }
#[tauri::command]
fn set_usage_hover(hovered: bool, head: Option<Point>, app: tauri::AppHandle) -> Result<(), String> {
    let bubble = app.get_webview_window("codex-usage").ok_or("Missing usage window")?;
    if !hovered { return bubble.hide().map_err(|e| e.to_string()); }
    let pet = app.get_webview_window("main").ok_or("Missing pet window")?;
    let position = pet.inner_position().map_err(|e| e.to_string())?;
    let size = pet.inner_size().map_err(|e| e.to_string())?;
    let head = head.unwrap_or(Point { x: 0.5, y: 0.0 });
    if !head.x.is_finite() || !head.y.is_finite() { return Err("Invalid head position".into()); }
    if let Some(monitor) = pet.current_monitor().map_err(|e| e.to_string())? {
        let area = monitor.work_area();
        // Keep the bubble legible at the pet's current monitor DPI.
        let scale = monitor.scale_factor();
        let width = (252.0 * scale).round() as u32;
        let height = (116.0 * scale).round() as u32;
        let target_size = tauri::PhysicalSize::new(width, height);
        if bubble.inner_size().map_err(|e| e.to_string())? != target_size { bubble.set_size(target_size).map_err(|e| e.to_string())?; }
        let x = f64::from(position.x) + head.x.clamp(0.0, 1.0) * f64::from(size.width) - f64::from(width) / 2.0;
        let y = f64::from(position.y) + head.y.clamp(0.0, 1.0) * f64::from(size.height) - f64::from(height);
        let min_x = area.position.x;
        let min_y = area.position.y;
        let max_x = (min_x + area.size.width as i32 - width as i32).max(min_x);
        let max_y = (min_y + area.size.height as i32 - height as i32).max(min_y);
        let target = tauri::PhysicalPosition::new((x.round() as i32).clamp(min_x, max_x), (y.round() as i32).clamp(min_y, max_y));
        if bubble.outer_position().map_err(|e| e.to_string())? != target { bubble.set_position(target).map_err(|e| e.to_string())?; }
    }
    if !bubble.is_visible().map_err(|e| e.to_string())? { bubble.show().map_err(|e| e.to_string())?; }
    Ok(())
}
#[tauri::command]
fn frontend_log(message: String) { log::info!("WebView: {message}"); }

pub fn run() {
    // Dragging and pixel click-through need global cursor/window coordinates.
    // Tao returns a constant cursor position on Wayland; use X11 (including
    // XWayland on WSLg) when available. This must precede GTK initialization.
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some_and(|display| !display.is_empty()) {
        gtk::gdk::set_allowed_backends("x11");
    }
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let project: Project = serde_json::from_str(include_str!("../../package.json"))?;
            let root = app.path().home_dir()?.join(".local/share").join(project.name);
            fs::create_dir_all(root.join("pets"))?;
            app.asset_protocol_scope().allow_directory(root.join("pets"), true)?;
            fs::create_dir_all(root.join("logs"))?;
            let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![simplelog::SimpleLogger::new(log::LevelFilter::Info, simplelog::Config::default())];
            if let Ok(file) = fs::File::create(root.join("logs/latest.log")) { loggers.push(simplelog::WriteLogger::new(log::LevelFilter::Info, simplelog::Config::default(), file)); }
            let _ = simplelog::CombinedLogger::init(loggers);
            log::info!("Content root: {}", root.display());
            let settings = Settings::load(&root.join("settings.json"));
            settings.save(&root.join("settings.json")).map_err(std::io::Error::other)?;
            let catalog = content::scan(&root.join("pets")).map_err(std::io::Error::other)?;
            app.manage(AppState { root, session: Mutex::new(Session { catalog, settings, render: None }) });
            app.manage(codex::UsageCache::default());
            if let Some(bubble) = app.get_webview_window("codex-usage") { window::initialize_cursor_passthrough(&bubble)?; }
            tray::build(app)?;
            if let Some(window) = app.get_webview_window("main") { window::initialize_cursor_passthrough(&window)?; }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_snapshot, reload_content, activate_pet, desktop_sample, set_cursor_passthrough, move_pet, settle_window, save_position, frontend_log, get_codex_usage, set_usage_hover])
        .run(tauri::generate_context!());
    if let Err(error) = result { eprintln!("Desktop Pet failed to start: {error}"); }
}
