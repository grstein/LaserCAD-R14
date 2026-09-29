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
//! **LCV-134's `lcv134_*` tests live here too**, and are named for their own
//! demand: they assert the headroom above exactly the content this file
//! already paints, scoped to exactly the body [`dialog_body`] already finds,
//! and a second file would have had to rebuild both to say anything at all.
//!
//! The collector itself is `tests/harness/paint.rs` (LCV-132). Its `Run`
//! carries the untruncated `pos: egui::Pos2` and `height: f32`
//! (`galley.size().y`) that AC 2 needs — "painted" and "painted inside the
//! clip rect" are different claims, and AC 2 exists precisely so this file
//! knows the difference — and derives rounded `y()`/`x()` for the callers that
//! group runs into visual lines. The screen size is a parameter of
//! `painted_runs_at` for the same reason: AC 1 requires three of them.
//!
//! ## Two traps this file does not fall into
//!
//! 1. **The window title is not a valid scope marker** — same trap as
//!    `tests/it/ui/shortcuts_command_line_group.rs`, and worse here: the menu bar
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

use crate::harness;

use harness::paint::{Run, painted_runs_at, scoped_runs};
use lasercad::app::App;
use lasercad::ui::{SHORTCUT_GROUPS, tool_rows};

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

/// Open the shortcuts dialog at `screen`, settle it (two frames — see module
/// doc), and hand back every run painted on the dialog's own body: the runs
/// whose clip rect is contained in the clip of the `Command line` heading,
/// which is unique to the dialog body (trap 1 above). Both columns share one
/// clip rect (`ui.columns` clones the parent painter without narrowing it), so
/// this scope already covers both without further work.
///
/// `harness::paint::scoped_runs` hands back borrows; they are cloned here
/// because every caller outlives the frame's run list. It also carries the
/// "the surface must be narrower than the frame" control this file's own
/// scoping did not have — a strengthening, and one that holds at all four
/// screen sizes used here, 1280×460 included.
fn dialog_body(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) -> (Vec<Run>, egui::Rect) {
    let _ = painted_runs_at(ctx, app, screen, Vec::new());
    let runs = painted_runs_at(ctx, app, screen, Vec::new());
    let (scoped, surface) = scoped_runs(&runs, "Command line");
    (scoped.into_iter().cloned().collect(), surface)
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
            "at {screen:?}, {cell:?} (pos.y={}, height={}) is fully contained by the \
             clip bottom ({}), where this control expects it to straddle. Two causes, \
             in this order: **the dialog content grew** and the `ScrollArea` is now \
             culling a different row — check \
             `lcv134_ac5_deepest_column_keeps_a_row_of_slack_at_1280x800`, which names \
             that cause in the units of the defect — **or**, only if that one is green, \
             this screen height no longer demonstrates the case and must be shrunk \
             further. Shrinking it while the content has grown weakens the control over \
             a sliced row, and the suite goes green on the defect (ADR 0009 §Context).",
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

// ── LCV-134: the headroom above that content, and the window around it ──────

/// One shortcut row's vertical pitch, in points: consecutive rows inside a
/// group are exactly this far apart at the egui 0.29.1 pin (ADR 0009 records
/// the row *widget* at 20.91pt; the pitch is what a new row actually costs).
///
/// [`lcv134_assert_slack_floor`] requires the deepest column to clear the body
/// clip bottom by at least this much, so the dialog always has room for one
/// more row than it shows. That is what makes the failure arrive on the commit
/// that adds the row instead of on the bug report that follows it (ADR 0009
/// decision 4). **Revisit when egui is unpinned from 0.29.1** — every number
/// here is a snapshot of that pin, and an upgrade that changes row metrics is
/// expected to change this one (ADR 0009 §Revisit criteria).
const MIN_SLACK_PT: f32 = 21.00;

/// How far the deepest painted run in the dialog body sits above the body clip
/// bottom. Both columns share one clip rect (see [`dialog_body`]), so the
/// deepest run *is* the deepest column's bottom, and one number covers both.
fn deepest_slack(runs: &[Run], surface: egui::Rect) -> f32 {
    let deepest = runs
        .iter()
        .map(|run| run.pos.y + run.height)
        .fold(f32::NEG_INFINITY, f32::max);
    surface.bottom() - deepest
}

/// The dialog's own window rect, read from egui's area memory under the `Id`
/// `Window::new` derives from its title. This is the **area** id, not a paint
/// surface, so trap 1 in the module doc does not apply: nothing here scopes a
/// painted run by the window title.
fn window_rect(ctx: &egui::Context) -> egui::Rect {
    ctx.memory(|m| m.area_rect(egui::Id::new("Keyboard shortcuts")))
        .expect("the shortcuts window has been placed by the settled frame")
}

/// AC 4 — **every** non-empty run in the dialog body, not just the eight
/// headings and the `F1` / `This dialog` row, is fully inside the body clip
/// rect. The first row to go when this dialog grows is `Ctrl+Y` / `Redo`,
/// which no hand-typed expectation in this repo mentions (ADR 0009 decision
/// 4), so an assertion over a literal list cannot see it.
fn lcv134_assert_every_run_is_inside_the_body(screen: [f32; 2]) {
    let (ctx, mut app) = new_app();
    let (scoped, surface) = dialog_body(&ctx, &mut app, screen);

    let outside: Vec<String> = scoped
        .iter()
        .filter(|run| !vertically_contained(run, surface))
        .map(|run| {
            format!(
                "{:?} (pos.y={}, height={})",
                run.text.trim(),
                run.pos.y,
                run.height
            )
        })
        .collect();
    assert!(
        outside.is_empty(),
        "at {screen:?}: {} of {} runs painted in the dialog body are not fully inside \
         the body clip rect ({}, {}): {outside:?}",
        outside.len(),
        scoped.len(),
        surface.top(),
        surface.bottom()
    );
}

/// AC 5 — the deepest column clears the body clip bottom by at least one row
/// pitch. Containment (AC 4) is the backstop; this is the tripwire, and it
/// fires one commit *before* content is lost.
fn lcv134_assert_slack_floor(screen: [f32; 2]) {
    let (ctx, mut app) = new_app();
    let (scoped, surface) = dialog_body(&ctx, &mut app, screen);
    let slack = deepest_slack(&scoped, surface);
    assert!(
        slack >= MIN_SLACK_PT,
        "at {screen:?}: the deepest column clears the body clip bottom ({}) by only \
         {slack:.2}pt, under the {MIN_SLACK_PT:.2}pt floor — one shortcut row's pitch, \
         so the next row added to SHORTCUT_GROUPS or tool added to TOOLS lands under \
         the fold. Either the dialog content grew (ADR 0009 §Context's growth table), \
         or `shortcuts_dialog`'s `.default_height(screen - 80)` was removed. Do not \
         lower this floor to make the assertion pass.",
        surface.bottom()
    );
}

/// AC 6 — the window is on the screen. Not redundant with AC 4: with the
/// sizing call the body clip bottom lands **exactly on the screen edge** at
/// 600-high screens, where a run-only assertion cannot tell "the content fits"
/// from "the window hangs off the bottom of the display and egui clipped it at
/// the screen edge".
fn lcv134_assert_window_is_on_screen(screen: [f32; 2]) {
    let (ctx, mut app) = new_app();
    let _ = dialog_body(&ctx, &mut app, screen);
    let window = window_rect(&ctx);
    let visible = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(screen[0], screen[1]));
    let escapes: Vec<String> = [
        (
            "top",
            window.top() >= visible.top(),
            window.top(),
            visible.top(),
        ),
        (
            "left",
            window.left() >= visible.left(),
            window.left(),
            visible.left(),
        ),
        (
            "right",
            window.right() <= visible.right(),
            window.right(),
            visible.right(),
        ),
        (
            "bottom",
            window.bottom() <= visible.bottom(),
            window.bottom(),
            visible.bottom(),
        ),
    ]
    .into_iter()
    .filter(|(_, ok, _, _)| !ok)
    .map(|(edge, _, got, limit)| format!("{edge} at {got:.2}, screen edge at {limit:.2}"))
    .collect();
    assert!(
        escapes.is_empty(),
        "at {screen:?}: the shortcuts window {window:?} is not fully inside the screen \
         — {escapes:?}. The runs can still all be painted here: egui clips the body at \
         the screen edge, so containment alone would call this fine.",
    );
}

