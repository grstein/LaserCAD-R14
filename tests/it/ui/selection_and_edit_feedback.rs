//! LCV-163 — selection and edit feedback (window/crossing boxes, hover
//! highlight, TRIM and ERASE danger previews, paint order, Esc and
//! pointer-leave), proven on the shapes `App::update_ui` paints.
//!
//! As in `cursor_and_picking.rs`, the canvas rect is never hard-coded:
//! [`boot`] hovers the middle of the free area on an empty document and
//! reads the canvas origin back from the resolved cursor; every world point
//! is placed in screen points from that pair.

use crate::harness;

use lasercad::app::App;
use lasercad::document::{CreateCircle, CreateLine, SelectionCommand};
use lasercad::geometry::{Circle, Line, Vec2};
use lasercad::render::palette::{DANGER, HOVER_WIDTH_PT, preview};
use lasercad::tools::{DeleteTool, LineTool, SelectTool, Tool, TrimTool};

/// A booted app with its canvas rect and one known (screen, world) pair.
struct Canvas {
    ctx: egui::Context,
    app: App,
    rect: egui::Rect,
    p: egui::Pos2,
    w0: Vec2,
}

impl Canvas {
    /// World point `dx`, `dy` screen points from `w0` (y up, as the world).
    fn world(&self, dx: f64, dy: f64) -> Vec2 {
        let px = self.app.camera.mm_per_px;
        self.w0 + Vec2::new(dx * px, dy * px)
    }

    /// Screen position of world point `w` under the live camera.
    fn screen(&self, w: Vec2) -> egui::Pos2 {
        self.app.camera.world_to_screen(w) + self.rect.min.to_vec2()
    }

    fn run(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let app = &mut self.app;
        self.ctx
            .run_ui(harness::raw_input(events), |ui| app.update_ui(ui))
    }

    fn hover(&mut self, pos: egui::Pos2) -> egui::FullOutput {
        self.run(vec![egui::Event::PointerMoved(pos)])
    }

    /// Press the primary button at `pos` (after a warm-up hover there).
    fn press(&mut self, pos: egui::Pos2) {
        let _ = self.hover(pos);
        let _ = self.run(vec![egui::Event::PointerMoved(pos), button(pos, true)]);
    }

    fn escape(&mut self) -> egui::FullOutput {
        self.run(harness::key_events(
            egui::Key::Escape,
            egui::Modifiers::NONE,
        ))
    }

    /// Every shape painted on the canvas surface, flattened, in paint order.
    fn shapes(&self, out: &egui::FullOutput) -> Vec<egui::Shape> {
        let mut shapes = Vec::new();
        for clipped in &out.shapes {
            if close_rect(clipped.clip_rect, self.rect) {
                flatten(&clipped.shape, &mut shapes);
            }
        }
        shapes
    }

    fn add(&mut self, entity: Line) {
        self.app.commit(Box::new(CreateLine::new(entity)));
    }

    fn add_circle(&mut self, circle: Circle) {
        self.app.commit(Box::new(CreateCircle::new(circle)));
    }

    /// Colour of entity `i`'s layer.
    fn layer_color(&self, i: usize) -> egui::Color32 {
        let [r, g, b] = self.app.document.layer_color(i);
        egui::Color32::from_rgb(r, g, b)
    }
}

