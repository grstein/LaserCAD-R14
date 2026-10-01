//! Path segments to world entities (LCV-172): lines, circular arcs, and the
//! report labels of what is not imported yet.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::document::entity::Entity;
use crate::geometry::{Arc, EPSILON, Line, Vec2};
use crate::io::svg::path_data::{PathData, Segment};
use crate::util::flip_y;

/// The report label of an `A` with `|rx| ≠ |ry|` (AC 7, until LCV-176).
const ELLIPTICAL_ARC: &str = "path elliptical arc";

/// The entities `data` draws, un-mirrored around `bed_h`, and one report
/// label per segment not imported, in order.
///
/// Zero-length lines are dropped (AC 3, AC 4). Arcs follow SVG 2 §F.6.6
/// (AC 6): equal endpoints draw nothing, a zero radius draws a line,
/// negative radii count as positive; `|rx| = |ry|` imports a circular arc
/// whatever its rotation (AC 5), any other arc is labelled (AC 7). Lengths
/// and radii are compared with [`EPSILON`] after the mirror.
pub(super) fn path_entities(data: &PathData, bed_h: f64) -> (Vec<Entity>, Vec<&'static str>) {
    let world = |p: Vec2| Vec2::new(p.x, flip_y(p.y, bed_h));
    let line = |a: Vec2, b: Vec2| (a.distance(b) >= EPSILON).then(|| Entity::Line(Line::new(a, b)));
    let (mut entities, mut labels) = (Vec::new(), Vec::new());
    for seg in &data.segments {
        match *seg {
            Segment::Line(a, b) => entities.extend(line(world(a), world(b))),
            Segment::Arc {
                from,
                to,
                rx,
                ry,
                large,
                sweep,
            } => {
                let (a, b) = (world(from), world(to));
                let (rx, ry) = (rx.abs(), ry.abs());
                if rx < EPSILON || ry < EPSILON {
                    entities.extend(line(a, b));
                } else if (rx - ry).abs() > EPSILON {
                    labels.push(ELLIPTICAL_ARC);
                } else {
                    entities.extend(circular_arc(a, b, rx, large, sweep));
                }
            }
            Segment::Skipped { label, .. } => labels.push(label),
        }
    }
    (entities, labels)
}

