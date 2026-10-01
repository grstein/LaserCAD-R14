//! LCV-177 (ADR 0016) — Bézier entities through SVG.
//!
//! Import (AC 1..4): `C`/`c` is one cubic and `Q`/`q` one quadratic with
//! the segment's absolute points; `S`/`T` reflect the previous `C`/`S` (or
//! `Q`/`T`) control point about the current point, else take the current
//! point; every point goes through the CTM and the Y mirror; a curve whose
//! points all coincide creates nothing and is reported
//! `path curve (degenerate)`.
//!
//! Export (AC 11): a cubic is one `M x0 y0 C x1 y1 x2 y2 x3 y3` path and a
//! quadratic one `M x0 y0 Q x1 y1 x2 y2` path, every point in SVG
//! coordinates (`y` mirrored about the document's own bed height), four
//! decimals. AC 12 (bytes unchanged without Béziers) is pinned by
//! `export_audit.rs::export_bytes_unchanged_for_golden_document`.

use lasercad::document::{Document, Entity};
use lasercad::geometry::{Bezier, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};

/// Bed height of every export scene, mm; not the default, so a constant
/// height in the mirror shows.
const BED_H: f64 = 150.0;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

/// The one entity line `export_svg` writes for `b` on a 300 × 150 bed.
fn exported(b: Bezier) -> String {
    let mut doc = Document::with_bed([300.0, BED_H]);
    doc.push_current(Entity::Bezier(b));
    let text = export_svg(&doc);
    let body: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("<svg") && !l.starts_with("<g") && !l.starts_with("</"))
        .collect();
    assert_eq!(body.len(), 1, "{text}");
    body[0].to_owned()
}

/// AC 11 — a cubic writes `M … C …` with every `y` mirrored, four decimals.
#[test]
fn cubic_exports_one_c_path() {
    let b = Bezier::Cubic([
        v(10.0, 20.0),
        v(30.5, 80.25),
        v(70.0, -10.0),
        v(100.123_456, 40.0),
    ]);
    assert_eq!(
        exported(b),
        r#"<path d="M 10.0000 130.0000 C 30.5000 69.7500 70.0000 160.0000 100.1235 110.0000"/>"#
    );
}

/// AC 11 — a quadratic writes `M … Q …`, never elevated to a cubic.
#[test]
fn quadratic_exports_one_q_path() {
    let b = Bezier::Quadratic([v(5.0, 5.0), v(50.0, 140.0), v(95.5, 5.000_04)]);
    assert_eq!(
        exported(b),
        r#"<path d="M 5.0000 145.0000 Q 50.0000 10.0000 95.5000 145.0000"/>"#
    );
}

/// `body` inside a 300 × 150 mm root whose user unit is one millimetre.
fn import(body: &str) -> (Vec<Entity>, Vec<(String, usize)>) {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300mm" height="{BED_H}mm" viewBox="0 0 300 {BED_H}">{body}</svg>"#
    );
    let imported = import_svg(&svg).unwrap_or_else(|e| panic!("{body}: {e:?}"));
    (imported.entities, imported.report)
}

