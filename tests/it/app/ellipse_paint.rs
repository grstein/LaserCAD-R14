//! LCV-176 AC 4 — an ellipse and an elliptical arc are painted with every
//! vertex on the curve and a chord deviation ≤ 0.5 px, in their layer colour,
//! under the selection halo and under the hover stroke, at two zooms.
//!
//! Proven on the painted shapes of a real `App::update_ui` frame: each entity
//! sits on a layer whose colour nothing else paints, and every painted
//! vertex is mapped back to the world through the camera.

use crate::harness;

use egui::Color32;
use egui::epaint::{ColorMode, Shape};
use harness::frame;
use lasercad::app::App;
use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Ellipse, EllipseSpan, Vec2};
use lasercad::tools::SelectTool;

const FULL_RGB: [u8; 3] = [10, 200, 30];
const ARC_RGB: [u8; 3] = [200, 30, 10];
/// `src/render/selection.rs::halo_stroke`.
fn halo() -> Color32 {
    Color32::from_rgba_unmultiplied(64, 160, 255, 180)
}
/// `src/render/palette.rs::HOVER_WIDTH_PT`.
const HOVER_WIDTH: f32 = 2.5;

/// Painted segments as `(a, b, colour, width)`, in screen points.
type Seg = (egui::Pos2, egui::Pos2, Color32, f32);

struct Scene {
    ctx: egui::Context,
    app: App,
    /// `rect.min` of the canvas: painted = `world_to_screen(w) + offset`.
    offset: egui::Vec2,
    full: Ellipse,
    arc: Ellipse,
}

impl Scene {
    fn new(mm_per_px: f64) -> Self {
        let ctx = egui::Context::default();
        let mut app = App::default();
        app.tool_manager.set_tool(Box::new(SelectTool::default()));
        let mut free = egui::Rect::NOTHING;
        let _ = ctx.run(harness::raw_input(vec![]), |c| {
            app.update_ui(c);
            free = c.available_rect();
        });
        app.camera.mm_per_px = mm_per_px;
        let p = free.center();
        frame(&ctx, &mut app, vec![egui::Event::PointerMoved(p)]);
        let w0 = app
            .last_cursor_world
            .expect("hovering the canvas sets the cursor");
        let offset = p - app.camera.world_to_screen(w0);
        // A thin ellipse (rx/ry = 10) around the cursor, and a 250° arc.
        let full = Ellipse::new(w0, 30.0, 3.0, 0.4, None);
        let arc = Ellipse::new(
            w0 + Vec2::new(0.0, 5.0),
            12.0,
            20.0,
            -0.3,
            Some(EllipseSpan::new(0.5, 0.5 - 250f64.to_radians(), false)),
        );
        let layer = |id, name: &str, color| Layer {
            id: LayerId(id),
            name: name.to_owned(),
            color,
            output: true,
        };
        app.document = Document::from_parts(
            [300.0, 200.0],
            vec![
                layer(0, "Cut", [255, 0, 0]),
                layer(1, "Full", FULL_RGB),
                layer(2, "Arc", ARC_RGB),
            ],
            LayerId(0),
            vec![Entity::Ellipse(full), Entity::Ellipse(arc)],
            vec![LayerId(1), LayerId(2)],
        )
        .expect("valid layers");
        Self {
            ctx,
            app,
            offset,
            full,
            arc,
        }
    }

    /// Every line segment painted on a frame with `events`.
    fn segments(&mut self, events: Vec<egui::Event>) -> Vec<Seg> {
        let out = self
            .ctx
            .run(harness::raw_input(events), |c| self.app.update_ui(c));
        let mut found = Vec::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut found);
        }
        found
    }

    fn world(&self, s: egui::Pos2) -> Vec2 {
        self.app.camera.screen_to_world(s - self.offset)
    }

    /// AC 4 for one painted run of `e`: every vertex on the curve (within
    /// float rounding), and every curve point within 0.5 px of a segment.
    fn assert_traced(&self, e: &Ellipse, segs: &[&Seg], what: &str) {
        let px = self.app.camera.mm_per_px;
        assert!(
            segs.len() >= 8,
            "{what}: painted as a polyline: {}",
            segs.len()
        );
        for (a, b, _, _) in segs {
            for v in [a, b] {
                let d = e.distance_to_point(self.world(*v));
                assert!(
                    d <= 1e-3 * px,
                    "{what}: vertex {v:?} is {d} mm off the curve"
                );
            }
        }
        let (start, sweep) = e.signed_range();
        for i in 0..=2000 {
            let q = e.point(start + sweep * f64::from(i) / 2000.0);
            let s = self.app.camera.world_to_screen(q) + self.offset;
            let d = segs
                .iter()
                .map(|(a, b, _, _)| dist_to_segment(s, *a, *b))
                .fold(f32::INFINITY, f32::min);
            assert!(d <= 0.5 + 1e-3, "{what}: chord deviation {d} px > 0.5 px");
        }
    }
}

fn walk(shape: &Shape, out: &mut Vec<Seg>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
        Shape::LineSegment { points, stroke } => {
            if let ColorMode::Solid(c) = stroke.color {
                out.push((points[0], points[1], c, stroke.width));
            }
        }
        _ => {}
    }
}

fn dist_to_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let d = b - a;
    let len2 = d.length_sq();
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((p - a).dot(d) / len2).clamp(0.0, 1.0)
    };
    (a + d * t - p).length()
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// AC 4 — the layer stroke and the selection halo trace both curves.
#[test]
fn ellipse_and_arc_paint_on_the_curve_in_layer_colour_and_halo() {
    for zoom in [0.05, 0.5] {
        let mut s = Scene::new(zoom);
        s.app.document.selection.set([1]);
        let segs = s.segments(Vec::new());
        let of = |c: Color32| segs.iter().filter(|g| g.2 == c).collect::<Vec<_>>();
        s.assert_traced(&s.full, &of(rgb(FULL_RGB)), &format!("full @{zoom}"));
        s.assert_traced(&s.arc, &of(rgb(ARC_RGB)), &format!("arc @{zoom}"));
        s.assert_traced(&s.arc, &of(halo()), &format!("halo @{zoom}"));
    }
}

/// AC 4 — hovering the curve with Select repaints it in its layer colour at
/// the hover width, on the curve.
#[test]
fn hovered_ellipse_paints_at_hover_width() {
    for zoom in [0.05, 0.5] {
        let mut s = Scene::new(zoom);
        let on = s.app.camera.world_to_screen(s.full.point(0.0)) + s.offset;
        let _ = s.segments(vec![egui::Event::PointerMoved(on)]);
        let segs = s.segments(vec![egui::Event::PointerMoved(on)]);
        let hover: Vec<&Seg> = segs
            .iter()
            .filter(|g| g.2 == rgb(FULL_RGB) && g.3 == HOVER_WIDTH)
            .collect();
        s.assert_traced(&s.full, &hover, &format!("hover @{zoom}"));
    }
}
