use pet_core::{geometry::{self, Point, Size, WorkArea}, manifest::RenderConfig};
use serde::Serialize;
use tauri::{Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

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
pub fn place(window: &WebviewWindow, render: &RenderConfig, scale: f64, requested: Option<Point>) -> Result<(), String> {
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
    let current_pos = window.outer_position().map_err(|e| e.to_string())?;
    let position = PhysicalPosition::new(origin.x.round() as i32, origin.y.round() as i32);
    if current_pos != position { window.set_position(position).map_err(|e| e.to_string())?; }
    let physical_size = PhysicalSize::new(size.width as u32, size.height as u32);
    if window.inner_size().map_err(|e| e.to_string())? != physical_size { window.set_size(physical_size).map_err(|e| e.to_string())?; }
    Ok(())
}
pub fn debug_render() -> RenderConfig {
    RenderConfig { canvas_width: 256, canvas_height: 256, scale: 1.0, anchor_x: 0.5, anchor_y: 0.9 }
}
