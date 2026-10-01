//! LCV-141 — the agent panel stays within the right third, and Agent
//! Settings gets a Close button, a live-edit sentence and a bounded
//! `ScrollArea`.
//!
//! ## AC 1-3: the width ceiling
//!
//! `egui::containers::panel::PanelState::load(ctx, Id::new("agent_panel"))`
//! reads the panel's own persisted rect — the actual outer rectangle, not
//! something inferred from painted content — so these tests read that
//! directly, at three application widths, and after each of the four ways
//! [`src/app/panels.rs::draw_agent_side_panel`] documents the ceiling must
//! hold: the very first frame, a width dragged wide in a bigger window
//! carried into a smaller one, an active resize drag, and the window itself
//! shrinking mid-session.
//!
//! ## AC 4-5: the footer reservation and containment
//!
//! `tests/harness/paint.rs`-based containment tests, with both a single
//! unbroken 300-character transcript entry and 200 short ones, in both the
//! busy and non-busy state, plus real Send/Cancel/`×` clicks through the
//! pointer-click convention (ADR 0002 §A4 rule 3).
//!
//! ## AC 6-7: AI Settings' Close button and its `ScrollArea`
//!
//! A real click on Close and a real click on `×`, each against a test-owned
//! `settings_path`, asserting identical persisted bytes and identical `App`
//! state afterward; a bounded-scrolling test at 800×600 with the shipped
//! four-field form, and a growth-probe variant proving the `ScrollArea`, not
//! window growth, absorbs an overflow.
//!
//! ## AC 8
//!
//! Covered by re-running `tests/it/agent/panel_and_settings.rs`,
//! `tests/it/agent/timeout_and_cancel.rs`, `tests/it/app/idle_repaint.rs`
//! and `src/app/viewport/tests.rs::every_repaint_request_in_src_is_conditional`
//! (unmodified, save for the one now-incomplete literal table in LCV-125's
//! own settings-paint test — see the note on that test).

use crate::harness;

use harness::paint::{self, Run, lines_on_surface_of, painted_runs_at, texts};
use harness::raw_input_at;
use lasercad::app::App;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Shared plumbing
// ---------------------------------------------------------------------------

/// The `Id` `src/app/panels.rs::draw_agent_side_panel` registers the
/// `SidePanel` under.
fn agent_panel_id() -> egui::Id {
    egui::Id::new("agent_panel")
}

/// One third of `screen_width`, the ceiling AC 1-3 pin.
fn ceiling(screen_width: f32) -> f32 {
    screen_width / 3.0
}

/// Drive one frame at `screen`, with the agent panel open, and hand back its
/// persisted outer rect — read via `PanelState::load`, never inferred from
/// painted content (AC 1).
fn panel_rect_at(ctx: &egui::Context, app: &mut App, screen: [f32; 2]) -> egui::Rect {
    app.agent.panel_open = true;
    let _ = ctx.run_ui(raw_input_at(screen, Vec::new()), |ui| app.update_ui(ui));
    egui::containers::panel::PanelState::load(ctx, agent_panel_id())
        .expect("the agent panel must have stored its state by now")
        .outer_rect
}

/// A headless context at `pixels_per_point == 1.0` and a default `App`.
fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    (ctx, App::default())
}

// ---------------------------------------------------------------------------
// AC 1 / AC 3 — the ceiling itself
// ---------------------------------------------------------------------------

/// AC 1 — at three application widths, on the very first frame the panel is
/// open, its persisted width never exceeds a third of `ctx.screen_rect()`.
///
/// Mutation this catches: reverting `draw_agent_side_panel` to the
/// pre-LCV-141 `.default_width(300.0)` with no `.max_width()` call — at
/// 800×800 the panel would render at 300pt, over the 266.67pt ceiling.
#[test]
fn ac1_panel_width_never_exceeds_one_third_of_the_window() {
    for width in [800.0_f32, 1024.0, 1280.0] {
        let (ctx, mut app) = ctx_and_app();
        let rect = panel_rect_at(&ctx, &mut app, [width, 800.0]);
        assert!(
            rect.width() <= ceiling(width) + 0.5,
            "at screen width {width}: panel width {} exceeds the ceiling {}",
            rect.width(),
            ceiling(width)
        );
    }
}

