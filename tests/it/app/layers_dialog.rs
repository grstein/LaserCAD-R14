//! LCV-156 AC 4 / AC 6 / AC 7 — the Layers… dialog's decisions, driven
//! headless through the `App` methods its buttons call
//! (`src/app/layers.rs`). The egui half is `src/ui/layers_dialog.rs`.

use lasercad::app::{App, LayersDialog};
use lasercad::document::{Entity, LayerId};
use lasercad::geometry::{Line, Vec2};

fn line(x: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x, 0.0), Vec2::new(x + 5.0, 5.0)))
}

fn app_with_lines(n: usize) -> App {
    let mut app = App::default();
    for i in 0..n {
        app.document.push_current(line(i as f64 * 10.0));
    }
    app
}

fn dialog(app: &App) -> &LayersDialog {
    app.layers_dialog.as_ref().expect("dialog open")
}

fn names(app: &App) -> Vec<String> {
    app.document
        .layers()
        .iter()
        .map(|l| l.name.clone())
        .collect()
}

/// AC 4 — opening shows the current layer; New adds a uniquely named,
/// uniquely colored layer and selects it; its fields can be renamed,
/// recolored and have Output toggled; it can be made current; the selection
/// moves onto it. Every change is one undo step.
#[test]
fn dialog_adds_edits_sets_current_and_moves_the_selection() {
    let mut app = app_with_lines(3);
    assert!(app.layers_dialog.is_none());
    app.open_layers_dialog();
    assert_eq!(dialog(&app).selected, LayerId(0));
    assert_eq!(dialog(&app).name, "Cut");

    let r0 = app.history.revision();
    app.layers_add();
    let new_id = dialog(&app).selected;
    assert_ne!(new_id, LayerId(0));
    assert_eq!(names(&app), vec!["Cut", "Layer1"]);
    let new_color = app.document.layer(new_id).unwrap().color;
    assert_ne!(new_color, [255, 0, 0], "a color no other layer uses");
    assert_eq!(app.history.revision(), r0 + 1);

    let d = app.layers_dialog.as_mut().unwrap();
    d.name = "Engrave".into();
    d.color = [0, 170, 0];
    d.output = false;
    app.layers_apply();
    let layer = app.document.layer(new_id).unwrap().clone();
    assert_eq!(
        (layer.name.as_str(), layer.color, layer.output),
        ("Engrave", [0, 170, 0], false)
    );
    assert!(dialog(&app).message.is_empty());

    app.layers_make_current();
    assert_eq!(app.document.current_layer(), new_id);

    app.document.selection.set([0, 2]);
    app.layers_move_selection();
    let members: Vec<_> = (0..3)
        .map(|i| app.document.entity_layer(i).unwrap())
        .collect();
    assert_eq!(members, vec![new_id, LayerId(0), new_id]);
    assert_eq!(app.history.revision(), r0 + 4, "four changes, four steps");

    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.entity_layer(0), Some(LayerId(0)));

    app.layers_select(LayerId(0));
    assert_eq!(dialog(&app).name, "Cut");
    app.close_layers_dialog();
    assert!(app.layers_dialog.is_none());
}

/// AC 4 — moving an empty selection says why and commits nothing.
#[test]
fn moving_an_empty_selection_is_refused() {
    let mut app = app_with_lines(1);
    app.open_layers_dialog();
    let r0 = app.history.revision();
    app.layers_move_selection();
    assert_eq!(app.history.revision(), r0);
    assert!(
        dialog(&app).message.contains("Select"),
        "{}",
        dialog(&app).message
    );
}

/// AC 6 — a rename that duplicates another name after sanitising and case
/// folding, or a recolor onto another layer's color, is refused with the
/// reason and commits nothing.
#[test]
fn duplicate_name_or_color_is_refused_with_the_reason() {
    let mut app = app_with_lines(0);
    app.open_layers_dialog();
    app.layers_add();
    let r0 = app.history.revision();

    app.layers_dialog.as_mut().unwrap().name = " cut! ".into();
    app.layers_apply();
    assert_eq!(app.history.revision(), r0);
    let msg = &dialog(&app).message;
    assert!(msg.contains("Cut") && msg.contains("already"), "{msg}");

    let d = app.layers_dialog.as_mut().unwrap();
    d.name = "Mark".into();
    d.color = [255, 0, 0];
    app.layers_apply();
    assert_eq!(app.history.revision(), r0);
    assert!(
        dialog(&app).message.contains("#ff0000"),
        "{}",
        dialog(&app).message
    );

    app.layers_dialog.as_mut().unwrap().name = "  //  ".into();
    app.layers_apply();
    assert_eq!(app.history.revision(), r0);
    assert!(!dialog(&app).message.is_empty());

    let d = app.layers_dialog.as_mut().unwrap();
    d.name = "Mark".into();
    d.color = [0, 0, 255];
    app.layers_apply();
    assert_eq!(app.history.revision(), r0 + 1, "positive control");
    assert!(dialog(&app).message.is_empty());
}

/// AC 7 — deleting a layer that still has entities, or the last layer, is
/// refused with the reason; an empty layer deletes in one undo step.
#[test]
fn delete_refuses_non_empty_and_last_layer() {
    let mut app = app_with_lines(1);
    app.open_layers_dialog();
    let r0 = app.history.revision();

    app.layers_delete();
    assert_eq!(app.history.revision(), r0);
    assert!(
        dialog(&app).message.contains("entities"),
        "{}",
        dialog(&app).message
    );

    app.layers_add();
    let added = dialog(&app).selected;
    app.layers_delete();
    assert!(app.document.layer(added).is_none());
    assert_eq!(app.history.revision(), r0 + 2);
    assert_eq!(dialog(&app).selected, LayerId(0), "selection falls back");

    let mut lone = App::default();
    lone.open_layers_dialog();
    lone.layers_delete();
    assert_eq!(lone.document.layers().len(), 1);
    assert!(dialog(&lone).message.contains("at least one layer"));
}