/// AC 4 at 1280×800.
#[test]
fn lcv134_ac4_every_run_is_inside_the_body_at_1280x800() {
    lcv134_assert_every_run_is_inside_the_body([1280.0, 800.0]);
}

/// AC 4 at 1024×600.
#[test]
fn lcv134_ac4_every_run_is_inside_the_body_at_1024x600() {
    lcv134_assert_every_run_is_inside_the_body([1024.0, 600.0]);
}

/// AC 4 at 800×600.
#[test]
fn lcv134_ac4_every_run_is_inside_the_body_at_800x600() {
    lcv134_assert_every_run_is_inside_the_body([800.0, 600.0]);
}

/// AC 5 at 1280×800.
#[test]
fn lcv134_ac5_deepest_column_keeps_a_row_of_slack_at_1280x800() {
    lcv134_assert_slack_floor([1280.0, 800.0]);
}

/// AC 5 at 1024×600.
#[test]
fn lcv134_ac5_deepest_column_keeps_a_row_of_slack_at_1024x600() {
    lcv134_assert_slack_floor([1024.0, 600.0]);
}

/// AC 5 at 800×600.
#[test]
fn lcv134_ac5_deepest_column_keeps_a_row_of_slack_at_800x600() {
    lcv134_assert_slack_floor([800.0, 600.0]);
}

/// AC 6 at 1280×800.
#[test]
fn lcv134_ac6_window_stays_on_screen_at_1280x800() {
    lcv134_assert_window_is_on_screen([1280.0, 800.0]);
}

/// AC 6 at 1024×600.
#[test]
fn lcv134_ac6_window_stays_on_screen_at_1024x600() {
    lcv134_assert_window_is_on_screen([1024.0, 600.0]);
}

/// AC 6 at 800×600.
#[test]
fn lcv134_ac6_window_stays_on_screen_at_800x600() {
    lcv134_assert_window_is_on_screen([800.0, 600.0]);
}
