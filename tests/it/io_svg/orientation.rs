//! Golden orientation fixtures for LCV-100 — the world (Y-up) ↔ SVG (Y-down)
//! mirror.
//!
//! These tests pin **absolute** exported strings for asymmetric figures. That
//! is the whole point: a round-trip test (`import_svg(&export_svg(&doc, p))`)
//! cannot detect a symmetric transform bug, because the exporter's error and
//! the importer's error cancel each other exactly. Only a hand-checked golden
//! tells us the file a laser actually reads is the right way up.

use lasercad::document::{Document, Entity};
use lasercad::geometry::{Arc, Circle, Line, Vec2, EPSILON};
use lasercad::io::svg::{export_svg, import_svg, Preset};
use lasercad::text::layout_text;
use lasercad::util::{flip_y, DEFAULT_BED_HEIGHT_MM};
use std::f64::consts::{FRAC_PI_2, PI};

/// Round-trip tolerance for coordinates that are not exactly representable in
/// the exporter's 4-decimal format (Hershey glyph points are scaled by
/// `height / CAP_HEIGHT_HERSHEY`, so they quantise by up to 5e-5 mm).
/// Geometry with round coordinates is compared against [`EPSILON`] instead.
const EXPORT_QUANTISATION_TOL: f64 = 1e-3;

fn doc_of(entities: Vec<Entity>) -> Document {
    let mut doc = Document::default();
    entities.into_iter().for_each(|e| doc.push_current(e));
    doc
}

/// AC 15 — the asymmetry gate.
///
/// This test exists because a round-trip test cannot catch a symmetric
/// transform bug: export and import cancel each other's error, so only an
/// absolute golden proves the exported file is not mirrored.
///
/// The figure is an "L": a vertical leg from `(10, 10)` up to `(10, 60)` and a
/// short horizontal leg along the **bottom**, `(10, 10) → (40, 10)`. Because
/// SVG's Y grows downward, the leg that sits lowest in the world must carry the
/// **largest** SVG `y`.
#[test]
fn l_shape_bottom_leg_has_larger_svg_y() {
    let doc = doc_of(vec![
        Entity::Line(Line::new(Vec2::new(10.0, 10.0), Vec2::new(10.0, 60.0))),
        Entity::Line(Line::new(Vec2::new(10.0, 10.0), Vec2::new(40.0, 10.0))),
    ]);
    let svg = export_svg(&doc, Preset::Cut);

    let vertical = r#"<line x1="10.0000" y1="390.0000" x2="10.0000" y2="340.0000"/>"#;
    let horizontal = r#"<line x1="10.0000" y1="390.0000" x2="40.0000" y2="390.0000"/>"#;
    assert!(svg.contains(vertical), "{svg}");
    assert!(svg.contains(horizontal), "{svg}");

    // The leg that is lowest in the world carries the largest SVG y.
    let bottom_leg_svg_y = flip_y(10.0, DEFAULT_BED_HEIGHT_MM);
    let free_end_svg_y = flip_y(60.0, DEFAULT_BED_HEIGHT_MM);
    assert!(
        bottom_leg_svg_y > free_end_svg_y,
        "bottom leg {bottom_leg_svg_y} must sit below {free_end_svg_y} in SVG space"
    );
}

