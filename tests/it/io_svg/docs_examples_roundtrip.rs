//! Import round-trip over the checked-in `docs/examples/*.svg` fixtures.
//!
//! Two fixtures, one bed size each — 128 mm and 400 mm square — each holding
//! all four geometry primitives the product supports: a line, a circle, an
//! arc and Hershey-rendered text. The fixtures are `include_str!`-ed at
//! compile time (no runtime path or `Path::display()` involved anywhere in
//! this file), so this test always exercises the exact bytes checked into
//! git.
//!
//! ## Why two bed heights
//!
//! The world is Y-up; SVG is Y-down; the mirror is
//! `y_svg = bed_height_mm - y_world` (`crate::util::flip_y`), and
//! `bed_height_mm` must be **the file's own declared bed height**, never a
//! constant (LCV-114). A round trip that only ever ran at one bed size could
//! not tell a correct mirror from one that hard-codes 400 mm: at the default
//! bed the hard-coded bug and the correct code agree. `bed-128mm.svg` is the
//! discriminator — 128 ≠ 400 — so [`bed128_line_round_trips_to_known_world_coordinates`]
//! asserts the *raw byte* the exporter wrote (`y1="118.0000"`, i.e.
//! `128 - 10`) alongside the *imported* world value (`10.0`, i.e.
//! `128 - 118`). A hard-coded-400 importer would read that same byte back as
//! `400 - 118 = 282`, which is nowhere near `10.0` and the assertion would
//! fail loudly. The 400 mm fixture is the fixture a human actually opens
//! most often (it is this app's default bed) and is asserted the same way,
//! but on its own it cannot catch the bug the 128 mm fixture exists to catch
//! — see AGENTS.md's SVG export section and `tests/it/io_svg/orientation.rs`
//! for the same principle applied to golden export strings.
//!
//! ## Tolerance
//!
//! Line, circle and arc coordinates in both fixtures are whole millimetres
//! (or halves), which the exporter's `{:.4}` format and `f64::parse` round
//! back exactly, so those assertions use the kernel's [`EPSILON`] (`1e-9`).
//! Hershey glyph points are scaled by `height_mm / 9.0` and land on
//! repeating decimals (thirds, ninths) that `{:.4}` truncates to four
//! places, losing up to `5e-5` mm per coordinate on export and the same
//! again on import; `TEXT_TOL = 1e-3` covers that with headroom and matches
//! the constant already justified in
//! `tests/it/io_svg/orientation.rs::EXPORT_QUANTISATION_TOL`.
//!
//! ## What "text round-trips" means here
//!
//! There is no `<text>` element anywhere in this pipeline: `export_svg`
//! forbids one (`src/io/svg/export.rs::no_forbidden_svg_elements`) and
//! `import_svg` does not recognise one (`collect` only matches `line`,
//! `circle` and `path`). `src/text/layout.rs::layout_text` converts a string
//! into `Entity::Line` strokes *before* export ever runs, so by the time a
//! `<line>` reaches this pipeline it is indistinguishable from a line the
//! operator drew by hand. "Text round-trips" therefore means exactly this,
//! and no more: **the stroke geometry `layout_text` produced for the fixture
//! string, at the fixture's origin and height, survives the export/import
//! round trip as plain lines.** The string `"128"` or `"400"` itself is not
//! recovered by import — nothing in this crate does OCR — and a criterion
//! claiming otherwise would be vacuously true of any line geometry
//! whatsoever. These tests assert the falsifiable version: entity-by-entity
//! equality against a fresh `layout_text` call with the fixture's own
//! parameters.

use core::f64::consts::FRAC_PI_2;
use lasercad::document::Entity;
use lasercad::document::Layer;
use lasercad::geometry::{EPSILON, Vec2};
use lasercad::io::svg::import_svg;
use lasercad::text::layout_text;

/// See the module doc's §Tolerance.
const TEXT_TOL: f64 = 1e-3;

const BED128_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/examples/bed-128mm.svg"
));
const BED400_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/examples/bed-400mm.svg"
));

