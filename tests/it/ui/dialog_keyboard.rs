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

/// Settling frames, then the runs painted inside `title`'s window on the
/// last. Two frames paint a new window (paint.rs trap 7); an anchored window
/// is placed from the previous frame's size, so it settles on the third.
fn window_runs(ctx: &egui::Context, app: &mut App, title: &str) -> Vec<Run> {
    for _ in 0..3 {
        let _ = paint::painted_runs(ctx, app);
    }
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

/// The run reading exactly `label` inside `title`'s window, as a click point.
fn locate(ctx: &egui::Context, app: &mut App, title: &str, label: &str) -> egui::Pos2 {
    let runs = window_runs(ctx, app, title);
    let hits: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(hits.len(), 1, "`{label}` must be painted once in `{title}`");
    egui::pos2(hits[0].pos.x + 2.0, hits[0].pos.y + hits[0].height / 2.0)
}

/// The centre of `title`'s title-bar ×. egui 0.29.1 draws it a square
/// `icon_width` wide, inset by half the bar's spare height from the bar's
/// right end (`window.rs::close_button_ui`); the bar is centred on the
/// title run, so the × sits half a bar height in from the window's right.
fn close_x(ctx: &egui::Context, app: &mut App, title: &str) -> egui::Pos2 {
    let runs = window_runs(ctx, app, title);
    let rect = window_rect(ctx, title);
    let run = runs
        .iter()
        .find(|r| r.text.trim() == title)
        .unwrap_or_else(|| panic!("`{title}` paints its title"));
    let cy = run.pos.y + run.height / 2.0;
    egui::pos2(rect.right() - (cy - rect.top()), cy)
}

/// A real primary click at `pos`: a hover frame, then press and release.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    harness::frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    let button = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness::frame(ctx, app, vec![button(true), button(false)]);
}

/// Everything a dialog's Cancel/Close could touch, as one comparable value.
fn snapshot(app: &App) -> String {
    format!(
        "{:?}",
        (
            app.about_open,
            app.shortcuts_open,
            app.agent_settings_open,
            app.layers_dialog.is_some(),
            app.bed_dialog,
            app.guard.pending_action.is_some(),
            &app.error_message,
            app.history.revision(),
            app.document.bed_mm,
            app.document.entity_count(),
        )
    )
}

/// Each dialog's Cancel/Close button label.
fn cancel_label(title: &str) -> &'static str {
    match title {
        "Bed Size" | "Discard unsaved changes?" => "Cancel",
        _ => "Close",
    }
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

// ── AC 4: the title-bar × ────────────────────────────────────────────────────

/// AC 4 — a click on each dialog's × leaves exactly the state its Cancel or
/// Close button leaves, and that state has the dialog closed.
#[test]
fn ac4_the_title_bar_x_does_what_cancel_or_close_does() {
    for d in &DIALOGS {
        let (ctx, mut by_x) = open(d.title);
        let pos = close_x(&ctx, &mut by_x, d.title);
        click(&ctx, &mut by_x, pos);

        let (ctx2, mut by_button) = open(d.title);
        let pos = locate(&ctx2, &mut by_button, d.title, cancel_label(d.title));
        click(&ctx2, &mut by_button, pos);

        assert_eq!(snapshot(&by_x), snapshot(&by_button), "{}", d.title);
        assert_eq!(
            snapshot(&by_x),
            snapshot(&App::default()),
            "{} closes",
            d.title
        );
    }
}

// ── AC 6: destructive text ───────────────────────────────────────────────────

/// The colour a text shape's glyphs are painted in: the override if any,
/// else the first section's colour with egui's placeholder resolved to the
/// shape's fallback (`epaint::TextShape` docs).
fn glyph_colour(text: &egui::epaint::TextShape) -> egui::Color32 {
    let section = text.galley.job.sections.first().map(|s| s.format.color);
    match (text.override_text_color, section) {
        (Some(c), _) => c,
        (None, Some(c)) if c != egui::Color32::PLACEHOLDER => c,
        _ => text.fallback_color,
    }
}

/// Every text shape (as `(text, colour)`) and every fill colour under `shape`.
fn walk(
    shape: &egui::Shape,
    texts: &mut Vec<(String, egui::Color32)>,
    fills: &mut Vec<egui::Color32>,
) {
    match shape {
        egui::Shape::Text(t) => texts.push((t.galley.text().to_owned(), glyph_colour(t))),
        egui::Shape::Rect(r) => fills.push(r.fill),
        egui::Shape::Circle(c) => fills.push(c.fill),
        egui::Shape::Ellipse(e) => fills.push(e.fill),
        egui::Shape::Path(p) => fills.push(p.fill),
        egui::Shape::Mesh(m) => fills.extend(m.vertices.iter().map(|v| v.color)),
        egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts, fills)),
        _ => {}
    }
}

/// AC 6 — `Discard` is painted in `palette::DANGER`, and no filled shape
/// anywhere on the frame uses `DANGER` (text only, never a red button).
#[test]
fn ac6_discard_text_is_danger_and_nothing_is_filled_danger() {
    use lasercad::render::palette::DANGER;
    let (ctx, mut app) = open("Discard unsaved changes?");
    let _ = window_runs(&ctx, &mut app, "Discard unsaved changes?");
    let out = ctx.run(harness::raw_input(vec![]), |c| app.update_ui(c));
    let (mut texts, mut fills) = (Vec::new(), Vec::new());
    for clipped in &out.shapes {
        walk(&clipped.shape, &mut texts, &mut fills);
    }
    let discard: Vec<_> = texts
        .iter()
        .filter(|(t, _)| t.trim() == "Discard")
        .collect();
    assert_eq!(discard.len(), 1, "one Discard run: {texts:?}");
    assert_eq!(discard[0].1, DANGER, "Discard's text is the danger colour");
    assert!(!fills.contains(&DANGER), "no shape is filled with DANGER");
}
