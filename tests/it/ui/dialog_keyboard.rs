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
    ctx.memory(|m| m.area_rect(crate::harness::window_id(title)))
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
    let out = ctx.run_ui(harness::raw_input(vec![]), |ui| app.update_ui(ui));
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

// ── AC 1 / AC 2: Enter and Escape ────────────────────────────────────────────

/// One real tap of `key` with no modifiers.
fn press(ctx: &egui::Context, app: &mut App, key: egui::Key) {
    harness::tap(ctx, app, key, egui::Modifiers::NONE);
}

/// AC 1 — with About then Shortcuts open, Escape closes only Shortcuts, the
/// topmost; a second Escape closes About.
#[test]
fn ac1_escape_closes_only_the_topmost_dialog() {
    let (ctx, mut app) = open("About LaserCAD");
    harness::frame(&ctx, &mut app, vec![]);
    app.shortcuts_open = true;
    harness::frame(&ctx, &mut app, vec![]);
    press(&ctx, &mut app, egui::Key::Escape);
    assert!(!app.shortcuts_open, "Shortcuts, the topmost, closed");
    assert!(app.about_open, "About stays open");
    press(&ctx, &mut app, egui::Key::Escape);
    assert!(!app.about_open);
}

/// AC 1 — Enter on Bed Size commits the draft and closes the dialog.
#[test]
fn ac1_enter_on_bed_size_commits_and_closes() {
    let (ctx, mut app) = open("Bed Size");
    app.bed_dialog = Some([123.0, 77.0]);
    harness::frame(&ctx, &mut app, vec![]);
    press(&ctx, &mut app, egui::Key::Enter);
    assert_eq!(app.bed_dialog, None, "closed");
    assert_eq!(app.document.bed_mm, [123.0, 77.0], "committed");
}

/// AC 2 — LINE past its first point, `12,3` typed into the focused command
/// line, About open: Escape leaves the text, the focus and the tool prompt
/// as they were, and Enter pushes nothing to the recall ring.
#[test]
fn ac2_dialog_keys_never_reach_the_command_line_or_the_tool() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    harness::frame(&ctx, &mut app, vec![]);
    harness::submit_command(&ctx, &mut app, "l");
    harness::submit_command(&ctx, &mut app, "0,0");
    harness::type_command(&ctx, &mut app, "12,3");
    app.about_open = true;
    harness::frame(&ctx, &mut app, vec![]);
    let prompt = app.tool_manager.active_status_text().into_owned();
    let ring = app.command_history.len();
    assert!(app.command_line_focused, "control: the field has focus");
    assert_eq!(prompt, "LINE  Specify next point <Enter to finish>:");

    press(&ctx, &mut app, egui::Key::Escape);
    assert!(!app.about_open, "Escape closed About");
    assert_eq!(app.command_line_input, "12,3");
    // The id `src/ui/command_line.rs::editor_id` pins on the field.
    let field = egui::Id::new("command_line_editor");
    assert!(
        ctx.memory(|m| m.has_focus(field)),
        "focus stays in the field"
    );
    assert_eq!(app.tool_manager.active_status_text(), prompt);

    app.about_open = true;
    harness::frame(&ctx, &mut app, vec![]);
    press(&ctx, &mut app, egui::Key::Enter);
    assert!(!app.about_open, "Enter closed About");
    assert_eq!(app.command_history.len(), ring, "nothing pushed to recall");
    assert_eq!(app.command_line_input, "12,3");
    assert_eq!(app.tool_manager.active_status_text(), prompt);
    assert_eq!(app.document.entity_count(), 0);
}

/// AC 3 — Enter on Bed Size is exactly OK: twin apps type an out-of-range
/// width into the same field, one presses Enter and one clicks OK, and both
/// land on the same clamped bed.
#[test]
fn ac3_enter_on_bed_size_is_exactly_ok() {
    let typed = |app: &mut App, ctx: &egui::Context| {
        app.bed_dialog = Some([100.0, 200.0]);
        let width = locate(ctx, app, "Bed Size", "100");
        click(ctx, app, width);
        harness::type_command(ctx, app, "99999");
    };
    let (ctx, mut by_enter) = open("Bed Size");
    typed(&mut by_enter, &ctx);
    press(&ctx, &mut by_enter, egui::Key::Enter);

    let (ctx2, mut by_ok) = open("Bed Size");
    typed(&mut by_ok, &ctx2);
    let ok = locate(&ctx2, &mut by_ok, "Bed Size", "OK");
    click(&ctx2, &mut by_ok, ok);

    let max = lasercad::util::BED_MAX_MM;
    assert_eq!(by_ok.bed_dialog, None, "control: OK closed the dialog");
    assert_eq!(by_ok.document.bed_mm, [max, 200.0], "control: OK clamps");
    assert_eq!(by_enter.bed_dialog, None);
    assert_eq!(by_enter.document.bed_mm, by_ok.document.bed_mm);
    assert_eq!(by_enter.history.revision(), by_ok.history.revision());
}

/// AC 1 — on each of the seven dialogs, alone, Escape leaves exactly the
/// state its Cancel or Close button leaves.
#[test]
fn ac1_escape_on_each_dialog_is_its_cancel_or_close() {
    for d in &DIALOGS {
        let (ctx, mut by_key) = open(d.title);
        let _ = window_runs(&ctx, &mut by_key, d.title);
        press(&ctx, &mut by_key, egui::Key::Escape);

        let (ctx2, mut by_button) = open(d.title);
        let pos = locate(&ctx2, &mut by_button, d.title, cancel_label(d.title));
        click(&ctx2, &mut by_button, pos);

        assert_eq!(snapshot(&by_key), snapshot(&by_button), "{}", d.title);
    }
}

/// AC 1 — with the AI Settings system prompt focused, Enter inserts a
/// newline into the prompt and the window stays open.
#[test]
fn ac1_enter_in_the_system_prompt_is_a_newline() {
    let (ctx, mut app) = open("AI Settings");
    let _ = window_runs(&ctx, &mut app, "AI Settings");
    let editor = egui::Id::new(lasercad::agent::SYSTEM_PROMPT_ID);
    ctx.memory_mut(|m| m.request_focus(editor));
    harness::frame(&ctx, &mut app, vec![]);
    assert!(ctx.memory(|m| m.has_focus(editor)), "control: focused");

    press(&ctx, &mut app, egui::Key::Enter);
    assert!(app.agent_settings_open, "the window stays open");
    let lines = |s: &str| s.matches('\n').count();
    let typed = app.settings.agent_system_prompt.as_deref().unwrap_or("");
    let default = lasercad::agent::DEFAULT_PROMPT;
    assert_eq!(lines(typed), lines(default) + 1, "Enter typed a newline");
}
