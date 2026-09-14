//! LCV-133 — the F1 dialog paints all eight groups without scrolling, at
//! three screen sizes, because `shortcuts_dialog` now lays its content out in
//! two columns instead of one.
//!
//! ## Why this exists
//!
//! LCV-126 proved the `Command line` group *renders*. It did not prove an
//! operator can *see* it: `Window::new`'s baked `.default_size([340.0,
//! 420.0])` caps a `ScrollArea`'s height at ~420pt regardless of screen size
//! (egui 0.29.1), and one column of this dialog's content needs ~836pt. Five
//! of eight headings were below the fold, on every screen size measured.
//! This file is the paint-assertion proof that the two-column layout fixes
//! that: every heading painted, painted *inside* its clip rect (not merely
//! present — see AC 2 below), and the total run count matching the content,
//! derived at runtime rather than hand-typed.
//!
//! ## Why this is an inline collector, not a `tests/harness/mod.rs` import
//!
//! Same reason as `tests/lcv126_command_line_group.rs` (read its module doc
//! first): LCV-132, which promotes `Run`/`collect_text`/`painted_runs`/
//! `lines_on_surface_of`/`texts` into a shared harness, is still `Draft` as
//! this file is written. This file's `Run`/`collect_text`/`painted_runs` are
//! the same shape as that file's, with two additions this demand's AC 2
//! needs and that one didn't: the untruncated `pos: egui::Pos2` and
//! `height: f32` (`galley.size().y`), because "painted" and "painted inside
//! the clip rect" are different claims and AC 2 exists precisely so the test
//! knows the difference. The screen size is also a parameter here instead of
//! `tests/harness`'s fixed `SCREEN`, because AC 1 requires three of them.
//! When LCV-132 lands, it should treat this file as a fifth caller of the
//! same machinery, not a second permanent home for a divergent one.
//!
//! ## Two traps this file does not fall into
//!
//! 1. **The window title is not a valid scope marker** — same trap as
//!    `tests/lcv126_command_line_group.rs`, and worse here: the menu bar
//!    carries `File`, `Edit`, `View`, `Tools` and `Help` as its own top-level
//!    labels, painted on a screen-wide clip. Four of this dialog's eight
//!    headings collide with a menu label by exact text. Every assertion here
//!    scopes to the dialog body first — the clip rect shared by both
//!    columns, found via the `Command line` heading, which no menu or
//!    toolbar surface carries — and only counts a heading once it is inside
//!    that scope.
//! 2. **At least two frames.** The dialog's first frame paints nothing of its
//!    own body (its `Area` is not yet placed); [`dialog_body`] always drives
//!    one throwaway frame before the one it reads.

mod harness;

use lasercad::app::App;
use lasercad::ui::dialogs::{tool_rows, SHORTCUT_GROUPS};

/// The eight headings the operator must see without scrolling.
const HEADINGS: [&str; 8] = [
    "Tools",
    "File",
    "Edit",
    "View",
    "Modes",
    "Drawing",
    "Command line",
    "Help",
];

// ── Painted text: what actually reached the screen ──────────────────────────

/// One painted text run. `pos` and `height` are the untruncated egui values
/// AC 2's containment check needs; unlike `tests/lcv126_command_line_group.rs`
/// this file never groups runs into visual lines, so it carries no rounded
/// `y`/`x` and no `SAME_LINE` tolerance.
struct Run {
    clip: egui::Rect,
    pos: egui::Pos2,
    height: f32,
    text: String,
}

/// Every `Shape::Text` under `shape`, including those nested inside a
/// `Shape::Vec`, which is how egui groups a widget's own painting — the only
/// nesting variant at 0.29.1.
fn collect_text(clip: egui::Rect, shape: &egui::Shape, out: &mut Vec<Run>) {
    match shape {
        egui::Shape::Text(text) => out.push(Run {
            clip,
            pos: text.pos,
            height: text.galley.size().y,
            text: text.galley.text().to_owned(),
        }),
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect_text(clip, s, out)),
        _ => {}
    }
}

