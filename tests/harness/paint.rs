//! tests/harness/paint.rs — what actually reached the screen, once.
//!
//! `egui` 0.29.1 cannot assert a rendered **pixel** (`egui_kittest` needs
//! ≥ 0.30, ADR 0002), and that half of the old premise stays true. It has
//! always been able to assert a rendered **string**: `Context::run` returns
//! `FullOutput { shapes, .. }`, and every `Shape::Text(TextShape { pos, galley,
//! .. })` carries the exact text laid out and where it landed. No new
//! dependency, no version bump.
//!
//! That matters because **a source scan asserts a call is written; nothing in
//! it asserts the paint happened.** The gap is measured: during the LCV-125
//! review five mutants — the transcript loop wrapped in `horizontal_wrapped`,
//! iterated `.rev()`, skipping the `note` role, and each of the two
//! always-visible settings sentences guarded on a non-empty key — survived
//! `fmt`, `clippy -D warnings` and 1208 green tests while breaking five stated
//! criteria. One takes the plaintext-key warning off the screen on exactly the
//! frame an operator is about to paste a key in, while the sentence a scan
//! looks for is still right there in the source.
//!
//! ## Eight traps, each with the reason it exists
//!
//! 1. **Paint order is not reading order.** `out.shapes` is emission order —
//!    widget construction order across every surface in the frame. Nothing here
//!    reads a run by its index; [`group_into_lines`] sorts by position first.
//! 2. **Surfaces interleave vertically; membership is clip-rect containment.**
//!    One frame paints five surfaces whose rows interleave in `y` — the
//!    toolbar's `Polyline` lands six points from the transcript's second row —
//!    so grouping a whole frame by `y` braids them together. egui clips a
//!    panel's contents to the panel and a scroll area's to a rect inside it, so
//!    every run of a surface is clipped to a rect inside that surface's and no
//!    run from another one is. **A `Window` title can never be the marker**: it
//!    paints on a clip covering the *entire screen*, so scoping by it selects
//!    the whole frame. Worse on the shortcuts dialog, where the menu bar
//!    carries `File`, `Edit`, `View`, `Tools` and `Help` on a screen-wide clip
//!    and four of that dialog's eight headings collide with one by exact text.
//!    Reading a window's *geometry* from `ctx.memory(|m| m.area_rect(…))` is a
//!    different thing and is fine — an `Area` id, not a paint surface.
//! 3. **Two columns share one clip rect.** `ui.columns` clones the parent
//!    painter without narrowing its clip, so both of the shortcuts dialog's
//!    columns carry the same `clip_rect`. A two-column surface must bucket by
//!    column **before** grouping by `y`, or a left-column run joins a
//!    right-column line whenever their `y` falls within [`SAME_LINE`] and an
//!    ordered assertion quietly turns into noise. That bucketing stays at the
//!    one call site needing it (`tests/it/ui/shortcuts_command_line_group.rs`); it is
//!    deliberately not a parameter here.
//! 4. **Runs on one visual row do not share a `y`.** A `Grid` row's label and
//!    its widget's text sit on baselines a point apart — `402` and `401` for
//!    `API Key` and its field — and the agent panel's heading and its `×`
//!    differ by four, so an exact `y` match splits a label from its own value.
//!    [`SAME_LINE`] is **6**, not a round number: the closest two genuinely
//!    different lines measured on these surfaces are 13 points apart, so 6
//!    clears the 4-point intra-row spread and still cannot bridge two rows.
//! 5. **A binding's monospace column is padded.** `shortcut_row` paints
//!    `format!("{binding:<20}")`, so the run for `"ArrowUp"` is
//!    `"ArrowUp             "`. [`group_into_lines`] trims, or every expected
//!    string in every table would carry hand-counted trailing spaces.
//! 6. **Empty and whitespace-only runs exist, and are dropped.** egui emits a
//!    `Shape::Text` with an empty galley for an empty `TextEdit`, carrying
//!    nothing an operator can read. Dropping it is what makes the line
//!    `["API Key"]` mean *the key field shows nothing at all*.
//! 7. **The first frame is not settled.** A `Window`'s `Area` is not yet
//!    placed, so a dialog paints none of its body on the frame it is first
//!    requested: an **absence** assertion must be made on a later one or it
//!    passes because nothing was painted at all. That is also what
//!    [`painted_runs_at`]'s `runs.len() > 10` control catches. A harness helper
//!    that asserts is unusual and deliberate — without it, an app painting
//!    nothing reads as "the string is absent, as expected".
//! 8. **Positions are collected untruncated and rounded only at the point of
//!    use.** [`Run`] stores `pos` and `height`; the whole-point `y`/`x` the
//!    line-grouping callers read come from [`Run::y`] and [`Run::x`]. Rounding
//!    at collection would be invisible to every test in this repository *and*
//!    would silently break the one assertion class that can see a sliced row:
//!    LCV-133's containment check is `pos.y >= clip.top() && pos.y + height <=
//!    clip.bottom()`, the `height` term cannot be rebuilt from a shape that
//!    never stored one, and at the 1280×460 control the `F1` run straddles the
//!    clip bottom by **0.682pt** (`pos.y = 446.682`, `height = 14.0`, clip
//!    bottom `460.000`) — under a whole rounding step, so which way the round
//!    falls, not the geometry, would decide whether the straddle is seen.
//!
//! ## The assertion idiom
//!
//! **A literal table of visual lines** — one assertion covering presence,
//! verbatim text, left-to-right order within a line and top-to-bottom order of
//! the lines: `assert_eq!(texts(&lines_on_surface_of(&runs, marker)),
//! vec![vec!["Endpoint URL", "https://…"], vec!["Model", "…"]])`. [`texts`]
//! drops the `y`, and that is what keeps absolute coordinates out of the
//! assertion. **The y-carrying variant** — [`lines_on_surface_of`] without
//! [`texts`] — is for the one case the table cannot express: proving two lines
//! are *distinct* rather than merely present and ordered, and then only as a
//! strict inequality between two of them, never as a value.
//!
//! `marker` is a string the test injected into its own fixture
//! (`tests/it/agent/panel_and_settings.rs` uses `LCV125ROW-{role}`) or a
//! shipped label it pinned elsewhere, and [`scoped_runs`] asserts it is painted
//! **exactly once**. Never a substring: a test controls its fixture text, not
//! the product's copy.

