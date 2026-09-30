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
use lasercad::cmdline::ToolKind;
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::tools::{LineTool, SelectTool, Tool, make};

/// A booted app with its canvas rect and one known (screen, world) pair.
struct Canvas {
    ctx: egui::Context,
    app: App,
    rect: egui::Rect,
    /// The pointer position `boot` hovered, over world point `w0`.
    p: egui::Pos2,
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
        p: pos,
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

/// A painted straight segment: its two ends, stroke width and solid colour.
fn segment(shape: &egui::Shape) -> Option<([egui::Pos2; 2], f32, egui::Color32)> {
    match shape {
        egui::Shape::LineSegment { points, stroke } => match stroke.color {
            egui::epaint::ColorMode::Solid(c) => Some((*points, stroke.width, c)),
            egui::epaint::ColorMode::UV(_) => None,
        },
        _ => None,
    }
}

/// A 1 pt segment spanning the whole canvas: `Some(true)` horizontal,
/// `Some(false)` vertical.
fn full_span(rect: egui::Rect, shape: &egui::Shape) -> Option<(bool, f32, egui::Color32)> {
    let ([a, b], width, color) = segment(shape)?;
    let (lo, hi) = (a.min(b), a.max(b));
    if !close(width, 1.0) {
        None
    } else if close(a.y, b.y) && close(lo.x, rect.min.x) && close(hi.x, rect.max.x) {
        Some((true, a.y, color))
    } else if close(a.x, b.x) && close(lo.y, rect.min.y) && close(hi.y, rect.max.y) {
        Some((false, a.x, color))
    } else {
        None
    }
}

/// The crosshair: the last two canvas shapes are one horizontal and one
/// vertical 1 pt full-span line in one colour. Returns its point and colour.
fn crosshair(c: &Canvas, out: &egui::FullOutput) -> (egui::Pos2, egui::Color32) {
    let shapes = c.canvas_shapes(out);
    assert!(shapes.len() >= 2, "the canvas paints shapes");
    let last: Vec<_> = shapes[shapes.len() - 2..]
        .iter()
        .map(|s| full_span(c.rect, s).expect("AC 2: the last canvas shapes are the crosshair"))
        .collect();
    let (h, v) = match (last[0], last[1]) {
        (h @ (true, ..), v @ (false, ..)) | (v @ (false, ..), h @ (true, ..)) => (h, v),
        _ => panic!("AC 2: one horizontal and one vertical line, got {last:?}"),
    };
    assert_eq!(h.2, v.2, "AC 2: both lines in the one cursor colour");
    (egui::pos2(v.1, h.1), h.2)
}

/// Full-span lines in `color` anywhere on the canvas.
fn crosshair_lines(c: &Canvas, out: &egui::FullOutput, color: egui::Color32) -> usize {
    c.canvas_shapes(out)
        .iter()
        .filter(|s| full_span(c.rect, s).is_some_and(|(_, _, k)| k == color))
        .count()
}

/// WCAG 2 contrast ratio of two opaque colours.
fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
    let lum = |c: egui::Color32| {
        let ch = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    };
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `bed.fill` (DESIGN.md §3): the surface the cursor colour is measured on.
const BED_FILL: egui::Color32 = egui::Color32::from_gray(40);

fn near(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a - b).length() < 0.5
}

/// AC 1, AC 2, AC 3 — over the canvas the OS cursor is hidden, and the last
/// canvas shapes are a full-span 1 pt crosshair through the pointer (no snap
/// on an empty document), in an opaque colour with ≥3:1 on the bed fill.
#[test]
fn a_hovered_canvas_hides_the_os_cursor_and_paints_the_crosshair_last() {
    let mut c = boot(Box::new(LineTool::default()));
    let p = c.p + egui::vec2(17.0, -9.0);
    let out = c.hover(p);
    assert_eq!(
        out.platform_output.cursor_icon,
        egui::CursorIcon::None,
        "AC 1: no OS cursor over the canvas"
    );
    let (at, color) = crosshair(&c, &out);
    assert!(near(at, p), "AC 3: at the pointer, {at:?} vs {p:?}");
    assert_eq!(color.a(), 255, "AC 2: an opaque cursor colour");
    assert!(
        contrast(color, BED_FILL) >= 3.0,
        "AC 2: cursor colour {color:?} has {:.2}:1 on the bed",
        contrast(color, BED_FILL)
    );
    assert_eq!(
        crosshair_lines(&c, &out, color),
        2,
        "AC 2: exactly two lines"
    );
}

