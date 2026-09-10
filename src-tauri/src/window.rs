use pet_core::{geometry::{self, Point, Size, WorkArea}, manifest::RenderConfig};
use serde::Serialize;
use tauri::{Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

// Call during setup on the main thread, before showing either overlay.
pub fn initialize_overlay(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior as Behavior};
        // SAFETY: Tauri owns this NSWindow, and setup runs on the main thread.
        let native = unsafe { &*window.ns_window()?.cast::<NSWindow>() };
        let mut behavior = native.collectionBehavior();
        behavior.remove(Behavior::MoveToActiveSpace | Behavior::FullScreenPrimary | Behavior::FullScreenNone);
        behavior.insert(Behavior::CanJoinAllSpaces | Behavior::FullScreenAuxiliary);
        if objc2::available!(macos = 13.0) {
            // Join other apps' fullscreen Spaces and Stage Manager groups.
            behavior.remove(Behavior::Primary | Behavior::Auxiliary);
            behavior.insert(Behavior::CanJoinAllApplications);
        }
        native.setCollectionBehavior(behavior);
    }
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::WidgetExt;
        // Hidden GTK windows may not have a native GDK window yet. Tao's
        // CursorIgnoreEvents handler requires one. Realize without showing it.
        window.gtk_window()?.realize();
    }
    window.set_ignore_cursor_events(true)
}

#[derive(Serialize)]
pub struct DesktopSample { pub cursor: Point, pub origin: Point, pub size: Size, pub scale: f64 }
fn area(monitor: &Monitor) -> WorkArea {
    let work = monitor.work_area();
    WorkArea { origin: Point { x: f64::from(work.position.x), y: f64::from(work.position.y) }, size: Size { width: f64::from(work.size.width), height: f64::from(work.size.height) }, scale: monitor.scale_factor() }
}
pub fn sample(window: &WebviewWindow) -> Result<DesktopSample, String> {
    let cursor = window.cursor_position().map_err(|e| e.to_string())?;
    let origin = window.inner_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    Ok(DesktopSample { cursor: Point { x: cursor.x, y: cursor.y }, origin: Point { x: f64::from(origin.x), y: f64::from(origin.y) }, size: Size { width: f64::from(size.width), height: f64::from(size.height) }, scale: window.scale_factor().map_err(|e| e.to_string())? })
}
pub fn anchor(window: &WebviewWindow, render: &RenderConfig) -> Result<Point, String> {
    let position = window.inner_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    Ok(geometry::anchor_from_origin(Point { x: f64::from(position.x), y: f64::from(position.y) }, Size { width: f64::from(size.width), height: f64::from(size.height) }, Point { x: render.anchor_x, y: render.anchor_y }))
}
/// Returns the final anchor calculated for the queued placement. Native window
/// changes can be asynchronous, so immediate geometry reads may still be stale.
pub fn place(window: &WebviewWindow, render: &RenderConfig, scale: f64, requested: Option<Point>) -> Result<Point, String> {
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let areas: Vec<_> = monitors.iter().map(area).collect();
    let work = if let Some(point) = requested {
        window.monitor_from_point(point.x, point.y).map_err(|e| e.to_string())?.as_ref().map(area).or_else(|| geometry::nearest_area(point, &areas))
    } else {
        window.primary_monitor().map_err(|e| e.to_string())?.as_ref().map(area).or_else(|| areas.first().copied())
    }.ok_or("No monitor work area available")?;
    let margin = 24.0 * work.scale;
    let fit = ((work.size.width - margin * 2.0).max(1.0) / f64::from(render.canvas_width)).min((work.size.height - margin * 2.0).max(1.0) / f64::from(render.canvas_height));
    let factor = (render.scale * scale * work.scale).min(fit);
    let size = Size { width: (f64::from(render.canvas_width) * factor).round().max(1.0), height: (f64::from(render.canvas_height) * factor).round().max(1.0) };
    let normalized = Point { x: render.anchor_x, y: render.anchor_y };
    let origin = requested.map(|p| geometry::origin_from_anchor(p, size, normalized)).unwrap_or(Point { x: work.origin.x + work.size.width - size.width - margin, y: work.origin.y + work.size.height - size.height - margin });
    let origin = geometry::clamp_origin(origin, size, work);
    let position = PhysicalPosition::new(origin.x.round() as i32, origin.y.round() as i32);
    let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);
    let resized = window.inner_size().map_err(|e| e.to_string())? != physical_size;
    if resized { window.set_size(physical_size).map_err(|e| e.to_string())?; }
    // On macOS resizing can shift the top-left corner. Position after resizing,
    // even if the old position already matched the target.
    if resized || window.outer_position().map_err(|e| e.to_string())? != position {
        window.set_position(position).map_err(|e| e.to_string())?;
    }
    Ok(geometry::anchor_from_origin(Point { x: f64::from(position.x), y: f64::from(position.y) }, size, normalized))
}
pub fn debug_render() -> RenderConfig {
    RenderConfig { canvas_width: 256, canvas_height: 256, scale: 1.0, anchor_x: 0.5, anchor_y: 0.9 }
}