fn button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// Boot `tool`, settle the layout, and learn the canvas rect from a hover.
fn boot(tool: Box<dyn Tool>) -> Canvas {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.tool_manager.set_tool(tool);
    let free = harness::settle(&ctx, &mut app);
    let pos = free.center();
    harness::frame(&ctx, &mut app, vec![egui::Event::PointerMoved(pos)]);
    let w0 = app
        .last_cursor_world
        .expect("positive control: hovering the canvas sets the cursor");
    let min = pos - app.camera.world_to_screen(w0).to_vec2();
    let [w, h] = app.camera.viewport_size_px;
    let rect = egui::Rect::from_min_size(min, egui::vec2(w, h));
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

/// A painted straight segment: its ends, width and solid colour.
fn segment(shape: &egui::Shape) -> Option<([egui::Pos2; 2], f32, egui::Color32)> {
    match shape {
        egui::Shape::LineSegment { points, stroke } => Some((*points, stroke.width, stroke.color)),
        _ => None,
    }
}

/// `(index, ends)` of every segment painted in `color`.
fn segments_in(shapes: &[egui::Shape], color: egui::Color32) -> Vec<(usize, [egui::Pos2; 2])> {
    shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            segment(s)
                .filter(|(_, _, c)| *c == color)
                .map(|(p, ..)| (i, p))
        })
        .collect()
}

/// `(index, ends)` of every segment at the hover width in `color`.
fn hover_segments(shapes: &[egui::Shape], color: egui::Color32) -> Vec<(usize, [egui::Pos2; 2])> {
    shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            segment(s)
                .filter(|(_, w, c)| *c == color && close(*w, HOVER_WIDTH_PT))
                .map(|(p, ..)| (i, p))
        })
        .collect()
}

fn near(a: egui::Pos2, b: egui::Pos2) -> bool {
    (a - b).length() < 0.5
}

fn same_segment(a: [egui::Pos2; 2], b: [egui::Pos2; 2]) -> bool {
    (near(a[0], b[0]) && near(a[1], b[1])) || (near(a[0], b[1]) && near(a[1], b[0]))
}

/// A Select box dragged from the pointer by `d` screen points.
fn drag_box(d: egui::Vec2) -> (Canvas, egui::FullOutput, egui::Rect) {
    let mut c = boot(Box::new(SelectTool::default()));
    let p = c.p;
    c.press(p);
    let out = c.hover(p + d);
    assert_eq!(c.app.tool_manager.preview().len(), 4, "positive control");
    let edges = egui::Rect::from_two_pos(p, p + d);
    (c, out, edges)
}

/// AC 1 — a left-to-right box is four solid `preview` segments, one per edge.
#[test]
fn a_left_to_right_box_paints_a_solid_outline() {
    let (c, out, edges) = drag_box(egui::vec2(80.0, 50.0));
    let segs = segments_in(&c.shapes(&out), preview());
    assert_eq!(segs.len(), 4, "AC 1: four solid edges, got {}", segs.len());
    let top = [edges.left_top(), edges.right_top()];
    assert!(
        segs.iter().any(|(_, s)| same_segment(*s, top)),
        "AC 1: the top edge is one unbroken segment"
    );
}

/// AC 2 — a right-to-left box is dashed: many short `preview` segments along
/// one edge, none of which spans it.
#[test]
fn a_right_to_left_box_paints_a_dashed_outline() {
    let (c, out, edges) = drag_box(egui::vec2(-80.0, 50.0));
    let segs = segments_in(&c.shapes(&out), preview());
    let on_top: Vec<_> = segs
        .iter()
        .filter(|(_, [a, b])| close(a.y, edges.top()) && close(b.y, edges.top()))
        .collect();
    assert!(
        on_top.len() > 4,
        "AC 2: the top edge is dashed, got {} pieces",
        on_top.len()
    );
    assert!(
        on_top
            .iter()
            .all(|(_, [a, b])| (a.x - b.x).abs() < edges.width() - 1.0),
        "AC 2: no piece spans the whole edge"
    );
}

