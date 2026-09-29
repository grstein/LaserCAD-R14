//! LCV-156 AC 5 — the status bar shows the current layer in a dropdown that
//! can also change it (one undo step, ADR 0012).

use crate::harness;

use harness::paint::{self, Run};
use lasercad::app::App;
use lasercad::document::{AddLayer, Command, LayerId};

fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
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

fn app_with_mark() -> (App, LayerId) {
    let mut app = App::default();
    let mut add = AddLayer::new("Mark", [0, 0, 255], true);
    add.do_(&mut app.document);
    let mark = add.id().expect("allocated");
    (app, mark)
}

/// The status-bar run reading `label`, if any: the bar is the bottom panel,
/// so its runs sit in the lowest 56pt of the screen.
fn in_status_bar(runs: &[Run], label: &str) -> bool {
    let floor = harness::SCREEN[1] - 56.0;
    runs.iter()
        .any(|r| r.text.trim() == label && r.pos.y >= floor)
}

/// AC 5 — the dropdown shows the current layer's name; picking another
/// layer makes it current in one undo step, and the bar follows.
#[test]
fn status_bar_dropdown_shows_and_changes_the_current_layer() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let (mut app, mark) = app_with_mark();

    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(in_status_bar(&runs, "Cut"), "the current layer is shown");
    assert!(!in_status_bar(&runs, "Mark"), "only the current one");

    let r0 = app.history.revision();
    click(&ctx, &mut app, "Cut");
    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(
        runs.iter().any(|r| r.text.trim() == "Mark"),
        "the open dropdown lists every layer"
    );
    click(&ctx, &mut app, "Mark");

    assert_eq!(app.document.current_layer(), mark);
    assert_eq!(app.history.revision(), r0 + 1, "one undo step");
    let runs = paint::painted_runs(&ctx, &mut app);
    assert!(in_status_bar(&runs, "Mark"), "the bar follows the change");

    assert!(app.history.undo(&mut app.document));
    assert_eq!(app.document.current_layer(), LayerId(0));
}

/// AC 5 — re-picking the current layer commits nothing.
#[test]
fn picking_the_current_layer_again_commits_nothing() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let (mut app, _) = app_with_mark();
    let r0 = app.history.revision();
    click(&ctx, &mut app, "Cut");
    let runs = paint::painted_runs(&ctx, &mut app);
    let in_list: Vec<_> = runs.iter().filter(|r| r.text.trim() == "Cut").collect();
    assert_eq!(in_list.len(), 2, "the bar and the open list both read Cut");
    let pos = egui::pos2(in_list[1].pos.x + 2.0, in_list[1].pos.y + 4.0);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness::frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    harness::frame(&ctx, &mut app, vec![press(true), press(false)]);
    assert_eq!(app.history.revision(), r0);
    assert_eq!(app.document.current_layer(), LayerId(0));
}
