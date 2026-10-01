//! LCV-157 — COPY driven from the command line through the real frame.

use crate::harness;

use harness::{frame, submit_command, tap, type_command};
use lasercad::app::App;
use lasercad::document::{AddLayer, Command, Entity};
use lasercad::geometry::{Line, Vec2};

const EPS: f64 = 1e-9;

fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

fn line_at(app: &App, i: usize) -> Line {
    match app.document.entities[i] {
        Entity::Line(l) => l,
        ref e => panic!("expected a Line at {i}, got {e:?}"),
    }
}

/// AC1, AC3–AC6, AC8 — `co` ⏎ `0,0` ⏎ `@10,0` ⏎ `@20,0` ⏎ ⏎ on a line
/// selected on a non-current layer places two copies on that layer, each its
/// own undo step, and the blank Enter ends the run.
#[test]
fn co_places_multiple_copies_on_the_source_layer() {
    let (ctx, mut app) = boot();
    AddLayer::new("Engrave", [0, 0, 255], true).do_(&mut app.document);
    let engrave = app.document.layer_by_name("Engrave").unwrap().id;
    assert_ne!(app.document.current_layer(), engrave);
    let source = Line::new(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0));
    app.document.push_entity(Entity::Line(source), engrave);
    app.document.selection.add(0);

    submit_command(&ctx, &mut app, "co");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "COPY  Specify base point:"
    );
    submit_command(&ctx, &mut app, "0,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "COPY  Specify second point:"
    );
    submit_command(&ctx, &mut app, "@10,0");
    submit_command(&ctx, &mut app, "@20,0");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "COPY  Specify second point:",
        "COPY stays armed after a placement"
    );

    // Focus the field, then Enter on a blank line (the PLINE idiom).
    type_command(&ctx, &mut app, "9");
    app.command_line_input.clear();
    tap(&ctx, &mut app, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(app.tool_manager.active_tool_name(), "COPY");
    assert_eq!(
        app.tool_manager.active_status_text(),
        "COPY  Specify base point:"
    );

    assert_eq!(app.document.entity_count(), 3);
    assert_eq!(line_at(&app, 0), source, "the source is unchanged");
    assert!((line_at(&app, 1).p1 - Vec2::new(10.0, 0.0)).length() <= EPS);
    assert!((line_at(&app, 2).p1 - Vec2::new(20.0, 0.0)).length() <= EPS);
    for i in 0..3 {
        assert_eq!(app.document.entity_layer(i), Some(engrave), "entity {i}");
    }
    assert_eq!(app.history.len(), 2, "one undo step per placement");

    tap(&ctx, &mut app, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(
        app.document.entity_count(),
        2,
        "Ctrl+Z removes only the last copy"
    );
    assert!((line_at(&app, 1).p1 - Vec2::new(10.0, 0.0)).length() <= EPS);
}
