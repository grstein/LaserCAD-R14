//! Path segments to world entities (LCV-172): lines, circular arcs, and the
//! report labels of what is not imported yet.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::document::entity::Entity;
use crate::io::svg::path_data::PathData;

/// The entities `data` draws, un-mirrored around `bed_h`, and one report
/// label per segment not imported, in order.
pub(super) fn path_entities(data: &PathData, bed_h: f64) -> (Vec<Entity>, Vec<&'static str>) {
    let _ = (data, bed_h);
    (Vec::new(), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, EPSILON, Line, Vec2};
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
