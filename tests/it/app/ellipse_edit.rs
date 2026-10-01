//! LCV-176 — editing ellipses through the real `App::update_ui`: picking and
//! box selection (AC 5), the modify tools typed on the command line (AC 6),
//! and TRIM/EXTEND leaving ellipses alone (AC 8).
//!
//! Each scene hovers the canvas middle on an empty document to learn the
//! world point `w0` under a known screen position `p`, then places geometry
//! in screen pixels from `w0`, so apertures read the same at any zoom.

use crate::harness;

use harness::{frame, submit_command, tap};
use lasercad::app::App;
use lasercad::document::Entity;
use lasercad::document::commands::CreateEntities;
use lasercad::geometry::{Ellipse, EllipseSpan, Line, Vec2};
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

    /// The thin ellipse (rx/ry = 10) of every scene: 150 × 15 px about `w0`,
    /// rotated by 0.3 rad, as a full ellipse or the span `0.2 → 2.0` CCW.
    fn ellipse(&self, arc: bool) -> Ellipse {
        let span = arc.then_some(EllipseSpan::new(0.2, 2.0, true));
        Ellipse::new(self.w0, 150.0 * self.px(), 15.0 * self.px(), 0.3, span)
    }

    fn add(&mut self, e: Ellipse) {
        self.add_all(vec![Entity::Ellipse(e)]);
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

/// The fixed arc of the AC 6 scenes: centre (10, 20), 30 × 5 mm, rotated by
/// 0.3 rad, span `0.2 → 2.0` CCW.
fn fixed() -> Ellipse {
    Ellipse::new(
        Vec2::new(10.0, 20.0),
        30.0,
        5.0,
        0.3,
        Some(EllipseSpan::new(0.2, 2.0, true)),
    )
}

fn assert_ellipse(actual: &Entity, want: Ellipse, what: &str) {
    let Entity::Ellipse(got) = actual else {
        panic!("{what}: expected an ellipse, got {actual:?}");
    };
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9;
    let spans = match (got.span, want.span) {
        (Some(g), Some(w)) => close(g.start, w.start) && close(g.end, w.end) && g.ccw == w.ccw,
        (g, w) => g == w,
    };
    assert!(
        got.center.approx_eq(want.center, 1e-9)
            && close(got.rx, want.rx)
            && close(got.ry, want.ry)
            && close(got.rotation, want.rotation)
            && spans,
        "{what}: got {got:?}, want {want:?}"
    );
}

/// AC 6 — each modify tool, typed on the command line with the arc
/// selected, transforms it exactly in one undo step, and Ctrl+Z restores it.
#[test]
fn modify_tools_transform_an_ellipse_exactly_in_one_step() {
    let e = fixed();
    let span = |start, end, ccw| Some(EllipseSpan::new(start, end, ccw));
    let moved = Ellipse {
        center: Vec2::new(15.0, 27.0),
        ..e
    };
    let rotated = Ellipse {
        center: Vec2::new(-20.0, 10.0),
        rotation: 0.3 + core::f64::consts::FRAC_PI_2,
        ..e
    };
    // Mirrored about the X axis: rotation negated, span negated and reversed.
    let mirrored = Ellipse {
        center: Vec2::new(10.0, -20.0),
        rotation: -0.3,
        span: span(-0.2, -2.0, false),
        ..e
    };
    let scaled = Ellipse::new(Vec2::new(20.0, 40.0), 60.0, 10.0, 0.3, e.span);
    let cases: [(&str, &[&str], Vec<Ellipse>); 5] = [
        ("MOVE", &["m", "0,0", "5,7"], vec![moved]),
        ("COPY", &["co", "0,0", "5,7"], vec![e, moved]),
        ("ROTATE", &["ro", "0,0", "90"], vec![rotated]),
        ("MIRROR", &["mi", "0,0", "1,0", "y"], vec![mirrored]),
        ("SCALE", &["sc", "0,0", "2"], vec![scaled]),
    ];
    for (name, typed, want) in cases {
        let mut s = Scene::new(Box::new(SelectTool::default()));
        s.add(e);
        let depth = s.app.history.len();
        s.app.document.selection.set([0]);
        for t in typed {
            submit_command(&s.ctx, &mut s.app, t);
        }
        let got = &s.app.document.entities;
        assert_eq!(got.len(), want.len(), "{name}: entity count");
        for (g, w) in got.iter().zip(&want) {
            assert_ellipse(g, *w, name);
        }
        assert_eq!(s.app.history.len(), depth + 1, "{name}: one undo step");
        tap(&s.ctx, &mut s.app, egui::Key::Z, ctrl());
        assert_eq!(s.app.document.entities.len(), 1, "{name}: undone");
        assert_ellipse(&s.app.document.entities[0], e, &format!("{name} undone"));
    }
}

fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..egui::Modifiers::NONE
    }
}