/// AC 3 — Select idle: the entity in the pickbox is repainted thicker in its
/// layer colour; with none in the pickbox nothing is.
#[test]
fn select_hover_highlights_the_entity_in_the_pickbox() {
    let mut c = boot(Box::new(SelectTool::default()));
    let (a, b) = (c.world(-60.0, 0.0), c.world(60.0, 0.0));
    c.add(Line::new(a, b));
    let color = c.layer_color(0);

    let on = c.hover(c.p + egui::vec2(10.0, 3.0));
    let hovered = hover_segments(&c.shapes(&on), color);
    assert_eq!(hovered.len(), 1, "AC 3: one highlighted line");
    assert!(
        same_segment(hovered[0].1, [c.screen(a), c.screen(b)]),
        "AC 3: the highlight is the hovered line"
    );

    let off = c.hover(c.p + egui::vec2(10.0, 20.0));
    assert!(
        hover_segments(&c.shapes(&off), color).is_empty(),
        "AC 3: no highlight when no entity is in the pickbox"
    );
}

/// Points of every `DANGER` segment.
fn danger_points(shapes: &[egui::Shape]) -> Vec<egui::Pos2> {
    segments_in(shapes, DANGER)
        .into_iter()
        .flat_map(|(_, [a, b])| [a, b])
        .collect()
}

/// AC 3, AC 4 — TRIM over a line cut by two cutters: the line is
/// highlighted, and both removed ends (outside the cutters) are dashed in
/// `danger`; the kept middle is not.
#[test]
fn trim_hover_paints_the_removed_ends_of_a_line_in_danger() {
    let mut c = boot(Box::new(TrimTool::default()));
    let (a, b) = (c.world(-100.0, 0.0), c.world(100.0, 0.0));
    c.add(Line::new(a, b));
    c.add(Line::new(c.world(-40.0, -50.0), c.world(-40.0, 50.0)));
    c.add(Line::new(c.world(40.0, -50.0), c.world(40.0, 50.0)));
    let color = c.layer_color(0);

    let out = c.hover(c.p + egui::vec2(0.0, 2.0));
    let shapes = c.shapes(&out);
    let hovered = hover_segments(&shapes, color);
    assert!(
        hovered
            .iter()
            .any(|(_, s)| same_segment(*s, [c.screen(a), c.screen(b)])),
        "AC 3: TRIM highlights the target"
    );

    let dashes = segments_in(&shapes, DANGER);
    assert!(dashes.len() > 2, "AC 4: the pieces are dashed");
    let (y, x0) = (c.screen(a).y, c.p.x);
    for p in danger_points(&shapes) {
        assert!(close(p.y, y), "AC 4: dashes lie on the target");
        let dx = p.x - x0;
        assert!(
            (-100.5..=-39.5).contains(&dx) || (39.5..=100.5).contains(&dx),
            "AC 4: only the removed ends, got dx = {dx}"
        );
    }
    let pts = danger_points(&shapes);
    assert!(
        pts.iter().any(|p| p.x - x0 < -80.0),
        "AC 4: left end painted"
    );
    assert!(
        pts.iter().any(|p| p.x - x0 > 80.0),
        "AC 4: right end painted"
    );
}

/// AC 4 — TRIM over a circle cut by a circle: the removed arc (the one
/// inside the cutter, away from the pointer) is dashed in `danger`.
#[test]
fn trim_hover_paints_the_removed_arc_of_a_circle_in_danger() {
    let mut c = boot(Box::new(TrimTool::default()));
    let r = 50.0 * c.app.camera.mm_per_px;
    c.add_circle(Circle::new(c.world(0.0, 0.0), r));
    c.add_circle(Circle::new(c.world(50.0, 0.0), r));

    // Hover the target's far side (angle π), keeping that side.
    let out = c.hover(c.p + egui::vec2(-50.0, 1.0));
    let pts = danger_points(&c.shapes(&out));
    assert!(pts.len() > 4, "AC 4: the removed arc is dashed");
    let (centre, cutter) = (c.p, c.p + egui::vec2(50.0, 0.0));
    for p in pts {
        assert!(
            ((p - centre).length() - 50.0).abs() < 0.5,
            "AC 4: on the target"
        );
        assert!(
            (p - cutter).length() <= 50.5,
            "AC 4: only the piece inside the cutter"
        );
    }
}

