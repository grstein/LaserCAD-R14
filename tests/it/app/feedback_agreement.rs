//! LCV-163 AC 6 — the click after a hover acts on exactly the highlighted
//! entity and removes exactly what was marked `Danger` (preview and result
//! agree). Driven through the real `App::update_ui`: the marks are read from
//! `ToolManager::feedback` at the frame's resolved cursor, then the same
//! point is clicked.
//!
//! The pointer stays at the canvas middle `p`; geometry is placed around the
//! world point `w0` under it, in screen points.

use crate::harness;

use harness::frame;
use lasercad::app::App;
use lasercad::document::{CreateArc, CreateCircle, CreateLine, Entity, SelectionCommand};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::tools::{DeleteTool, Mark, SelectTool, Tool, TrimTool};

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

    /// Hover `p` and return this frame's feedback marks.
    fn marks(&mut self) -> Vec<Mark> {
        let p = self.p;
        frame(&self.ctx, &mut self.app, vec![egui::Event::PointerMoved(p)]);
        let cursor = self.app.last_cursor_world;
        assert!(cursor.is_some(), "positive control: the canvas is hovered");
        self.app.tool_manager.feedback(&self.app.document, cursor)
    }

    /// Press and release at `p` in one frame (after the hover in `marks`).
    fn click(&mut self) {
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
}

/// Boot `tool` and learn `w0` under the canvas middle `p`.
fn scene(tool: Box<dyn Tool>) -> Scene {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(tool);
    let mut free = egui::Rect::NOTHING;
    let _ = ctx.run_ui(harness::raw_input(vec![]), |ui| {
        let c = &ui.ctx().clone();
        app.update_ui(ui);
        free = crate::harness::canvas_rect(c);
    });
    let p = free.center();
    frame(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]);
    let w0 = app
        .last_cursor_world
        .expect("positive control: hovering the canvas sets the cursor");
    Scene { ctx, app, p, w0 }
}

fn hovers(marks: &[Mark]) -> Vec<usize> {
    marks
        .iter()
        .filter_map(|m| match m {
            Mark::Hover(i) => Some(*i),
            _ => None,
        })
        .collect()
}

fn dangers(marks: &[Mark]) -> Vec<Entity> {
    marks
        .iter()
        .filter_map(|m| match m {
            Mark::Danger(e) => Some(*e),
            _ => None,
        })
        .collect()
}

const TOL: f64 = 1e-6;

fn same_point(a: Vec2, b: Vec2) -> bool {
    a.approx_eq(b, TOL)
}

/// `a` and `b` are the same entity within [`TOL`] (arcs by their ends and
/// sweep, so an angle stored as −π or π compares equal).
fn same(a: &Entity, b: &Entity) -> bool {
    match (a, b) {
        (Entity::Line(x), Entity::Line(y)) => same_point(x.p1, y.p1) && same_point(x.p2, y.p2),
        (Entity::Arc(x), Entity::Arc(y)) => {
            same_point(x.center, y.center)
                && (x.r - y.r).abs() < TOL
                && x.ccw == y.ccw
                && same_point(x.start_point(), y.start_point())
                && same_point(x.end_point(), y.end_point())
                && (x.sweep_angle() - y.sweep_angle()).abs() < TOL
        }
        (Entity::Circle(x), Entity::Circle(y)) => {
            same_point(x.center, y.center) && (x.r - y.r).abs() < TOL
        }
        _ => false,
    }
}

fn same_set(a: &[Entity], b: &[Entity]) -> bool {
    a.len() == b.len() && a.iter().all(|x| b.iter().any(|y| same(x, y)))
}

/// AC 6 — Select: the click selects exactly the hovered index, here the
/// nearer of two lines 3 pt and 4 pt from the pointer.
#[test]
fn select_click_picks_the_hovered_entity() {
    let mut s = scene(Box::new(SelectTool::default()));
    s.line((-100.0, 4.0), (100.0, 4.0));
    s.line((-100.0, -3.0), (100.0, -3.0));
    let hovered = hovers(&s.marks());
    assert_eq!(hovered, vec![1], "positive control: one hover");
    s.click();
    let selected: Vec<usize> = s.app.document.selection.iter().collect();
    assert_eq!(
        selected, hovered,
        "AC 6: the click selects the hovered entity"
    );
}

