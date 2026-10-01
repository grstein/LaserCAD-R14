//! LCV-196 — each parsed [`Shape`] turned into the `Line|Circle|Arc` items
//! ADR 0010 §4 carries. The expanded count is known, and capped, before the
//! output is built.
//!
//! Names no drawing-state type; MUST NOT import `egui`, `eframe`, or `rfd`.

use core::f64::consts::{FRAC_PI_2, TAU};

use core::ops::Range;

use super::items::{Shape, Step};
use super::{DrawingItem, MAX_DRAWING_ENTITIES, arg};
use crate::agent::tools::ToolCallError;
use crate::geometry::{EPSILON, Transform, Vec2};
use crate::text::layout::{DEFAULT_SPACING_FACTOR, text_strokes};

/// The batch's items in batch order, each shape expanded in place; an
/// array appends `count − 1` copies of its sources' output after its own
/// position, copy by copy.
///
/// # Errors
///
/// At `entities` when the expansion has no item or more than
/// [`MAX_DRAWING_ENTITIES`]; the count is known before any copy is built.
pub(super) fn expand(shapes: &[Shape]) -> Result<Vec<DrawingItem>, ToolCallError> {
    let total = lens(shapes).into_iter().fold(0usize, usize::saturating_add);
    if total == 0 || total > MAX_DRAWING_ENTITIES {
        let form = "items that expand to 1 to 1000 entities";
        let reason = format!("expands to {total} entities");
        return Err(arg("entities".to_owned(), &reason, form));
    }
    let mut out: Vec<DrawingItem> = Vec::with_capacity(total);
    let mut ranges: Vec<Range<usize>> = Vec::with_capacity(shapes.len());
    for shape in shapes {
        let start = out.len();
        if let Shape::Array { of, count, step } = shape {
            let sources: Vec<usize> = sources(of, shapes)
                .into_iter()
                .flat_map(|j| ranges[j].clone())
                .collect();
            for k in 1..*count {
                for &i in &sources {
                    out.push(place(&out[i], *step, k as f64));
                }
            }
        } else {
            out.extend(base(shape));
        }
        ranges.push(start..out.len());
    }
    Ok(out)
}

/// How many items each shape expands to, saturating; built from the shapes
/// alone, so a huge array costs one number.
fn lens(shapes: &[Shape]) -> Vec<usize> {
    let mut lens: Vec<usize> = Vec::with_capacity(shapes.len());
    for shape in shapes {
        let n = match shape {
            Shape::Array { of, count, .. } => sources(of, shapes)
                .into_iter()
                .fold(0usize, |sum, j| sum.saturating_add(lens[j]))
                .saturating_mul(count - 1),
            _ => base(shape).len(),
        };
        lens.push(n);
    }
    lens
}

/// The items an array copies, in batch order: those it lists plus, for each
/// listed array, the items that one lists (its whole output).
fn sources(of: &[usize], shapes: &[Shape]) -> Vec<usize> {
    let mut all = of.to_vec();
    for &j in of {
        if let Shape::Array { of: inner, .. } = &shapes[j] {
            all.extend(inner);
        }
    }
    all.sort_unstable();
    all.dedup();
    all
}

/// Copy `k` of `item`: moved by k · offset, or turned by k · angle about the
/// centre through `Transform::Rotate`, arc angles turning with it.
fn place(item: &DrawingItem, step: Step, k: f64) -> DrawingItem {
    let (map, turn) = match step {
        Step::Linear(offset) => (Placement::Move(offset * k), 0.0),
        Step::Polar { center, angle } => {
            let turn = angle * k;
            let rotate = Transform::Rotate {
                base: center,
                angle: turn,
            };
            (Placement::Turn(rotate), turn)
        }
    };
    let at = |x: f64, y: f64| map.point(Vec2::new(x, y));
    match *item {
        DrawingItem::Line { x1, y1, x2, y2 } => line(at(x1, y1), at(x2, y2)),
        DrawingItem::Circle { cx, cy, r } => {
            let c = at(cx, cy);
            DrawingItem::Circle {
                cx: c.x,
                cy: c.y,
                r,
            }
        }
        DrawingItem::Arc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
        } => {
            let c = at(cx, cy);
            DrawingItem::Arc {
                cx: c.x,
                cy: c.y,
                r,
                start: start + turn,
                end: end + turn,
                ccw,
            }
        }
    }
}

/// A copy's point map: a translation, or a kernel rotation.
enum Placement {
    Move(Vec2),
    Turn(Transform),
}

impl Placement {
    fn point(&self, p: Vec2) -> Vec2 {
        match self {
            Placement::Move(offset) => p + *offset,
            Placement::Turn(t) => t.point(p),
        }
    }
}

/// The items of one non-array shape; an array has none of its own, its
/// copies come from the earlier output (see [`expand`]).
fn base(shape: &Shape) -> Vec<DrawingItem> {
    match shape {
        Shape::Array { .. } => Vec::new(),
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