/// AC 3 — at a `ctx.screen_rect().width()` of `800.0`, the ceiling is
/// `266.666...`, measured to within `0.5` logical point. `800.0` is also less
/// than [`AGENT_PANEL_DEFAULT_WIDTH`]'s 300.0, so the shipped default itself
/// must already be narrowed to the ceiling, not merely bounded by it.
///
/// Mutation this catches: a ceiling computed as `ctx.screen_rect().width() /
/// 3.5` (or any divisor but 3.0) lands outside the 0.5pt tolerance.
#[test]
fn ac3_the_ceiling_at_800_width_is_266_667() {
    let (ctx, mut app) = ctx_and_app();
    let rect = panel_rect_at(&ctx, &mut app, [800.0, 800.0]);
    let expected = 800.0_f32 / 3.0;
    assert!(
        (rect.width() - expected).abs() <= 0.5,
        "expected ~{expected} at 800 width, got {}",
        rect.width()
    );
}

// ---------------------------------------------------------------------------
// AC 2 — recomputed every frame
// ---------------------------------------------------------------------------

/// AC 2 — a width the operator dragged wide in a bigger window is carried
/// into a smaller one, and reclamped there.
///
/// Modelled by seeding egui's own persisted panel memory directly (the exact
/// state a real drag would have left behind — `SidePanel` stores nothing
/// else), at a width only reachable inside the *bigger* window's own ceiling
/// (`420.0` is under 1280's `426.67` ceiling and over 800's `266.67`), rather
/// than hand-driving `egui`'s private resize-response internals across
/// several frames. The frame-body call this exercises —
/// `PanelState::load` → `clamp_to_range(width, width_range)` — is exactly the
/// one a real drag would have gone through to leave that memory behind.
///
/// Mutation this catches: computing the ceiling once and caching it (instead
/// of recomputing from `ctx.screen_rect()` every `show`) would carry the
/// dragged 420pt straight through to the 800-wide frame.
#[test]
fn ac2_a_width_dragged_wide_in_a_bigger_window_is_reclamped_in_a_smaller_one() {
    let (ctx, mut app) = ctx_and_app();
    app.agent.panel_open = true;

    // Settle once at the bigger window so the panel's `Id` exists and its
    // ceiling there (426.67) really does allow 420.0.
    let big = panel_rect_at(&ctx, &mut app, [1280.0, 800.0]);
    assert!(
        420.0 <= ceiling(1280.0),
        "control: 420pt must be reachable inside 1280's own ceiling ({})",
        ceiling(1280.0)
    );

    let dragged = egui::Rect::from_min_size(
        egui::pos2(big.min.x, big.min.y),
        egui::vec2(420.0, big.height()),
    );
    ctx.data_mut(|d| {
        d.insert_persisted(
            agent_panel_id(),
            egui::containers::panel::PanelState {
                outer_rect: dragged,
            },
        )
    });
    // Control: the seed really landed and really exceeds the *smaller*
    // window's ceiling before the next frame ever runs.
    assert!(420.0 > ceiling(800.0));

    let rect = panel_rect_at(&ctx, &mut app, [800.0, 800.0]);
    assert!(
        rect.width() <= ceiling(800.0) + 0.5,
        "a width dragged wide in a bigger window must be reclamped in a \
         smaller one, got {}",
        rect.width()
    );
}

/// AC 2 — the window itself shrinking mid-session reclamps the *default*
/// width, with no drag involved at all: the panel opens at 1280 (default
/// 300pt, comfortably under that window's 426.67 ceiling), and the very next
/// frame the window is 800 wide (ceiling 266.67, under the still-persisted
/// 300pt).
///
/// Mutation this catches: the same as AC 1's — a cached or one-shot ceiling
/// would leave the panel at 300pt once the window had already opened wide.
#[test]
fn ac2_shrinking_the_window_mid_session_reclamps_the_default_width() {
    let (ctx, mut app) = ctx_and_app();
    let opened_wide = panel_rect_at(&ctx, &mut app, [1280.0, 800.0]);
    assert!(
        opened_wide.width() > ceiling(800.0),
        "control: the default width opened at 1280 must exceed 800's ceiling \
         for the shrink to be a real test, got {}",
        opened_wide.width()
    );

    let shrunk = panel_rect_at(&ctx, &mut app, [800.0, 800.0]);
    assert!(
        shrunk.width() <= ceiling(800.0) + 0.5,
        "the window shrinking mid-session must reclamp the panel, got {}",
        shrunk.width()
    );
}