/// The entities of one `<path d>`, asserting an empty report.
fn curves(d: &str) -> Vec<Entity> {
    let (entities, report) = import(&format!(r#"<path d="{d}"/>"#));
    assert!(report.is_empty(), "{d}: {report:?}");
    entities
}

/// World point of an SVG point on the 1 mm-per-unit root.
fn w(x: f64, y: f64) -> Vec2 {
    v(x, BED_H - y)
}

fn cubic(p: [(f64, f64); 4]) -> Entity {
    Entity::Bezier(Bezier::Cubic(p.map(|(x, y)| w(x, y))))
}

fn quad(p: [(f64, f64); 3]) -> Entity {
    Entity::Bezier(Bezier::Quadratic(p.map(|(x, y)| w(x, y))))
}

/// AC 1 — `C` and `c`: one cubic each, with absolute points.
#[test]
fn c_segments_import_as_cubics() {
    assert_eq!(
        curves("M 10 20 C 30 40 50 60 70 80 c 1 2 3 4 5 6"),
        [
            cubic([(10.0, 20.0), (30.0, 40.0), (50.0, 60.0), (70.0, 80.0)]),
            cubic([(70.0, 80.0), (71.0, 82.0), (73.0, 84.0), (75.0, 86.0)]),
        ]
    );
}

/// AC 2 — `S` reflects the second control point of a previous `C` or `S`
/// about the current point; after any other command it takes the current
/// point. `s` is relative.
#[test]
fn s_reflects_the_previous_cubic_control() {
    assert_eq!(
        curves("M 0 0 C 10 0 20 10 30 10 S 50 20 60 0 s 20 0 30 10"),
        [
            cubic([(0.0, 0.0), (10.0, 0.0), (20.0, 10.0), (30.0, 10.0)]),
            cubic([(30.0, 10.0), (40.0, 10.0), (50.0, 20.0), (60.0, 0.0)]),
            cubic([(60.0, 0.0), (70.0, -20.0), (80.0, 0.0), (90.0, 10.0)]),
        ]
    );
    assert_eq!(
        curves("M 0 0 L 10 10 S 20 30 40 40"),
        [
            Entity::Line(Line::new(w(0.0, 0.0), w(10.0, 10.0))),
            cubic([(10.0, 10.0), (10.0, 10.0), (20.0, 30.0), (40.0, 40.0)]),
        ]
    );
    assert_eq!(
        curves("M 0 0 Q 5 9 10 10 S 20 30 40 40"),
        [
            quad([(0.0, 0.0), (5.0, 9.0), (10.0, 10.0)]),
            cubic([(10.0, 10.0), (10.0, 10.0), (20.0, 30.0), (40.0, 40.0)]),
        ],
        "a quadratic control is not reflected into S"
    );
}

/// AC 3 — `Q` and `q`: one quadratic each, never elevated to a cubic.
#[test]
fn q_segments_import_as_quadratics() {
    assert_eq!(
        curves("M 0 0 Q 10 20 30 0 q 10 20 30 0"),
        [
            quad([(0.0, 0.0), (10.0, 20.0), (30.0, 0.0)]),
            quad([(30.0, 0.0), (40.0, 20.0), (60.0, 0.0)]),
        ]
    );
}

/// AC 3 — `T` reflects the control point of a previous `Q` or `T`; after a
/// cubic (or anything else) it takes the current point. `t` is relative.
#[test]
fn t_reflects_the_previous_quadratic_control() {
    assert_eq!(
        curves("M 0 0 Q 10 20 30 0 T 60 0 t 30 0"),
        [
            quad([(0.0, 0.0), (10.0, 20.0), (30.0, 0.0)]),
            quad([(30.0, 0.0), (50.0, -20.0), (60.0, 0.0)]),
            quad([(60.0, 0.0), (70.0, 20.0), (90.0, 0.0)]),
        ]
    );
    assert_eq!(
        curves("M 0 0 C 5 5 10 5 20 0 T 40 0"),
        [
            cubic([(0.0, 0.0), (5.0, 5.0), (10.0, 5.0), (20.0, 0.0)]),
            quad([(20.0, 0.0), (20.0, 0.0), (40.0, 0.0)]),
        ]
    );
}

/// AC 1, AC 3 — extra argument groups repeat the command, each one curve,
/// and a repeated `S`/`T` reflects the curve just before it.
#[test]
fn implicit_repetition_draws_one_curve_per_group() {
    assert_eq!(
        curves("M 0 0 C 1 1 2 2 3 0 4 -1 5 -1 6 0"),
        [
            cubic([(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 0.0)]),
            cubic([(3.0, 0.0), (4.0, -1.0), (5.0, -1.0), (6.0, 0.0)]),
        ]
    );
    assert_eq!(
        curves("M 0 0 Q 1 1 2 0 3 -1 4 0 T 6 0 8 0"),
        [
            quad([(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)]),
            quad([(2.0, 0.0), (3.0, -1.0), (4.0, 0.0)]),
            quad([(4.0, 0.0), (5.0, 1.0), (6.0, 0.0)]),
            quad([(6.0, 0.0), (7.0, -1.0), (8.0, 0.0)]),
        ]
    );
}

/// AC 1 — under `skewX(30)` every point is its CTM image `(x + y·tan 30°, y)`,
/// then mirrored: a Bézier maps exactly under any affine map.
#[test]
fn a_skewed_cubic_takes_the_ctm_images() {
    let (entities, report) =
        import(r#"<path transform="skewX(30)" d="M 10 20 C 30 40 50 60 70 80"/>"#);
    assert!(report.is_empty(), "{report:?}");
    let [Entity::Bezier(Bezier::Cubic(got))] = entities.as_slice() else {
        panic!("{entities:?}");
    };
    let k = 30f64.to_radians().tan();
    let want = [(10.0, 20.0), (30.0, 40.0), (50.0, 60.0), (70.0, 80.0)];
    for (g, (x, y)) in got.iter().zip(want) {
        assert!(g.approx_eq(w(x + y * k, y), 1e-9), "{g:?} vs ({x}, {y})");
    }
}

/// AC 4 — a `C` or `Q` whose points all coincide creates nothing and is
/// counted under `path curve (degenerate)`; a closed loop is still a curve.
#[test]
fn degenerate_curves_are_reported_not_imported() {
    let (entities, report) = import(r#"<path d="M 5 5 C 5 5 5 5 5 5 Q 5 5 5 5 L 9 5"/>"#);
    assert_eq!(
        entities,
        [Entity::Line(Line::new(w(5.0, 5.0), w(9.0, 5.0)))]
    );
    assert_eq!(report, [("path curve (degenerate)".to_owned(), 2)]);
    assert_eq!(
        curves("M 5 5 C 9 0 9 10 5 5"),
        [cubic([(5.0, 5.0), (9.0, 0.0), (9.0, 10.0), (5.0, 5.0)])]
    );
}
