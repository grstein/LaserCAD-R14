//! LCV-176 — editing ellipses through the real `App::update_ui`: picking and
//! box selection (AC 5).
//!
//! Each scene hovers the canvas middle on an empty document to learn the
//! world point `w0` under a known screen position `p`, then places geometry
//! in screen pixels from `w0`, so apertures read the same at any zoom.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::document::commands::CreateEntities;
use lasercad::geometry::{Ellipse, EllipseSpan, Vec2};
use lasercad::tools::{SelectTool, Tool};

struct Scene {
    ctx: egui::Context,
    app: App,
    p: egui::Pos2,
    w0: Vec2,
}

impl Scene {
    /// Boot `tool` and learn `w0` under the canvas middle `p`.
    fn new(tool: Box<dyn Tool>) -> Self {
        let ctx = egui::Context::default();
        let mut app = App::default();
        app.tool_manager.set_tool(tool);
        let mut free = egui::Rect::NOTHING;
        let _ = ctx.run(harness::raw_input(vec![]), |c| {
            app.update_ui(c);
            free = c.available_rect();
        });
        let p = free.center();
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]);
        let w0 = app
            .last_cursor_world
            .expect("hovering the canvas sets the cursor");
        Self { ctx, app, p, w0 }
    }

    /// One screen pixel, in mm.
    fn px(&self) -> f64 {
        self.app.camera.mm_per_px
    }

    /// World point `dx`, `dy` pixels from `w0` (Y up).
    fn w(&self, dx: f64, dy: f64) -> Vec2 {
        self.w0 + Vec2::new(dx * self.px(), dy * self.px())
    }

    fn screen(&self, w: Vec2) -> egui::Pos2 {
        let cam = &self.app.camera;
        self.p + (cam.world_to_screen(w) - cam.world_to_screen(self.w0))
    }

    /// The thin ellipse (rx/ry = 10) of every scene: 150 × 15 px about `w0`,
    /// rotated by 0.3 rad, as a full ellipse or the span `0.2 → 2.0` CCW.
    fn ellipse(&self, arc: bool) -> Ellipse {
        let span = arc.then_some(EllipseSpan::new(0.2, 2.0, true));
        Ellipse::new(self.w0, 150.0 * self.px(), 15.0 * self.px(), 0.3, span)
    }

    fn add(&mut self, e: Ellipse) {
        let cmd = CreateEntities::new(vec![Entity::Ellipse(e)]);
        self.app.commit(Box::new(cmd));
    }

    fn press(&mut self, at: egui::Pos2, pressed: bool) {
        let ev = egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &self.ctx,
            &mut self.app,
            vec![egui::Event::PointerMoved(at), ev],
        );
    }

    /// Hover, then press and release at world point `w`.
    fn click(&mut self, w: Vec2) {
        let at = self.screen(w);
        frame(
            &self.ctx,
            &mut self.app,
            vec![egui::Event::PointerMoved(at)],
        );
        self.press(at, true);
        self.press(at, false);
    }

    /// Drag a box from world `a` to world `b`.
    fn drag(&mut self, a: Vec2, b: Vec2) {
        let (sa, sb) = (self.screen(a), self.screen(b));
        frame(
            &self.ctx,
            &mut self.app,
            vec![egui::Event::PointerMoved(sa)],
        );
        self.press(sa, true);
        frame(
            &self.ctx,
            &mut self.app,
            vec![egui::Event::PointerMoved(sb)],
        );
        self.press(sb, false);
    }

    fn selected(&self) -> Vec<usize> {
        self.app.document.selection.iter().collect()
    }
}

/// Outward unit normal of `e` at parameter `t`.
fn normal(e: &Ellipse, t: f64) -> Vec2 {
    let (s, c) = e.rotation.sin_cos();
    let (x, y) = (e.ry * t.cos(), e.rx * t.sin());
    Vec2::new(x * c - y * s, x * s + y * c)
        .normalize()
        .expect("non-degenerate")
}

/// AC 5 — a click 3 px off the curve selects; a click at the centre, or on
/// the parent ellipse outside an arc's span, does not.
#[test]
fn click_within_the_aperture_selects_an_ellipse() {
    for arc in [false, true] {
        let mut s = Scene::new(Box::new(SelectTool::default()));
        let e = s.ellipse(arc);
        s.add(e);
        for t in [0.3, 1.0, 1.9] {
            s.app.document.selection.set([]);
            s.click(e.point(t) + normal(&e, t) * (3.0 * s.px()));
            assert_eq!(s.selected(), vec![0], "arc={arc} t={t}: 3 px picks");
        }
        s.app.document.selection.set([]);
        s.click(e.center);
        assert!(
            s.selected().is_empty(),
            "arc={arc}: the centre does not pick"
        );
        if arc {
            s.click(e.point(4.0));
            assert!(s.selected().is_empty(), "outside the span does not pick");
        }
    }
}

/// AC 5 — a window box selects an ellipse only when it holds the whole
/// curve; a crossing box selects it when an edge meets the curve, and not
/// when the box sits inside the ellipse without touching it.
#[test]
fn window_and_crossing_boxes_select_like_an_arc() {
    for arc in [false, true] {
        let mut s = Scene::new(Box::new(SelectTool::default()));
        let e = s.ellipse(arc);
        s.add(e);
        let (lo, hi) = e.bbox();
        let m = 4.0 * s.px();
        let (a, b) = (lo - Vec2::new(m, m), hi + Vec2::new(m, m));
        s.drag(Vec2::new(a.x, b.y), Vec2::new(b.x, a.y));
        assert_eq!(s.selected(), vec![0], "arc={arc}: a window around it");

        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(e);
        s.drag(Vec2::new(a.x, b.y), Vec2::new(e.center.x, a.y));
        assert!(s.selected().is_empty(), "arc={arc}: a half window");

        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(e);
        let on = e.point(1.0);
        s.drag(on + Vec2::new(m, m), on - Vec2::new(m, m));
        assert_eq!(
            s.selected(),
            vec![0],
            "arc={arc}: a crossing over the curve"
        );

        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(e);
        s.drag(s.w(4.0, 3.0), s.w(-4.0, -3.0));
        assert!(s.selected().is_empty(), "arc={arc}: a crossing inside only");
    }
}