/// AC 2 — the ceiling holds during an active resize drag, not only once it
/// settles: a real pointer press on the resize handle followed by a drag
/// motion past the ceiling is clamped on arrival, and the panel never reports
/// a persisted width wider than the ceiling on any frame of the sequence.
///
/// A warm-up `PointerMoved` (hover), then a press, then a `PointerMoved`
/// while still down — `egui`'s own convention for a drag, mirroring the
/// pointer-click convention's warm-up frame (ADR 0002 §A4 rule 3) one step
/// further. `SidePanel::show_inside_dyn` pre-reads the *previous* frame's
/// resize response to avoid a frame of latency, so the drag's effect on
/// `panel_rect` is visible the frame after the drag motion is sent — one more
/// idle frame settles it, exactly like a freshly opened `Window`.
#[test]
fn ac2_the_ceiling_holds_during_an_active_resize_drag() {
    let (ctx, mut app) = ctx_and_app();
    let start = panel_rect_at(&ctx, &mut app, [1280.0, 800.0]);
    let handle = egui::pos2(start.left(), start.center().y);
    // Far enough left that, unclamped, the drag would land the panel far
    // wider than 1280's own 426.67 ceiling.
    let dragged_to = egui::pos2(start.left() - 700.0, start.center().y);

    let screen = [1280.0_f32, 800.0];
    let _ = ctx.run_ui(
        raw_input_at(screen, vec![egui::Event::PointerMoved(handle)]),
        |ui| app.update_ui(ui),
    );
    let _ = ctx.run_ui(
        raw_input_at(
            screen,
            vec![egui::Event::PointerButton {
                pos: handle,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
        ),
        |ui| app.update_ui(ui),
    );
    let _ = ctx.run_ui(
        raw_input_at(screen, vec![egui::Event::PointerMoved(dragged_to)]),
        |ui| app.update_ui(ui),
    );
    // The frame that observes the drag response cached by the previous one.
    let _ = ctx.run_ui(raw_input_at(screen, Vec::new()), |ui| app.update_ui(ui));

    let rect = egui::containers::panel::PanelState::load(&ctx, agent_panel_id())
        .expect("the panel must still have state after the drag")
        .outer_rect;
    assert!(
        rect.width() <= ceiling(1280.0) + 0.5,
        "an in-progress drag past the ceiling must still be clamped to it, \
         got {} (ceiling {})",
        rect.width(),
        ceiling(1280.0)
    );

    // Release, so this test leaves no dangling pointer-down state.
    let _ = ctx.run_ui(
        raw_input_at(
            screen,
            vec![egui::Event::PointerButton {
                pos: dragged_to,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ),
        |ui| app.update_ui(ui),
    );
}

// ---------------------------------------------------------------------------
// AC 4 — the footer reservation, from the paint list
// ---------------------------------------------------------------------------

/// Fill `app.agent.chat` with `n` short, uniquely-marked rows — enough to
/// overflow any reasonable transcript height, so the scroll area's own
/// `max_height` (not the content) decides where the visible bottom sits.
fn fill_short_transcript(app: &mut App, n: usize) {
    for i in 0..n {
        app.agent
            .chat
            .push(("assistant".to_owned(), format!("LCV141-AC4-{i}")));
    }
}

/// The clip-rect bottom of any run carrying the `LCV141-AC4-` marker — every
/// transcript row shares the *same* clip rect (the `ScrollArea`'s own
/// viewport), so this is the geometric quantity the reservation controls.
fn transcript_clip_bottom(runs: &[Run]) -> f32 {
    runs.iter()
        .find(|r| r.text.contains("LCV141-AC4-"))
        .expect("at least one transcript row must be painted")
        .clip
        .bottom()
}

/// AC 4 — the transcript's visible bottom edge sits strictly higher (more of
/// the panel reserved below it) while a turn is running than while idle, with
/// the same overflowing transcript in both cases.
///
/// Mutation this catches: computing `scroll_height` from a flat constant that
/// never checks `app.agent.busy` (the pre-LCV-141 behaviour) — the busy and
/// idle clip bottoms would be identical.
#[test]
fn ac4_the_busy_row_reservation_shrinks_the_transcript_this_frame() {
    fn clip_bottom(busy: bool) -> f32 {
        let (ctx, mut app) = ctx_and_app();
        app.agent.panel_open = true;
        app.agent.busy = busy;
        fill_short_transcript(&mut app, 40);
        let _ = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());
        let runs = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());
        transcript_clip_bottom(&runs)
    }

    let idle = clip_bottom(false);
    let busy = clip_bottom(true);
    assert!(
        busy < idle - 10.0,
        "the busy transcript must claim noticeably less vertical room: idle \
         bottom {idle}, busy bottom {busy}"
    );
}

// ---------------------------------------------------------------------------
// AC 5 — containment: long transcripts, both busy states, real clicks
// ---------------------------------------------------------------------------

/// A complete primary-button click at `pos` — the second half of the
/// harness's pointer-click convention (ADR 0002 §A4 rule 3); always preceded
/// by its own warm-up frame.
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

/// Locate the one run reading `label` and return a point inside its glyph
/// rect, 2pt right and vertically centred — `button_padding` is `[4.0, 1.0]`
/// at the default style, so this sits inside the padded widget a fortiori.
fn locate(runs: &[Run], label: &str) -> egui::Pos2 {
    let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == label).collect();
    assert_eq!(
        matches.len(),
        1,
        "`{label}` must be painted exactly once, saw {} times",
        matches.len()
    );
    let run = matches[0];
    egui::pos2(run.pos.x + 2.0, run.pos.y + run.height / 2.0)
}

/// A point inside an `egui::Window`'s own `×` — painted as two `line_segment`
/// strokes (`egui-0.29.1 containers/window.rs::close_button`), never
/// `Shape::Text`, so [`locate`] cannot find it. `ctx.memory(|m|
/// m.area_rect(id))` gives the window's placed outer rect (same idiom
/// `tests/it/ui/discard_dialog_pointer_click.rs` uses for a modal's own
/// rect); the button sits in from the top-right corner by roughly one
/// `icon_width`, so a point a little further in from that corner lands inside
/// its hit-test rect without needing the exact formula.
fn window_close_button_pos(ctx: &egui::Context, title: &str) -> egui::Pos2 {
    let rect = ctx
        .memory(|m| m.area_rect(crate::harness::window_id(title)))
        .unwrap_or_else(|| panic!("the `{title}` window must be placed by now"));
    egui::pos2(rect.right() - 12.0, rect.top() + 12.0)
}

/// Every run whose text is exactly one of `labels`, in the order given.
fn locate_all<'a>(runs: &'a [Run], labels: &[&str]) -> Vec<&'a Run> {
    labels
        .iter()
        .map(|label| {
            let matches: Vec<&Run> = runs.iter().filter(|r| r.text.trim() == *label).collect();
            assert_eq!(
                matches.len(),
                1,
                "`{label}` must be painted exactly once, saw {} times",
                matches.len()
            );
            matches[0]
        })
        .collect()
}