use lasercad::app::App;

/// Vertical slack, in points, within which two runs on one surface count as
/// sitting on the same visual line. Six — see trap 4 in the module header for
/// why that number and not a round one.
pub const SAME_LINE: i32 = 6;

/// One painted text run: the surface it was clipped to, where egui laid it out,
/// how tall the galley is, and the string inside it.
///
/// `pos` and `height` are **untruncated** (trap 8). Positions are only
/// meaningful at `pixels_per_point = 1.0`, which every test reading one sets
/// explicitly.
#[derive(Clone)]
pub struct Run {
    /// The `ClippedShape::clip_rect` this run was painted under — its surface.
    pub clip: egui::Rect,
    /// `TextShape::pos`, exactly as egui laid it out.
    pub pos: egui::Pos2,
    /// `galley.size().y` — the term LCV-133's containment check needs.
    pub height: f32,
    /// `galley.text()`, untrimmed. [`group_into_lines`] trims; nothing else does.
    pub text: String,
}

impl Run {
    /// `pos.y` rounded to a whole point, for line grouping only.
    pub fn y(&self) -> i32 {
        self.pos.y.round() as i32
    }

    /// `pos.x` rounded to a whole point, for left-to-right ordering only.
    pub fn x(&self) -> i32 {
        self.pos.x.round() as i32
    }
}

