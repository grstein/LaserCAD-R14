//! LCV-188 AC 7 — entity ids are never written: the mother SVG, the per-layer
//! SVG and the autosave envelope do not depend on them (ADR 0014 §5).

use std::path::PathBuf;

use lasercad::app::App;
use lasercad::document::{CreateLine, DeleteEntities, Document, Entity, History};
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::{export_layer_svg, export_svg};

fn seg(y: f64) -> Line {
    Line::new(Vec2::new(10.0, y), Vec2::new(60.0, y))
}

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv188_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn ids(doc: &Document) -> Vec<String> {
    (0..doc.entity_count())
        .map(|i| doc.entity_id(i).unwrap().to_string())
        .collect()
}

/// Three lines pushed straight: `e1, e2, e3`.
fn pushed() -> Document {
    let mut doc = Document::default();
    for y in [10.0, 20.0, 30.0] {
        doc.push_current(Entity::Line(seg(y)));
    }
    doc
}

/// The same three lines built by create, delete and undo: other ids.
fn edited() -> Document {
    let mut doc = Document::default();
    let mut history = History::new();
    for y in [10.0, 99.0, 20.0, 30.0] {
        history.commit(Box::new(CreateLine::new(seg(y))), &mut doc);
    }
    history.commit(Box::new(DeleteEntities::new(vec![1])), &mut doc);
    history.commit(Box::new(CreateLine::new(seg(77.0))), &mut doc);
    assert!(history.undo(&mut doc));
    doc
}

/// The autosave envelope `doc` writes, as text.
fn autosave_text(doc: Document, name: &str) -> String {
    let path = tempdir(name).join("autosave.json");
    let app = App {
        document: doc,
        autosave_path: Some(path.clone()),
        ..App::default()
    };
    assert!(app.write_autosave());
    std::fs::read_to_string(path).unwrap()
}

/// AC 7 — same geometry, different ids: byte-identical exports, no id.
#[test]
fn exports_and_autosave_do_not_depend_on_ids() {
    let (a, b) = (pushed(), edited());
    let b_ids = ids(&b);
    assert_eq!(ids(&a), ["e1", "e2", "e3"]);
    assert_eq!(b_ids, ["e1", "e3", "e4"]);
    let layer = a.current_layer();
    let (mother, part) = (export_svg(&a), export_layer_svg(&a, layer));
    assert_eq!(mother, export_svg(&b));
    assert_eq!(part, export_layer_svg(&b, layer));
    for svg in [&mother, &part] {
        assert!(!svg.contains(" id="), "{svg}");
        for id in &b_ids {
            assert!(!svg.contains(&format!("\"{id}\"")), "{svg}");
        }
    }
    let saved = autosave_text(a, "autosave_a");
    assert_eq!(saved, autosave_text(b, "autosave_b"));
    for id in &b_ids {
        assert!(!saved.contains(&format!("\"{id}\"")), "{saved}");
    }
}