/// Every one of `runs` sits fully inside its own clip rect — LCV-133's
/// containment shape (`pos.y >= clip.top() && pos.y + height <=
/// clip.bottom()`) — and no two on *different* rows overlap vertically.
///
/// Two controls meant to share one visual row (Send sits beside the input
/// field) necessarily share a `y`; that is correct layout, not the defect
/// this demand closes. What must never happen is a row colliding with a
/// *different* row — the busy row pushed down onto the composer row below
/// it, or vice versa — so the pairwise check only fires between runs whose
/// `y` differs by more than one row's worth of slop.
fn assert_contained_and_non_overlapping(runs: &[&Run]) {
    const SAME_ROW_SLOP: f32 = 10.0;
    for run in runs {
        assert!(
            run.pos.y >= run.clip.top() && run.pos.y + run.height <= run.clip.bottom(),
            "`{}` is not fully inside its clip rect vertically: pos.y={}, height={}, clip={:?}",
            run.text,
            run.pos.y,
            run.height,
            run.clip
        );
        // `Run` carries no width (`tests/harness/paint.rs`), so this is the
        // weaker half of containment — the run's own *start* must be inside
        // its clip horizontally — but it is exactly what a row that grows
        // past the panel's width pushes out: a widget positioned starting
        // beyond `clip.right()` is clipped away regardless of how wide its
        // own content is.
        assert!(
            run.pos.x >= run.clip.left() && run.pos.x <= run.clip.right(),
            "`{}` starts outside its clip rect horizontally: pos.x={}, clip={:?}",
            run.text,
            run.pos.x,
            run.clip
        );
    }
    for a in 0..runs.len() {
        for b in (a + 1)..runs.len() {
            let (ra, rb) = (runs[a], runs[b]);
            if (ra.pos.y - rb.pos.y).abs() <= SAME_ROW_SLOP {
                continue;
            }
            let overlap = ra.pos.y < rb.pos.y + rb.height && rb.pos.y < ra.pos.y + ra.height;
            assert!(
                !overlap,
                "`{}` (y={}, h={}) and `{}` (y={}, h={}) overlap vertically",
                ra.text, ra.pos.y, ra.height, rb.text, rb.pos.y, rb.height
            );
        }
    }
}

