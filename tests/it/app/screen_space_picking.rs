//! LCV-162 AC 7, AC 8, AC 9, AC 11 — entity picks and the box-drag threshold
//! are screen points, the same at 0.05 and 20 mm/pt, and no running snap
//! helps an entity pick. Driven through the real `App::update_ui`.
//!
//! Each scene sets the zoom, then hovers the canvas middle on an empty
//! document to learn the world point `w0` under a known screen position `p`.
//! Geometry is placed in screen points from `w0`, so a distance of "4 pt"
//! is 4 × `mm_per_px` millimetres in the world at either zoom.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::{CreateLine, Entity};
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::{ExtendTool, SelectTool, Tool, TrimTool};

/// The two zoom levels every criterion is checked at.
const ZOOMS: [f64; 2] = [0.05, 20.0];

struct Scene {
    ctx: egui::Context,
    app: App,
    p: egui::Pos2,
    w0: Vec2,
}

impl Scene {
    /// World point `dx`, `dy` screen points from `w0`.
    fn w(&self, dx: f64, dy: f64) -> Vec2 {
        let s = self.app.camera.mm_per_px;
        self.w0 + Vec2::new(dx * s, dy * s)
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64)) {
        let l = Line::new(self.w(a.0, a.1), self.w(b.0, b.1));
        self.app.commit(Box::new(CreateLine::new(l)));
    }

    fn hover(&mut self) {
        let p = self.p;
        frame(&self.ctx, &mut self.app, vec![egui::Event::PointerMoved(p)]);
    }

    fn button(&mut self, at: egui::Pos2, pressed: bool) {
        frame(
            &self.ctx,
            &mut self.app,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }

    /// Warm-up hover, then press and release at `p` in one frame.
    fn click(&mut self) {
        self.hover();
        let p = self.p;
        let press = |pressed| egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            &self.ctx,
            &mut self.app,
            vec![egui::Event::PointerMoved(p), press(true), press(false)],
        );
    }

    fn entities(&self) -> Vec<Entity> {
        self.app.document.entities.clone()
    }
}

/// Boot `tool` at `mm_per_px` with snap on, and learn `w0` under `p`.
fn scene(tool: Box<dyn Tool>, mm_per_px: f64) -> Scene {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(tool);
    let mut free = egui::Rect::NOTHING;
    let _ = ctx.run(harness::raw_input(vec![]), |c| {
        app.update_ui(c);
        free = c.available_rect();
    });
    app.camera.mm_per_px = mm_per_px;
    app.snap_enabled = true;
    let p = free.center();
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]);
    let w0 = app
        .last_cursor_world
        .expect("positive control: hovering the canvas sets the cursor");
    assert_eq!(app.camera.mm_per_px, mm_per_px, "setup: zoom unchanged");
    Scene { ctx, app, p, w0 }
}

fn selected(s: &Scene) -> Vec<usize> {
    s.app.document.selection.iter().collect()
}

/// AC 7, AC 11 — Select picks a line 4 pt away, not one 6 pt away, and not
/// one whose endpoint is 8 pt away although that endpoint is inside the
/// 12 pt snap aperture: no running snap while an entity pick is pending.
#[test]
fn select_pick_aperture_is_five_points_at_both_zooms() {
    for zoom in ZOOMS {
        let mut s = scene(Box::new(SelectTool::default()), zoom);
        s.line((-100.0, -4.0), (100.0, -4.0));
        s.click();
        assert_eq!(selected(&s), vec![0], "AC 7 @{zoom}: 4 pt picks");

        let mut s = scene(Box::new(SelectTool::default()), zoom);
        s.line((-100.0, -6.0), (100.0, -6.0));
        s.click();
        assert!(selected(&s).is_empty(), "AC 7 @{zoom}: 6 pt does not pick");

        let mut s = scene(Box::new(SelectTool::default()), zoom);
        s.line((8.0, 0.0), (100.0, 0.0));
        s.hover();
        assert!(s.app.active_snap.is_none(), "AC 11 @{zoom}: no snap");
        s.click();
        assert!(
            selected(&s).is_empty(),
            "AC 11 @{zoom}: an endpoint 8 pt away does not pick through a snap"
        );
    }
}

/// AC 8, AC 11 — TRIM changes a line 4 pt away at a cutter, and leaves the
/// document alone at 6 pt and at an endpoint 8 pt away.
#[test]
fn trim_pick_aperture_is_five_points_at_both_zooms() {
    for zoom in ZOOMS {
        for (target, trims) in [
            (((-100.0, -4.0), (100.0, -4.0)), true),
            (((-100.0, -6.0), (100.0, -6.0)), false),
            (((8.0, 0.0), (100.0, 0.0)), false),
        ] {
            let mut s = scene(Box::new(TrimTool::default()), zoom);
            s.line(target.0, target.1);
            s.line((50.0, -50.0), (50.0, 50.0));
            let before = s.entities();
            s.click();
            assert!(s.app.active_snap.is_none(), "AC 11 @{zoom}: no snap");
            assert_eq!(
                s.entities() != before,
                trims,
                "AC 8 @{zoom}: target {target:?} trims = {trims}"
            );
        }
    }
}

/// AC 8, AC 11 — EXTEND previews a line whose endpoint is 4 pt away, and
/// nothing at 6 pt or 8 pt (the latter inside the snap aperture).
#[test]
fn extend_pick_aperture_is_five_points_at_both_zooms() {
    for zoom in ZOOMS {
        for (end, previews) in [(-4.0, true), (-6.0, false), (-8.0, false)] {
            let mut s = scene(Box::new(ExtendTool::default()), zoom);
            s.line((-100.0, 0.0), (end, 0.0));
            s.line((50.0, -50.0), (50.0, 50.0));
            s.hover();
            assert!(s.app.active_snap.is_none(), "AC 11 @{zoom}: no snap");
            assert_eq!(
                !s.app.tool_manager.preview().is_empty(),
                previews,
                "AC 8 @{zoom}: endpoint {end} pt away previews = {previews}"
            );
        }
    }
}

/// AC 9 — a Select press that moves 1.5 pt is still a click; 3 pt starts a
/// box drag (the four-edge box preview).
#[test]
fn select_drag_threshold_is_two_points_at_both_zooms() {
    for zoom in ZOOMS {
        for (dx, drags) in [(1.5_f32, false), (3.0, true)] {
            let mut s = scene(Box::new(SelectTool::default()), zoom);
            let p = s.p;
            s.hover();
            s.button(p, true);
            let to = p + egui::vec2(dx, 0.0);
            frame(&s.ctx, &mut s.app, vec![egui::Event::PointerMoved(to)]);
            assert_eq!(
                s.app.tool_manager.preview().len() == 4,
                drags,
                "AC 9 @{zoom}: a {dx} pt move starts a box = {drags}"
            );
            s.button(to, false);
        }
    }
}
