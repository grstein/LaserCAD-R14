//! LCV-178 — `<use>`, `<defs>` and `<symbol>` on import (SVG 2 ch. 5), end
//! to end through `import_svg`. The page is 100 mm square with one user unit
//! = 1 mm, so a world point is `(x, 100 − y)`.

use lasercad::document::Entity;
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::{ImportedSvg, import_svg};

const TOL: f64 = 1e-9;

/// The import of a 100 mm page holding `inner`.
fn page(inner: &str) -> ImportedSvg {
    let src = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100mm" height="100mm" viewBox="0 0 100 100">{inner}</svg>"#
    );
    import_svg(&src).unwrap()
}

/// World point of SVG page point `(x, y)`.
fn w(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, 100.0 - y)
}

/// Assert `e` is the line from SVG point `a` to `b`.
fn assert_line(e: &Entity, a: (f64, f64), b: (f64, f64)) {
    let Entity::Line(Line { p1, p2 }) = e else {
        panic!("not a line: {e:?}")
    };
    assert!(
        p1.approx_eq(w(a.0, a.1), TOL) && p2.approx_eq(w(b.0, b.1), TOL),
        "{e:?} vs {a:?} {b:?}"
    );
}

/// The report count of `label`, 0 when absent.
fn count(imported: &ImportedSvg, label: &str) -> usize {
    let entry = imported.report.iter().find(|(l, _)| l == label);
    entry.map_or(0, |(_, n)| *n)
}

/// The name of the layer entity `i` landed on.
fn layer_of(imported: &ImportedSvg, i: usize) -> &str {
    let id = imported.entity_layers[i];
    let layer = imported.layers.iter().find(|l| l.id == id).unwrap();
    &layer.name
}

const LINE: &str = r#"x1="0" y1="0" x2="5" y2="0""#;

/// AC 1 — a `<use>` of a `<line>` in `<defs>` places a copy at `x`/`y`,
/// after the `<use>`'s own transform.
#[test]
fn a_use_of_a_line_places_a_copy_at_x_y() {
    let svg = page(&format!(
        r##"<defs><line id="l" {LINE}/></defs>
            <use href="#l" x="10" y="20"/>
            <use href="#l" transform="translate(0 50)" x="10"/>"##
    ));
    assert_eq!(svg.entities.len(), 2, "{:?}", svg.entities);
    assert_line(&svg.entities[0], (10.0, 20.0), (15.0, 20.0));
    assert_line(&svg.entities[1], (10.0, 50.0), (15.0, 50.0));
}

/// AC 1 — a `<use>` of a `<g>` copies every child under the group's own
/// transform.
#[test]
fn a_use_of_a_group_copies_its_children() {
    let svg = page(&format!(
        r##"<defs><g id="g" transform="translate(1 0)"><line {LINE}/><line x1="0" y1="1" x2="0" y2="2"/></g></defs>
            <use href="#g" y="10"/>"##
    ));
    assert_eq!(svg.entities.len(), 2, "{:?}", svg.entities);
    assert_line(&svg.entities[0], (1.0, 10.0), (6.0, 10.0));
    assert_line(&svg.entities[1], (1.0, 11.0), (1.0, 12.0));
}

/// AC 3 — a `<symbol>`'s viewBox maps into the `<use>`'s width/height.
#[test]
fn a_use_of_a_symbol_maps_its_view_box() {
    let svg = page(
        r##"<symbol id="s" viewBox="0 0 10 10"><line x1="0" y1="0" x2="10" y2="10"/></symbol>
            <use href="#s" x="5" width="20" height="20"/>"##,
    );
    assert_eq!(svg.entities.len(), 1, "{:?}", svg.entities);
    assert_line(&svg.entities[0], (5.0, 0.0), (25.0, 20.0));
}

/// AC 2 — `href` wins over `xlink:href`; `xlink:href` alone works.
#[test]
fn href_wins_over_xlink_href() {
    let svg = page(
        r##"<defs><line id="a" x1="1" y1="1" x2="2" y2="1"/><line id="b" x1="3" y1="3" x2="4" y2="3"/></defs>
            <use href="#a" xlink:href="#b"/><use xlink:href="#b"/>"##,
    );
    assert_eq!(svg.entities.len(), 2, "{:?}", svg.entities);
    assert_line(&svg.entities[0], (1.0, 1.0), (2.0, 1.0));
    assert_line(&svg.entities[1], (3.0, 3.0), (4.0, 3.0));
}

/// AC 4 — the `<use>`'s stroke is inherited, but a sheet rule and the
/// target's own stroke win, as in place.
#[test]
fn styles_inherit_from_the_use_and_the_target_s_own_win() {
    let svg = page(&format!(
        r##"<style>.c {{ stroke: #0000ff }}</style>
            <defs><line id="plain" {LINE}/><line id="own" stroke="#00ff00" {LINE}/><line id="cls" class="c" {LINE}/></defs>
            <use href="#plain" stroke="#ff0000"/><use href="#own" stroke="#ff0000"/><use href="#cls" stroke="#ff0000"/>"##
    ));
    let names: Vec<_> = (0..3).map(|i| layer_of(&svg, i)).collect();
    assert_eq!(names, ["#ff0000", "#00ff00", "#0000ff"]);
}

/// AC 5 — an instance lands on the `<use>`'s layer group; outside any, on
/// the `<use>`'s color layer.
#[test]
fn an_instance_lands_on_the_use_s_layer() {
    let svg = page(&format!(
        r##"<defs><line id="l" {LINE}/></defs>
            <g data-layer="Cut" stroke="#ff0000"><use href="#l"/></g>
            <use href="#l" stroke="#0000ff"/>"##
    ));
    assert_eq!(svg.entities.len(), 2, "{:?}", svg.entities);
    assert_eq!(layer_of(&svg, 0), "Cut");
    assert_eq!(layer_of(&svg, 1), "#0000ff");
}

/// AC 6 — a `<use>` inside referenced content expands too.
#[test]
fn a_use_of_a_use_expands_recursively() {
    let svg = page(&format!(
        r##"<defs><line id="l" {LINE}/><use id="u" href="#l" x="1"/></defs>
            <use href="#u" x="10"/>"##
    ));
    assert_eq!(svg.entities.len(), 1, "{:?}", svg.entities);
    assert_line(&svg.entities[0], (11.0, 0.0), (16.0, 0.0));
}

/// AC 7 — mutual references are skipped once re-entered and reported.
#[test]
fn a_cycle_is_skipped_and_reported() {
    let svg = page(&format!(
        r##"<defs><g id="a"><line {LINE}/><use href="#b"/></g><g id="b"><use href="#a"/></g></defs>
            <use href="#a"/>"##
    ));
    assert_eq!(svg.entities.len(), 1, "{:?}", svg.entities);
    assert_eq!(count(&svg, "use (cycle)"), 1, "{:?}", svg.report);
}

/// AC 8 — an unknown or external reference imports nothing, counted.
#[test]
fn an_unresolved_reference_is_skipped_and_reported() {
    let svg = page(r##"<use href="#nope"/><use href="other.svg#a"/><use/>"##);
    assert!(svg.entities.is_empty(), "{:?}", svg.entities);
    assert_eq!(svg.report, [("use (unresolved)".to_owned(), 3)]);
}