/// The layer a v0.2 file's red geometry opens on (LCV-175).
fn red_layer() -> Layer {
    Layer {
        name: "#ff0000".to_owned(),
        ..Layer::default_cut()
    }
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

fn assert_line_eq(entity: &Entity, p1: Vec2, p2: Vec2, tol: f64, ctx: &str) {
    let Entity::Line(l) = entity else {
        panic!("{ctx}: expected a line, got {entity:?}")
    };
    assert!(
        close(l.p1.x, p1.x, tol) && close(l.p1.y, p1.y, tol),
        "{ctx}: p1 = {:?}, expected {p1:?}",
        l.p1
    );
    assert!(
        close(l.p2.x, p2.x, tol) && close(l.p2.y, p2.y, tol),
        "{ctx}: p2 = {:?}, expected {p2:?}",
        l.p2
    );
}

// ── bed-128mm.svg — header ──────────────────────────────────────────────

/// LCV-175 AC 8, AC 10 — a v0.2 file declares no layer and every entity
/// strokes red, so it opens on one `#ff0000` layer, no `Cut`.
#[test]
fn bed128_fixture_declares_its_own_bed_and_a_red_layer() {
    let imported = import_svg(BED128_SRC).unwrap();
    assert_eq!(imported.bed_mm, [128.0, 128.0]);
    assert_eq!(imported.layers, vec![red_layer()]);
    // 4 non-text primitives + every Hershey stroke for "128".
    let text = layout_text("128", Vec2::new(80.0, 78.0), 11.0, 1.0);
    assert_eq!(imported.entities.len(), 3 + text.len());
}

/// The discriminator (see module doc §Why two bed heights): pins the raw
/// mirrored byte the exporter wrote *and* the world value import recovers
/// from it, at a bed height that is not the 400 mm default.
#[test]
fn bed128_line_round_trips_to_known_world_coordinates() {
    assert!(
        BED128_SRC.contains(r#"y1="118.0000""#),
        "expected the mirrored byte 128-10=118 in the fixture"
    );
    let imported = import_svg(BED128_SRC).unwrap();
    assert_line_eq(
        &imported.entities[0],
        Vec2::new(10.0, 10.0),
        Vec2::new(48.0, 10.0),
        EPSILON,
        "bed128 line",
    );
}

#[test]
fn bed128_circle_round_trips_to_known_world_coordinates() {
    let imported = import_svg(BED128_SRC).unwrap();
    let Entity::Circle(c) = imported.entities[1] else {
        panic!("expected a circle")
    };
    assert!(close(c.center.x, 98.0, EPSILON));
    assert!(close(c.center.y, 30.0, EPSILON));
    assert!(close(c.r, 19.0, EPSILON));
}

#[test]
fn bed128_arc_round_trips_to_known_world_coordinates() {
    let imported = import_svg(BED128_SRC).unwrap();
    let Entity::Arc(a) = imported.entities[2] else {
        panic!("expected an arc")
    };
    assert!(close(a.center.x, 30.0, EPSILON) && close(a.center.y, 98.0, EPSILON));
    assert!(close(a.r, 19.0, EPSILON));
    assert!(a.ccw, "the fixture's arc is CCW in world space");
    let (sp, ep) = (a.start_point(), a.end_point());
    assert!(close(sp.x, 49.0, EPSILON) && close(sp.y, 98.0, EPSILON));
    assert!(close(ep.x, 30.0, EPSILON) && close(ep.y, 117.0, EPSILON));
    assert!(close(a.sweep_angle(), FRAC_PI_2, EPSILON));
}

/// See module doc §What "text round-trips" means here.
#[test]
fn bed128_text_strokes_round_trip_against_a_fresh_layout_text_call() {
    let imported = import_svg(BED128_SRC).unwrap();
    let expected = layout_text("128", Vec2::new(80.0, 78.0), 11.0, 1.0);
    assert!(!expected.is_empty(), "fixture text must lay out to strokes");
    assert_eq!(imported.entities.len() - 3, expected.len());
    for (i, exp) in expected.iter().enumerate() {
        let Entity::Line(exp_line) = exp else {
            panic!("layout_text must yield only lines")
        };
        assert_line_eq(
            &imported.entities[3 + i],
            exp_line.p1,
            exp_line.p2,
            TEXT_TOL,
            &format!("bed128 text stroke {i}"),
        );
    }
}

// ── bed-400mm.svg — the default-bed fixture ─────────────────────────────

/// LCV-175 AC 8, AC 10 — a v0.2 file declares no layer and every entity
/// strokes red, so it opens on one `#ff0000` layer, no `Cut`.
#[test]
fn bed400_fixture_declares_its_own_bed_and_a_red_layer() {
    let imported = import_svg(BED400_SRC).unwrap();
    assert_eq!(imported.bed_mm, [400.0, 400.0]);
    assert_eq!(imported.layers, vec![red_layer()]);
    let text = layout_text("400", Vec2::new(250.0, 245.0), 35.0, 1.0);
    assert_eq!(imported.entities.len(), 3 + text.len());
}

/// The counterpart golden: asserted the same way as the 128 mm fixture, but
/// — per the module doc — insufficient on its own to catch a hard-coded
/// mirror height, because 400 mm is this app's default.
#[test]
fn bed400_line_round_trips_to_known_world_coordinates() {
    assert!(
        BED400_SRC.contains(r#"y1="370.0000""#),
        "expected the mirrored byte 400-30=370 in the fixture"
    );
    let imported = import_svg(BED400_SRC).unwrap();
    assert_line_eq(
        &imported.entities[0],
        Vec2::new(30.0, 30.0),
        Vec2::new(150.0, 30.0),
        EPSILON,
        "bed400 line",
    );
}

#[test]
fn bed400_circle_round_trips_to_known_world_coordinates() {
    let imported = import_svg(BED400_SRC).unwrap();
    let Entity::Circle(c) = imported.entities[1] else {
        panic!("expected a circle")
    };
    assert!(close(c.center.x, 305.0, EPSILON));
    assert!(close(c.center.y, 95.0, EPSILON));
    assert!(close(c.r, 60.0, EPSILON));
}

#[test]
fn bed400_arc_round_trips_to_known_world_coordinates() {
    let imported = import_svg(BED400_SRC).unwrap();
    let Entity::Arc(a) = imported.entities[2] else {
        panic!("expected an arc")
    };
    assert!(close(a.center.x, 95.0, EPSILON) && close(a.center.y, 305.0, EPSILON));
    assert!(close(a.r, 60.0, EPSILON));
    assert!(a.ccw, "the fixture's arc is CCW in world space");
    let (sp, ep) = (a.start_point(), a.end_point());
    assert!(close(sp.x, 155.0, EPSILON) && close(sp.y, 305.0, EPSILON));
    assert!(close(ep.x, 95.0, EPSILON) && close(ep.y, 365.0, EPSILON));
    assert!(close(a.sweep_angle(), FRAC_PI_2, EPSILON));
}

/// See module doc §What "text round-trips" means here.
#[test]
fn bed400_text_strokes_round_trip_against_a_fresh_layout_text_call() {
    let imported = import_svg(BED400_SRC).unwrap();
    let expected = layout_text("400", Vec2::new(250.0, 245.0), 35.0, 1.0);
    assert!(!expected.is_empty(), "fixture text must lay out to strokes");
    assert_eq!(imported.entities.len() - 3, expected.len());
    for (i, exp) in expected.iter().enumerate() {
        let Entity::Line(exp_line) = exp else {
            panic!("layout_text must yield only lines")
        };
        assert_line_eq(
            &imported.entities[3 + i],
            exp_line.p1,
            exp_line.p2,
            TEXT_TOL,
            &format!("bed400 text stroke {i}"),
        );
    }
}

// ── Cross-fixture invariant ──────────────────────────────────────────────

/// The property both fixtures exist to pin, stated directly: the same
/// nominal shape of assertion (raw mirrored byte ↔ recovered world value)
/// holds at two bed heights that are not equal to one another, so no single
/// constant substituted for "the file's own bed height" can satisfy both.
#[test]
fn the_y_mirror_is_pinned_at_two_different_bed_heights() {
    let bed128 = import_svg(BED128_SRC).unwrap();
    let bed400 = import_svg(BED400_SRC).unwrap();
    assert_ne!(bed128.bed_mm[1], bed400.bed_mm[1]);

    let Entity::Line(l128) = bed128.entities[0] else {
        panic!("expected a line")
    };
    let Entity::Line(l400) = bed400.entities[0] else {
        panic!("expected a line")
    };
    // Both lines recover a world Y of 10 mm above their own bed's *bottom*
    // edge margin — 10 mm on the 128 mm bed, 30 mm on the 400 mm bed — each
    // read back only correctly if the mirror used that file's own bed
    // height rather than a shared constant.
    assert!(close(l128.p1.y, 10.0, EPSILON));
    assert!(close(l400.p1.y, 30.0, EPSILON));
}