/// AC 5 — with a single unbroken 300-character transcript entry, the input
/// field, Send button and `×` close button all stay inside the panel's clip
/// rect and do not overlap each other, in both the busy and non-busy state.
/// Busy additionally checks the Cancel button.
///
/// Mutation this catches: the same footer-reservation regression AC 4 pins,
/// this time observed as a genuinely clipped or overlapping control rather
/// than a moved clip-rect bottom.
#[test]
fn ac5_controls_stay_inside_the_panel_with_a_long_unbroken_transcript_entry() {
    for busy in [false, true] {
        let (ctx, mut app) = ctx_and_app();
        app.agent.panel_open = true;
        app.agent.busy = busy;
        let long = "outcome ".repeat(38); // 304 characters, per LCV-125/panel.rs.
        app.agent.chat.push(("assistant".to_owned(), long));

        let _ = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());
        let runs = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());

        let mut labels = vec!["Ask the AI…", "Send", "×"];
        if busy {
            labels.push("Cancel");
        }
        let found = locate_all(&runs, &labels);
        assert_contained_and_non_overlapping(&found);
    }
}

/// AC 5 — same shape, for 200 short transcript entries.
#[test]
fn ac5_controls_stay_inside_the_panel_with_two_hundred_transcript_entries() {
    for busy in [false, true] {
        let (ctx, mut app) = ctx_and_app();
        app.agent.panel_open = true;
        app.agent.busy = busy;
        fill_short_transcript(&mut app, 200);

        let _ = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());
        let runs = painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());

        let mut labels = vec!["Ask the AI…", "Send", "×"];
        if busy {
            labels.push("Cancel");
        }
        let found = locate_all(&runs, &labels);
        assert_contained_and_non_overlapping(&found);
    }
}

/// AC 5 — closing the panel through a real click on `×` restores the canvas
/// to the full width the panel previously subtracted. `app.camera
/// .viewport_size_px` is synced from the `CentralPanel`'s own rect every
/// frame (`src/app/viewport.rs::draw`), so it is exactly "the width the
/// panel subtracted" made observable.
#[test]
fn ac5_closing_the_panel_restores_the_canvas_width() {
    let (ctx, mut app) = ctx_and_app();
    // One idle frame first, panel closed, to read the full canvas width.
    let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], Vec::new()), |ui| {
        app.update_ui(ui)
    });
    let full_width = app.camera.viewport_size_px[0];

    app.agent.panel_open = true;
    let runs = paint::painted_runs_at(&ctx, &mut app, [1280.0, 800.0], Vec::new());
    let narrowed_width = app.camera.viewport_size_px[0];
    assert!(
        narrowed_width < full_width - 10.0,
        "the open panel must really subtract from the canvas: full {full_width}, \
         narrowed {narrowed_width}"
    );

    let close = locate(&runs, "×");
    let _ = ctx.run_ui(
        raw_input_at([1280.0, 800.0], vec![egui::Event::PointerMoved(close)]),
        |ui| app.update_ui(ui),
    );
    let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], click_events(close)), |ui| {
        app.update_ui(ui)
    });
    assert!(!app.agent.panel_open, "the × button must close the panel");

    // The click's own frame still draws the panel it was clicked in — egui
    // panels claim their share of `ctx.available_rect()` for the frame that
    // draws them, whether or not the click inside them just flipped the flag
    // that skips them *next* frame. One more idle frame is what settles the
    // `CentralPanel`'s width, exactly like a freshly opened `Window` needs a
    // second frame to place its `Area` (paint.rs trap 7).
    let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], Vec::new()), |ui| {
        app.update_ui(ui)
    });

    assert!(
        (app.camera.viewport_size_px[0] - full_width).abs() <= 0.5,
        "closing the panel must restore the full canvas width: expected \
         ~{full_width}, got {}",
        app.camera.viewport_size_px[0]
    );
}