/// The circular arc of radius `rx` from `s` to `e` (world points, already
/// un-mirrored) with the SVG flags; `None` when the endpoints coincide or
/// `rx ≤ 0`.
pub(super) fn circular_arc(
    s: Vec2,
    e: Vec2,
    rx: f64,
    large_arc: bool,
    sweep_flag: bool,
) -> Option<Entity> {
    let (sx, sy, ex, ey) = (s.x, s.y, e.x, e.y);
    // Both endpoints are un-mirrored into world space first, then the centre
    // is reconstructed there (LCV-057 §Arc reconstruction, mirrored by LCV-100).
    let (dx, dy) = (ex - sx, ey - sy);
    let chord = dx.hypot(dy);
    if chord < EPSILON || rx <= 0.0 {
        return None;
    }
    // Out-of-range radius (SVG 2 §F.6.6): rounding can push a half turn's
    // chord past the diameter, so the radius scales up to reach it.
    let r = if chord > 2.0 * rx { chord / 2.0 } else { rx };
    let (mx, my) = ((sx + ex) / 2.0, (sy + ey) / 2.0);
    let h = (r * r - (chord / 2.0).powi(2)).max(0.0).sqrt();
    let (ux, uy) = (-dy / chord, dx / chord);
    // Sign branches swapped relative to the un-mirrored reading: in world
    // space the SVG sweep flag denotes the opposite handedness.
    let sign: f64 = if large_arc == sweep_flag { 1.0 } else { -1.0 };
    let (cx, cy) = (mx + sign * h * ux, my + sign * h * uy);
    let (sa, ea) = ((sy - cy).atan2(sx - cx), (ey - cy).atan2(ex - cx));
    let ctr = Vec2::new(cx, cy);
    Some(Entity::Arc(Arc::new(ctr, r, sa, ea, !sweep_flag)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::svg::path_data::parse_path_data;

    const BED_H: f64 = 100.0;

    fn entities(d: &str) -> (Vec<Entity>, Vec<&'static str>) {
        path_entities(&parse_path_data(d), BED_H)
    }

    /// A world line from SVG points (y flipped around [`BED_H`]).
    fn line(a: [f64; 2], b: [f64; 2]) -> Entity {
        let w = |p: [f64; 2]| Vec2::new(p[0], BED_H - p[1]);
        Entity::Line(Line::new(w(a), w(b)))
    }

    fn only_arc(d: &str) -> Arc {
        match entities(d) {
            (es, labels) if labels.is_empty() => match es.as_slice() {
                [Entity::Arc(a)] => *a,
                other => panic!("{d:?}: {other:?}"),
            },
            (_, labels) => panic!("{d:?}: {labels:?}"),
        }
    }

    /// AC 3 — zero-length `L`, `H`, `V` draw nothing.
    #[test]
    fn zero_length_lines_yield_nothing() {
        assert_eq!(entities("M 5 5 L 5 5 H 5 V 5 l 0 0"), (vec![], vec![]));
        assert_eq!(
            entities("M 5 5 H 9").0,
            [line([5.0, 5.0], [9.0, 5.0])],
            "a non-zero H is a line"
        );
    }

    /// AC 4 — `Z` adds the closing line only when it has length.
    #[test]
    fn z_closes_only_an_open_subpath() {
        assert_eq!(
            entities("M 0 0 L 10 0 L 10 10 Z").0,
            [
                line([0.0, 0.0], [10.0, 0.0]),
                line([10.0, 0.0], [10.0, 10.0]),
                line([10.0, 10.0], [0.0, 0.0]),
            ]
        );
        assert_eq!(
            entities("M 0 0 L 10 0 L 0 0 Z").0,
            [line([0.0, 0.0], [10.0, 0.0]), line([10.0, 0.0], [0.0, 0.0])]
        );
    }

    /// AC 5 — the rotation of a circular arc is ignored.
    #[test]
    fn a_circular_arc_ignores_its_rotation() {
        assert_eq!(
            only_arc("M 10 50 A 10 10 30 0 1 30 50"),
            only_arc("M 10 50 A 10 10 0 0 1 30 50")
        );
        let a = only_arc("M 10 50 A 10 10 0 0 1 30 50");
        assert!((a.center.x - 20.0).abs() < EPSILON && (a.center.y - 50.0).abs() < EPSILON);
    }

    /// AC 6 — SVG 2 §F.6.6 out-of-range radii.
    #[test]
    fn out_of_range_radii_follow_svg_2() {
        assert_eq!(
            entities("M 5 5 A 10 10 0 0 1 5 5"),
            (vec![], vec![]),
            "equal endpoints"
        );
        for d in ["M 0 0 A 0 5 0 0 1 10 0", "M 0 0 A 5 0 0 0 1 10 0"] {
            assert_eq!(
                entities(d),
                (vec![line([0.0, 0.0], [10.0, 0.0])], vec![]),
                "{d}"
            );
        }
        assert_eq!(
            only_arc("M 10 50 A -10 -10 0 0 1 30 50"),
            only_arc("M 10 50 A 10 10 0 0 1 30 50")
        );
        let a = only_arc("M 0 50 A 5 5 0 0 1 30 50");
        assert!((a.r - 15.0).abs() < EPSILON, "λ > 1: r = chord / 2");
        assert!((a.center.x - 15.0).abs() < EPSILON && (a.center.y - 50.0).abs() < EPSILON);
    }

    /// AC 7 — an elliptical arc or a curve imports nothing and is labelled.
    #[test]
    fn ellipses_and_curves_are_labelled_not_imported() {
        assert_eq!(
            entities("M 0 0 A 10 5 0 0 1 10 0"),
            (vec![], vec!["path elliptical arc"])
        );
        assert_eq!(
            entities("M 0 0 C 1 1 2 2 3 3 L 3 10 q 1 1 2 2"),
            (
                vec![line([3.0, 3.0], [3.0, 10.0])],
                vec!["path C", "path Q"]
            )
        );
    }
}
