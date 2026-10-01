//! LCV-188 AC 2 — File > New and Open continue the id counter, so no id of
//! the previous document names an entity of the next one (ADR 0014 §5).

use lasercad::app::App;
use lasercad::document::{Entity, EntityId};
use lasercad::geometry::{Line, Vec2};
use std::path::PathBuf;

fn line(x: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x, 0.0), Vec2::new(x, 10.0)))
}

/// A private, empty directory under the system temp dir.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv188_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An app with paths in `dir` and `e1..=e3` placed, `e2` deleted.
fn app_after_deletes(dir: &std::path::Path) -> App {
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        ..App::default()
    };
    for x in 0..3 {
        app.document.push_current(line(f64::from(x)));
    }
    app.document.remove_entity(1);
    app
}

const TWO_LINES_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
<line x1="10" y1="10" x2="150" y2="10" stroke="#ff0000" stroke-width="0.1"/>
<line x1="10" y1="20" x2="150" y2="20" stroke="#ff0000" stroke-width="0.1"/>
</svg>"##;

/// AC 2 — after File > New, the next entity takes `e4`, not `e1`.
#[test]
fn file_new_continues_the_id_counter() {
    let mut app = app_after_deletes(&tempdir("new"));
    app.action_new();
    assert_eq!(app.document.entity_count(), 0);
    app.document.push_current(line(9.0));
    assert_eq!(app.document.entity_id(0), Some(EntityId(4)));
}

/// AC 2 — an opened file's entities take ids after the old document's.
#[test]
fn open_path_continues_the_id_counter() {
    let dir = tempdir("open");
    let svg = dir.join("two.svg");
    std::fs::write(&svg, TWO_LINES_SVG).unwrap();
    let mut app = app_after_deletes(&dir);
    app.action_open_path(svg);
    assert_eq!(app.error_message, None, "the open must succeed");
    let ids: Vec<_> = (0..2).map(|i| app.document.entity_id(i)).collect();
    assert_eq!(ids, [Some(EntityId(4)), Some(EntityId(5))]);
    app.document.push_current(line(9.0));
    assert_eq!(app.document.entity_id(2), Some(EntityId(6)));
    assert_eq!(app.document.index_of(EntityId(1)), None);
}
