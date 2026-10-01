//! LCV-175 — the style cascade on import: `display`/`visibility` (AC 7) and
//! `<style>` sheets, `style`, attributes, inheritance, `currentColor` and
//! invalid colors end to end (AC 1–6, AC 11).
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::io::svg::{ImportedSvg, import_svg};

const HEADER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">"#;

fn import(body: &str) -> ImportedSvg {
    import_svg(&format!("{HEADER}\n{body}\n</svg>")).expect("imports")
}

fn report(entries: &[(&str, usize)]) -> Vec<(String, usize)> {
    entries.iter().map(|&(l, n)| (l.to_owned(), n)).collect()
}

const L: &str = r#"<line x1="1" y1="1" x2="2" y2="2"/>"#;

/// AC 7 — `display:none` on a `<g>` hides its whole subtree, noted once
/// for that element; a sibling still imports.
#[test]
fn display_none_hides_the_subtree() {
    let body = format!(
        r#"<g style="display:none">{L}<circle cx="5" cy="5" r="1"/><g display="none">{L}</g></g>{L}"#
    );
    let imported = import(&body);
    assert_eq!(imported.entities.len(), 1);
    assert_eq!(imported.report, report(&[("hidden (display:none)", 1)]));
}

/// AC 7 — `visibility:hidden` on a `<g>` hides its lines, but a
/// `visibility:visible` child imports; each hidden element is noted.
#[test]
fn visibility_hidden_is_inherited_and_revertible() {
    let shown = r#"<line x1="7" y1="7" x2="8" y2="8" visibility="visible"/>"#;
    let body = format!(r#"<g visibility="hidden">{L}<g>{L}</g>{shown}</g>"#);
    let imported = import(&body);
    assert_eq!(imported.entities.len(), 1);
    assert_eq!(imported.report, report(&[("hidden (visibility)", 2)]));
}

/// AC 7 — `collapse` is `hidden`.
#[test]
fn visibility_collapse_is_hidden() {
    let body = r#"<line x1="1" y1="1" x2="2" y2="2" style="visibility:collapse"/>"#;
    let imported = import(body);
    assert!(imported.entities.is_empty());
    assert_eq!(imported.report, report(&[("hidden (visibility)", 1)]));
}

/// AC 7 — a `display:none` layer group still declares its layer; its
/// geometry is hidden.
#[test]
fn a_hidden_layer_group_still_declares_its_layer() {
    let body = format!(
        r##"<g data-layer="A" stroke="#ff0000" display="none">{L}</g><g data-layer="B" stroke="#0000ff">{L}</g>"##
    );
    let imported = import(&body);
    let names: Vec<&str> = imported.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["A", "B"]);
    assert_eq!(imported.entities.len(), 1);
    assert_eq!(imported.entity_layers, [imported.layers[1].id]);
    assert_eq!(imported.report, report(&[("hidden (display:none)", 1)]));
}

/// Each entity's layer name, in entity order.
fn memberships(imported: &ImportedSvg) -> Vec<String> {
    let name = |id| {
        let layer = imported.layers.iter().find(|l| l.id == id);
        layer.map_or_else(|| "?".to_owned(), |l| l.name.clone())
    };
    imported.entity_layers.iter().map(|&id| name(id)).collect()
}

fn line(attrs: &str) -> String {
    format!(r#"<line x1="1" y1="1" x2="2" y2="2" {attrs}/>"#)
}

/// AC 1, AC 3 — an Illustrator-style `<defs><style>` class colors its
/// geometry; the `<defs>` holding only a `<style>` is not reported.
#[test]
fn illustrator_style_classes_color_geometry() {
    let body = format!(
        "<defs><style>.cls-1{{fill:none;stroke:#f00;stroke-miterlimit:10}}</style></defs>{}",
        line(r#"class="cls-1""#)
    );
    let imported = import(&body);
    assert_eq!(memberships(&imported), ["#ff0000"]);
    assert_eq!(imported.layers[0].color, [255, 0, 0]);
    assert_eq!(imported.report, report(&[]));
}

/// AC 1 — `style` beats a class rule, which beats the attribute.
#[test]
fn style_attribute_beats_rule_beats_presentation_attribute() {
    let body = format!(
        "<style>.c{{stroke:blue}}</style>{}{}{}",
        line(r#"class="c" stroke="red" style="stroke:lime""#),
        line(r#"class="c" stroke="red""#),
        line(r#"stroke="red""#),
    );
    let imported = import(&body);
    assert_eq!(memberships(&imported), ["#00ff00", "#0000ff", "#ff0000"]);
}

/// AC 2, AC 3, AC 4 — the id rule wins; at-rules and unsupported selectors
/// change nothing and are reported.
#[test]
fn specificity_and_dropped_rules_end_to_end() {
    let body = format!(
        "<style>#a{{stroke:red}} line{{stroke:blue}} @media print{{line{{stroke:lime}}}} g line{{stroke:lime}}</style>{}{}",
        line(r#"id="a""#),
        line(""),
    );
    let imported = import(&body);
    assert_eq!(memberships(&imported), ["#ff0000", "#0000ff"]);
    let want = [
        ("style @media", 1),
        ("style rule (unsupported selector)", 1),
    ];
    assert_eq!(imported.report, report(&want));
}

/// AC 5, AC 6 — `currentColor` takes the `color` inherited from an ancestor.
#[test]
fn current_color_through_an_ancestor() {
    let body = format!(
        r##"<g color="#00aa00">{}</g>"##,
        line(r#"stroke="currentColor""#)
    );
    assert_eq!(memberships(&import(&body)), ["#00aa00"]);
}

/// AC 11 — an invalid color is dropped, the inherited one applies, and
/// the report counts it.
#[test]
fn an_invalid_color_is_reported_and_falls_back() {
    let body = format!(r#"<g stroke="blue">{}</g>"#, line(r#"stroke="bogus""#));
    let imported = import(&body);
    assert_eq!(memberships(&imported), ["#0000ff"]);
    assert_eq!(imported.report, report(&[("stroke (invalid color)", 1)]));
}
