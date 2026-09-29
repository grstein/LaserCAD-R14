//! LCV-156 AC 4 / AC 6 / AC 7 — the Layers… dialog's decisions, driven
//! headless through the `App` methods its buttons call
//! (`src/app/layers.rs`). The egui half is `src/ui/layers_dialog.rs`.

use crate::harness;

use harness::paint;
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

// ---------------------------------------------------------------------------
// The painted window (src/ui/layers_dialog.rs)
// ---------------------------------------------------------------------------

fn locate(runs: &[paint::Run], label: &str) -> egui::Pos2 {
    let hits: Vec<_> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(hits.len(), 1, "`{label}` painted once, saw {}", hits.len());
    egui::pos2(hits[0].pos.x + 2.0, hits[0].pos.y + hits[0].height / 2.0)
}

fn click(ctx: &egui::Context, app: &mut App, label: &str) {
    let pos = locate(&paint::painted_runs(ctx, app), label);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness::frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    harness::frame(ctx, app, vec![press(true), press(false)]);
}

/// AC 4 — `Format > Layers…` opens the window; it paints every layer (the
/// current one marked in words, not by hue), the edit fields and the
/// buttons, and a click on `New` adds a layer through the same `App` method.
#[test]
fn format_layers_opens_a_window_listing_every_layer() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = app_with_lines(2);
    app.document.selection.set([0]);

    click(&ctx, &mut app, "Format");
    click(&ctx, &mut app, "Layers…");
    assert!(
        app.layers_dialog.is_some(),
        "the menu item opens the dialog"
    );

    let runs = paint::painted_runs(&ctx, &mut app);
    let texts: Vec<&str> = runs.iter().map(|r| r.text.trim()).collect();
    for want in [
        "Layers",
        "current",
        "Entities: 2",
        "Name",
        "Color",
        "Apply",
        "New",
        "Delete",
        "Set Current",
        "Move Selection Here (1)",
        "Close",
    ] {
        assert!(texts.contains(&want), "`{want}` not painted: {texts:?}");
    }

    let r0 = app.history.revision();
    click(&ctx, &mut app, "New");
    assert_eq!(names(&app), vec!["Cut", "Layer1"]);
    assert_eq!(app.history.revision(), r0 + 1);
    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(runs.iter().any(|r| r.text.trim() == "Layer1"));

    click(&ctx, &mut app, "Close");
    assert!(app.layers_dialog.is_none());
}

/// ADR 0009 — with many layers the list scrolls and the controls below it
/// stay painted inside the window at 800×600.
#[test]
fn many_layers_keep_the_controls_inside_the_capped_body() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = app_with_lines(0);
    app.open_layers_dialog();
    for _ in 0..20 {
        app.layers_add();
    }
    assert_eq!(app.document.layers().len(), 21, "positive control");
    let screen = [800.0, 600.0];
    let _ = paint::painted_runs_at(&ctx, &mut app, screen, Vec::new());
    let runs = paint::painted_runs_at(&ctx, &mut app, screen, Vec::new());
    for label in ["Name", "Apply", "Set Current", "Close"] {
        let run = runs
            .iter()
            .find(|r| r.text.trim() == label)
            .unwrap_or_else(|| panic!("`{label}` not painted"));
        assert!(
            run.pos.y + run.height <= run.clip.max.y && run.pos.y >= run.clip.min.y,
            "`{label}` clipped: {:?} in {:?}",
            run.pos,
            run.clip
        );
    }
}
