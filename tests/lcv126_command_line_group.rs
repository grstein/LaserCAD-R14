//! LCV-126 AC 4 — the F1 dialog actually **paints** the "Command line" group,
//! not just carries it in `SHORTCUT_GROUPS`.
//!
//! `SHORTCUT_GROUPS` and the loop in `shortcuts_dialog` that renders it are two
//! different things: a data-structure test (`src/ui/dialogs.rs`'s
//! `static_rows_cover_every_documented_binding` and
//! `every_documented_binding_exists_in_the_two_key_readers`) proves the table
//! is right; nothing proves the loop actually reaches the screen. This file is
//! that second proof, built on `egui::Context::run`'s `FullOutput { shapes,
//! .. }` and `Shape::Text(TextShape { pos, galley, .. })` — assertable today at
//! the pinned egui 0.29.1, no `egui_kittest`, no version bump.
//!
//! ## Why this is an inline collector, not a `tests/harness/mod.rs` import
//!
//! LCV-132 promotes this exact machinery (`Run`, `collect_text`,
//! `painted_runs`, `lines_on_surface_of`, `texts`) out of
//! `tests/lcv125_agent_panel_and_settings.rs` into the shared harness — but
//! LCV-132 is still `Draft` as this file is written. Per LCV-126 AC 4's second
//! branch, this file carries its own copy instead of waiting. **When LCV-132
//! lands, it absorbs this file as a fourth caller** (its AC 3 was amended at
//! `562a2b7` to expect exactly this); do not add a second permanent home for
//! this code.
//!
//! ## Five things that would silently break a lazier version of this test
//!
//! 1. **The window title is not a valid scope marker.** `Window::new("Keyboard
//!    shortcuts")`'s title paints on a `ClippedShape` clipped to the *entire
//!    screen*, not to the dialog body. Scoping by it would select every
//!    surface painted this frame (menu bar, toolbar, statusbar, command line,
//!    the dialog itself). This file scopes by the `"Command line"` heading —
//!    a marker *inside* the dialog body — instead.
//! 2. **A binding's monospace column is padded.** `shortcut_row` paints
//!    `format!("{binding:<20}")`, so the run egui actually laid out for
//!    `"ArrowUp"` is `"ArrowUp             "` (20 characters). [`texts`] trims
//!    each run before comparing, or every expected string in this file would
//!    have to carry hand-counted trailing spaces.
//! 3. **The dialog's `Window` is shorter than its content, on purpose.** It has
//!    no `.default_height()`, so egui's own default — 420 points
//!    (`Resize::default_size` in the vendored `egui-0.29.1` source) — becomes
//!    the `ScrollArea`'s *maximum* height regardless of screen size (verified:
//!    a 2000-point-tall screen doesn't change it). That is exactly why the
//!    dialog carries `egui::ScrollArea::vertical()` at all ("so every row is
//!    reachable on a short window" — the doc comment on `shortcuts_dialog`).
//!    With eight groups plus the ten tool rows, "Command line" and "Help" sit
//!    below the fold at the window's default scroll position, so this test
//!    scrolls the dialog before asserting — see [`open_and_scroll_to_bottom`].
//!    That is a **test-driving detail**, not a product change: nothing here
//!    resizes the window, adds scroll-position memory, or touches
//!    `shortcuts_dialog`'s body (LCV-126 §Out of scope).
//! 4. **Two settle pairs, not one.** ADR 0002's "first frame is not settled"
//!    rule applies to *any* state change, not only to opening a window: one
//!    frame opens the dialog and one frame settles its first layout: as
//!    confirmed by measurement (see `open_and_scroll_to_bottom`), that first
//!    settle frame is also the first one whose rect is known well enough for
//!    a `MouseWheel` event over the scroll area to register at all. Then one
//!    frame applies the scroll and one more frame settles *that*. Asserting
//!    on the third of the four (rather than the fourth) reads a
//!    still-scrolling frame and silently proves nothing.
//! 5. **Empty runs exist and are dropped**, same as LCV-125's collector: an
//!    empty galley carries nothing an operator can read, and keeping it would
//!    make `retain` a no-op that hides a genuinely blank run.
//!
//! No test here reaches a real endpoint or touches the agent at all.

mod harness;

use lasercad::app::App;

// ── Painted text: what actually reached the screen ──────────────────────────

/// One painted text run: the surface it was clipped to, the y it was laid out
/// at, the x it starts at, and the string inside it (trimmed — see trap 2
/// above). Positions are rounded to whole points; this file runs at
/// `pixels_per_point = 1.0`.
struct Run {
    clip: egui::Rect,
    y: i32,
    x: i32,
    text: String,
}

/// Vertical slack, in points, within which two runs on one surface count as
/// sitting on the same visual line. Measured on this dialog: a binding and its
/// description share the exact same `y` (`ui.horizontal` top-aligns both), and
/// the closest two genuinely different rows are 21 points apart, so 6 (the
/// tolerance LCV-125 measured on a different surface) leaves room on both
/// sides here too.
const SAME_LINE: i32 = 6;

