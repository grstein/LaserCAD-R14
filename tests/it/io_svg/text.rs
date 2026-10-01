//! LCV-179 — SVG `<text>` imported as glyph outlines, end to end: the
//! bundled OFL font "LCV Test Sans" (`tests/fixtures/fonts/`), never the
//! system fonts (AC 12).

use std::path::{Path, PathBuf};

use lasercad::io::svg::{ImportedSvg, export_svg, import_svg, import_svg_with};
use lasercad::text::FontBook;

/// A fresh book of the bundled Regular, Bold and Italic faces.
fn book() -> FontBook {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts");
    let files: Vec<PathBuf> = [
        "LCVTestSans-Regular.ttf",
        "LCVTestSans-Bold.ttf",
        "LCVTestSans-Italic.ttf",
    ]
    .iter()
    .map(|f| dir.join(f))
    .collect();
    FontBook::from_files(&files)
}

/// `tests/fixtures/svg/text-outlines.svg` imported with a fresh [`book`].
fn imported() -> ImportedSvg {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg/text-outlines.svg");
    let src = std::fs::read_to_string(path).unwrap();
    import_svg_with(&src, &book()).unwrap()
}

/// AC 11 — imported outlines export as lines and Bézier paths, never
/// `<text>`, and import back to as many entities.
#[test]
fn text_outlines_export_as_lines_and_curves() {
    let imported = imported();
    assert!(imported.report.is_empty(), "{:?}", imported.report);
    let count = imported.entities.len();
    assert!(count > 0);
    let svg = export_svg(&imported.into_document().unwrap());
    assert!(!svg.contains("<text"), "{svg}");
    assert!(svg.contains("<line "), "{svg}");
    assert!(svg.contains(" Q "), "{svg}");
    let again = import_svg(&svg).unwrap();
    assert_eq!(again.entities.len(), count);
    assert!(again.report.is_empty(), "{:?}", again.report);
}

/// AC 12 — the same file through two fresh font books imports identical
/// entities on identical layers.
#[test]
fn the_same_file_and_fonts_import_identically() {
    let (a, b) = (imported(), imported());
    assert!(!a.entities.is_empty());
    assert_eq!(a.entities, b.entities);
    assert_eq!(a.entity_layers, b.entity_layers);
    assert_eq!(a.layers, b.layers);
}