/// AC 3 — a resolved snap moves the crosshair onto the snap point.
#[test]
fn the_crosshair_sits_on_the_snap_point() {
    let mut c = boot(Box::new(LineTool::default()));
    let (e, f) = (c.world(-60.0, 40.0), c.world(60.0, 90.0));
    add_line(&mut c, e, f);
    let p = c.screen(e) + egui::vec2(3.0, 2.0);
    let out = c.hover(p);
    assert!(c.app.active_snap.is_some(), "positive control: a snap");
    let (at, _) = crosshair(&c, &out);
    assert!(near(at, c.screen(e)), "AC 3: at the snap, {at:?}");
    assert!(!near(at, p), "AC 3: not at the pointer");
}

/// AC 3 — with Ortho on, LINE's second point is locked to the axis through
/// its anchor, and the crosshair sits on that locked point.
#[test]
fn the_crosshair_sits_on_the_ortho_point() {
    let mut c = boot(Box::new(LineTool::default()));
    c.app.ortho_enabled = true;
    let p = c.p;
    let press = |pressed| egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = c.run(vec![
        egui::Event::PointerMoved(p),
        press(true),
        press(false),
    ]);
    assert!(c.app.tool_manager.anchor().is_some(), "positive control");
    let out = c.hover(p + egui::vec2(40.0, 10.0));
    let (at, _) = crosshair(&c, &out);
    assert!(
        near(at, p + egui::vec2(40.0, 0.0)),
        "AC 3: at Ortho, {at:?}"
    );
}

/// AC 10 — after `PointerGone` the canvas paints no crosshair and no
/// pickbox, and the OS cursor is back to the default arrow.
#[test]
fn leaving_the_canvas_clears_the_crosshair_and_the_pickbox() {
    let mut c = boot(Box::new(SelectTool::default()));
    let hovered = c.hover(c.p);
    let (at, color) = crosshair(&c, &hovered);
    assert_eq!(pickboxes(&c, &hovered, at), 1, "positive control: pickbox");

    let gone = c.run(vec![egui::Event::PointerGone]);
    assert_eq!(crosshair_lines(&c, &gone, color), 0, "AC 10: no crosshair");
    assert_eq!(pickboxes(&c, &gone, at), 0, "AC 10: no pickbox");
    assert_eq!(
        gone.platform_output.cursor_icon,
        egui::CursorIcon::Default,
        "AC 10: the default OS cursor is back"
    );
}

/// Hollow squares of side 2 × 5 pt centred within 0.5 pt of `at`.
fn pickboxes(c: &Canvas, out: &egui::FullOutput, at: egui::Pos2) -> usize {
    c.canvas_shapes(out)
        .iter()
        .filter(|s| match s {
            egui::Shape::Rect(r) => {
                r.fill == egui::Color32::TRANSPARENT
                    && r.stroke.width > 0.0
                    && close(r.rect.width(), 10.0)
                    && close(r.rect.height(), 10.0)
                    && near(r.rect.center(), at)
            }
            _ => false,
        })
        .count()
}

/// AC 5 — Select idle, TRIM and EXTEND wait for an entity pick: one hollow
/// square of side 2 × 5 pt, centred on the crosshair.
#[test]
fn entity_picks_paint_the_pickbox_on_the_crosshair() {
    for kind in [ToolKind::Select, ToolKind::Trim, ToolKind::Extend] {
        let mut c = boot(make(kind));
        let out = c.hover(c.p + egui::vec2(11.0, 7.0));
        let (at, _) = crosshair(&c, &out);
        assert_eq!(pickboxes(&c, &out, at), 1, "AC 5: {kind:?} paints it");
    }
}

/// AC 6 — tools waiting for a point paint no pickbox, and neither does a
/// Select box drag in progress.
#[test]
fn point_picks_and_box_drags_paint_no_pickbox() {
    for kind in [
        ToolKind::Line,
        ToolKind::Copy,
        ToolKind::Move,
        ToolKind::Rotate,
        ToolKind::Mirror,
        ToolKind::Scale,
        ToolKind::Dist,
    ] {
        let mut c = boot(make(kind));
        let out = c.hover(c.p + egui::vec2(11.0, 7.0));
        let (at, _) = crosshair(&c, &out);
        assert_eq!(pickboxes(&c, &out, at), 0, "AC 6: {kind:?} paints none");
    }

    let mut c = boot(make(ToolKind::Select));
    let p = c.p;
    let _ = c.run(vec![
        egui::Event::PointerMoved(p),
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    let out = c.hover(p + egui::vec2(30.0, 20.0));
    assert_eq!(
        c.app.tool_manager.preview().len(),
        4,
        "positive control: a box drag is in progress"
    );
    let (at, _) = crosshair(&c, &out);
    assert_eq!(pickboxes(&c, &out, at), 0, "AC 6: no pickbox mid-drag");
}