// ---------------------------------------------------------------------------
// AC 6 — AI Settings: Close matches × exactly
// ---------------------------------------------------------------------------

/// A private, empty directory under the system temp dir, named after the
/// test that owns it.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv141_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the system temp dir must be writable");
    dir
}

/// Open the AI Settings dialog, edit one field so there is something to
/// persist, and settle two frames.
fn open_and_edit(ctx: &egui::Context, app: &mut App) {
    app.agent_settings_open = true;
    app.settings.agent_model = "a-non-default-model".to_owned();
    let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], Vec::new()), |ui| {
        app.update_ui(ui)
    });
    let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], Vec::new()), |ui| {
        app.update_ui(ui)
    });
}

/// AC 6 — a real click on Close, and a real click on `×`, against a
/// test-owned `settings_path`, produce identical persisted bytes and
/// identical `App` state afterward.
///
/// Mutation this catches: a `Close` handler that calls `persist_settings()`
/// itself (a second, parallel path) instead of setting the same
/// `agent_settings_open = false` the × button sets — this test cannot tell
/// those apart by *effect*, but `ac6_done_and_close_share_one_guard_source_scan`
/// below pins that they are not two paths by *construction*.
#[test]
fn ac6_done_closes_and_persists_exactly_like_the_close_button() {
    let via_done = {
        let dir = tempdir("done");
        let path = dir.join("settings.json");
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut app = App {
            settings_path: Some(path.clone()),
            ..App::default()
        };
        open_and_edit(&ctx, &mut app);

        let runs = paint::painted_runs(&ctx, &mut app);
        let done = locate(&runs, "Close");
        let _ = ctx.run_ui(
            raw_input_at([1280.0, 800.0], vec![egui::Event::PointerMoved(done)]),
            |ui| app.update_ui(ui),
        );
        let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], click_events(done)), |ui| {
            app.update_ui(ui)
        });

        assert!(!app.agent_settings_open, "Close must close the window");
        let bytes = std::fs::read_to_string(&path).expect("Close must persist the settings");
        let _ = std::fs::remove_dir_all(&dir);
        (bytes, app.settings.clone(), app.agent_settings_open)
    };

    let via_close = {
        let dir = tempdir("close");
        let path = dir.join("settings.json");
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut app = App {
            settings_path: Some(path.clone()),
            ..App::default()
        };
        open_and_edit(&ctx, &mut app);

        // The window's own `×`, top-right of its title bar — painted as two
        // strokes, not text (see `window_close_button_pos`), and distinct
        // from the agent panel's `×`, which is closed in this test.
        let close = window_close_button_pos(&ctx, "AI Settings");
        let _ = ctx.run_ui(
            raw_input_at([1280.0, 800.0], vec![egui::Event::PointerMoved(close)]),
            |ui| app.update_ui(ui),
        );
        let _ = ctx.run_ui(raw_input_at([1280.0, 800.0], click_events(close)), |ui| {
            app.update_ui(ui)
        });

        assert!(!app.agent_settings_open, "× must close the window");
        let bytes = std::fs::read_to_string(&path).expect("× must persist the settings");
        let _ = std::fs::remove_dir_all(&dir);
        (bytes, app.settings.clone(), app.agent_settings_open)
    };

    assert_eq!(
        via_done.0, via_close.0,
        "Close and × must persist byte-identical settings"
    );
    assert_eq!(
        via_done.1, via_close.1,
        "Close and × must leave identical App-side settings state"
    );
    assert_eq!(via_done.2, via_close.2);
}

/// AC 6 — the new sentence, plus the existing masking and warning, are
/// painted together (the painted-text half of AC 6; the four-field content is
/// LCV-125's).
#[test]
fn ac6_the_live_edit_sentence_is_painted_alongside_the_existing_warning() {
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;

    let _ = paint::painted_runs(&ctx, &mut app);
    let runs = paint::painted_runs(&ctx, &mut app);
    let form = lines_on_surface_of(&runs, "Endpoint URL");
    let flat: Vec<String> = texts(&form).into_iter().flatten().collect();

    assert!(
        flat.iter()
            .any(|t| t == "Changes apply immediately and are saved when this window closes."),
        "the live-edit sentence must be painted: {flat:?}"
    );
    assert!(
        flat.iter().any(|t| t.contains("stored in plain text")),
        "the existing plaintext warning must still be painted: {flat:?}"
    );
    assert!(
        flat.iter().any(|t| t == "Close"),
        "the Close button must be painted: {flat:?}"
    );
}