/// AC 16 — Hershey text rides the same code path as any other line; there is
/// no text-specific special case, so engraved text comes out readable rather
/// than mirrored.
#[test]
fn hershey_text_is_not_mirrored() {
    let entities = layout_text("F", Vec2::new(10.0, 10.0), 10.0, 1.0);
    assert!(!entities.is_empty(), "layout_text produced no geometry");

    let mut min_world_y = f64::INFINITY;
    for entity in &entities {
        let Entity::Line(l) = entity else {
            panic!("layout_text must yield only lines")
        };
        min_world_y = min_world_y.min(l.p1.y).min(l.p2.y);
    }

    let doc = doc_of(entities.clone());
    let svg = export_svg(&doc, Preset::Cut);

    // (a) Round-trip preserves every endpoint.
    let imported = import_svg(&svg).unwrap().entities;
    assert_eq!(imported.len(), entities.len());
    for (before, after) in entities.iter().zip(imported.iter()) {
        let (Entity::Line(a), Entity::Line(b)) = (before, after) else {
            panic!("expected lines")
        };
        assert!(
            (a.p1.x - b.p1.x).abs() < EXPORT_QUANTISATION_TOL
                && (a.p1.y - b.p1.y).abs() < EXPORT_QUANTISATION_TOL
        );
        assert!(
            (a.p2.x - b.p2.x).abs() < EXPORT_QUANTISATION_TOL
                && (a.p2.y - b.p2.y).abs() < EXPORT_QUANTISATION_TOL
        );
    }

    // (b) The lowest world Y appears mirrored, and never raw as a y1/y2 value.
    assert!(
        svg.contains(&format!(
            "{:.4}",
            flip_y(min_world_y, DEFAULT_BED_HEIGHT_MM)
        )),
        "flipped min Y missing: {svg}"
    );
    assert!(
        !svg.contains(&format!("y1=\"{min_world_y:.4}\"")),
        "raw world Y leaked into y1: {svg}"
    );
    assert!(
        !svg.contains(&format!("y2=\"{min_world_y:.4}\"")),
        "raw world Y leaked into y2: {svg}"
    );
}

/// AC 17 — the round trip still holds for every entity kind, including all
/// four golden arcs. (Necessary, but not sufficient on its own — see the doc
/// comment of `l_shape_bottom_leg_has_larger_svg_y`.)
#[test]
fn round_trip_preserves_all_entity_kinds() {
    let arcs = [
        Arc::new(Vec2::new(0.0, 0.0), 10.0, 0.0, FRAC_PI_2, true),
        Arc::new(Vec2::new(0.0, 0.0), 10.0, FRAC_PI_2, 0.0, false),
        Arc::new(Vec2::new(0.0, 0.0), 10.0, 0.0, PI, true),
        Arc::new(Vec2::new(100.0, 100.0), 10.0, 0.0, 3.0 * FRAC_PI_2, true),
    ];
    let line = Line::new(Vec2::new(1.0, 2.0), Vec2::new(11.0, 7.0));
    let circle = Circle::new(Vec2::new(100.0, 250.0), 5.0);

    let mut entities = vec![Entity::Line(line), Entity::Circle(circle)];
    entities.extend(arcs.iter().map(|a| Entity::Arc(*a)));

    let imported = import_svg(&export_svg(&doc_of(entities), Preset::Cut))
        .unwrap()
        .entities;
    assert_eq!(imported.len(), 6);

    let Entity::Line(l) = imported[0] else {
        panic!("expected line")
    };
    assert!((l.p1.x - line.p1.x).abs() < EPSILON && (l.p1.y - line.p1.y).abs() < EPSILON);
    assert!((l.p2.x - line.p2.x).abs() < EPSILON && (l.p2.y - line.p2.y).abs() < EPSILON);

    let Entity::Circle(c) = imported[1] else {
        panic!("expected circle")
    };
    assert!((c.center.x - circle.center.x).abs() < EPSILON);
    assert!((c.center.y - circle.center.y).abs() < EPSILON);
    assert!((c.r - circle.r).abs() < EPSILON);

    for (i, expected) in arcs.iter().enumerate() {
        let Entity::Arc(a) = imported[2 + i] else {
            panic!("expected arc at {i}")
        };
        assert!((a.center.x - expected.center.x).abs() < EPSILON, "arc {i}");
        assert!((a.center.y - expected.center.y).abs() < EPSILON, "arc {i}");
        assert!((a.r - expected.r).abs() < EPSILON, "arc {i}");
        let (sp, esp) = (a.start_point(), expected.start_point());
        let (ep, eep) = (a.end_point(), expected.end_point());
        assert!(
            (sp.x - esp.x).abs() < EPSILON && (sp.y - esp.y).abs() < EPSILON,
            "arc {i} start"
        );
        assert!(
            (ep.x - eep.x).abs() < EPSILON && (ep.y - eep.y).abs() < EPSILON,
            "arc {i} end"
        );
        assert_eq!(a.ccw, expected.ccw, "arc {i} handedness");
        assert!(
            (a.sweep_angle() - expected.sweep_angle()).abs() < EPSILON,
            "arc {i} sweep"
        );
    }
}
