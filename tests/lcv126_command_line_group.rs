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
//! 3. **Two columns share one clip rect (LCV-133).** `shortcuts_dialog` no
//!    longer renders one column scrolled to a `.default_height()`-capped
//!    window (that one-line fix was measured and rejected — see LCV-133
//!    §Problem); it lays its content out in two columns wide enough that all
//!    eight groups fit inside the window's baked ~420pt cap with no scroll
//!    input at all. `ui.columns` clones the parent painter without narrowing
//!    its clip, so every run in both columns carries the *same*
//!    `ClippedShape::clip_rect` — which means grouping by `y` alone would
//!    braid a left-column run into a right-column line whenever their `y`
//!    values happen to fall within [`SAME_LINE`] of each other, silently
//!    turning this test's ordered assertion into noise. [`lines_on_surface_of`]
//!    buckets by column — the widest gap between distinct run-start `x`
//!    values, always the column boundary because it dwarfs the gap between a
//!    row's own binding and description — before it groups by `y`.
//! 4. **No scroll.** With nothing left below the fold, [`open_and_settle`]
//!    drives exactly the two frames ADR 0002 already requires for any state
//!    change (one opens the dialog, one settles its first layout) and no
//!    `MouseWheel` at all — a scroll event that moves nothing would be a
//!    comment pretending to be code.
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

/// The `x` at which the two columns split: the midpoint of the widest gap
/// between the distinct, sorted `x` values in `xs`. LCV-133's two columns are
/// separated by a wide, empty band in `x` — the left column's rightmost run
/// ends well before the right column's leftmost begins — and that band is
/// always wider than the gap between one row's own binding and its
/// description, so the widest gap is always the column boundary.
fn column_split_x(xs: &[i32]) -> i32 {
    xs.windows(2)
        .max_by_key(|pair| pair[1] - pair[0])
        .map(|pair| (pair[0] + pair[1]) / 2)
        .expect("at least two distinct x values: a heading and a row, in one column")
}

/// Every visual line painted on the same surface *and in the same column* as
/// the run reading `marker`, top to bottom, each line read left to right.
///
/// "Same surface" is `ClippedShape::clip_rect` containment (trap 1 above):
/// `marker` must be a run genuinely inside the surface, never a `Window`
/// title, which is clipped to the whole screen and would make every surface
/// in the frame match. "Same column" is [`column_split_x`] (trap 3 above):
/// without it, a two-column layout's shared clip rect would let a run from
/// the *other* column join a line here just because its `y` is close enough.
fn lines_on_surface_of(runs: &[Run], marker: &str) -> Vec<Vec<String>> {
    let matches: Vec<&Run> = runs.iter().filter(|run| run.text == marker).collect();
    assert_eq!(
        matches.len(),
        1,
        "`{marker}` must be painted exactly once, saw {} times",
        matches.len()
    );
    let surface = matches[0].clip;
    let marker_x = matches[0].x;

    let mut scoped: Vec<&Run> = runs
        .iter()
        .filter(|run| surface.contains_rect(run.clip))
        .collect();
    assert!(
        scoped.len() < runs.len(),
        "positive control: the surface must be narrower than the frame"
    );

    let mut xs: Vec<i32> = scoped.iter().map(|run| run.x).collect();
    xs.sort_unstable();
    xs.dedup();
    let split_x = column_split_x(&xs);
    let marker_side = marker_x < split_x;
    scoped.retain(|run| (run.x < split_x) == marker_side);
    assert!(
        !scoped.is_empty(),
        "positive control: `{marker}`'s own column must contain at least itself"
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

/// Open the shortcuts dialog and settle it, returning the runs of the second
/// frame (trap 4 above: no scroll — the two-column layout leaves nothing
/// below the fold to scroll to).
///
/// `app.shortcuts_open = true` is set directly (ADR 0002 §A2: `App::default()`
/// only, never `App::new()`); no key is sent to reach it, because opening the
/// dialog is not what this test is about.
fn open_and_settle(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    // Frame 1 — the window is requested for the first time. It paints nothing
    // of its own body yet (ADR 0002: the first frame is not settled).
    let _ = painted_runs(ctx, app, vec![]);
    // Frame 2 — settled: the dialog's first real layout is everything this
    // test asserts on.
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

    let runs = open_and_settle(&ctx, &mut app);
    let lines = lines_on_surface_of(&runs, "Command line");

    let start = lines
        .iter()
        .position(|line| line.as_slice() == ["Command line"])
        .expect("the Command line heading must be painted on the settled frame");
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