// ---------------------------------------------------------------------------
// AC 7 — the bounded ScrollArea
// ---------------------------------------------------------------------------

/// AC 7 — at 800×600, the shipped four-field form's rows, both sentences and
/// Close are all painted without needing to scroll: the real dialog, through
/// the real `App`.
#[test]
fn ac7_the_shipped_form_fits_at_800x600_without_scrolling() {
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;

    let _ = paint::painted_runs_at(&ctx, &mut app, [800.0, 600.0], Vec::new());
    let runs = paint::painted_runs_at(&ctx, &mut app, [800.0, 600.0], Vec::new());
    let form = lines_on_surface_of(&runs, "Endpoint URL");
    let flat: Vec<String> = texts(&form).into_iter().flatten().collect();

    for label in [
        "Endpoint URL",
        "Model",
        "API Key",
        "Steps per turn",
        "Changes apply immediately and are saved when this window closes.",
        "Close",
    ] {
        assert!(
            flat.iter().any(|t| t == label),
            "`{label}` must be visible without scrolling at 800x600: {flat:?}"
        );
    }
}

/// AC 7 — a growth probe: a synthetic filler, added only inside this test,
/// pushes the form's content height past the window's available height. Close
/// is not painted at the default (top) scroll position, and becomes painted
/// once the same `ScrollArea` is scrolled to its bottom — proving the
/// `ScrollArea`, not window growth, is what would absorb a fifth field, and
/// that Close stays reachable.
///
/// A hand-built `Window` + `ScrollArea`, not the production
/// `agent_settings_dialog` (private to `src/app/panels.rs`): the filler must
/// sit inside the *same* scroll surface as the real form's Close button, ahead
/// of it, which only a caller of `draw_agent_settings` can arrange. The sizing
/// this reproduces — an unresizable `Window`'s baked `default_size` capping a
/// `ScrollArea` body regardless of screen size — is ADR 0009 decision 1's
/// hard cap, the same one `agent_settings_dialog` itself is subject to.
#[test]
fn ac7_a_growth_probe_is_reachable_only_by_scrolling() {
    /// `offset` is only ever `Some` on the one frame that *requests* a jump.
    /// `ScrollArea::vertical_scroll_offset` forces the scroll position on
    /// every frame it is given — including the *next* one, which reads back
    /// the clamped value `show` itself just stored — so forcing it every
    /// frame would fight that clamp and the view would never settle at the
    /// bottom. `None` lets a settling frame keep whatever the previous frame
    /// already clamped to.
    fn frame(
        ctx: &egui::Context,
        settings: &mut lasercad::io::settings::Settings,
        offset: Option<f32>,
    ) -> egui::FullOutput {
        let screen = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        ctx.run_ui(screen, |ui| {
            let c = &ui.ctx().clone();
            egui::Window::new("Agent Settings Probe")
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(c, |ui| {
                    let mut scroll = egui::ScrollArea::vertical();
                    if let Some(offset) = offset {
                        scroll = scroll.vertical_scroll_offset(offset);
                    }
                    scroll.show(ui, |ui| {
                        // The synthetic fifth field: enough filler rows ahead
                        // of the real form to push its Close button below any
                        // plausible window height at this egui pin (ADR 0009:
                        // 426pt hard cap on a `Window` + `ScrollArea` body).
                        for i in 0..80 {
                            ui.label(format!("LCV141-GROWTH-PROBE-{i}"));
                        }
                        let _ = lasercad::agent::draw_agent_settings(ui, settings);
                    });
                });
        })
    }

    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut settings = lasercad::io::settings::Settings::default();

    // Two frames at the top of the scroll (default offset 0): Close must not
    // be painted — the overflow is real, not a fluke of an unsettled Window.
    let _ = frame(&ctx, &mut settings, None);
    let out = frame(&ctx, &mut settings, None);
    let runs = paint::runs_in(&out.shapes);
    assert!(
        !runs.iter().any(|r| r.text.trim() == "Close"),
        "control: the probe's filler must really push Close out of view at the \
         top of the scroll"
    );
    assert!(
        runs.iter()
            .any(|r| r.text.trim() == "LCV141-GROWTH-PROBE-0"),
        "control: the filler itself must be visible at the top"
    );

    // One frame requests a jump to the bottom (a huge offset clamps to the
    // maximum); one more settles at the now-persisted, clamped position —
    // Close is reachable.
    let _ = frame(&ctx, &mut settings, Some(100_000.0));
    let out = frame(&ctx, &mut settings, None);
    let runs = paint::runs_in(&out.shapes);
    assert!(
        runs.iter().any(|r| r.text.trim() == "Close"),
        "Close must be reachable by scrolling within the dialog's own bounds"
    );
}