/// Every `Shape::Text` under `shape`, including those nested inside a
/// `Shape::Vec`, which is how egui groups a widget's own painting — a
/// `Button`'s label is inside one, so a collector that did not recurse would
/// never see `Cancel`. `Shape::Vec` is **the only shape variant at egui 0.29.1
/// that contains other shapes**, so this `match` is exhaustive in practice, and
/// that is the one fact here an egui bump has to revisit.
pub fn collect_text(clip: egui::Rect, shape: &egui::Shape, out: &mut Vec<Run>) {
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

/// Every non-empty text run in `shapes`, in paint order (trap 1: not reading
/// order). Split out of [`painted_runs_at`] so a unit test can hand it a
/// synthetic shape list without driving a real frame.
pub fn runs_in(shapes: &[egui::epaint::ClippedShape]) -> Vec<Run> {
    let mut runs = Vec::new();
    for clipped in shapes {
        collect_text(clipped.clip_rect, &clipped.shape, &mut runs);
    }
    runs.retain(|run| !run.text.trim().is_empty());
    runs
}

/// Drive one `App::update_ui` frame at `screen` with `events`, and hand back
/// every non-empty text run egui painted. This is the part no source scan can
/// reach: a scan proves a call is *written*, this proves it *ran*, that its
/// output reached the paint list, and where it landed relative to everything
/// else on its surface. The `runs.len() > 10` control is load-bearing (trap 7).
pub fn painted_runs_at(
    ctx: &egui::Context,
    app: &mut App,
    screen: [f32; 2],
    events: Vec<egui::Event>,
) -> Vec<Run> {
    let out = ctx.run_ui(super::raw_input_at(screen, events), |ui| app.update_ui(ui));
    // Positions are only meaningful at pixels_per_point == 1.0 (ADR 0002 §A3
    // rule 4): epaint rebuilds the font atlas when the value changes, so a
    // galley's `pos` at any other scale is not the position a test's own
    // arithmetic assumes. Checked here, once, **after** `ctx.run_ui` returns —
    // never before, since a pre-run read on a fresh context reports the
    // default and would pass on exactly the frame that broke the assumption
    // (`set_pixels_per_point` "becomes active at the start of the next
    // pass", egui-0.29.1 `context.rs`) — so every caller gets this for free
    // instead of remembering `ctx.set_pixels_per_point(1.0)` itself.
    assert_eq!(
        ctx.pixels_per_point(),
        1.0,
        "painted_runs_at: positions are only meaningful at pixels_per_point \
         == 1.0 (ADR 0002 §A3 rule 4) — call ctx.set_pixels_per_point(1.0) \
         before driving a frame through this harness"
    );
    let runs = runs_in(&out.shapes);
    assert!(
        runs.len() > 10,
        "positive control: a frame of this app paints text, saw {} runs",
        runs.len()
    );
    runs
}

/// [`painted_runs_at`] at the harness [`SCREEN`](super::SCREEN) with no events.
/// The driver, its positive control and its empty-run rule live there; this
/// supplies two defaults and nothing else.
pub fn painted_runs(ctx: &egui::Context, app: &mut App) -> Vec<Run> {
    painted_runs_at(ctx, app, super::SCREEN, Vec::new())
}

/// The runs painted on the same surface as the run reading `marker`, plus that
/// surface's clip rect. "Same surface" is clip-rect containment (trap 2), so
/// `marker` must be a run genuinely inside the surface and never a `Window`
/// title, and it must be painted exactly once — a substring match against the
/// product's own copy is not a scope, it is a coincidence waiting to happen.
pub fn scoped_runs<'a>(runs: &'a [Run], marker: &str) -> (Vec<&'a Run>, egui::Rect) {
    let matches: Vec<&Run> = runs
        .iter()
        .filter(|run| run.text.trim() == marker)
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "`{marker}` must be painted exactly once, saw {} times",
        matches.len()
    );
    let surface = matches[0].clip;

    let scoped: Vec<&Run> = runs
        .iter()
        .filter(|run| surface.contains_rect(run.clip))
        .collect();
    assert!(
        scoped.len() < runs.len(),
        "positive control: the surface must be narrower than the frame"
    );
    (scoped, surface)
}

/// `runs` as visual lines, top to bottom, each read left to right, carrying the
/// whole-point `y` it starts at. Grouping walks in `y` order, and within one
/// line that is not `x` order (trap 4: a `Grid`'s widget text sits a point
/// *above* its own label), so each line is re-sorted by `x` before its texts
/// are taken. Every run is trimmed (trap 5).
pub fn group_into_lines(runs: &[&Run]) -> Vec<(i32, Vec<String>)> {
    let mut scoped: Vec<&Run> = runs.to_vec();
    scoped.sort_by_key(|run| (run.y(), run.x()));

    let mut lines: Vec<(i32, Vec<&Run>)> = Vec::new();
    for run in scoped {
        match lines.last_mut() {
            Some((top, members)) if run.y() - *top <= SAME_LINE => members.push(run),
            _ => lines.push((run.y(), vec![run])),
        }
    }
    lines
        .into_iter()
        .map(|(top, mut members)| {
            members.sort_by_key(|run| run.x());
            (
                top,
                members
                    .into_iter()
                    .map(|run| run.text.trim().to_owned())
                    .collect(),
            )
        })
        .collect()
}

/// Every visual line painted on the same surface as the run reading `marker` —
/// [`scoped_runs`] then [`group_into_lines`]. The idiom in the module header.
pub fn lines_on_surface_of(runs: &[Run], marker: &str) -> Vec<(i32, Vec<String>)> {
    let (scoped, _) = scoped_runs(runs, marker);
    group_into_lines(&scoped)
}

/// The texts of `lines`, dropping the `y` each was laid out at — which is what
/// keeps an absolute coordinate out of an assertion.
pub fn texts(lines: &[(i32, Vec<String>)]) -> Vec<Vec<String>> {
    lines.iter().map(|(_, t)| t.clone()).collect()
}