const REFUSAL: &str = "Cannot trim/extend an ellipse";

/// AC 8 — a TRIM or EXTEND click on an ellipse or elliptical arc (on its
/// curve, or at an arc end) changes nothing, adds no undo step and says why.
#[test]
fn trim_and_extend_aimed_at_an_ellipse_refuse() {
    for arc in [false, true] {
        let tools: [fn() -> Box<dyn Tool>; 2] = [
            || Box::new(TrimTool::default()),
            || Box::new(ExtendTool::default()),
        ];
        for tool in tools {
            let mut s = Scene::new(tool());
            let e = s.ellipse(arc);
            let crossing = s.line((-300.0, 0.0), (300.0, 0.0));
            s.add_all(vec![Entity::Ellipse(e), crossing]);
            let name = s.app.tool_manager.active_tool_name();
            let mut aims = vec![e.point(1.0)];
            aims.extend(e.end_point());
            for at in aims {
                let depth = s.app.history.len();
                s.app.command_feedback.clear();
                s.click(at);
                assert_eq!(
                    s.app.document.entities,
                    vec![Entity::Ellipse(e), crossing],
                    "{name} arc={arc} at {at:?}: unchanged"
                );
                assert_eq!(s.app.history.len(), depth, "{name}: no undo step");
                assert_eq!(s.app.command_feedback, REFUSAL, "{name} arc={arc}");
            }
        }
    }
}

/// AC 8 — an ellipse is no cutter for TRIM and no boundary for EXTEND: a
/// line through it is cut, and grown, only at a line beyond it.
#[test]
fn an_ellipse_is_no_cutter_and_no_boundary() {
    for arc in [false, true] {
        let mut s = Scene::new(Box::new(TrimTool::default()));
        let e = s.ellipse(arc);
        let wall = s.line((250.0, -50.0), (250.0, 50.0));
        let through = s.line((-300.0, 0.0), (300.0, 0.0));
        s.add_all(vec![Entity::Ellipse(e), through, wall]);
        // TRIM keeps the clicked side: everything left of the wall.
        s.click(s.w(-280.0, 0.0));
        let trimmed = s.line((-300.0, 0.0), (250.0, 0.0));
        let Entity::Line(got) = s.app.document.entities[1] else {
            panic!("the trimmed line stays a line");
        };
        let Entity::Line(want) = trimmed else {
            unreachable!()
        };
        assert!(
            got.p1.approx_eq(want.p1, 1e-9) && got.p2.approx_eq(want.p2, 1e-9),
            "arc={arc}: TRIM cuts at the wall only: {got:?}"
        );

        let mut s = Scene::new(Box::new(ExtendTool::default()));
        let e = s.ellipse(arc);
        let wall = s.line((250.0, -50.0), (250.0, 50.0));
        let short = s.line((-300.0, 0.0), (-200.0, 0.0));
        s.add_all(vec![Entity::Ellipse(e), short, wall]);
        s.click(s.w(-202.0, 0.0));
        let Entity::Line(got) = s.app.document.entities[1] else {
            panic!("the extended line stays a line");
        };
        assert!(
            got.p2.approx_eq(s.w(250.0, 0.0), 1e-9),
            "arc={arc}: EXTEND grows to the wall: {got:?}"
        );
        assert_ne!(s.app.command_feedback, REFUSAL, "arc={arc}");
    }
}
