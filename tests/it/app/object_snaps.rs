//! LCV-161 AC 2, AC 3, AC 9 — Perpendicular and Tangent snaps from the
//! active tool's anchor, driven through the real `App::update_ui`; LCV-176
//! AC 7, the snap kinds an ellipse offers; LCV-177 AC 9, those of a Bézier.
//!
//! Every size is in screen pixels converted with the live camera, so the
//! aperture (12 px) and the gaps between candidates do not depend on the
//! zoom the first frame settles at. World points are placed relative to the
//! anchor the app itself read back for a known screen position, so the test
//! never needs the canvas origin as a number.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::commands::CreateEntities;
use lasercad::document::{CreateCircle, CreateLine, Entity};
use lasercad::geometry::{Bezier, Circle, Ellipse, EllipseSpan, Line, SnapKind, Vec2};
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
    let canvas = harness::settle(&ctx, &mut app);
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

/// LINE's anchor clicked on an empty canvas, then the entities `make`
/// builds from the point `c` 200 px right of the anchor and one pixel `px`.
fn anchored_scene(make: impl FnOnce(Vec2, f64) -> Vec<Entity>) -> Scene {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(Box::new(LineTool::default()));
    let canvas = harness::settle(&ctx, &mut app);
    let pos = canvas.center();
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos), press(true), press(false)],
    );
    let a = app
        .tool_manager
        .anchor()
        .expect("LINE holds its first point");
    let px = app.camera.mm_per_px;
    let c = a + Vec2::new(200.0 * px, 0.0);
    app.commit(Box::new(CreateEntities::new(make(c, px))));
    Scene {
        ctx,
        app,
        pos,
        a,
        px,
    }
}

/// LCV-176 — an elliptical arc (index 0) 200 px right of LINE's anchor, and
/// a vertical line (index 1) that crosses the arc. Returns the scene and the
/// arc.
fn ellipse_scene() -> (Scene, Ellipse) {
    let mut e = None;
    let s = anchored_scene(|c, px| {
        let span = Some(EllipseSpan::new(-1.0, 2.5, true));
        let el = Ellipse::new(c, 100.0 * px, 40.0 * px, 0.2, span);
        e = Some(el);
        let cross = Line::new(
            c + Vec2::new(-30.0 * px, -200.0 * px),
            c + Vec2::new(-30.0 * px, 200.0 * px),
        );
        vec![Entity::Ellipse(el), Entity::Line(cross)]
    });
    (s, e.expect("built"))
}

/// LCV-177 — an S-shaped cubic (index 0) 200 px right of LINE's anchor, and
/// a vertical line (index 1) that crosses it. Returns the scene and the cubic.
fn bezier_scene() -> (Scene, Bezier) {
    let mut b = None;
    let s = anchored_scene(|c, px| {
        let at = |x: f64, y: f64| c + Vec2::new(x * px, y * px);
        let cubic = Bezier::Cubic([
            at(-100.0, 0.0),
            at(-30.0, 120.0),
            at(30.0, -120.0),
            at(100.0, 0.0),
        ]);
        b = Some(cubic);
        let cross = Line::new(at(-30.0, -200.0), at(-30.0, 200.0));
        vec![Entity::Bezier(cubic), Entity::Line(cross)]
    });
    (s, b.expect("built"))
}

/// LCV-176 AC 7 — near an elliptical arc, Endpoint (span ends), Center,
/// Quadrant (vertices inside the span) and Nearest are offered.
#[test]
fn ellipse_offers_endpoint_center_quadrant_and_nearest() {
    let (mut s, e) = ellipse_scene();
    let off = Vec2::new(2.0 * s.px, -3.0 * s.px);
    let start = e.start_point().expect("arc");
    s.hover(start + off);
    assert_snap(&s, SnapKind::Endpoint, start);
    let end = e.end_point().expect("arc");
    s.hover(end + off);
    assert_snap(&s, SnapKind::Endpoint, end);
    s.hover(e.center + off);
    assert_snap(&s, SnapKind::Center, e.center);
    for t in [0.0, core::f64::consts::FRAC_PI_2] {
        s.hover(e.point(t) + off);
        assert_snap(&s, SnapKind::Quadrant, e.point(t));
    }
    s.hover(e.point(core::f64::consts::PI) + off);
    assert!(
        s.app
            .active_snap
            .is_none_or(|r| r.kind != SnapKind::Quadrant),
        "a vertex outside the span is no Quadrant"
    );
    s.app.settings.object_snaps.nearest = true;
    let q = e.point(1.2) + off;
    s.hover(q);
    // The hover lands on a whole screen point; the foot is of that cursor.
    let cursor = s.app.last_cursor_world.expect("hovering sets the cursor");
    assert_snap(&s, SnapKind::Nearest, e.nearest(cursor));
}

