//! Integration tests for SVG import — full coverage of LCV-057 acceptance
//! criteria (AC 7–18). Smoke checks (AC 4–6) live in `src/io/svg/import.rs`.
//!
//! Coordinates follow the LCV-100 convention: SVG is Y-down over a 400 mm bed,
//! so an imported world Y is `400 − y_svg` and an SVG `sweep-flag` of 1 means a
//! **clockwise** world arc.

use lasercad::document::{Document, Entity};
use lasercad::geometry::{Arc, Circle, EPSILON, Line, Vec2};
use lasercad::io::svg::{SvgImportError, export_svg, import_svg};
use std::f64::consts::{FRAC_PI_2, PI};

fn svg_wrap(inner: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{inner}</svg>"#)
}

/// AC 7 — `<line>` element parsed to Entity::Line with correct coordinates.
#[test]
fn line_element_parsed_to_entity_line() {
    let src = svg_wrap(r#"<line x1="1.0000" y1="2.0000" x2="11.0000" y2="7.0000"/>"#);
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    let Entity::Line(l) = &entities[0] else {
        panic!("expected Entity::Line")
    };
    assert!((l.p1.x - 1.0).abs() < EPSILON);
    assert!((l.p1.y - 398.0).abs() < EPSILON);
    assert!((l.p2.x - 11.0).abs() < EPSILON);
    assert!((l.p2.y - 393.0).abs() < EPSILON);
}

/// AC 8 — `<circle>` element parsed to Entity::Circle with correct fields.
#[test]
fn circle_element_parsed_to_entity_circle() {
    let src = svg_wrap(r#"<circle cx="5.0000" cy="5.0000" r="3.0000"/>"#);
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    let Entity::Circle(c) = &entities[0] else {
        panic!("expected Entity::Circle")
    };
    assert!((c.center.x - 5.0).abs() < EPSILON);
    assert!((c.center.y - 395.0).abs() < EPSILON);
    assert!((c.r - 3.0).abs() < EPSILON);
}

/// AC 9 — CCW quarter-arc reconstructed to correct center, r, angles, ccw.
#[test]
fn arc_ccw_quarter_reconstructed_correctly() {
    let src = svg_wrap(r#"<path d="M 10.0000 400.0000 A 10.0000 10.0000 0 0 0 0.0000 390.0000"/>"#);
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    let Entity::Arc(a) = &entities[0] else {
        panic!("expected Entity::Arc")
    };
    assert!((a.center.x).abs() < 1e-6, "cx={}", a.center.x);
    assert!((a.center.y).abs() < 1e-6, "cy={}", a.center.y);
    assert!((a.r - 10.0).abs() < EPSILON);
    assert!((a.start_angle).abs() < 1e-6, "start={}", a.start_angle);
    assert!(
        (a.end_angle - FRAC_PI_2).abs() < 1e-6,
        "end={}",
        a.end_angle
    );
    assert!(a.ccw);
}

/// AC 10 — large-arc flag selects the far center; the mirror leaves it alone.
#[test]
fn arc_large_flag_selects_correct_center() {
    let src =
        svg_wrap(r#"<path d="M 110.0000 300.0000 A 10.0000 10.0000 0 1 0 100.0000 310.0000"/>"#);
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    let Entity::Arc(a) = &entities[0] else {
        panic!("expected Entity::Arc")
    };
    assert!((a.center.x - 100.0).abs() < 1e-6, "cx={}", a.center.x);
    assert!((a.center.y - 100.0).abs() < 1e-6, "cy={}", a.center.y);
    assert!((a.r - 10.0).abs() < EPSILON);
    assert!((a.start_angle).abs() < 1e-6);
    assert!(
        (a.sweep_angle() - 3.0 * PI / 2.0).abs() < 1e-6,
        "sweep={}",
        a.sweep_angle()
    );
    assert!(a.ccw);
}

/// AC 11 — an SVG sweep flag of 1 is a clockwise world arc (the mirror
/// reverses handedness — LCV-100).
#[test]
fn arc_cw_sweep_flag_one_sets_ccw_false() {
    let src = svg_wrap(r#"<path d="M 0.0000 390.0000 A 10.0000 10.0000 0 0 1 10.0000 400.0000"/>"#);
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    let Entity::Arc(a) = &entities[0] else {
        panic!("expected Entity::Arc")
    };
    assert!(!a.ccw);
}

/// AC 12 — circle with r <= 0 returns MalformedAttribute.
#[test]
fn circle_negative_radius_returns_malformed_attribute() {
    let src = svg_wrap(r#"<circle cx="0" cy="0" r="-1.0000"/>"#);
    assert!(matches!(
        import_svg(&src),
        Err(SvgImportError::MalformedAttribute {
            element: "circle",
            attr: "r",
            ..
        })
    ));
}

/// AC 13 — line with non-numeric x1 returns MalformedAttribute.
#[test]
fn line_bad_attribute_returns_malformed_attribute() {
    let src = svg_wrap(r#"<line x1="abc" y1="0" x2="0" y2="0"/>"#);
    assert!(matches!(
        import_svg(&src),
        Err(SvgImportError::MalformedAttribute {
            element: "line",
            attr: "x1",
            ..
        })
    ));
}

/// AC 14, rewritten by LCV-172 AC 8 — an arc path with a non-numeric `A`
/// token opens, imports nothing and reports `path (data error)`.
#[test]
fn path_with_non_numeric_a_command_reports_a_data_error() {
    let src = svg_wrap(r#"<path d="M 0 0 A notanumber 10 0 0 1 5 5"/>"#);
    let imported = import_svg(&src).unwrap();
    assert!(imported.entities.is_empty());
    assert_eq!(imported.report, [("path (data error)".to_owned(), 1)]);
}

/// AC 15, rewritten by LCV-172 AC 3 — a non-arc `<path>` (L command)
/// imports its line.
#[test]
fn non_arc_path_imports_its_line() {
    let src = svg_wrap(r#"<path d="M 0 0 L 10 10"/>"#);
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    assert!(matches!(entities[0], Entity::Line(_)));
}

/// AC 16 — `<rect>` and other unknown elements are silently skipped.
#[test]
fn unknown_elements_silently_skipped() {
    let src = svg_wrap(
        r#"<line x1="0" y1="0" x2="1" y2="1"/>
           <rect width="10" height="10"/>
           <circle cx="5" cy="5" r="3"/>"#,
    );
    let entities = import_svg(&src).unwrap().entities;
    assert_eq!(entities.len(), 2);
    assert!(matches!(entities[0], Entity::Line(_)));
    assert!(matches!(entities[1], Entity::Circle(_)));
}

/// AC 17 — entities nested inside `<g>` groups are collected.
#[test]
fn entities_inside_g_groups_collected() {
    let src = r#"<svg xmlns="http://www.w3.org/2000/svg">
        <g id="cut"><line x1="0" y1="0" x2="5" y2="5"/></g>
        <g id="mark"></g>
        <g id="engrave"></g>
    </svg>"#;
    let entities = import_svg(src).unwrap().entities;
    assert_eq!(entities.len(), 1);
    assert!(matches!(entities[0], Entity::Line(_)));
}

/// AC 18 — round-trip export → import preserves all entity fields within EPSILON.
#[test]
fn round_trip_line_circle_arc() {
    let mut doc = Document::default();
    doc.push_current(Entity::Line(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 5.0),
    )));
    doc.push_current(Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 3.0)));
    doc.push_current(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        10.0,
        0.0,
        FRAC_PI_2,
        true,
    )));

    let svg = export_svg(&doc);
    let imported = import_svg(&svg).unwrap().entities;
    assert_eq!(imported.len(), 3);

    let Entity::Line(l) = &imported[0] else {
        panic!("expected line")
    };
    assert!((l.p1.x).abs() < 1e-3);
    assert!((l.p1.y).abs() < 1e-3);
    assert!((l.p2.x - 10.0).abs() < 1e-3);
    assert!((l.p2.y - 5.0).abs() < 1e-3);

    let Entity::Circle(c) = &imported[1] else {
        panic!("expected circle")
    };
    assert!((c.center.x - 20.0).abs() < 1e-3);
    assert!((c.center.y - 20.0).abs() < 1e-3);
    assert!((c.r - 3.0).abs() < 1e-3);

    let Entity::Arc(a) = &imported[2] else {
        panic!("expected arc")
    };
    assert!((a.center.x).abs() < 1e-3, "cx={}", a.center.x);
    assert!((a.center.y).abs() < 1e-3, "cy={}", a.center.y);
    assert!((a.r - 10.0).abs() < 1e-3);
    assert!((a.start_angle).abs() < 1e-3);
    assert!((a.end_angle - FRAC_PI_2).abs() < 1e-3);
    assert!(a.ccw);
}
