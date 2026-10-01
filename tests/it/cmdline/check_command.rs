//! LCV-190 — `check` from the command line: dock summary, report lines, and
//! nothing in the document, selection or history moves.

use std::f64::consts::FRAC_PI_2;

use crate::harness;

use harness::{frame, submit_command};
use lasercad::app::{App, Severity};
use lasercad::document::{Document, Entity, check_drawing};
use lasercad::geometry::{Arc, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};

fn boot() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    let mut app = App::default();
    frame(&ctx, &mut app, vec![]);
    (ctx, app)
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
}

/// A closed contour: three lines and a CCW quarter-arc fillet.
fn fillet_contour() -> Vec<Entity> {
    let fillet = Arc::new(Vec2::new(50.0, 20.0), 10.0, -FRAC_PI_2, 0.0, true);
    vec![
        line(10.0, 10.0, 50.0, 10.0),
        Entity::Arc(fillet),
        line(60.0, 20.0, 60.0, 60.0),
        line(60.0, 60.0, 10.0, 60.0),
        line(10.0, 60.0, 10.0, 10.0),
    ]
}

/// What `check` must never move.
fn snapshot(app: &App) -> (u64, usize, Vec<usize>, bool, usize) {
    let mut selected: Vec<usize> = app.document.selection.iter().collect();
    selected.sort_unstable();
    (
        app.history.revision(),
        app.history.len(),
        selected,
        app.has_unsaved_changes(),
        app.document.entity_count(),
    )
}

/// AC 1, AC 8 — `check` puts the joined summary in the dock and every report
/// line in `check_report`, and changes neither document, selection nor
/// history.
#[test]
fn check_fills_the_dock_and_the_report() {
    let (ctx, mut app) = boot();
    app.document.push_current(line(10.0, 10.0, 50.0, 10.0));
    app.document.push_current(line(50.0, 10.0, 10.0, 10.0));
    app.document.selection.add(1);
    let before = snapshot(&app);
    let entities = app.document.entities.clone();

    submit_command(&ctx, &mut app, "check");

    assert_eq!(
        app.command_feedback,
        "CHECK: 2 open ends; CHECK: 1 duplicate"
    );
    assert_eq!(app.command_feedback_severity, Severity::Warning);
    let expected = check_drawing(&app.document).lines();
    assert_eq!(app.check_report.as_ref(), Some(&expected));
    assert_eq!(expected.len(), 5);
    assert_eq!(snapshot(&app), before);
    assert_eq!(app.document.entities, entities);
    frame(&ctx, &mut app, vec![]);
    assert_eq!(snapshot(&app), before, "no later frame sees a change");
}

/// AC 1 — running `check` again refreshes the report.
#[test]
fn check_again_refreshes_the_report() {
    let (ctx, mut app) = boot();
    app.document.push_current(line(10.0, 10.0, 50.0, 10.0));
    submit_command(&ctx, &mut app, "check");
    assert_eq!(app.check_report.as_ref().map(Vec::len), Some(3));
    app.document.push_current(line(10.0, 10.0, 10.0, 10.0));
    submit_command(&ctx, &mut app, "check");
    assert_eq!(app.check_report, Some(check_drawing(&app.document).lines()));
    assert_eq!(app.check_report.as_ref().map(Vec::len), Some(5));
}

/// AC 7, AC 8 — a clean drawing prints `CHECK: no problems found.` and opens
/// no report; a stale report is dropped.
#[test]
fn clean_check_says_so_and_opens_no_report() {
    let (ctx, mut app) = boot();
    for e in fillet_contour() {
        app.document.push_current(e);
    }
    app.check_report = Some(vec!["stale".into()]);
    let before = snapshot(&app);

    submit_command(&ctx, &mut app, "check");

    assert_eq!(app.command_feedback, "CHECK: no problems found.");
    assert_eq!(app.command_feedback_severity, Severity::Info);
    assert_eq!(app.check_report, None);
    assert_eq!(snapshot(&app), before);
}

/// AC 7 — a closed line-and-arc contour saved and reopened through `io::svg`
/// (four-decimal coordinates, rebuilt arc centre) still checks clean.
#[test]
fn contour_through_svg_round_trip_checks_clean() {
    let mut doc = Document::with_bed([400.0, 300.0]);
    for e in fillet_contour() {
        doc.push_current(e);
    }
    let svg = export_svg(&doc);
    let reopened = import_svg(&svg)
        .expect("own export parses")
        .into_document()
        .expect("own export rebuilds");
    assert_eq!(reopened.entity_count(), 5);
    assert_eq!(
        check_drawing(&reopened).lines(),
        vec!["CHECK: no problems found."]
    );

    let (ctx, mut app) = boot();
    app.document = reopened;
    submit_command(&ctx, &mut app, "check");
    assert_eq!(app.command_feedback, "CHECK: no problems found.");
}
