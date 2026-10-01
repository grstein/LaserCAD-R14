//! LCV-172 AC 7 — curves and elliptical arcs in a path import nothing,
//! advance the current point, and are reported once each, in order.

use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::import_svg;

/// A 100 × 100 bed: world y = 100 − svg y.
const MIXED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
  <path d="M 10 10 L 20 10 C 30 10 30 20 20 20 L 20 30 q 5 5 10 0 T 40 30 L 40 40 A 10 5 0 0 1 60 40 L 60 50"/>
</svg>"#;

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
}

#[test]
fn curves_and_elliptical_arcs_are_reported_and_lines_land_after_them() {
    let imported = import_svg(MIXED).unwrap();
    assert_eq!(
        imported.entities,
        [
            line(10.0, 90.0, 20.0, 90.0),
            line(20.0, 80.0, 20.0, 70.0),
            line(40.0, 70.0, 40.0, 60.0),
            line(60.0, 60.0, 60.0, 50.0),
        ]
    );
    let want = ["path C", "path Q", "path T", "path elliptical arc"].map(|l| (l.to_owned(), 1));
    assert_eq!(imported.report, want);
}
