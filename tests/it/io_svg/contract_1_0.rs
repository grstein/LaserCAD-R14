//! LCV-202 AC 3 (ADR 0018) — the SVG export contract frozen at 1.0.
//!
//! One contract document holds every 1.0 entity kind — line, Polyline
//! output (lines), TEXT strokes (`text::layout_text`, lines), circle, arc,
//! full ellipse unrotated and rotated, elliptical arc, quadratic and cubic
//! Bézier — on three layers: the default `Cut`, `Engrave & mark` (a name
//! that needs XML escaping, current) and an empty `Spare` with Output off.
//! Its export must be byte-identical to `tests/fixtures/svg/contract-1.0.svg`,
//! and the fixture must import back to the same layers, membership and
//! geometry within `FORMAT_TOL`. The fixture is never regenerated to make
//! this test pass within 1.x (ADR 0018 §2).

use core::f64::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_6, PI};
use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Arc, Bezier, Circle, Ellipse, EllipseSpan, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};
use lasercad::text::layout_text;

/// Half a unit in the fourth decimal, plus room for the mirror's own rounding.
const FORMAT_TOL: f64 = 5e-5 + 1e-9;

/// The frozen byte reference.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/svg/contract-1.0.svg"
);

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn line(a: Vec2, b: Vec2) -> Entity {
    Entity::Line(Line::new(a, b))
}

/// The contract document on a 400 × 250 bed (not the default, so a
/// constant height in the mirror shows).
fn contract_doc() -> Document {
    let cut = LayerId(0);
    let engrave = LayerId(1);
    let layers = vec![
        Layer::default_cut(),
        Layer {
            id: engrave,
            name: "Engrave & mark".to_owned(),
            color: [0, 0, 255],
            output: true,
        },
        Layer {
            id: LayerId(2),
            name: "Spare".to_owned(),
            color: [0, 170, 0],
            output: false,
        },
    ];
    let mut entities = vec![
        line(v(10.0, 20.0), v(110.0, 45.5)),
        // Polyline output: one line per segment.
        line(v(20.0, 200.0), v(60.0, 200.0)),
        line(v(60.0, 200.0), v(60.0, 230.0)),
        line(v(60.0, 230.0), v(20.0, 200.0)),
        Entity::Circle(Circle::new(v(150.0, 60.0), 25.0)),
        Entity::Arc(Arc::new(v(250.0, 60.0), 30.0, 0.0, 2.0, true)),
        Entity::Arc(Arc::new(v(250.0, 160.0), 20.0, PI, 0.5, false)),
        Entity::Ellipse(Ellipse::new(v(80.0, 120.0), 40.0, 20.0, 0.0, None)),
        Entity::Ellipse(Ellipse::new(v(180.0, 140.0), 30.0, 12.5, FRAC_PI_6, None)),
        Entity::Ellipse(Ellipse::new(
            v(320.0, 120.0),
            35.0,
            15.0,
            -FRAC_PI_3,
            Some(EllipseSpan::new(0.0, 3.0 * FRAC_PI_2, true)),
        )),
        Entity::Bezier(Bezier::Quadratic([
            v(300.0, 200.0),
            v(330.0, 240.0),
            v(360.0, 200.0),
        ])),
        Entity::Bezier(Bezier::Cubic([
            v(300.0, 20.0),
            v(320.0, 60.0),
            v(360.0, -5.0),
            v(390.0, 30.0),
        ])),
    ];
    let mut entity_layers = vec![
        cut, cut, cut, cut, cut, engrave, cut, engrave, cut, engrave, cut, engrave,
    ];
    let text = layout_text("V1", v(120.0, 200.0), 10.0, 1.0);
    assert!(!text.is_empty(), "TEXT strokes");
    entity_layers.extend(text.iter().map(|_| engrave));
    entities.extend(text);
    Document::from_parts([400.0, 250.0], layers, engrave, entities, entity_layers)
        .expect("valid contract document")
}

/// Points that pin an entity's geometry, independent of how import picks
/// equivalent parameters (full-ellipse vertices compare as a set).
fn samples(e: &Entity) -> (&'static str, Vec<Vec2>) {
    match e {
        Entity::Line(l) => ("line", vec![l.p1, l.p2]),
        Entity::Circle(c) => ("circle", vec![c.center, c.center + v(c.r, 0.0)]),
        Entity::Arc(a) => {
            let half = if a.ccw { 0.5 } else { -0.5 } * a.sweep_angle();
            let mid = Circle::new(a.center, a.r).point_at_angle(a.start_angle + half);
            ("arc", vec![a.start_point(), mid, a.end_point()])
        }
        Entity::Ellipse(el) => match el.span {
            Some(s) => {
                let half = if s.ccw { 0.5 } else { -0.5 } * el.sweep();
                let ends = [el.start_point(), el.end_point()].map(Option::unwrap_or_default);
                ("arc", vec![ends[0], el.point(s.start + half), ends[1]])
            }
            None => {
                let mut q = el.quadrants();
                q.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
                q.insert(0, el.center);
                ("ellipse", q)
            }
        },
        Entity::Bezier(Bezier::Quadratic(p)) => ("quadratic", p.to_vec()),
        Entity::Bezier(Bezier::Cubic(p)) => ("cubic", p.to_vec()),
    }
}

/// AC 3 — the export is byte-identical to the frozen fixture.
#[test]
fn contract_document_exports_the_frozen_bytes() {
    let svg = export_svg(&contract_doc());
    let fixture = std::fs::read_to_string(FIXTURE).expect("contract-1.0.svg is committed");
    assert_eq!(
        svg, fixture,
        "export drifted from the 1.0 contract (ADR 0018)"
    );
}

/// AC 3 / ADR 0018 §5 — the fixture imports back to the same layers,
/// current layer, membership and geometry within `FORMAT_TOL`.
#[test]
fn contract_fixture_reimports_within_format_tolerance() {
    let doc = contract_doc();
    let fixture = std::fs::read_to_string(FIXTURE).expect("contract-1.0.svg is committed");
    let back = import_svg(&fixture)
        .expect("imports")
        .into_document()
        .expect("valid layers");
    assert_eq!(back.bed_mm, doc.bed_mm);
    assert_eq!(back.layers(), doc.layers());
    assert_eq!(back.current_layer(), doc.current_layer());
    assert_eq!(back.entity_count(), doc.entity_count());
    for i in 0..doc.entity_count() {
        assert_eq!(back.entity_layer(i), doc.entity_layer(i), "entity {i}");
        let (kind, want) = samples(&doc.entities[i]);
        let (got_kind, got) = samples(&back.entities[i]);
        assert_eq!(got_kind, kind, "entity {i}");
        assert_eq!(got.len(), want.len(), "entity {i}");
        for (g, w) in got.iter().zip(&want) {
            assert!(
                (g.x - w.x).abs() <= FORMAT_TOL && (g.y - w.y).abs() <= FORMAT_TOL,
                "entity {i} ({kind}): {g:?} vs {w:?}"
            );
        }
    }
}
