//! LCV-161 AC 2, AC 3, AC 9 — Perpendicular and Tangent snaps from the
//! active tool's anchor, driven through the real `App::update_ui`.
//!
//! Every size is in screen pixels converted with the live camera, so the
//! aperture (12 px) and the gaps between candidates do not depend on the
//! zoom the first frame settles at. World points are placed relative to the
//! anchor the app itself read back for a known screen position, so the test
//! never needs the canvas origin as a number.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::{CreateCircle, CreateLine};
use lasercad::geometry::{Circle, Line, SnapKind, Vec2};
use lasercad::tools::LineTool;

/// LINE with its first point placed: the anchor `a` clicked at screen
/// position `pos`, and one pixel in mm `px`.
struct Scene {
    ctx: egui::Context,
    app: App,
    pos: egui::Pos2,
    a: Vec2,
    px: f64,
}

impl Scene {
    /// Screen position of `w`, offset from the known pair (`pos`, `a`).
    fn to_screen(&self, w: Vec2) -> egui::Pos2 {
        let cam = &self.app.camera;
        self.pos + (cam.world_to_screen(w) - cam.world_to_screen(self.a))
    }

    fn hover(&mut self, w: Vec2) {
        let pos = self.to_screen(w);
        frame(
            &self.ctx,
            &mut self.app,
            vec![egui::Event::PointerMoved(pos)],
        );
    }

    /// Circle 200 px right of the anchor, radius 100 px: tangent points at
    /// ±120° from its centre, 52 px from the nearest quadrant.
    fn tangent_point(&self) -> Vec2 {
        let c = self.a + Vec2::new(200.0 * self.px, 0.0);
        let r = 100.0 * self.px;
        c + Vec2::new(-0.5 * r, 3.0_f64.sqrt() / 2.0 * r)
    }

    /// Horizontal segment 150 px below the anchor: the foot sits 50 px from
    /// its midpoint and 50 px from its right end.
    fn perpendicular_foot(&self) -> Vec2 {
        Vec2::new(self.a.x, self.a.y - 150.0 * self.px)
    }
}

/// Boot, hover the canvas centre to learn its world point, commit the
/// circle and the segment around it, then click LINE's first point there.
fn scene() -> Scene {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(Box::new(LineTool::default()));
    let mut canvas = egui::Rect::NOTHING;
    let _ = ctx.run(harness::raw_input(vec![]), |c| {
        app.update_ui(c);
        canvas = c.available_rect();
    });
    let pos = canvas.center();
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    let a0 = app
        .last_cursor_world
        .expect("positive control: hovering the canvas sets the cursor");
    let px = app.camera.mm_per_px;
    let c = a0 + Vec2::new(200.0 * px, 0.0);
    app.commit(Box::new(CreateCircle::new(Circle::new(c, 100.0 * px))));
    let y = a0.y - 150.0 * px;
    let seg = Line::new(
        Vec2::new(a0.x - 150.0 * px, y),
        Vec2::new(a0.x + 50.0 * px, y),
    );
    app.commit(Box::new(CreateLine::new(seg)));

    frame(
        &ctx,
        &mut app,
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
    let a = app
        .tool_manager
        .anchor()
        .expect("positive control: LINE must hold its first point as the anchor");
    assert!(
        a.approx_eq(a0, 1e-6 * px.max(1.0)),
        "setup: no snap may move the first point: {a:?} vs {a0:?}"
    );
    Scene {
        ctx,
        app,
        pos,
        a,
        px,
    }
}

fn assert_snap(s: &Scene, kind: SnapKind, expected: Vec2) {
    let snap = s
        .app
        .active_snap
        .unwrap_or_else(|| panic!("expected a {kind:?} snap"));
    assert_eq!(snap.kind, kind);
    assert!(
        snap.point.approx_eq(expected, 1e-6 * s.px.max(1.0)),
        "{kind:?} at {:?}, expected {expected:?}",
        snap.point
    );
}

/// AC 3 — hovering near the tangent point from LINE's anchor snaps Tangent.
#[test]
fn line_first_point_hover_near_tangent_point_snaps_tangent() {
    let mut s = scene();
    let t = s.tangent_point();
    s.hover(t + Vec2::new(3.0 * s.px, -2.0 * s.px));
    assert_snap(&s, SnapKind::Tangent, t);
}

/// AC 2 — hovering near the foot on the segment snaps Perpendicular.
#[test]
fn line_first_point_hover_near_foot_snaps_perpendicular() {
    let mut s = scene();
    let f = s.perpendicular_foot();
    s.hover(f + Vec2::new(-2.0 * s.px, 3.0 * s.px));
    assert_snap(&s, SnapKind::Perpendicular, f);
}

/// AC 9 — with the kind off in `settings.object_snaps` no candidate of it.
#[test]
fn a_disabled_kind_gives_no_snap() {
    let mut s = scene();
    s.app.settings.object_snaps.tangent = false;
    s.app.settings.object_snaps.perpendicular = false;
    let t = s.tangent_point();
    s.hover(t + Vec2::new(3.0 * s.px, -2.0 * s.px));
    assert!(
        s.app
            .active_snap
            .is_none_or(|r| r.kind != SnapKind::Tangent),
        "Tangent off must give no Tangent snap"
    );
    let f = s.perpendicular_foot();
    s.hover(f + Vec2::new(-2.0 * s.px, 3.0 * s.px));
    assert!(
        s.app
            .active_snap
            .is_none_or(|r| r.kind != SnapKind::Perpendicular),
        "Perpendicular off must give no Perpendicular snap"
    );
}

/// AC 9 — F3 stays the master switch: snap off gives no snap at all.
#[test]
fn f3_off_gives_no_snap_at_all() {
    let mut s = scene();
    s.app.snap_enabled = false;
    let t = s.tangent_point();
    s.hover(t);
    assert!(s.app.active_snap.is_none());
}
