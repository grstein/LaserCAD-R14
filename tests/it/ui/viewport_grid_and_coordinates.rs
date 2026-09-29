//! tests/it/ui/viewport_grid_and_coordinates.rs — LCV-137 AC 4, AC 6, AC 7.
//!
//! AC 1/AC 2 (paint order), AC 3 (source scans) and AC 5 (grid line position)
//! live next to the code they cover — `src/app/viewport.rs` and
//! `src/render/grid.rs` — since they call private functions or scan a single
//! file's own source. This file covers the three acceptance criteria that
//! need a real, running `App` with real chrome: wheel-zoom anchoring, the
//! flipped `Camera::pan` Y sign end-to-end, and cross-subsystem agreement
//! after navigation.
//!
//! Every test here drives the real `App::update_ui`, so the `CentralPanel`
//! rect it lays out carries a genuine nonzero origin — the menubar and
//! toolbar always claim space first — without ever needing that origin's
//! numeric value: [`boot`] reads back the real rect via `ctx.available_rect()`
//! (mirroring `tests/it/app/idle_repaint.rs::boot` and
//! `tests/it/app/autosave_dirty.rs::boot_with_line_tool`), and every world-point check below
//! is read back from the app's own state (`last_cursor_world`, `active_snap`,
//! a committed entity's own field) rather than hand-computed from a rect this
//! file does not control.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::geometry::Vec2;
use lasercad::io::export_svg;
use lasercad::tools::{LineTool, SelectTool};

/// Spend frame 0 (which always settles layout for fixed panels — no `Window`/
/// `Area` placement is in play here, unlike `tests/harness/paint.rs` trap 7)
/// and report the `CentralPanel` rect egui left once the chrome claimed its
/// share.
fn boot(app: App) -> (egui::Context, App, egui::Rect) {
    let ctx = egui::Context::default();
    let mut app = app;
    let mut canvas = egui::Rect::NOTHING;
    let _ = ctx.run(harness::raw_input(vec![]), |c| {
        app.update_ui(c);
        canvas = c.available_rect();
    });
    (ctx, app, canvas)
}

/// A single hover frame. No warm-up is needed here specifically: every call
/// site below hovers only after `boot` (or an earlier hover/click) already
/// laid the canvas out at least once, so the "first frame ever" concern
/// `tests/harness/mod.rs` rule 3 documents does not apply — mirrors
/// `tests/it/app/idle_repaint.rs::a_hovered_canvas_keeps_asking_for_frames`,
/// whose first hover is likewise a single frame right after `boot`.
fn hover(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
}

/// A complete primary-button click at `pos`: a warm-up hover (harness rule 3
/// — the widget rect must already be registered before the frame carrying
/// `PointerButton`), then move, press, release in one frame.
fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    hover(ctx, app, pos);
    frame(
        ctx,
        app,
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
        ],
    );
}

