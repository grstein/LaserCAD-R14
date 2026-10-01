//! tests/it/ui/dialog_keyboard.rs — LCV-169: every dialog's buttons, its
//! title-bar ×, Enter and Escape, and the Save/Discard/Cancel prompt.
//!
//! Buttons are read from painted text (`tests/harness/paint.rs`), scoped to
//! the dialog's own `Area` rect (`Memory::area_rect` of the window title's
//! id), and clicked with real pointer events; keys are real taps. No test
//! here sends Ctrl+O/S (ADR 0005), and no parked `Open` is ever confirmed.

use crate::harness;

use harness::paint::{self, Run};
use lasercad::app::{App, PendingAction};

/// One of the seven dialogs: its window title and how a test opens it.
struct Dialog {
    title: &'static str,
    open: fn(&mut App),
}

/// The seven dialogs of LCV-169 AC 4, in `Dialog` declaration order.
const DIALOGS: [Dialog; 7] = [
    Dialog {
        title: "About LaserCAD",
        open: |a| a.about_open = true,
    },
    Dialog {
        title: "Keyboard Shortcuts",
        open: |a| a.shortcuts_open = true,
    },
    Dialog {
        title: "AI Settings",
        open: |a| a.agent_settings_open = true,
    },
    Dialog {
        title: "Layers",
        open: App::open_layers_dialog,
    },
    Dialog {
        title: "Bed Size",
        open: |a| a.bed_dialog = Some([100.0, 100.0]),
    },
    Dialog {
        title: "Discard unsaved changes?",
        open: |a| a.guard.pending_action = Some(PendingAction::New),
    },
    Dialog {
        title: "Error",
        open: |a| a.error_message = Some("boom".into()),
    },
];

/// The dialog titled `title`.
fn dialog(title: &str) -> &'static Dialog {
    DIALOGS
        .iter()
        .find(|d| d.title == title)
        .unwrap_or_else(|| panic!("no dialog titled {title}"))
}

/// A fresh context at `pixels_per_point = 1` and a default app with `title`
/// open.
fn open(title: &str) -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = App::default();
    (dialog(title).open)(&mut app);
    (ctx, app)
}

/// The rect of the window titled `title`, as laid out by the last frame.
fn window_rect(ctx: &egui::Context, title: &str) -> egui::Rect {
    ctx.memory(|m| m.area_rect(egui::Id::new(title)))
        .unwrap_or_else(|| panic!("`{title}` must be placed"))
}

/// Two frames (paint.rs trap 7), then the runs painted inside `title`'s
/// window on the second.
fn window_runs(ctx: &egui::Context, app: &mut App, title: &str) -> Vec<Run> {
    let _ = paint::painted_runs(ctx, app);
    let runs = paint::painted_runs(ctx, app);
    let rect = window_rect(ctx, title);
    runs.into_iter().filter(|r| rect.contains(r.pos)).collect()
}

/// The window's visual lines, top to bottom, each left to right.
fn window_lines(ctx: &egui::Context, app: &mut App, title: &str) -> Vec<Vec<String>> {
    let runs = window_runs(ctx, app, title);
    let refs: Vec<&Run> = runs.iter().collect();
    let lines = paint::texts(&paint::group_into_lines(&refs));
    assert_eq!(
        lines.first().map(|l| l.join(" ")),
        Some(title.to_owned()),
        "control: the first line is the title bar"
    );
    lines
}

// ── AC 4 / AC 5: button runs ─────────────────────────────────────────────────

/// AC 4 / AC 5 — the one-button dialogs end on a lone `Close`.
#[test]
fn ac4_one_button_dialogs_end_on_close() {
    for title in [
        "About LaserCAD",
        "Keyboard Shortcuts",
        "Error",
        "AI Settings",
    ] {
        let (ctx, mut app) = open(title);
        let lines = window_lines(&ctx, &mut app, title);
        assert_eq!(
            lines.last(),
            Some(&vec!["Close".to_owned()]),
            "{title}: {lines:?}"
        );
    }
}

/// AC 5 — Bed Size reads `OK` then `Cancel`, on its last line.
#[test]
fn ac5_bed_size_reads_ok_then_cancel() {
    let (ctx, mut app) = open("Bed Size");
    let lines = window_lines(&ctx, &mut app, "Bed Size");
    assert_eq!(
        lines.last(),
        Some(&vec!["OK".to_owned(), "Cancel".to_owned()])
    );
}

/// AC 5 — Layers' buttons start with `Apply` and end with `Close`.
#[test]
fn ac5_layers_reads_apply_first_and_close_last() {
    let (ctx, mut app) = open("Layers");
    let lines = window_lines(&ctx, &mut app, "Layers");
    let apply = lines
        .iter()
        .position(|l| l.first().is_some_and(|t| t == "Apply"))
        .unwrap_or_else(|| panic!("a line must start with Apply: {lines:?}"));
    let last = lines.last().expect("lines");
    assert_eq!(last.last().map(String::as_str), Some("Close"), "{lines:?}");
    assert!(apply < lines.len() - 1, "Apply's row is above Close's");
}

/// AC 5 / AC 7 — the discard prompt reads `Save`, `Discard`, `Cancel`.
#[test]
fn ac7_discard_reads_save_discard_cancel() {
    let (ctx, mut app) = open("Discard unsaved changes?");
    let lines = window_lines(&ctx, &mut app, "Discard unsaved changes?");
    let want: Vec<String> = ["Save", "Discard", "Cancel"].map(String::from).into();
    assert_eq!(lines.last(), Some(&want));
}
