//! LCV-178 — `<use>`, `<defs>` and `<symbol>` on import (SVG 2 ch. 5), end
//! to end through `import_svg`. The page is 100 mm square with one user unit
//! = 1 mm, so a world point is `(x, 100 − y)`.

use lasercad::document::{Entity, History, MoveEntities};
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

/// A file of `n` nested `<use>` around one line: `u1 → line`, `uk → u(k-1)`,
/// and a drawn `<use>` of `u(n-1)`.
fn nested_uses(n: usize) -> String {
    let mut defs = format!(r#"<line id="u0" {LINE}/>"#);
    for k in 1..n {
        defs += &format!(r##"<use id="u{k}" href="#u{}"/>"##, k - 1);
    }
    format!(r##"<defs>{defs}</defs><use href="#u{}"/>"##, n - 1)
}

/// The import error of a 100 mm page holding `inner`, as `Debug` text.
fn refused(inner: &str) -> String {
    let src = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{inner}</svg>"#
    );
    match import_svg(&src) {
        Ok(svg) => panic!("imported {} entities", svg.entities.len()),
        Err(e) => format!("{e:?}: {e}"),
    }
}

/// AC 9 — 32 nested `<use>` import; 33 refuse the file, naming the limit.
#[test]
fn use_nesting_is_capped_at_depth_32() {
    assert_eq!(page(&nested_uses(32)).entities.len(), 1);
    let err = refused(&nested_uses(33));
    assert!(err.starts_with("LimitExceeded"), "{err}");
    assert!(err.contains("depth 32"), "{err}");
}

/// AC 9 — a six-level ×10 fan-out (10⁶ lines) refuses the file once it
/// passes 100 000 instanced entities, without expanding the rest.
#[test]
fn a_use_fan_out_is_refused_past_100000_entities() {
    let mut defs = format!(r#"<g id="g0"><line {LINE}/></g>"#);
    for k in 1..=6 {
        let uses = format!(r##"<use href="#g{}"/>"##, k - 1).repeat(10);
        defs += &format!(r#"<g id="g{k}">{uses}</g>"#);
    }
    let err = refused(&format!(r##"<defs>{defs}</defs><use href="#g6"/>"##));
    assert!(err.starts_with("LimitExceeded"), "{err}");
    assert!(err.contains("100000"), "{err}");
}

/// AC 9 — every expansion counts toward the cap, so a ten-level ×10
/// fan-out whose leaf draws nothing (an empty group, a hidden or invalid
/// shape, an unresolved `<use>`) is refused instead of running for ever.
#[test]
fn a_fan_out_that_draws_nothing_is_refused_quickly() {
    for leaf in [
        r#"<g id="g0"/>"#,
        r#"<line id="g0" display="none" x2="1"/>"#,
        r#"<circle id="g0" r="-1"/>"#,
        r##"<use id="g0" href="#nope"/>"##,
    ] {
        let mut defs = leaf.to_owned();
        for k in 1..=10 {
            let uses = format!(r##"<use href="#g{}"/>"##, k - 1).repeat(10);
            defs += &format!(r#"<g id="g{k}">{uses}</g>"#);
        }
        let start = std::time::Instant::now();
        let err = refused(&format!(r##"<defs>{defs}</defs><use href="#g10"/>"##));
        assert!(err.starts_with("LimitExceeded"), "{leaf}: {err}");
        assert!(
            start.elapsed().as_secs() < 30,
            "{leaf}: {:?}",
            start.elapsed()
        );
    }
}

/// AC 9 — exactly 100 000 expansions import; one more is refused. Nine
/// uses of a 4-level ×10 fan-out expand 9 × 11 111 times, one more `<use>`
/// makes 100 000.
#[test]
fn the_expansion_budget_is_inclusive() {
    let mut defs = r#"<g id="g0"/>"#.to_owned();
    for k in 1..=4 {
        let uses = format!(r##"<use href="#g{}"/>"##, k - 1).repeat(10);
        defs += &format!(r#"<g id="g{k}">{uses}</g>"#);
    }
    let body = format!(
        r##"<defs>{defs}</defs>{}<use href="#g0"/>"##,
        r##"<use href="#g4"/>"##.repeat(9)
    );
    assert!(page(&body).entities.is_empty());
    let err = refused(&format!(r##"{body}<use href="#g0"/>"##));
    assert!(err.starts_with("LimitExceeded"), "{err}");
}

/// AC 9 — the entity cap applies to instances only: 100 000 plain lines
/// import.
#[test]
fn plain_geometry_is_not_capped() {
    let lines = format!("<line {LINE}/>").repeat(100_000);
    assert_eq!(page(&lines).entities.len(), 100_000);
}

/// AC 12 — two instances of one `<line>` are two independent entities:
/// moving one leaves the other bit-identical.
#[test]
fn instances_are_independent_entities() {
    let svg = page(&format!(
        r##"<defs><line id="l" {LINE}/></defs><use href="#l"/><use href="#l" y="10"/>"##
    ));
    let mut doc = svg.into_document().unwrap();
    assert_eq!(doc.entity_count(), 2);
    let other = doc.entities[1];
    let mut history = History::default();
    history.commit(
        Box::new(MoveEntities::new(vec![0], Vec2::new(3.0, 4.0))),
        &mut doc,
    );
    assert_line(&doc.entities[0], (3.0, -4.0), (8.0, -4.0));
    assert_eq!(doc.entities[1], other);
}
