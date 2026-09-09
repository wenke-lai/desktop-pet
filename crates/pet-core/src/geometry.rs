use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Point { pub x: f64, pub y: f64 }
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Size { pub width: f64, pub height: f64 }
#[derive(Clone, Copy, Debug)]
pub struct WorkArea { pub origin: Point, pub size: Size, pub scale: f64 }

pub fn anchor_from_origin(origin: Point, size: Size, normalized: Point) -> Point {
    Point { x: origin.x + size.width * normalized.x, y: origin.y + size.height * normalized.y }
}
pub fn origin_from_anchor(anchor: Point, size: Size, normalized: Point) -> Point {
    Point { x: anchor.x - size.width * normalized.x, y: anchor.y - size.height * normalized.y }
}
pub fn clamp_origin(origin: Point, size: Size, area: WorkArea) -> Point {
    Point {
        x: origin.x.clamp(area.origin.x, area.origin.x + (area.size.width - size.width).max(0.0)),
        y: origin.y.clamp(area.origin.y, area.origin.y + (area.size.height - size.height).max(0.0)),
    }
}
pub fn nearest_area(point: Point, areas: &[WorkArea]) -> Option<WorkArea> {
    areas.iter().copied().min_by(|a, b| distance_to_area(point, *a).total_cmp(&distance_to_area(point, *b)))
}
fn distance_to_area(point: Point, area: WorkArea) -> f64 {
    let x = point.x.clamp(area.origin.x, area.origin.x + area.size.width);
    let y = point.y.clamp(area.origin.y, area.origin.y + area.size.height);
    (point.x - x).powi(2) + (point.y - y).powi(2)
}
