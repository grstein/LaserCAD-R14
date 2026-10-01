//! LCV-156 AC 3 — every entity is drawn in its layer's color; selection and
//! preview keep their own colors.
//!
//! Proven on the painted shapes of a real `App::update_ui` frame: each layer
//! gets a color nothing else in the UI paints, so finding a stroke of that
//! color on the right shape kind proves the entity painter read the layer.

use crate::harness;

use egui::Color32;
use egui::epaint::{ColorMode, Shape};
use lasercad::app::App;
use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Circle, Line, Vec2};

const LINE_RGB: [u8; 3] = [10, 200, 30];
const CIRCLE_RGB: [u8; 3] = [200, 30, 10];
/// `src/render/selection.rs::halo_stroke`.
fn halo() -> Color32 {
    Color32::from_rgba_unmultiplied(64, 160, 255, 180)
}

/// `src/render/preview.rs::preview_stroke`.
fn preview() -> Color32 {
    Color32::from_rgba_unmultiplied(255, 220, 100, 160)
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// A line on one layer, a circle on the other; the current layer is neither
/// entity's, so "current layer color" cannot pass for "layer color".
fn two_layer_app() -> App {
    let layer = |id, name: &str, color| Layer {
        id: LayerId(id),
        name: name.to_owned(),
        color,
        output: true,
    };
    let document = Document::from_parts(
        [300.0, 200.0],
        vec![
            layer(0, "Cut", [255, 0, 0]),
            layer(1, "Green", LINE_RGB),
            layer(2, "Rust", CIRCLE_RGB),
        ],
        LayerId(0),
        vec![
            Entity::Line(Line::new(Vec2::new(20.0, 20.0), Vec2::new(120.0, 80.0))),
            Entity::Circle(Circle::new(Vec2::new(150.0, 100.0), 30.0)),
        ],
        vec![LayerId(1), LayerId(2)],
    )
    .expect("valid layers");
    App {
        document,
        ..App::default()
    }
}

/// Every `(kind, color)` stroke in the frame's paint list, nested shapes
/// included.
fn strokes(
    ctx: &egui::Context,
    app: &mut App,
    events: Vec<egui::Event>,
) -> Vec<(&'static str, Color32)> {
    let out = ctx.run(harness::raw_input(events), |ctx| app.update_ui(ctx));
    let mut found = Vec::new();
    for clipped in &out.shapes {
        walk(&clipped.shape, &mut found);
    }
    found
}

fn walk(shape: &Shape, out: &mut Vec<(&'static str, Color32)>) {
    match shape {
        Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
        Shape::LineSegment { stroke, .. } => {
            if let ColorMode::Solid(c) = stroke.color {
                out.push(("line", c));
            }
        }
        // A circle is one closed path (LCV-164 AC 6).
        Shape::Path(p) if p.closed => {
            if let ColorMode::Solid(c) = p.stroke.color {
                out.push(("circle", c));
            }
        }
        _ => {}
    }
}

#[test]
fn entities_stroke_in_their_layer_color_selection_and_preview_keep_theirs() {
    let ctx = egui::Context::default();
    let mut app = two_layer_app();
    app.document.selection.set([1]);
    let _ = strokes(&ctx, &mut app, Vec::new());
    let painted = strokes(&ctx, &mut app, Vec::new());

    assert!(
        painted.contains(&("line", rgb(LINE_RGB))),
        "the line is stroked in its layer's color: {painted:?}"
    );
    assert!(
        painted.contains(&("circle", rgb(CIRCLE_RGB))),
        "the circle is stroked in its layer's color: {painted:?}"
    );
    assert!(
        !painted.contains(&("line", rgb(CIRCLE_RGB)))
            && !painted.contains(&("circle", rgb(LINE_RGB))),
        "no entity borrows the other layer's color"
    );
    assert!(
        !painted.contains(&("circle", Color32::from_gray(220))),
        "the old single entity gray is gone"
    );
    assert!(
        painted.contains(&("circle", halo())),
        "the selected circle keeps the selection halo color: {painted:?}"
    );

    // A Line in progress: first point typed, cursor over the canvas.
    app.document.selection.clear();
    harness::submit_command(&ctx, &mut app, "L");
    harness::submit_command(&ctx, &mut app, "10,10");
    let hover = vec![egui::Event::PointerMoved(egui::pos2(640.0, 400.0))];
    let _ = strokes(&ctx, &mut app, hover.clone());
    let painted = strokes(&ctx, &mut app, hover);
    assert!(
        painted.contains(&("line", preview())),
        "the rubber-band preview keeps its own color: {painted:?}"
    );
}