/// `tests/harness::raw_input`, parameterized on screen size — AC 1 requires
/// three of them, where the shared harness fixes one.
fn raw_input_at(screen: [f32; 2], events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(screen[0], screen[1]),
        )),
        events,
        ..Default::default()
    }
}

/// Drive one frame at `screen` and hand back every non-empty text run egui
/// painted. The positive control is load-bearing for the same reason it is
/// in `tests/lcv126_command_line_group.rs`: without it, a dialog that
/// rendered nothing at all would read as "the rows are absent, as expected"
/// instead of as the broken test it actually is.
fn painted_runs_at(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) -> Vec<Run> {
    let out = ctx.run(raw_input_at(screen, Vec::new()), |ctx| app.update_ui(ctx));
    let mut runs = Vec::new();
    for clipped in &out.shapes {
        collect_text(clipped.clip_rect, &clipped.shape, &mut runs);
    }
    runs.retain(|run| !run.text.trim().is_empty());
    assert!(
        runs.len() > 10,
        "positive control: a frame of this app paints text, saw {} runs",
        runs.len()
    );
    runs
}

/// Open the shortcuts dialog at `screen`, settle it (two frames — see module
/// doc), and hand back every run painted on the dialog's own body: the runs
/// whose clip rect is contained in the clip of the `Command line` heading,
/// which is unique to the dialog body (trap 1 above). Both columns share one
/// clip rect (`ui.columns` clones the parent painter without narrowing it),
/// so this scope already covers both without further work.
fn dialog_body(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) -> (Vec<Run>, egui::Rect) {
    let _ = painted_runs_at(ctx, app, screen);
    let runs = painted_runs_at(ctx, app, screen);

    let marker = "Command line";
    let hits = runs.iter().filter(|r| r.text.trim() == marker).count();
    assert_eq!(
        hits, 1,
        "at {screen:?}: `{marker}` must be painted exactly once, saw {hits}"
    );
    let surface = runs
        .iter()
        .find(|r| r.text.trim() == marker)
        .expect("checked above")
        .clip;

    let scoped: Vec<Run> = runs
        .into_iter()
        .filter(|r| surface.contains_rect(r.clip))
        .collect();
    (scoped, surface)
}

fn new_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let app = App {
        shortcuts_open: true,
        ..App::default()
    };
    (ctx, app)
}

/// Is `run` fully inside `surface` on the vertical axis — AC 2's exact test:
/// `pos.y >= clip.top()` and `pos.y + galley.size().y <= clip.bottom()`. An
/// egui `ScrollArea` culls content far outside the view but still emits a
/// shape for a run straddling the edge, so presence in the run list is not
/// enough (see `ac2_containment_check_is_not_decorative` below, which proves
/// this discriminates a real case).
fn vertically_contained(run: &Run, surface: egui::Rect) -> bool {
    run.pos.y >= surface.top() && run.pos.y + run.height <= surface.bottom()
}

// ── AC 1 + AC 2: every heading painted, and painted inside the clip ────────

/// Shared body of the three size-specific tests below. Every heading is
/// required to be painted **exactly once** in the dialog body and **fully
/// contained** in its clip rect; both the `F1` and `This dialog` runs (the
/// last row of the last group, AC 2's second claim) get the same check.
/// Failures are collected rather than reported one at a time, so a
/// regression that hides several headings at once (as the one-column revert
/// mutation does) names all of them in one assertion instead of only the
/// first.
fn assert_every_group_fits(screen: [f32; 2]) {
    let (ctx, mut app) = new_app();
    let (scoped, surface) = dialog_body(&ctx, &mut app, screen);

    let mut not_painted_once = Vec::new();
    let mut not_contained = Vec::new();
    for heading in HEADINGS {
        let matches: Vec<&Run> = scoped.iter().filter(|r| r.text.trim() == heading).collect();
        if matches.len() != 1 {
            not_painted_once.push(format!("{heading} (painted {} times)", matches.len()));
            continue;
        }
        if !vertically_contained(matches[0], surface) {
            not_contained.push(heading);
        }
    }
    assert!(
        not_painted_once.is_empty(),
        "at {screen:?}: headings not painted exactly once in the dialog body: {not_painted_once:?}"
    );
    assert!(
        not_contained.is_empty(),
        "at {screen:?}: headings painted but not fully inside the clip rect: {not_contained:?}"
    );

    for cell in ["F1", "This dialog"] {
        let matches: Vec<&Run> = scoped.iter().filter(|r| r.text.trim() == cell).collect();
        assert_eq!(
            matches.len(),
            1,
            "at {screen:?}: {cell:?} (the last row of the last group) must be painted \
             exactly once, saw {}",
            matches.len()
        );
        assert!(
            vertically_contained(matches[0], surface),
            "at {screen:?}: {cell:?} (the last row of the last group) is painted but not \
             fully inside the clip rect: pos.y={}, height={}, clip=({}, {})",
            matches[0].pos.y,
            matches[0].height,
            surface.top(),
            surface.bottom()
        );
    }
}

