//! LCV-161 AC 8 — View > Object Snap: one checkbox per snap kind, persisted.
//!
//! The submenu is opened through real pointer input: a click on `View`, then
//! a hover on the nested `Object Snap` row (egui 0.29.1 opens a nested menu on
//! hover, never on click — see
//! `tests/it/app/document_title_and_file_feedback.rs::open_recent_submenu`).

use crate::harness;

use harness::paint::{self, Run};
use harness::raw_input;
use lasercad::app::App;
use lasercad::geometry::SnapKind;
use lasercad::io::settings::Settings;
use std::path::PathBuf;

const LABELS: [&str; 8] = [
    "Endpoint",
    "Midpoint",
    "Center",
    "Intersection",
    "Quadrant",
    "Perpendicular",
    "Tangent",
    "Nearest",
];

fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv161_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn click_events(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
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
    ]
}

/// A point inside the one run reading `label`.
fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
    let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(matches.len(), 1, "`{label}` must be painted exactly once");
    let run = matches[0];
    egui::pos2(run.pos.x + 2.0, run.pos.y + run.height / 2.0)
}

fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    let _ = ctx.run_ui(raw_input(vec![egui::Event::PointerMoved(pos)]), |ui| {
        app.update_ui(ui)
    });
    let _ = ctx.run_ui(raw_input(click_events(pos)), |ui| app.update_ui(ui));
}

/// Open View, hover `Object Snap`, and return the runs painted with the
/// submenu open.
fn open_object_snap_menu(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    ctx.set_pixels_per_point(1.0);
    let runs = paint::painted_runs(ctx, app);
    click(ctx, app, locate(&runs, "View"));
    let runs = paint::painted_runs(ctx, app);
    let row = locate(&runs, "Object Snap");
    let _ = ctx.run_ui(raw_input(vec![egui::Event::PointerMoved(row)]), |ui| {
        app.update_ui(ui)
    });
    paint::painted_runs(ctx, app)
}

/// AC 8 — the submenu paints one checkbox label per kind, one column, in
/// order. Menu popups are clipped to the whole screen, so the column is
/// found by the x of the `Endpoint` run rather than by clip rect.
#[test]
fn object_snap_submenu_lists_one_checkbox_per_kind() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    let runs = open_object_snap_menu(&ctx, &mut app);
    let column_x = locate(&runs, "Endpoint").x - 2.0;
    let mut last_y = f32::NEG_INFINITY;
    for label in LABELS {
        let in_column: Vec<&Run> = runs
            .iter()
            .filter(|r| r.text.trim() == label && (r.pos.x - column_x).abs() < 1.0)
            .collect();
        assert_eq!(
            in_column.len(),
            1,
            "`{label}` must be painted once in the submenu"
        );
        assert!(in_column[0].pos.y > last_y, "`{label}` is out of order");
        last_y = in_column[0].pos.y;
    }
}

/// AC 8 — clicking a kind's checkbox flips `settings.object_snaps` and
/// writes the settings file at the injected path.
#[test]
fn clicking_a_kind_flips_and_persists_it() {
    let dir = tempdir("menu_toggle");
    let path = dir.join("settings.json");
    let ctx = egui::Context::default();
    let mut app = App {
        settings_path: Some(path.clone()),
        ..App::default()
    };
    assert!(
        !app.settings.object_snaps.nearest,
        "Nearest is off by default"
    );
    let runs = open_object_snap_menu(&ctx, &mut app);
    let nearest = runs
        .iter()
        .find(|r| r.text.trim() == "Nearest")
        .map(|r| egui::pos2(r.pos.x + 2.0, r.pos.y + r.height / 2.0))
        .expect("Nearest is painted");
    let _ = ctx.run_ui(raw_input(click_events(nearest)), |ui| app.update_ui(ui));

    assert!(
        app.settings.object_snaps.nearest,
        "the click must turn Nearest on"
    );
    let bytes = std::fs::read_to_string(&path).expect("the toggle must persist the settings");
    let saved: Settings = serde_json::from_str(&bytes).expect("valid settings JSON");
    assert!(saved.object_snaps.contains(SnapKind::Nearest));
    let _ = std::fs::remove_dir_all(&dir);
}

/// AC 8 — `App::set_object_snap` is the one write path: it flips the kind
/// and persists, and a process without a settings path writes nothing.
#[test]
fn set_object_snap_flips_and_persists() {
    let dir = tempdir("set_object_snap");
    let path = dir.join("settings.json");
    let mut app = App {
        settings_path: Some(path.clone()),
        ..App::default()
    };
    app.set_object_snap(SnapKind::Tangent, false);
    assert!(!app.settings.object_snaps.tangent);
    let saved: Settings =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("persisted")).unwrap();
    assert!(!saved.object_snaps.contains(SnapKind::Tangent));
    assert!(saved.object_snaps.contains(SnapKind::Endpoint));
    let _ = std::fs::remove_dir_all(&dir);

    let mut unpersisted = App::default();
    unpersisted.set_object_snap(SnapKind::Nearest, true);
    assert!(unpersisted.settings.object_snaps.nearest);
}