/// LCV-176 AC 7 — along the whole parent ellipse, with an anchor and a
/// crossing line, no Intersection, Midpoint, Perpendicular or Tangent ever
/// comes from the ellipse, with Nearest off and on.
#[test]
fn ellipse_never_offers_other_kinds() {
    let (mut s, e) = ellipse_scene();
    let allowed = [
        SnapKind::Endpoint,
        SnapKind::Center,
        SnapKind::Quadrant,
        SnapKind::Nearest,
    ];
    let mut seen = 0;
    for nearest in [false, true] {
        s.app.settings.object_snaps.nearest = nearest;
        for k in 0..240 {
            let t = f64::from(k) * core::f64::consts::TAU / 240.0;
            s.hover(e.point(t) + Vec2::new(0.5 * s.px, 0.5 * s.px));
            let Some(r) = s.app.active_snap else { continue };
            assert_ne!(r.kind, SnapKind::Intersection, "t={t}: {r:?}");
            if r.primary_idx == 0 {
                seen += 1;
                assert!(allowed.contains(&r.kind), "t={t}: {r:?}");
            }
        }
    }
    assert!(
        seen > 100,
        "positive control: the ellipse snapped {seen} times"
    );
}

/// LCV-177 AC 9 — near a cubic, Endpoint (both ends) and Nearest (on the
/// curve) are offered.
#[test]
fn bezier_offers_endpoint_and_nearest() {
    let (mut s, b) = bezier_scene();
    let off = Vec2::new(2.0 * s.px, -3.0 * s.px);
    s.hover(b.start() + off);
    assert_snap(&s, SnapKind::Endpoint, b.start());
    s.hover(b.end() + off);
    assert_snap(&s, SnapKind::Endpoint, b.end());
    s.app.settings.object_snaps.nearest = true;
    for t in [0.2, 0.7] {
        let q = b.point(t) + off;
        s.hover(q);
        // The cursor reaches the app through f32 screen points, so the foot
        // is compared within 1e-4 px rather than `assert_snap`'s 1e-6.
        let r = s.app.active_snap.expect("a Nearest snap");
        assert_eq!(r.kind, SnapKind::Nearest, "t={t}");
        assert!(
            r.point.approx_eq(b.nearest(q).1, 1e-4 * s.px),
            "t={t}: {r:?}"
        );
        assert!(b.distance_to_point(r.point) <= 1e-9, "t={t}: on the curve");
    }
}

/// LCV-177 AC 9 — along the whole cubic, with an anchor and a crossing
/// line, the Bézier offers only Endpoint and Nearest (with Nearest off and
/// on), and no Intersection is ever found, not even where the line crosses.
#[test]
fn bezier_never_offers_other_kinds() {
    let (mut s, b) = bezier_scene();
    let allowed = [SnapKind::Endpoint, SnapKind::Nearest];
    let mut seen = 0;
    for nearest in [false, true] {
        s.app.settings.object_snaps.nearest = nearest;
        for k in 0..=240 {
            let t = f64::from(k) / 240.0;
            s.hover(b.point(t) + Vec2::new(0.5 * s.px, 0.5 * s.px));
            let Some(r) = s.app.active_snap else { continue };
            assert_ne!(r.kind, SnapKind::Intersection, "t={t}: {r:?}");
            if r.primary_idx == 0 {
                seen += 1;
                assert!(allowed.contains(&r.kind), "t={t}: {r:?}");
            }
        }
    }
    assert!(
        seen > 100,
        "positive control: the cubic snapped {seen} times"
    );
}
