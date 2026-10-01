//! Property test: `import_svg` never panics (`io::svg::import`).
//!
//! Arbitrary text, and exported SVG that is truncated or has a span replaced
//! by XML-ish noise, must come back as `Ok` or `Err` — never a panic. The
//! importer reads files from disk, so it sees whatever a user opens. So does
//! an arbitrary `d` inside a `<path>` (LCV-172), and an arbitrary
//! `transform`, `viewBox` or `preserveAspectRatio` (LCV-173).

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

/// Path-data fragments: every command letter, separators, number pieces,
/// glued flags, extreme and non-finite numbers (LCV-172).
fn path_token() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec![
            "M", "m", "L", "l", "H", "h", "V", "v", "Z", "z", "A", "a", "C", "c", "S", "s", "Q",
            "q", "T", "t", " ", ",", "\t", "-", "+", ".", "e", "E", "0", "1", "1110", "1e999",
            "NaN", "inf", "x",
        ])
        .prop_map(str::to_owned),
        any::<f64>().prop_map(|v| v.to_string()),
        "[0-9.eE+-]{1,6}",
    ]
}

/// `d` as an attribute value inside an otherwise valid file.
fn path_svg(d: &str) -> String {
    let d = d
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;");
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm"><path d="{d}"/></svg>"#
    )
}

/// Transform-list fragments: every function name, separators, number
/// pieces, extreme and non-finite numbers (LCV-173).
fn transform_token() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(vec![
            "matrix", "translate", "scale", "rotate", "skewX", "skewY", "(", ")", " ", ",", "-",
            ".", "e", "0", "1", "90", "1e308", "1e999", "NaN", "inf", "x",
        ])
        .prop_map(str::to_owned),
        any::<f64>().prop_map(|v| v.to_string()),
        "[0-9.eE+-]{1,6}",
    ]
}

/// `s` escaped for a double-quoted attribute value.
fn attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

/// `transform` on a group and on a nested `<svg>` inside a valid file.
fn transform_svg(t: &str) -> String {
    let t = attr(t);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm"><g transform="{t}"><circle cx="5" cy="5" r="2"/><path d="M 0 0 A 3 3 0 0 1 6 0"/><svg transform="{t}" width="50%"><line x1="0" y1="0" x2="3" y2="4"/></svg></g></svg>"#
    )
}

/// `viewBox` and `preserveAspectRatio` on the root, with or without
/// `width`/`height`, and on a nested `<svg>`.
fn view_box_svg(vb: &str, par: &str, sized: bool) -> String {
    let (vb, par) = (attr(vb), attr(par));
    let size = if sized { r#" width="100mm" height="50mm""# } else { "" };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg"{size} viewBox="{vb}" preserveAspectRatio="{par}"><circle cx="5" cy="5" r="2"/><svg x="10%" viewBox="{vb}" preserveAspectRatio="{par}"><line x1="0" y1="0" x2="3" y2="4"/></svg></svg>"#
    )
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

    /// LCV-172 AC 1/AC 8 — any `d` built from path tokens opens: a path
    /// data error is reported, it never fails the file or panics.
    #[test]
    fn import_never_fails_on_path_token_data(parts in prop::collection::vec(path_token(), 0..24)) {
        let imported = import_svg(&path_svg(&parts.concat()));
        prop_assert!(imported.is_ok(), "{:?}", imported.err());
    }

    /// LCV-172 — an arbitrary `d` string never panics the importer.
    #[test]
    fn import_never_panics_on_arbitrary_path_data(d in any::<String>()) {
        let _ = import_svg(&path_svg(&d));
    }

    /// LCV-173 AC 5 — any transform list built from tokens opens: an invalid
    /// or singular one is reported, it never fails the file or panics.
    #[test]
    fn import_never_fails_on_transform_tokens(parts in prop::collection::vec(transform_token(), 0..16)) {
        let imported = import_svg(&transform_svg(&parts.concat()));
        prop_assert!(imported.is_ok(), "{:?}", imported.err());
    }

    /// LCV-173 — an arbitrary `transform` string never panics the importer.
    #[test]
    fn import_never_panics_on_arbitrary_transform(t in any::<String>()) {
        let _ = import_svg(&transform_svg(&t));
    }

    /// LCV-173 — random `viewBox` and `preserveAspectRatio` strings, on a
    /// sized or unsized root and a nested `<svg>`, never panic the importer.
    #[test]
    fn import_never_panics_on_view_box_strings(
        vb in prop::collection::vec(transform_token(), 0..8),
        par in prop_oneof![
            prop::sample::select(vec!["", "none", "xMinYMax slice", "defer xMidYMid meet"])
                .prop_map(str::to_owned),
            any::<String>(),
        ],
        sized in any::<bool>(),
    ) {
        let _ = import_svg(&view_box_svg(&vb.join(" "), &par, sized));
    }
}
