//! LCV-162 — crosshair cursor, pickbox and this-frame feedback, proven on the
//! shapes `App::update_ui` paints (`FullOutput.shapes`).
//!
//! The canvas rect is never hard-coded: [`boot`] hovers the middle of the
//! free area on an empty document, where no snap can fire, and reads the
//! canvas origin back as `pointer - world_to_screen(last_cursor_world)`.
//! Every world point below is placed in screen points from that one pair,
//! so the tests do not depend on the zoom the first frame settles at.

use crate::harness;

use lasercad::app::App;
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::{LineTool, Tool};

/// A booted app with its canvas rect and one known (screen, world) pair.
struct Canvas {
    ctx: egui::Context,
    app: App,
    rect: egui::Rect,
    w0: Vec2,
}

impl Canvas {
    /// One millimetre-per-point unit of the live camera.
    fn px(&self) -> f64 {
        self.app.camera.mm_per_px
    }

    /// World point `dx`, `dy` screen points from `w0` (y up, as the world).
    fn world(&self, dx: f64, dy: f64) -> Vec2 {
        self.w0 + Vec2::new(dx * self.px(), dy * self.px())
    }

    /// Screen position of world point `w` under the live camera.
    fn screen(&self, w: Vec2) -> egui::Pos2 {
        self.app.camera.world_to_screen(w) + self.rect.min.to_vec2()
    }

    /// Drive one frame with `events` and return what it painted.
    fn run(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let app = &mut self.app;
        self.ctx
            .run(harness::raw_input(events), |c| app.update_ui(c))
    }

    /// One frame carrying only `PointerMoved(pos)`.
    fn hover(&mut self, pos: egui::Pos2) -> egui::FullOutput {
        self.run(vec![egui::Event::PointerMoved(pos)])
    }

    /// Every shape painted on the canvas surface, flattened, in paint order.
    fn canvas_shapes(&self, out: &egui::FullOutput) -> Vec<egui::Shape> {
        let mut shapes = Vec::new();
        for clipped in &out.shapes {
            if close_rect(clipped.clip_rect, self.rect) {
                flatten(&clipped.shape, &mut shapes);
            }
        }
        shapes
    }
}

/// Boot `tool`, settle one frame, and learn the canvas rect from a hover.
fn boot(tool: Box<dyn Tool>) -> Canvas {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(tool);
    let mut free = egui::Rect::NOTHING;
    let _ = ctx.run(harness::raw_input(vec![]), |c| {
        app.update_ui(c);
        free = c.available_rect();
    });
    let pos = free.center();
    harness::frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    let w0 = app
        .last_cursor_world
        .expect("positive control: hovering the canvas sets the cursor");
    let min = pos - app.camera.world_to_screen(w0).to_vec2();
    let [w, h] = app.camera.viewport_size_px;
    let rect = egui::Rect::from_min_size(min, egui::vec2(w, h));
    assert!(rect.contains(pos), "setup: the pointer is on the canvas");
    Canvas {
        ctx,
        app,
        rect,
        w0,
    }
}

fn flatten(shape: &egui::Shape, out: &mut Vec<egui::Shape>) {
    match shape {
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten(s, out)),
        s => out.push(s.clone()),
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn close_rect(a: egui::Rect, b: egui::Rect) -> bool {
    close(a.min.x, b.min.x)
        && close(a.min.y, b.min.y)
        && close(a.max.x, b.max.x)
        && close(a.max.y, b.max.y)
}

/// Filled squares (an Endpoint snap glyph) centred within 0.5 pt of `at`.
fn filled_squares_at(shapes: &[egui::Shape], at: egui::Pos2) -> usize {
    shapes
        .iter()
        .filter(|s| match s {
            egui::Shape::Rect(r) => {
                r.fill != egui::Color32::TRANSPARENT
                    && close(r.rect.width(), r.rect.height())
                    && r.rect.width() < 20.0
                    && (r.rect.center() - at).length() < 0.5
            }
            _ => false,
        })
        .count()
}

/// Commit one line through the app's own history.
fn add_line(c: &mut Canvas, a: Vec2, b: Vec2) {
    c.app.commit(Box::new(CreateLine::new(Line::new(a, b))));
}

/// AC 4 — the snap glyph in the frame carrying `PointerMoved(P)` is the one
/// for P: the frame hovering near the second endpoint paints its glyph
/// there, and none at the endpoint the previous frame snapped to.
#[test]
fn the_snap_glyph_follows_this_frames_pointer() {
    let mut c = boot(Box::new(LineTool::default()));
    let (e1, e2) = (c.world(-100.0, 0.0), c.world(100.0, 0.0));
    add_line(&mut c, e1, e2);

    // Two frames on the first endpoint, so the control holds either way.
    let near_e1 = c.screen(e1) + egui::vec2(3.0, 2.0);
    let _ = c.hover(near_e1);
    let first = c.hover(near_e1);
    let shapes = c.canvas_shapes(&first);
    assert_eq!(
        filled_squares_at(&shapes, c.screen(e1)),
        1,
        "positive control: hovering near the first endpoint paints its glyph"
    );

    let near_e2 = c.screen(e2) + egui::vec2(-2.0, 3.0);
    let second = c.hover(near_e2);
    let shapes = c.canvas_shapes(&second);
    assert_eq!(
        filled_squares_at(&shapes, c.screen(e2)),
        1,
        "AC 4: the glyph must be at the endpoint this frame's pointer snaps to"
    );
    assert_eq!(
        filled_squares_at(&shapes, c.screen(e1)),
        0,
        "AC 4: no glyph may linger at the previous frame's snap"
    );
}
