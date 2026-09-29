//! Property test: `import_svg` never panics (`io::svg::import`).
//!
//! Arbitrary text, and exported SVG that is truncated or has a span replaced
//! by XML-ish noise, must come back as `Ok` or `Err` — never a panic. The
//! importer reads files from disk, so it sees whatever a user opens.

use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

const CASES: u32 = 256;

/// `CASES` cases; failing seeds persist under the repo-root
/// `proptest-regressions/`, since `tests/` holds only `it/` and `harness/`.
fn config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/proptest-regressions/io_svg/import_fuzz.txt"
        )))),
        ..ProptestConfig::default()
    }
}

/// A small fixed drawing with every entity kind on two layers (one with an
/// escaped name, Output off and current), so mutations land in lines, circles,
/// arc paths, layer groups and the header alike.
fn sample_svg() -> String {
    let layer = |id, name: &str, color, output| Layer {
        id: LayerId(id),
        name: name.to_owned(),
        color,
        output,
    };
    let (cut, mark) = (LayerId(0), LayerId(1));
    let doc = Document::from_parts(
        [300.0, 200.0],
        vec![
            layer(0, "Cut", [255, 0, 0], true),
            layer(1, "Mark & <score>", [0, 0, 255], false),
        ],
        mark,
        vec![
            Entity::Line(Line::new(Vec2::new(10.0, 20.0), Vec2::new(30.5, 40.25))),
            Entity::Circle(Circle::new(Vec2::new(50.0, 60.0), 7.5)),
            Entity::Arc(Arc::new(Vec2::new(80.0, 80.0), 12.0, 0.3, 2.9, true)),
            Entity::Arc(Arc::new(Vec2::new(90.0, 20.0), 5.0, 4.0, 0.5, false)),
        ],
        vec![cut, mark, cut, mark],
    )
    .expect("a valid two-layer document");
    export_svg(&doc)
}

/// Noise that is likely to matter to an XML or path parser.
fn noise() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("<"),
            Just(">"),
            Just("\""),
            Just("/"),
            Just("="),
            Just(" "),
            Just("-"),
            Just("."),
            Just("e"),
            Just("0"),
            Just("9"),
            Just("A"),
            Just("M"),
            Just("NaN"),
            Just("inf"),
            Just("&amp;"),
            Just("<g id=\"cut\">"),
            Just("<g data-layer=\"x\" stroke=\"#fff\">"),
            Just(" data-output=\"2\""),
            Just(" data-current=\"1\""),
            Just("style=\"stroke:inherit\""),
            Just("</g>"),
            Just("<path d=\"M 1 1 A"),
            Just("viewBox=\"0 0 "),
        ],
        0..8,
    )
    .prop_map(|parts| parts.concat())
}

/// Largest char boundary of `s` that is `<= at`.
fn floor_boundary(s: &str, at: usize) -> usize {
    (0..=at.min(s.len()))
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0)
}

proptest! {
    #![proptest_config(config())]

    /// Arbitrary text is rejected or accepted, never a panic.
    #[test]
    fn import_never_panics_on_arbitrary_text(src in any::<String>()) {
        let _ = import_svg(&src);
    }

    /// Every prefix of a real export imports without panicking.
    #[test]
    fn import_never_panics_on_truncated_export(cut in any::<prop::sample::Index>()) {
        let svg = sample_svg();
        let at = floor_boundary(&svg, cut.index(svg.len() + 1));
        let _ = import_svg(&svg[..at]);
    }

    /// A real export with one span replaced by noise imports without panicking.
    #[test]
    fn import_never_panics_on_mutated_export(
        from in any::<prop::sample::Index>(),
        len in 0usize..16,
        patch in noise(),
    ) {
        let svg = sample_svg();
        let start = floor_boundary(&svg, from.index(svg.len() + 1));
        let end = floor_boundary(&svg, start + len);
        let mutated = format!("{}{}{}", &svg[..start], patch, &svg[end..]);
        let _ = import_svg(&mutated);
    }
}
