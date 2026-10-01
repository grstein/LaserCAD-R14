//! LCV-179 — Open SVG draws `<text>` with the `App`'s font book: the test
//! constructor's book is empty (ADR 0006, ADR 0017 §4), so a text is
//! reported `text (no font)`; an injected book draws it.

use lasercad::app::App;
use lasercad::text::FontBook;
use std::path::PathBuf;

const TEXT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100"><text x="10" y="20" font-family="LCV Test Sans">Hi</text></svg>"#;

/// A fresh temporary directory holding `text.svg`.
fn text_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv179_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let svg = dir.join("text.svg");
    std::fs::write(&svg, TEXT_SVG).unwrap();
    svg
}

/// AC 3 — `App::default()` reads no font: the text is skipped and the
/// open reports it.
#[test]
fn the_test_app_has_no_font_and_reports_the_text() {
    let mut app = App::default();
    app.action_open_path(text_file("default"));
    assert_eq!(app.error_message, None);
    assert_eq!(app.document.entities.len(), 0);
    assert_eq!(app.command_feedback, "Ignored: 1 text (no font)");
}

/// AC 1 — Open draws the text with the `App`'s own book.
#[test]
fn open_draws_text_with_the_app_font_book() {
    let fonts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts");
    let mut app = App {
        fonts: FontBook::from_files(&[fonts.join("LCVTestSans-Regular.ttf")]),
        ..App::default()
    };
    app.action_open_path(text_file("injected"));
    assert_eq!(app.error_message, None);
    assert!(!app.document.entities.is_empty());
    assert_eq!(app.command_feedback, "");
}
