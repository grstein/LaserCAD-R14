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
//! ## Why the column bucketing stays here
//!
//! `Run`, `collect_text`, `painted_runs`, `scoped_runs` and `group_into_lines`
//! live in `tests/harness/paint.rs` (LCV-132), and this file is one of their
//! callers. What does *not* live there is [`column_split_x`]: bucketing by
//! column is true of this one two-column surface and of nothing else in the
//! repository, and folding it into the shared helper behind a flag would put a
//! decision about one dialog into code four other demands depend on. It sits
//! between the two shared steps instead — scope, bucket, group — which is why
//! the harness exposes them separately rather than as one function.
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
//!    turning this test's ordered assertion into noise. [`lines_in_column_of`]
//!    buckets by column — the widest gap between distinct run-start `x`
//!    values, always the column boundary because it dwarfs the gap between a
//!    row's own binding and description — before it groups by `y`.
//! 4. **No scroll.** With nothing left below the fold, [`open_and_settle`]
//!    drives exactly the two frames ADR 0002 already requires for any state
//!    change (one opens the dialog, one settles its first layout) and no
//!    `MouseWheel` at all — a scroll event that moves nothing would be a
//!    comment pretending to be code.
//! 5. **Empty runs exist and are dropped** by the shared collector: an empty
//!    galley carries nothing an operator can read, and a whitespace-only one
//!    reads as a blank line that is not there.
//!
//! No test here reaches a real endpoint or touches the agent at all.

use crate::harness;

use harness::paint::Run;
use lasercad::app::App;

// ── Painted text: what actually reached the screen ──────────────────────────

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
/// The scoping and the grouping are `harness::paint`'s; the step in the middle
/// is this file's (trap 3 above). Without it, a two-column layout's shared clip
/// rect would let a run from the *other* column join a line here just because
/// its `y` is close enough.
fn lines_in_column_of(runs: &[Run], marker: &str) -> Vec<Vec<String>> {
    let (mut scoped, _surface) = harness::paint::scoped_runs(runs, marker);
    let marker_x = scoped
        .iter()
        .find(|run| run.text.trim() == marker)
        .map(|run| run.x())
        .expect("`scoped_runs` returns the marker among the runs it scoped");

    let mut xs: Vec<i32> = scoped.iter().map(|run| run.x()).collect();
    xs.sort_unstable();
    xs.dedup();
    let split_x = column_split_x(&xs);
    let marker_side = marker_x < split_x;
    scoped.retain(|run| (run.x() < split_x) == marker_side);
    assert!(
        !scoped.is_empty(),
        "positive control: `{marker}`'s own column must contain at least itself"
    );

    harness::paint::texts(&harness::paint::group_into_lines(&scoped))
}

/// Open the shortcuts dialog and settle it, returning the runs of the second
/// frame (trap 4 above: no scroll — the two-column layout leaves nothing below
/// the fold to scroll to).
///
/// `app.shortcuts_open = true` is set directly (ADR 0002 §A2: `App::default()`
/// only, never `App::new()`); no key is sent to reach it, because opening the
/// dialog is not what this test is about.
fn open_and_settle(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    // Frame 1 — the window is requested for the first time. It paints nothing
    // of its own body yet (ADR 0002: the first frame is not settled).
    let _ = harness::paint::painted_runs(ctx, app);
    // Frame 2 — settled: the dialog's first real layout is everything this
    // test asserts on.
    harness::paint::painted_runs(ctx, app)
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
    let lines = lines_in_column_of(&runs, "Command line");

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