/// Select entities `ids` through history.
fn select(c: &mut Canvas, ids: Vec<usize>) {
    c.app.commit(Box::new(SelectionCommand::new(ids)));
}

/// AC 5 — ERASE with a selection paints every selected entity dashed in
/// `danger`; the unselected one is not.
#[test]
fn erase_paints_every_selected_entity_in_danger() {
    let mut c = boot(Box::new(DeleteTool));
    c.add(Line::new(c.world(-80.0, 60.0), c.world(80.0, 60.0)));
    c.add(Line::new(c.world(-80.0, -60.0), c.world(80.0, -60.0)));
    c.add(Line::new(c.world(-80.0, 0.0), c.world(-80.0, 30.0)));
    select(&mut c, vec![0, 1]);

    let out = c.hover(c.p + egui::vec2(20.0, 20.0));
    let pts = danger_points(&c.shapes(&out));
    let on = |dy: f32| pts.iter().filter(|p| close(p.y, c.p.y - dy)).count();
    assert!(on(60.0) > 2, "AC 5: selected line 0 dashed");
    assert!(on(-60.0) > 2, "AC 5: selected line 1 dashed");
    assert_eq!(on(60.0) + on(-60.0), pts.len(), "AC 5: nothing else");
}

/// Index of the first shape matching `f`.
fn first(shapes: &[egui::Shape], f: impl Fn(&egui::Shape) -> bool) -> usize {
    shapes.iter().position(f).expect("shape painted")
}

/// Index of the last shape matching `f`.
fn last(shapes: &[egui::Shape], f: impl Fn(&egui::Shape) -> bool) -> usize {
    shapes.iter().rposition(f).expect("shape painted")
}

fn is_halo(s: &egui::Shape) -> bool {
    segment(s).is_some_and(|(_, w, _)| close(w, 3.0))
}

fn in_color(color: egui::Color32) -> impl Fn(&egui::Shape) -> bool {
    move |s| segment(s).is_some_and(|(_, _, c)| c == color)
}

fn is_hover(color: egui::Color32) -> impl Fn(&egui::Shape) -> bool {
    move |s| segment(s).is_some_and(|(_, w, c)| c == color && close(w, HOVER_WIDTH_PT))
}

fn is_pickbox(s: &egui::Shape) -> bool {
    matches!(s, egui::Shape::Rect(r)
        if r.fill == egui::Color32::TRANSPARENT && close(r.rect.width(), 10.0))
}

fn is_snap_glyph(s: &egui::Shape) -> bool {
    matches!(s, egui::Shape::Rect(r)
        if r.fill != egui::Color32::TRANSPARENT && r.rect.width() < 20.0)
}

