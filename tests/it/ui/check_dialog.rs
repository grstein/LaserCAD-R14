//! LCV-190 AC 1 — the Check window paints the report, keeps its body within
//! the ADR 0009 cap however long the report is, and Close clears it.

use crate::harness;

use harness::paint::{self, Run};
use harness::raw_input;
use lasercad::app::App;

/// ADR 0009: a dialog body never grows past this many points.
const BODY_CAP: f32 = 426.0;

fn new_app(lines: &[String]) -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App {
        check_report: Some(lines.to_vec()),
        ..App::default()
    };
    (ctx, app)
}

/// Two frames at `screen`: the window's first frame places its area only.
fn settle_at(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) -> Vec<Run> {
    let _ = paint::painted_runs_at(ctx, app, screen, Vec::new());
    paint::painted_runs_at(ctx, app, screen, Vec::new())
}

fn painted<'a>(runs: &'a [Run], text: &str) -> Vec<&'a Run> {
    runs.iter().filter(|r| r.text.trim() == text).collect()
}

fn report(n: usize) -> Vec<String> {
    let mut lines = vec![format!("CHECK: {n} open ends")];
    lines.extend((0..n).map(|i| format!("open end: entity {i} at ({i}.000, 10.000) mm")));
    lines
}

/// AC 1 — every line of a short report is painted once, with a Close button.
#[test]
fn check_window_paints_every_report_line() {
    let lines = report(3);
    let (ctx, mut app) = new_app(&lines);
    let runs = settle_at(&ctx, &mut app, harness::SCREEN);
    for line in &lines {
        assert_eq!(painted(&runs, line).len(), 1, "{line:?} painted once");
    }
    assert_eq!(painted(&runs, "Close").len(), 1);
    assert_eq!(painted(&runs, "Check").len(), 1, "the window title");
}

/// ADR 0009 — a 200-finding report scrolls: from its first line to the
/// bottom of Close the body stays within 426 pt, even on a tall screen, and
/// fills it (only the button's padding under Close's text is left), and the
/// last line is not painted (the report really is longer than the body).
#[test]
fn long_report_body_stays_within_the_cap() {
    let lines = report(200);
    for screen in [harness::SCREEN, [1280.0, 1600.0], [800.0, 600.0]] {
        let (ctx, mut app) = new_app(&lines);
        let runs = settle_at(&ctx, &mut app, screen);
        let first = painted(&runs, &lines[0]);
        let close = painted(&runs, "Close");
        assert_eq!((first.len(), close.len()), (1, 1), "{screen:?}");
        let body = close[0].pos.y + close[0].height - first[0].pos.y;
        assert!(body <= BODY_CAP, "{screen:?}: body {body} pt");
        assert!(
            body >= BODY_CAP - 4.0,
            "{screen:?}: body {body} pt fills the cap"
        );
        assert!(
            close[0].pos.y + close[0].height <= screen[1],
            "{screen:?}: Close on screen"
        );
        let last = lines.last().expect("non-empty");
        assert!(painted(&runs, last).is_empty(), "{screen:?}: scrolls");
    }
}

/// AC 1 — Close clears `check_report`, which closes the window.
#[test]
fn close_clears_the_report() {
    let lines = report(3);
    let (ctx, mut app) = new_app(&lines);
    let runs = settle_at(&ctx, &mut app, harness::SCREEN);
    let close = painted(&runs, "Close")[0];
    let pos = egui::pos2(close.pos.x + 2.0, close.pos.y + close.height / 2.0);
    let _ = ctx.run_ui(raw_input(vec![egui::Event::PointerMoved(pos)]), |ui| {
        app.update_ui(ui)
    });
    let click = vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ];
    let _ = ctx.run_ui(raw_input(click), |ui| app.update_ui(ui));
    assert_eq!(app.check_report, None);
    let runs = settle_at(&ctx, &mut app, harness::SCREEN);
    assert!(painted(&runs, &lines[1]).is_empty(), "the window is gone");
}

/// AC 1 — with no report, no Check window is painted.
#[test]
fn no_report_no_window() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App::default();
    let runs = settle_at(&ctx, &mut app, harness::SCREEN);
    assert!(painted(&runs, "Check").is_empty());
}
