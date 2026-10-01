//! LCV-196 — each parsed [`Shape`] turned into the `Line|Circle|Arc` items
//! ADR 0010 §4 carries. The expanded count is known, and capped, before the
//! output is built.
//!
//! Names no drawing-state type; MUST NOT import `egui`, `eframe`, or `rfd`.

use core::f64::consts::{FRAC_PI_2, TAU};

use super::items::Shape;
use super::{DrawingItem, MAX_DRAWING_ENTITIES, arg};
use crate::agent::tools::ToolCallError;
use crate::geometry::{EPSILON, Vec2};
use crate::text::layout::{DEFAULT_SPACING_FACTOR, text_strokes};

/// The batch's items in batch order, each shape expanded in place.
///
/// # Errors
///
/// At `entities` when the expansion has no item or more than
/// [`MAX_DRAWING_ENTITIES`].
pub(super) fn expand(shapes: &[Shape]) -> Result<Vec<DrawingItem>, ToolCallError> {
    let total = shapes
        .iter()
        .fold(0usize, |sum, s| sum.saturating_add(base(s).len()));
    if total == 0 || total > MAX_DRAWING_ENTITIES {
        let form = "items that expand to 1 to 1000 entities";
        let reason = format!("expands to {total} entities");
        return Err(arg("entities".to_owned(), &reason, form));
    }
    let mut out = Vec::with_capacity(total);
    for shape in shapes {
        out.extend(base(shape));
    }
    Ok(out)
}

/// The items of one shape.
fn base(shape: &Shape) -> Vec<DrawingItem> {
    match shape {
        Shape::Item(item) => vec![item.clone()],
        Shape::Polyline { points, closed } => {
            let close = closed.then(|| (points[points.len() - 1], points[0]));
            let pairs = points.windows(2).map(|w| (w[0], w[1]));
            pairs.chain(close).map(|(p, q)| line(p, q)).collect()
        }
        &Shape::Rect {
            corner,
            width,
            height,
            radius,
        } => rect(corner, width, height, radius),
        &Shape::Polygon {
            center,
            r,
            sides,
            start,
        } => {
            let step = TAU / sides as f64;
            let vertex = |k: usize| {
                let a = start + step * (k % sides) as f64;
                center + Vec2::new(r * a.cos(), r * a.sin())
            };
            (0..sides).map(|k| line(vertex(k), vertex(k + 1))).collect()
        }
        Shape::Text {
            origin,
            height,
            text,
        } => text_strokes(text, *origin, *height, DEFAULT_SPACING_FACTOR)
            .into_iter()
            .map(|(p, q)| line(p, q))
            .collect(),
    }
}

/// CCW from the bottom side: each side that has length, then the CCW
/// quarter arc after it when `radius` > 0.
fn rect(corner: Vec2, width: f64, height: f64, radius: f64) -> Vec<DrawingItem> {
    let (x, y, c) = (corner.x, corner.y, radius);
    let (x2, y2) = (x + width, y + height);
    let sides = [
        (Vec2::new(x + c, y), Vec2::new(x2 - c, y), width),
        (Vec2::new(x2, y + c), Vec2::new(x2, y2 - c), height),
        (Vec2::new(x2 - c, y2), Vec2::new(x + c, y2), width),
        (Vec2::new(x, y2 - c), Vec2::new(x, y + c), height),
    ];
    let centres = [
        (x2 - c, y + c),
        (x2 - c, y2 - c),
        (x + c, y2 - c),
        (x + c, y + c),
    ];
    let mut out = Vec::with_capacity(8);
    for (k, ((p, q, side), (cx, cy))) in sides.into_iter().zip(centres).enumerate() {
        if side - 2.0 * c > EPSILON {
            out.push(line(p, q));
        }
        if c > 0.0 {
            let start = (k as f64 - 1.0) * FRAC_PI_2;
            out.push(DrawingItem::Arc {
                cx,
                cy,
                r: c,
                start,
                end: start + FRAC_PI_2,
                ccw: true,
            });
        }
    }
    out
}

fn line(p: Vec2, q: Vec2) -> DrawingItem {
    DrawingItem::Line {
        x1: p.x,
        y1: p.y,
        x2: q.x,
        y2: q.y,
    }
}