/// AC 7 — paint order: selection halo < hover < danger < pickbox <
/// crosshair (TRIM), and preview/danger < snap glyph < crosshair (LINE,
/// ERASE).
#[test]
fn feedback_paints_in_the_canvas_order() {
    let mut c = boot(Box::new(TrimTool::default()));
    c.add(Line::new(c.world(-100.0, 0.0), c.world(100.0, 0.0)));
    c.add(Line::new(c.world(40.0, -50.0), c.world(40.0, 50.0)));
    select(&mut c, vec![1]);
    let color = c.layer_color(0);
    let out = c.hover(c.p + egui::vec2(0.0, 2.0));
    let s = c.shapes(&out);
    let n = s.len();
    assert!(
        last(&s, is_halo) < first(&s, is_hover(color)),
        "halo < hover"
    );
    assert!(
        last(&s, is_hover(color)) < first(&s, in_color(DANGER)),
        "hover < danger"
    );
    assert!(
        last(&s, in_color(DANGER)) < first(&s, is_pickbox),
        "danger < pickbox"
    );
    assert!(first(&s, is_pickbox) < n - 2, "pickbox < crosshair");

    // ERASE: danger below the snap glyph (ERASE is a point pick, so it snaps).
    let mut c = boot(Box::new(DeleteTool));
    let e = c.world(-60.0, 30.0);
    c.add(Line::new(e, c.world(60.0, 30.0)));
    select(&mut c, vec![0]);
    let out = c.hover(c.screen(e) + egui::vec2(3.0, 2.0));
    assert!(c.app.active_snap.is_some(), "positive control: a snap");
    let s = c.shapes(&out);
    assert!(
        last(&s, in_color(DANGER)) < first(&s, is_snap_glyph),
        "danger < snap"
    );
    assert!(last(&s, is_snap_glyph) < s.len() - 2, "snap < crosshair");

    // LINE: the rubber band below the snap glyph.
    let mut c = boot(Box::new(LineTool::default()));
    let e = c.world(60.0, 30.0);
    c.add(Line::new(e, c.world(90.0, 60.0)));
    let p = c.p;
    let _ = c.run(vec![
        egui::Event::PointerMoved(p),
        button(p, true),
        button(p, false),
    ]);
    let out = c.hover(c.screen(e) + egui::vec2(3.0, 2.0));
    assert!(c.app.active_snap.is_some(), "positive control: a snap");
    let s = c.shapes(&out);
    assert!(
        last(&s, in_color(preview())) < first(&s, is_snap_glyph),
        "preview < snap"
    );
}

/// AC 9 — Esc hides the hover highlight and the danger preview while the
/// pointer stays put; a move brings them back.
#[test]
fn escape_hides_hover_and_danger_until_the_pointer_moves() {
    let mut c = boot(Box::new(TrimTool::default()));
    c.add(Line::new(c.world(-100.0, 0.0), c.world(100.0, 0.0)));
    c.add(Line::new(c.world(40.0, -50.0), c.world(40.0, 50.0)));
    let color = c.layer_color(0);
    let at = c.p + egui::vec2(0.0, 2.0);
    let before = c.hover(at);
    let s = c.shapes(&before);
    assert!(!hover_segments(&s, color).is_empty(), "positive control");
    assert!(!segments_in(&s, DANGER).is_empty(), "positive control");

    let _ = c.escape();
    let after = c.hover(at);
    let s = c.shapes(&after);
    assert!(
        hover_segments(&s, color).is_empty(),
        "AC 9: no hover after Esc"
    );
    assert!(
        segments_in(&s, DANGER).is_empty(),
        "AC 9: no danger after Esc"
    );

    let moved = c.hover(at + egui::vec2(-3.0, 0.0));
    assert!(
        !hover_segments(&c.shapes(&moved), color).is_empty(),
        "a move re-arms the hover"
    );
}

/// AC 9 — leaving the canvas hides the hover highlight (Select) and the
/// danger preview (ERASE).
#[test]
fn leaving_the_canvas_hides_hover_and_danger() {
    let mut c = boot(Box::new(SelectTool::default()));
    c.add(Line::new(c.world(-60.0, 0.0), c.world(60.0, 0.0)));
    let color = c.layer_color(0);
    let on = c.hover(c.p + egui::vec2(10.0, 3.0));
    assert!(
        !hover_segments(&c.shapes(&on), color).is_empty(),
        "positive control"
    );
    let gone = c.run(vec![egui::Event::PointerGone]);
    assert!(
        hover_segments(&c.shapes(&gone), color).is_empty(),
        "AC 9: no hover"
    );

    let mut c = boot(Box::new(DeleteTool));
    c.add(Line::new(c.world(-60.0, 40.0), c.world(60.0, 40.0)));
    select(&mut c, vec![0]);
    let on = c.hover(c.p);
    assert!(
        !segments_in(&c.shapes(&on), DANGER).is_empty(),
        "positive control"
    );
    let gone = c.run(vec![egui::Event::PointerGone]);
    assert!(
        segments_in(&c.shapes(&gone), DANGER).is_empty(),
        "AC 9: no danger"
    );
}