/// Every `Shape::Text` under `shape`, including those nested inside a
/// `Shape::Vec`, which is how egui groups a widget's own painting.
fn collect_text(clip: egui::Rect, shape: &egui::Shape, out: &mut Vec<Run>) {
    match shape {
        egui::Shape::Text(text) => out.push(Run {
            clip,
            y: text.pos.y.round() as i32,
            x: text.pos.x.round() as i32,
            text: text.galley.text().to_owned(),
        }),
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect_text(clip, s, out)),
        _ => {}
    }
}

/// Drive one frame and hand back every non-empty text run egui painted.
///
/// The positive control is load-bearing: without it, a dialog that rendered
/// nothing at all — say, a closed window, or a panic swallowed upstream —
/// would read as "the rows are absent, as expected" instead of as the broken
/// test it actually is.
fn painted_runs(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Vec<Run> {
    let out = ctx.run(harness::raw_input(events), |ctx| app.update_ui(ctx));
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

/// Every visual line painted on the same surface as the run reading `marker`,
/// top to bottom, each line read left to right.
///
/// "Same surface" is `ClippedShape::clip_rect` containment (trap 1 above):
/// `marker` must be a run genuinely inside the surface, never a `Window`
/// title, which is clipped to the whole screen and would make every surface
/// in the frame match.
fn lines_on_surface_of(runs: &[Run], marker: &str) -> Vec<Vec<String>> {
    let matches: Vec<&Run> = runs.iter().filter(|run| run.text == marker).collect();
    assert_eq!(
        matches.len(),
        1,
        "`{marker}` must be painted exactly once, saw {} times",
        matches.len()
    );
    let surface = matches[0].clip;

    let mut scoped: Vec<&Run> = runs
        .iter()
        .filter(|run| surface.contains_rect(run.clip))
        .collect();
    assert!(
        scoped.len() < runs.len(),
        "positive control: the surface must be narrower than the frame"
    );
    scoped.sort_by_key(|run| (run.y, run.x));

    let mut lines: Vec<(i32, Vec<&Run>)> = Vec::new();
    for run in scoped {
        match lines.last_mut() {
            Some((top, members)) if run.y - *top <= SAME_LINE => members.push(run),
            _ => lines.push((run.y, vec![run])),
        }
    }
    lines
        .into_iter()
        .map(|(_, mut members)| {
            members.sort_by_key(|run| run.x);
            members
                .into_iter()
                .map(|run| run.text.trim().to_owned())
                .collect()
        })
        .collect()
}

/// Open the shortcuts dialog and scroll its body all the way down, returning
/// the runs of the fourth and final frame — the one whose scroll offset has
/// settled (trap 4 above).
///
/// `app.shortcuts_open = true` is set directly (ADR 0002 §A2: `App::default()`
/// only, never `App::new()`); no key is sent to reach it, because opening the
/// dialog is not what this test is about.
fn open_and_scroll_to_bottom(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    // Frame 1 — the window is requested for the first time. It paints nothing
    // of its own body yet (ADR 0002: the first frame is not settled).
    let _ = painted_runs(ctx, app, vec![]);
    // Frame 2 — the dialog's first real layout. Its scroll-area rect is now
    // known well enough for a pointer-scoped `MouseWheel` to hit it.
    let _ = painted_runs(ctx, app, vec![]);

    // Frame 3 — hover the middle of the screen (inside the dialog body, which
    // is centre-anchored) and scroll far past the content's actual height so
    // the offset clamps to the maximum regardless of exactly how tall the
    // content is.
    let centre = egui::pos2(harness::SCREEN[0] / 2.0, harness::SCREEN[1] / 2.0);
    let scroll = vec![
        egui::Event::PointerMoved(centre),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -3000.0),
            modifiers: egui::Modifiers::NONE,
        },
    ];
    let _ = painted_runs(ctx, app, scroll);

    // Frame 4 — settled: the scrolled content is what this test asserts on.
    painted_runs(ctx, app, vec![])
}

// ── AC 4: the group is painted, in position, with its three rows ───────────

/// AC 4 — the dialog paints `Command line` between `Drawing` and `Help`, each
/// of the three AC 1 rows as its binding and its description, in that order,
/// as consecutive visual lines. A data-structure assertion over
/// `SHORTCUT_GROUPS` cannot make this claim (module doc comment); this is the
/// one test in the demand that can.
#[test]
fn the_command_line_group_is_painted_between_drawing_and_help() {
    let ctx = egui::Context::default();
    let mut app = App {
        shortcuts_open: true,
        ..App::default()
    };

    let runs = open_and_scroll_to_bottom(&ctx, &mut app);
    let lines = lines_on_surface_of(&runs, "Command line");

    let start = lines
        .iter()
        .position(|line| line.as_slice() == ["Command line"])
        .expect("the Command line heading must be painted on the scrolled, settled frame");
    assert!(
        start + 5 <= lines.len(),
        "not enough lines painted after Command line: only {} available, need 5",
        lines.len() - start
    );
    let window = &lines[start..start + 5];

    assert_eq!(
        window,
        [
            vec!["Command line".to_string()],
            vec![
                "Any other character".to_string(),
                "Start a command in the command line".to_string(),
            ],
            vec![
                "ArrowUp".to_string(),
                "Previous command (command line focused)".to_string(),
            ],
            vec![
                "ArrowDown".to_string(),
                "Next command (command line focused)".to_string(),
            ],
            vec!["Help".to_string()],
        ],
        "the Command line group must paint its heading and its three rows, in \
         order, immediately followed by the Help heading"
    );
}