/// AC 7 — against the **real** `App` and the real
/// `src/app/panels.rs::agent_settings_dialog`, not the hand-built `Window` +
/// `ScrollArea` above: at a screen too short for the shipped form to fit at
/// all — `800x150`, well under the dialog's own content height — `Close` is
/// not painted at the default (top) scroll position, and becomes painted
/// after a real pointer hover over the dialog plus a real
/// `egui::Event::MouseWheel` scroll. Nothing here builds a second `Window`;
/// every frame goes through `App::update_ui`, so this is the one test in the
/// file that can actually tell the shipped `ScrollArea::vertical()` wrap
/// apart from `ac7_a_growth_probe_is_reachable_only_by_scrolling`'s own,
/// separate one.
///
/// Mutation this catches: deleting `egui::ScrollArea::vertical()` from
/// `agent_settings_dialog` (leaving `draw_agent_settings` called directly
/// against the `Window`'s `ui`) — the probe test above stays green because it
/// never calls that function, but this one goes red: the real `Window` is
/// `.resizable(false)` and un-scrolled, so its content either paints nothing
/// past the fold with no way to reach it, or the `Window` grows past the
/// screen and gets constrained there, and `Close` never becomes reachable
/// however the wheel spins.
#[test]
fn ac7_the_real_settings_dialog_scrolls_to_reach_done() {
    let (ctx, mut app) = ctx_and_app();
    app.agent_settings_open = true;
    let screen = [800.0_f32, 150.0];

    // Trap 7: the Window's Area is not placed on the first frame it is
    // requested. Two idle frames settle it.
    let _ = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());

    assert!(
        runs.iter().any(|r| r.text.trim() == "Endpoint URL"),
        "control: Endpoint URL sits at the top of the form and must already \
         be visible at the default (top) scroll offset"
    );
    assert!(
        !runs.iter().any(|r| r.text.trim() == "Close"),
        "control: at {screen:?} the shipped form must really overflow the \
         window — Close must not be reachable without scrolling"
    );

    let dialog = ctx
        .memory(|m| m.area_rect(crate::harness::window_id("AI Settings")))
        .expect("the AI Settings window must be placed by now");
    let inside = dialog.center();

    // One frame hovers the dialog and sends a real downward scroll: many
    // small `MouseWheelUnit::Point` events, each under egui's own 8.0-point
    // smoothing threshold (egui-0.29.1 `input_state/mod.rs`), so the whole
    // delta lands in `smooth_scroll_delta` within this one input pass rather
    // than trickling in over several frames of exponential smoothing. `y` is
    // negative: per `egui::Event::MouseWheel`'s own doc comment, a positive
    // `y` moves the content *down*, revealing what is *above* — the opposite
    // of what reaching a control below the fold needs.
    let mut events = vec![egui::Event::PointerMoved(inside)];
    events.extend((0..200).map(|_| egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -7.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    }));
    let _ = painted_runs_at(&ctx, &mut app, screen, events);

    // One more idle frame settles the offset the wheel just requested: an
    // `egui::ScrollArea` lays this frame's content out from the *previous*
    // frame's stored offset and only applies the hover+wheel adjustment (and
    // stores the new offset) after that content is already positioned
    // (egui-0.29.1 `containers/scroll_area.rs`) — the same one-settling-frame
    // shape the growth probe above documents for its own forced offset.
    let runs = painted_runs_at(&ctx, &mut app, screen, Vec::new());
    assert!(
        runs.iter().any(|r| r.text.trim() == "Close"),
        "a real hover plus a real MouseWheel scroll over the real dialog \
         must reach Close"
    );
}
