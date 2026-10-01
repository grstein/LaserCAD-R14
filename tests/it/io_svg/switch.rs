//! LCV-178 — conditional processing and `<switch>` on import (SVG 2 §5.8),
//! end to end through `import_svg`. The page is 100 mm square with one user
//! unit = 1 mm, so a world point is `(x, 100 − y)`.

use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::import_svg;

const TOL: f64 = 1e-9;

/// `(entities, report)` of a 100 mm page holding `inner`.
fn page(inner: &str) -> (Vec<Entity>, Vec<(String, usize)>) {
    let src = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{inner}</svg>"#
    );
    let imported = import_svg(&src).unwrap();
    (imported.entities, imported.report)
}

/// World point of SVG page point `(x, y)`.
fn w(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, 100.0 - y)
}

fn entry(label: &str, count: usize) -> (String, usize) {
    (label.to_owned(), count)
}

fn line(e: &Entity) -> Line {
    match e {
        Entity::Line(l) => *l,
        other => panic!("not a line: {other:?}"),
    }
}

/// AC 11 — a geometry element whose `systemLanguage` is not English is not
/// imported and is counted as `<name> (conditions)`; English passes.
#[test]
fn a_failing_system_language_skips_the_element() {
    let (es, report) = page(
        r#"<line systemLanguage="fr" x1="0" y1="0" x2="5" y2="0"/>
           <line systemLanguage="en-US" x1="0" y1="1" x2="5" y2="1"/>"#,
    );
    assert_eq!(es.len(), 1, "{es:?}");
    let l = line(&es[0]);
    assert!(l.p1.approx_eq(w(0.0, 1.0), TOL), "{l:?}");
    assert_eq!(report, vec![entry("line (conditions)", 1)]);
}

/// AC 11 — a group with a required extension hides its whole subtree,
/// counted once.
#[test]
fn a_failing_group_hides_its_subtree() {
    let (es, report) = page(
        r#"<g requiredExtensions="http://example.com/ext">
             <line x1="0" y1="0" x2="5" y2="0"/><circle r="3"/>
           </g>"#,
    );
    assert!(es.is_empty(), "{es:?}");
    assert_eq!(report, vec![entry("g (conditions)", 1)]);
}

/// AC 10 — only the first direct child whose conditions pass is imported;
/// every other element child is counted as a skipped branch.
#[test]
fn a_switch_imports_only_its_first_passing_child() {
    let (es, report) = page(
        r#"<switch>
             <foreignObject requiredExtensions="http://www.w3.org/1999/xhtml"/>
             <g><line x1="1" y1="2" x2="3" y2="4"/></g>
             <line x1="9" y1="9" x2="8" y2="8"/>
           </switch>"#,
    );
    assert_eq!(es.len(), 1, "{es:?}");
    let l = line(&es[0]);
    assert!(
        l.p1.approx_eq(w(1.0, 2.0), TOL) && l.p2.approx_eq(w(3.0, 4.0), TOL),
        "{l:?}"
    );
    assert_eq!(report, vec![entry("switch (branch skipped)", 2)]);
}

/// AC 10 — a `<switch>` composes its own transform like a `<g>`.
#[test]
fn a_switch_transform_moves_its_child() {
    let (es, report) = page(
        r#"<switch transform="translate(10 20)">
             <line systemLanguage="de" x1="0" y1="0" x2="1" y2="0"/>
             <line x1="0" y1="0" x2="5" y2="0"/>
           </switch>"#,
    );
    assert_eq!(es.len(), 1, "{es:?}");
    let l = line(&es[0]);
    assert!(
        l.p1.approx_eq(w(10.0, 20.0), TOL) && l.p2.approx_eq(w(15.0, 20.0), TOL),
        "{l:?}"
    );
    assert_eq!(report, vec![entry("switch (branch skipped)", 1)]);
}

/// AC 10 — `title`, `desc` and other silent children are neither chosen
/// nor counted: the first rendering child is the branch.
#[test]
fn a_switch_skips_silent_children_when_choosing() {
    let (es, report) = page(
        r#"<switch><title>t</title><g><line x1="1" y1="2" x2="3" y2="4"/></g></switch>
           <switch><desc/><x:a xmlns:x="urn:x"/><line x1="0" y1="0" x2="5" y2="0"/><circle r="1"/></switch>"#,
    );
    assert_eq!(es.len(), 2, "{es:?}");
    let l = line(&es[0]);
    assert!(l.p1.approx_eq(w(1.0, 2.0), TOL), "{l:?}");
    let l = line(&es[1]);
    assert!(l.p2.approx_eq(w(5.0, 0.0), TOL), "{l:?}");
    assert_eq!(report, vec![entry("switch (branch skipped)", 1)]);
}
