//! LCV-156 AC 10 / AC 11 / AC 12 — `File > Export layers`: one LaserGRBL file
//! per layer with Output on and entities, next to the mother file.
//!
//! Every `App` is `App::default()` with `current_file` pointed into a tempdir
//! this file owns (ADR 0006); no dialog is armed.

use std::path::{Path, PathBuf};

use lasercad::app::App;
use lasercad::document::{AddLayer, Command, Document, Entity, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_layer_svg, import_svg};
use lasercad::io::{action_export_layers, layer_exports};

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv156_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn add(doc: &mut Document, name: &str, color: [u8; 3], output: bool) -> LayerId {
    let mut cmd = AddLayer::new(name, color, output);
    cmd.do_(doc);
    cmd.id().expect("allocated by do_")
}

/// `Cut` (line + arc), `Fine mark` (circle), `Off` (Output off, a line) and
/// an empty `Engrave`, on a 300 × 180 bed.
fn layered_doc() -> (Document, LayerId, LayerId) {
    let mut doc = Document::with_bed([300.0, 180.0]);
    let cut = doc.current_layer();
    let mark = add(&mut doc, "Fine mark", [0, 0, 255], true);
    let off = add(&mut doc, "Off", [9, 9, 9], false);
    add(&mut doc, "Engrave", [0, 170, 0], true);
    let line = Line::new(Vec2::new(10.0, 20.0), Vec2::new(60.0, 20.0));
    doc.push_entity(Entity::Line(line), cut);
    doc.push_entity(
        Entity::Circle(Circle::new(Vec2::new(50.0, 50.0), 5.0)),
        mark,
    );
    doc.push_entity(Entity::Line(line), off);
    let arc = Arc::new(Vec2::new(150.0, 90.0), 10.0, 0.0, 2.0, true);
    doc.push_entity(Entity::Arc(arc), cut);
    (doc, cut, mark)
}

fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// AC 10 — the plan names `<stem>-<file_key>.svg` in the mother's folder for
/// exactly the layers with Output on and entities, each holding only its own
/// layer's geometry under the LaserGRBL header.
#[test]
fn plan_has_one_file_per_output_layer_with_entities() {
    let (doc, cut, mark) = layered_doc();
    let mother = Path::new("/jobs/sign.svg");
    let plan = layer_exports(&doc, mother);

    let paths: Vec<&Path> = plan.iter().map(|(p, _)| p.as_path()).collect();
    assert_eq!(
        paths,
        vec![
            Path::new("/jobs/sign-Cut.svg"),
            Path::new("/jobs/sign-Fine_mark.svg")
        ]
    );
    assert_eq!(plan[0].1, export_layer_svg(&doc, cut));
    assert_eq!(plan[1].1, export_layer_svg(&doc, mark));

    let cut_file = &plan[0].1;
    assert!(cut_file.starts_with(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="300mm" height="180mm" viewBox="0 0 300 180" fill="none">"#
    ));
    assert!(cut_file.contains(r#"stroke-width="0.1""#));
    assert!(
        cut_file.contains(" A 10.0000 10.0000 "),
        "arcs stay arcs: {cut_file}"
    );
    assert!(
        cut_file.contains(r#"y1="160.0000""#),
        "Y is flipped on the bed"
    );
    assert!(
        !cut_file.contains("<circle"),
        "only the Cut layer's geometry"
    );
    assert!(!cut_file.contains("data-current"));

    let back = import_svg(cut_file).unwrap().into_document().unwrap();
    assert_eq!(back.entity_count(), 2);
    assert_eq!(back.layers().len(), 1);
    assert_eq!(back.layers()[0].name, "Cut");
}

/// AC 10 — the files are written (an existing one overwritten), nothing else
/// lands in the folder, and the feedback lists the file names.
#[test]
fn export_writes_overwrites_and_lists_the_files() {
    let dir = tempdir("export_layers_writes");
    let (document, cut, _) = layered_doc();
    let mother = dir.join("sign.svg");
    std::fs::write(dir.join("sign-Cut.svg"), "stale").unwrap();
    let mut app = App {
        document,
        current_file: Some(mother.clone()),
        ..App::default()
    };
    let revision = app.history.revision();

    action_export_layers(&mut app);

    assert_eq!(names(&dir), vec!["sign-Cut.svg", "sign-Fine_mark.svg"]);
    let written = std::fs::read_to_string(dir.join("sign-Cut.svg")).unwrap();
    assert_eq!(written, export_layer_svg(&app.document, cut));
    assert!(!mother.exists(), "Export layers does not save the mother");
    assert!(app.error_message.is_none());
    assert!(
        app.command_feedback.contains("sign-Cut.svg")
            && app.command_feedback.contains("sign-Fine_mark.svg"),
        "{}",
        app.command_feedback
    );
    assert_eq!(app.history.revision(), revision, "no document change");
}

/// AC 11 — an unsaved drawing asks to be saved first and writes nothing.
#[test]
fn unsaved_drawing_asks_to_save_first() {
    let (document, ..) = layered_doc();
    let mut app = App {
        document,
        ..App::default()
    };
    assert!(app.current_file.is_none());

    action_export_layers(&mut app);

    assert!(
        app.command_feedback.to_lowercase().contains("save"),
        "{}",
        app.command_feedback
    );
    assert!(app.error_message.is_none());
}

/// AC 12 — when no layer has both Output on and entities, nothing is written
/// and the operator is told so.
#[test]
fn nothing_to_export_writes_nothing_and_says_so() {
    let dir = tempdir("export_layers_nothing");
    let mut document = Document::default();
    let off = add(&mut document, "Off", [9, 9, 9], false);
    let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0));
    document.push_entity(Entity::Line(line), off);
    let mut app = App {
        document,
        current_file: Some(dir.join("empty.svg")),
        ..App::default()
    };

    action_export_layers(&mut app);

    assert!(names(&dir).is_empty(), "{:?}", names(&dir));
    assert!(
        app.command_feedback.to_lowercase().contains("nothing"),
        "{}",
        app.command_feedback
    );
}
