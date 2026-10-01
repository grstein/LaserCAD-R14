//! LCV-177 — editing Béziers through the real `App::update_ui`: picking, box
//! selection and ZOOM Extents on the tight extent (AC 6, AC 7), the modify
//! tools typed on the command line (AC 8), and TRIM/EXTEND leaving Béziers
//! alone (AC 10).
//!
//! Each scene hovers the canvas middle on an empty document to learn the
//! world point `w0` under a known screen position `p`, then places geometry
//! in screen pixels from `w0`, so apertures read the same at any zoom.

use crate::harness;

use harness::{frame, submit_command, tap};
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::document::commands::CreateEntities;
use lasercad::geometry::{Bezier, Line, Transform, Vec2};
use lasercad::tools::{ExtendTool, SelectTool, Tool, TrimTool};

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
        let free = harness::settle(&ctx, &mut app);
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

    /// The S-curve of every scene, 300 px wide about `w0`: its controls
    /// reach ±200 px, the curve itself only about ±58 px.
    fn cubic(&self) -> Bezier {
        Bezier::Cubic([
            self.w(-150.0, 0.0),
            self.w(-50.0, 200.0),
            self.w(50.0, -200.0),
            self.w(150.0, 0.0),
        ])
    }

    /// A quadratic hump, 200 px wide: its control reaches 200 px, the
    /// curve 100 px.
    fn quadratic(&self) -> Bezier {
        Bezier::Quadratic([self.w(-100.0, 0.0), self.w(0.0, 200.0), self.w(100.0, 0.0)])
    }

    fn add(&mut self, b: Bezier) {
        self.add_all(vec![Entity::Bezier(b)]);
    }

    fn add_all(&mut self, entities: Vec<Entity>) {
        self.app.commit(Box::new(CreateEntities::new(entities)));
    }

    /// The line from `a` to `b`, both in pixels from `w0`.
    fn line(&self, a: (f64, f64), b: (f64, f64)) -> Entity {
        Entity::Line(Line::new(self.w(a.0, a.1), self.w(b.0, b.1)))
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

/// Unit normal of `b` at parameter `t`.
fn normal(b: &Bezier, t: f64) -> Vec2 {
    let d = b.point(t + 1e-6) - b.point(t - 1e-6);
    Vec2::new(-d.y, d.x).normalize().expect("non-degenerate")
}

/// AC 7 — a click 3 px off the curve selects; a click on an off-curve
/// control point does not.
#[test]
fn click_within_the_aperture_selects_a_bezier() {
    let s0 = Scene::new(Box::new(SelectTool::default()));
    for b in [s0.cubic(), s0.quadratic()] {
        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(b);
        for t in [0.1, 0.3, 0.5, 0.8] {
            s.app.document.selection.set([]);
            s.click(b.point(t) + normal(&b, t) * (3.0 * s.px()));
            assert_eq!(s.selected(), vec![0], "{b:?} t={t}: 3 px picks");
        }
        for c in &b.points()[1..b.points().len() - 1] {
            s.app.document.selection.set([]);
            s.click(*c);
            assert!(s.selected().is_empty(), "control {c:?} does not pick");
        }
    }
}

/// AC 6, AC 7 — a window box tight around the curve (well inside its
/// control polygon) selects it, a half window does not; a crossing box over
/// the curve selects it, one over only the control polygon does not.
#[test]
fn window_and_crossing_boxes_use_the_curve() {
    let s0 = Scene::new(Box::new(SelectTool::default()));
    for b in [s0.cubic(), s0.quadratic()] {
        let (lo, hi) = b.bbox();
        let m = 4.0 * s0.px();
        let (a, c) = (lo - Vec2::new(m, m), hi + Vec2::new(m, m));
        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(b);
        s.drag(Vec2::new(a.x, c.y), Vec2::new(c.x, a.y));
        assert_eq!(s.selected(), vec![0], "a window around the curve");

        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(b);
        s.drag(Vec2::new(a.x, c.y), Vec2::new(s.w0.x, a.y));
        assert!(s.selected().is_empty(), "a half window");

        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(b);
        let on = b.point(0.3);
        s.drag(on + Vec2::new(m, m), on - Vec2::new(m, m));
        assert_eq!(s.selected(), vec![0], "a crossing over the curve");

        // Over the control point(s) only: above the curve, below the polygon.
        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(b);
        s.drag(
            Vec2::new(lo.x, s.w(0.0, 190.0).y),
            Vec2::new(hi.x, hi.y + m),
        );
        assert!(s.selected().is_empty(), "a crossing over the polygon only");
    }
}

/// AC 6 — ZOOM Extents centres and fits the curve's tight box, not the
/// control polygon's.
#[test]
fn zoom_extents_frames_the_tight_box() {
    let mut s = Scene::new(Box::new(SelectTool::default()));
    let b = s.quadratic();
    s.add(b);
    submit_command(&s.ctx, &mut s.app, "ze");
    let (lo, hi) = b.bbox();
    let cam = &s.app.camera;
    assert!(
        cam.center_world.approx_eq((lo + hi) * 0.5, 1e-9),
        "{:?} vs {lo:?} {hi:?}",
        cam.center_world
    );
    let [vw, vh] = cam.viewport_size_px.map(f64::from);
    let want = ((hi.x - lo.x) / vw).max((hi.y - lo.y) / vh) / 0.8;
    assert!(
        (cam.mm_per_px - want).abs() <= 1e-12,
        "{} vs {want}",
        cam.mm_per_px
    );
}

/// The fixed curves of the AC 8 scenes, in mm.
fn fixed() -> [Bezier; 2] {
    let v = Vec2::new;
    [
        Bezier::Cubic([v(10.0, 20.0), v(15.0, 40.0), v(30.0, 0.0), v(40.0, 25.0)]),
        Bezier::Quadratic([v(-5.0, 3.0), v(8.0, 30.0), v(20.0, 2.0)]),
    ]
}

fn assert_bezier(actual: &Entity, want: Bezier, what: &str) {
    let Entity::Bezier(got) = actual else {
        panic!("{what}: expected a Bézier, got {actual:?}");
    };
    let same = got.points().len() == want.points().len()
        && got
            .points()
            .iter()
            .zip(want.points())
            .all(|(g, w)| g.approx_eq(*w, 1e-9));
    assert!(same, "{what}: got {got:?}, want {want:?}");
}

fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

/// AC 8 — each modify tool, typed on the command line with the curve
/// selected, maps every control point through `Transform::point` in one
/// undo step, and Ctrl+Z restores it.
#[test]
fn modify_tools_transform_a_bezier_exactly_in_one_step() {
    let origin = Vec2::default();
    for b in fixed() {
        let moved = b.map(|p| p + Vec2::new(5.0, 7.0));
        let rotate = Transform::Rotate {
            base: origin,
            angle: core::f64::consts::FRAC_PI_2,
        };
        let mirror = Transform::Mirror {
            a: origin,
            b: Vec2::new(1.0, 0.0),
        };
        let scale = Transform::Scale {
            base: origin,
            factor: 2.0,
        };
        let cases: [(&str, &[&str], Vec<Bezier>); 5] = [
            ("MOVE", &["m", "0,0", "5,7"], vec![moved]),
            ("COPY", &["co", "0,0", "5,7"], vec![b, moved]),
            ("ROTATE", &["ro", "0,0", "90"], vec![rotate.bezier(b)]),
            ("MIRROR", &["mi", "0,0", "1,0", "y"], vec![mirror.bezier(b)]),
            ("SCALE", &["sc", "0,0", "2"], vec![scale.bezier(b)]),
        ];
        for (name, typed, want) in cases {
            let mut s = Scene::new(Box::new(SelectTool::default()));
            s.add(b);
            let depth = s.app.history.len();
            s.app.document.selection.set([0]);
            for t in typed {
                submit_command(&s.ctx, &mut s.app, t);
            }
            let got = &s.app.document.entities;
            assert_eq!(got.len(), want.len(), "{name}: entity count");
            for (g, w) in got.iter().zip(&want) {
                assert_bezier(g, *w, name);
            }
            assert_eq!(s.app.history.len(), depth + 1, "{name}: one undo step");
            tap(&s.ctx, &mut s.app, egui::Key::Z, ctrl());
            assert_eq!(
                s.app.document.entities,
                vec![Entity::Bezier(b)],
                "{name}: undone"
            );
        }
    }
}

const REFUSAL: &str = "Cannot trim/extend a curve";

/// AC 10 — a TRIM or EXTEND click on a Bézier (mid-curve or at an end)
/// changes nothing, adds no undo step and says why.
#[test]
fn trim_and_extend_aimed_at_a_bezier_refuse() {
    let tools: [fn() -> Box<dyn Tool>; 2] = [
        || Box::new(TrimTool::default()),
        || Box::new(ExtendTool::default()),
    ];
    for tool in tools {
        let s0 = Scene::new(tool());
        for b in [s0.cubic(), s0.quadratic()] {
            let mut s = Scene::new(tool());
            let crossing = s.line((-300.0, 20.0), (300.0, 20.0));
            s.add_all(vec![Entity::Bezier(b), crossing]);
            let name = s.app.tool_manager.active_tool_name();
            for at in [b.point(0.3), b.start(), b.end()] {
                let depth = s.app.history.len();
                s.app.command_feedback.clear();
                s.click(at);
                assert_eq!(
                    s.app.document.entities,
                    vec![Entity::Bezier(b), crossing],
                    "{name} at {at:?}: unchanged"
                );
                assert_eq!(s.app.history.len(), depth, "{name}: no undo step");
                assert_eq!(s.app.command_feedback, REFUSAL, "{name} at {at:?}");
            }
        }
    }
}

/// AC 10 — a Bézier is no cutter for TRIM and no boundary for EXTEND: a
/// line through it is cut, and grown, only at a line beyond it.
#[test]
fn a_bezier_is_no_cutter_and_no_boundary() {
    let s0 = Scene::new(Box::new(TrimTool::default()));
    for b in [s0.cubic(), s0.quadratic()] {
        let mut s = Scene::new(Box::new(TrimTool::default()));
        let wall = s.line((250.0, -50.0), (250.0, 50.0));
        let through = s.line((-300.0, 20.0), (300.0, 20.0));
        s.add_all(vec![Entity::Bezier(b), through, wall]);
        // TRIM keeps the clicked side: everything left of the wall.
        s.click(s.w(-280.0, 20.0));
        let Entity::Line(got) = s.app.document.entities[1] else {
            panic!("the trimmed line stays a line");
        };
        assert!(
            got.p1.approx_eq(s.w(-300.0, 20.0), 1e-9) && got.p2.approx_eq(s.w(250.0, 20.0), 1e-9),
            "{b:?}: TRIM cuts at the wall only: {got:?}"
        );

        let mut s = Scene::new(Box::new(ExtendTool::default()));
        let wall = s.line((250.0, -50.0), (250.0, 50.0));
        let short = s.line((-300.0, 20.0), (-200.0, 20.0));
        s.add_all(vec![Entity::Bezier(b), short, wall]);
        s.click(s.w(-202.0, 20.0));
        let Entity::Line(got) = s.app.document.entities[1] else {
            panic!("the extended line stays a line");
        };
        assert!(
            got.p2.approx_eq(s.w(250.0, 20.0), 1e-9),
            "{b:?}: EXTEND grows to the wall: {got:?}"
        );
        assert_ne!(s.app.command_feedback, REFUSAL);
    }
}