/// One wheel-zoom step at `pos`: a hover plus an `Event::MouseWheel` in the
/// same frame. `delta_y` stays under egui's own 8.0-point smoothing
/// threshold (`egui-0.29.1` `input_state/mod.rs`), so the whole delta lands
/// in `smooth_scroll_delta` within this one input pass rather than being
/// spread across several frames of damping.
fn wheel_zoom_at(ctx: &egui::Context, app: &mut App, pos: egui::Pos2, delta_y: f32) {
    frame(
        ctx,
        app,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::Vec2::new(0.0, delta_y),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}

/// LCV-137 AC 4 — wheel zoom keeps the world point under the pointer within
/// 0.5 logical point of its pre-zoom position after reprojection, driven
/// through the real `App::update_ui`.
///
/// Never needs the `CentralPanel`'s origin as a number: `world_before` is
/// read back from `app.last_cursor_world`, which `handle_hover` wrote using
/// the real (unknown-to-this-test) origin; the post-zoom check reprojects
/// that same world point through the *new* camera and compares the result to
/// the pre-zoom local anchor reconstructed from `camera_before` — both are
/// viewport-local, so the shared additive `rect.min` term the real
/// `handle_hover` applied cancels out of the comparison exactly, without this
/// test ever reading it. A pre-LCV-137 build (anchor = global `hover_pos`)
/// fails this by the full magnitude of that origin — tens of points, not
/// sub-point noise.
fn ac4_case(agent_panel_open: bool, label: &str) {
    let mut app = App::default();
    app.agent.panel_open = agent_panel_open;
    let (ctx, mut app, canvas) = boot(app);
    let p = canvas.center();

    hover(&ctx, &mut app, p);
    let camera_before = app.camera.clone();
    let world_before = app
        .last_cursor_world
        .expect("positive control: hovering the canvas center must set last_cursor_world");
    let local_before = camera_before.world_to_screen(world_before);

    wheel_zoom_at(&ctx, &mut app, p, 5.0);
    assert_ne!(
        app.camera.mm_per_px, camera_before.mm_per_px,
        "positive control ({label}): the wheel event must have zoomed the camera"
    );

    let local_after = app.camera.world_to_screen(world_before);
    assert!(
        (local_after.x - local_before.x).abs() < 0.5
            && (local_after.y - local_before.y).abs() < 0.5,
        "AC 4 ({label}): the world point under the cursor drifted from \
         {local_before:?} to {local_after:?} after zooming"
    );
}

#[test]
fn ac4_wheel_zoom_pins_the_world_point_under_the_cursor_default_chrome() {
    ac4_case(false, "default chrome");
}

#[test]
fn ac4_wheel_zoom_pins_the_world_point_under_the_cursor_agent_panel_open() {
    ac4_case(true, "agent panel open");
}

/// A middle-button drag from `from` by six diagonal 6px/4px steps (mirrors
/// `tests/it/app/idle_repaint.rs`'s drag shape, which crosses egui's own
/// drag-start threshold gradually), released cleanly at the end — unlike
/// that file's own drag, which deliberately leaves the button down to probe
/// the `dragged() && !hovered()` idle-repaint edge case. Leaving it down
/// here would misread every later `PointerMoved` (including a plain hover)
/// as a continuing drag, panning the camera again on the very next frame
/// that moves the pointer at all.
fn middle_drag_pan(ctx: &egui::Context, app: &mut App, from: egui::Pos2) {
    hover(ctx, app, from); // warm-up before the frame carrying PointerButton
    frame(
        ctx,
        app,
        vec![egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Middle,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    let mut last = from;
    for step in 1..=6 {
        last = egui::pos2(from.x + step as f32 * 6.0, from.y + step as f32 * 4.0);
        frame(ctx, app, vec![egui::Event::PointerMoved(last)]);
    }
    frame(
        ctx,
        app,
        vec![egui::Event::PointerButton {
            pos: last,
            button: egui::PointerButton::Middle,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}

/// LCV-137 AC 6 — a diagonal middle-drag pan moves a known, committed
/// entity's projected screen position in the *same* direction as the drag on
/// both axes at once, driven through the real `App::update_ui` (real
/// nonzero-origin rect, real drag-threshold detection) rather than
/// `Camera::pan` in isolation, which `src/render/camera.rs` already covers.
/// Step shape mirrors
/// `tests/it/app/idle_repaint.rs::a_middle_drag_keeps_asking_for_frames`.
#[test]
fn ac6_middle_drag_pan_moves_a_known_entity_the_same_direction_as_the_drag() {
    let mut app = App::default();
    app.tool_manager.set_tool(Box::new(LineTool::default()));
    let (ctx, mut app, canvas) = boot(app);

    let p1 = canvas.center();
    click(&ctx, &mut app, p1);
    let p2 = egui::pos2(p1.x + 120.0, p1.y + 80.0);
    click(&ctx, &mut app, p2);
    assert_eq!(
        app.document.entity_count(),
        1,
        "positive control: the setup line must be committed"
    );
    let entity_p1 = match &app.document.entities[0] {
        Entity::Line(l) => l.p1,
        other => panic!("expected a Line entity, got {other:?}"),
    };
    let before = app.camera.world_to_screen(entity_p1);

    middle_drag_pan(&ctx, &mut app, p1);

    let after = app.camera.world_to_screen(entity_p1);
    assert!(
        after.x > before.x,
        "AC 6: dragging right must move the entity right on screen ({before:?} -> {after:?})"
    );
    assert!(
        after.y > before.y,
        "AC 6: dragging down must move the entity down on screen ({before:?} -> {after:?})"
    );
}

/// The status-bar cursor readout and the active snap marker agree with
/// `world_point` when the cursor hovers its current screen position.
fn assert_hover_and_snap_agree(
    ctx: &egui::Context,
    app: &mut App,
    canvas: egui::Rect,
    world_point: Vec2,
    context: &str,
) {
    let screen = app.camera.world_to_screen(world_point) + canvas.min.to_vec2();
    hover(ctx, app, screen);
    let cursor = app
        .last_cursor_world
        .unwrap_or_else(|| panic!("{context}: hovering the canvas must set last_cursor_world"));
    assert!(
        cursor.approx_eq(world_point, 1e-6),
        "{context}: the status-bar cursor readout disagreed: {cursor:?} vs {world_point:?}"
    );
    let snap = app.active_snap.unwrap_or_else(|| {
        panic!("{context}: hovering exactly at a committed endpoint must engage snap")
    });
    assert!(
        snap.point.approx_eq(world_point, 1e-6),
        "{context}: the active snap marker disagreed: {:?} vs {world_point:?}",
        snap.point
    );
}

/// LCV-137 AC 7 — after each navigation gesture (wheel zoom, middle-drag pan,
/// `F` fit extents), the status-bar cursor readout and the active snap
/// marker agree with a committed entity's own world coordinate for the same
/// screen input, and navigation never mutates `Document`/`History` or
/// changes the bytes an SVG export would produce. The next click's
/// committed endpoint agrees with the same world point too.
#[test]
fn ac7_navigation_preserves_the_document_and_keeps_hover_snap_and_click_in_agreement() {
    let mut app = App::default();
    app.tool_manager.set_tool(Box::new(LineTool::default()));
    let (ctx, mut app, canvas) = boot(app);

    let p1 = canvas.center();
    click(&ctx, &mut app, p1);
    let p2 = egui::pos2(p1.x + 120.0, p1.y + 80.0);
    click(&ctx, &mut app, p2);
    assert_eq!(
        app.document.entity_count(),
        1,
        "positive control: setup must commit exactly one line"
    );
    let entity_p1 = match &app.document.entities[0] {
        Entity::Line(l) => l.p1,
        other => panic!("expected a Line entity, got {other:?}"),
    };

    // Navigation gestures are not tool input: switch away from LineTool so
    // the clicks inside `click()` below cannot be misread as a second
    // anchor.
    app.tool_manager.set_tool(Box::new(SelectTool::default()));

    let revision_before = app.history.revision();
    let svg_before = export_svg(&app.document);

    // Gesture 1: wheel zoom.
    wheel_zoom_at(&ctx, &mut app, p1, 5.0);
    assert_eq!(
        app.history.revision(),
        revision_before,
        "AC 7: wheel zoom must not mutate history"
    );
    assert_eq!(
        export_svg(&app.document),
        svg_before,
        "AC 7: wheel zoom must not change the exported SVG"
    );
    assert_hover_and_snap_agree(&ctx, &mut app, canvas, entity_p1, "after wheel zoom");

    // Gesture 2: middle-drag pan.
    middle_drag_pan(&ctx, &mut app, p1);
    assert_eq!(
        app.history.revision(),
        revision_before,
        "AC 7: middle-drag pan must not mutate history"
    );
    assert_eq!(
        export_svg(&app.document),
        svg_before,
        "AC 7: middle-drag pan must not change the exported SVG"
    );
    assert_hover_and_snap_agree(&ctx, &mut app, canvas, entity_p1, "after middle-drag pan");

    // Gesture 3: F fit-extents.
    harness::tap(&ctx, &mut app, egui::Key::F, egui::Modifiers::NONE);
    assert_eq!(
        app.history.revision(),
        revision_before,
        "AC 7: F fit-extents must not mutate history"
    );
    assert_eq!(
        export_svg(&app.document),
        svg_before,
        "AC 7: F fit-extents must not change the exported SVG"
    );
    assert_hover_and_snap_agree(&ctx, &mut app, canvas, entity_p1, "after F fit-extents");

    // The next click's committed endpoint agrees too.
    app.tool_manager.set_tool(Box::new(LineTool::default()));
    let screen_now = app.camera.world_to_screen(entity_p1) + canvas.min.to_vec2();
    click(&ctx, &mut app, screen_now);
    assert_eq!(
        app.tool_manager.anchor(),
        Some(entity_p1),
        "AC 7: the click's anchor must snap to the exact committed endpoint"
    );
    let p3 = egui::pos2(screen_now.x + 90.0, screen_now.y - 50.0);
    click(&ctx, &mut app, p3);
    assert_eq!(
        app.document.entity_count(),
        2,
        "positive control: the second line must commit"
    );
    let second_p1 = match &app.document.entities[1] {
        Entity::Line(l) => l.p1,
        other => panic!("expected a Line entity, got {other:?}"),
    };
    assert!(
        second_p1.approx_eq(entity_p1, 1e-6),
        "AC 7: the committed endpoint {second_p1:?} must match the world point \
         hover/snap agreed on {entity_p1:?}"
    );
}