/// What `before` lost to become `after`: the part of the original not kept,
/// restated from the geometry rather than from the product helper.
fn lost(before: &Entity, after: &Entity) -> Vec<Entity> {
    let pieces = match (before, after) {
        (Entity::Line(b), Entity::Line(a)) => vec![
            Entity::Line(Line::new(b.p1, a.p1)),
            Entity::Line(Line::new(a.p2, b.p2)),
        ],
        (Entity::Arc(b), Entity::Arc(a)) => vec![
            Entity::Arc(Arc::new(b.center, b.r, b.start_angle, a.start_angle, b.ccw)),
            Entity::Arc(Arc::new(b.center, b.r, a.end_angle, b.end_angle, b.ccw)),
        ],
        (Entity::Circle(b), Entity::Arc(a)) => vec![Entity::Arc(Arc::new(
            b.center,
            b.r,
            a.end_angle,
            a.start_angle,
            a.ccw,
        ))],
        _ => panic!("unexpected trim {before:?} -> {after:?}"),
    };
    pieces
        .into_iter()
        .filter(|p| match p {
            Entity::Line(l) => l.length() > TOL,
            Entity::Arc(a) => a.arc_length() > TOL,
            Entity::Circle(_) | Entity::Ellipse(_) | Entity::Bezier(_) => true,
        })
        .collect()
}

/// Hover, read the marks, click, and check the trim of entity 0 removed
/// exactly the `Danger` pieces.
fn assert_trim_agrees(mut s: Scene) {
    let before = s.app.document.entities[0];
    let marks = s.marks();
    assert_eq!(hovers(&marks), vec![0], "AC 6: TRIM hovers the target");
    let danger = dangers(&marks);
    assert!(!danger.is_empty(), "positive control: a danger preview");
    s.click();
    let after = s.app.document.entities[0];
    assert_ne!(before, after, "positive control: the click trimmed");
    let removed = lost(&before, &after);
    assert!(
        same_set(&danger, &removed),
        "AC 6: painted {danger:?}, removed {removed:?}"
    );
}

/// AC 6 — TRIM a line cut by two cutters: the two removed ends are exactly
/// the painted pieces.
#[test]
fn trim_line_removes_exactly_the_danger_pieces() {
    let mut s = scene(Box::new(TrimTool::default()));
    s.line((-100.0, 2.0), (100.0, 2.0));
    s.line((-40.0, -50.0), (-40.0, 50.0));
    s.line((40.0, -50.0), (40.0, 50.0));
    assert_trim_agrees(s);
}

/// AC 6 — TRIM an arc across ±π cut by one line: the removed sub-arc is
/// exactly the painted piece.
#[test]
fn trim_arc_removes_exactly_the_danger_pieces() {
    let mut s = scene(Box::new(TrimTool::default()));
    let r = 50.0 * s.app.camera.mm_per_px;
    // Centre 50 pt right of the pointer: the pointer sits on the arc at π.
    let arc = Arc::new(
        s.w(50.0, 0.0),
        r,
        0.6 * core::f64::consts::PI,
        -0.6 * core::f64::consts::PI,
        true,
    );
    s.app.commit(Box::new(CreateArc::new(arc)));
    s.line((20.0, -60.0), (20.0, -20.0));
    assert_trim_agrees(s);
}

/// AC 6 — TRIM a circle cut by a circle: the removed arc is exactly the
/// painted piece.
#[test]
fn trim_circle_removes_exactly_the_danger_piece() {
    let mut s = scene(Box::new(TrimTool::default()));
    let r = 50.0 * s.app.camera.mm_per_px;
    let target = Circle::new(s.w(50.0, 0.0), r);
    s.app.commit(Box::new(CreateCircle::new(target)));
    s.app
        .commit(Box::new(CreateCircle::new(Circle::new(s.w(100.0, 0.0), r))));
    assert_trim_agrees(s);
}

/// AC 6 — ERASE removes exactly the entities painted as `Danger`.
#[test]
fn erase_removes_exactly_the_danger_set() {
    let mut s = scene(Box::new(DeleteTool));
    s.line((-80.0, 60.0), (80.0, 60.0));
    s.line((-80.0, -60.0), (80.0, -60.0));
    s.line((-80.0, 20.0), (-80.0, 40.0));
    s.app.commit(Box::new(SelectionCommand::new(vec![0, 2])));
    let before = s.app.document.entities.clone();
    let danger = dangers(&s.marks());
    assert_eq!(danger.len(), 2, "positive control: two selected");
    s.click();
    let after = s.app.document.entities.clone();
    let removed: Vec<Entity> = before
        .iter()
        .filter(|e| !after.iter().any(|a| same(a, e)))
        .copied()
        .collect();
    assert!(
        same_set(&danger, &removed),
        "AC 6: painted {danger:?}, removed {removed:?}"
    );
}