/// AC 1 + AC 2 at 1280×800 — an entirely ordinary laptop screen, and the one
/// on which the shipped, one-column dialog left `Help` clipped even with
/// `.default_height(screen - 80)` (LCV-133 §Problem).
#[test]
fn ac1_ac2_every_group_fits_at_1280x800() {
    assert_every_group_fits([1280.0, 800.0]);
}

/// AC 1 + AC 2 at 1024×600 — a small laptop screen.
#[test]
fn ac1_ac2_every_group_fits_at_1024x600() {
    assert_every_group_fits([1024.0, 600.0]);
}

/// AC 1 + AC 2 at 800×600 — the shortest width in this demand's table.
#[test]
fn ac1_ac2_every_group_fits_at_800x600() {
    assert_every_group_fits([800.0, 600.0]);
}

/// AC 2 — proof the containment check is not decorative. At 1280×460 the
/// window is too short for the two-column layout's own ~426pt, so the
/// `ScrollArea` safety net starts culling: the true bottom of the content —
/// the `F1` / `This dialog` row, the last row of the last group — is still
/// painted, straddling the clip edge by well under a point, exactly the case
/// a presence-only assertion cannot tell from a fully visible one. This is
/// not one of AC 1's three required sizes; it exists only to show the
/// containment check distinguishes painted-and-clipped from
/// painted-and-visible (mutation record: LCV-133 handover).
#[test]
fn ac2_containment_check_is_not_decorative() {
    let (ctx, mut app) = new_app();
    let screen = [1280.0, 460.0];
    let (scoped, surface) = dialog_body(&ctx, &mut app, screen);

    for cell in ["F1", "This dialog"] {
        let matches: Vec<&Run> = scoped.iter().filter(|r| r.text.trim() == cell).collect();
        assert_eq!(
            matches.len(),
            1,
            "positive control: {cell:?} is still painted at {screen:?} — a \
             presence-only assertion would call this fine"
        );
        assert!(
            !vertically_contained(matches[0], surface),
            "at {screen:?}, {cell:?} (pos.y={}, height={}) was expected to straddle the \
             clip bottom ({}) — if it is now fully contained, this screen height no \
             longer demonstrates the case and must be shrunk further",
            matches[0].pos.y,
            matches[0].height,
            surface.bottom()
        );
    }
}

// ── AC 3: the run count is derived, never a hand-typed 65 ──────────────────

/// One run per heading, two per row (binding + description) — computed from
/// [`SHORTCUT_GROUPS`] and [`tool_rows`] at test time, never a literal.
fn expected_run_count() -> usize {
    let tools = 1 + tool_rows().len() * 2;
    let groups: usize = SHORTCUT_GROUPS.iter().map(|g| 1 + g.rows.len() * 2).sum();
    tools + groups
}

/// AC 3 — every documented row is painted, not just the headings: the total
/// non-empty run count on the dialog's surface equals [`expected_run_count`].
#[test]
fn ac3_run_count_matches_the_content_derived_at_runtime() {
    let (ctx, mut app) = new_app();
    let (scoped, _surface) = dialog_body(&ctx, &mut app, [1280.0, 800.0]);
    assert_eq!(
        scoped.len(),
        expected_run_count(),
        "the dialog body must paint exactly one run per heading and two per row \
         (binding + description); a row or a heading went missing without the \
         heading-presence check above also failing"
    );
}
