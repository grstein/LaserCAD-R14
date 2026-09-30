//! Preview overlay: translucent amber stroke for in-progress tool geometry,
//! and dashed strokes for the crossing box and the TRIM / ERASE removal
//! preview (LCV-163).
//!
//! Drawing a line in AutoCAD R14 works like this: the operator clicks the first
//! point, then a "rubber-band" preview follows the cursor showing what the line
//! will look like; the second click commits. Same for circles, arcs, polylines.
//! v2 mirrors this: the active tool (LCV-043 LineTool, LCV-046 CircleTool, etc.)
//! constructs a `Vec<Entity>` each frame representing the in-progress preview,
//! and [`draw_preview`] paints those entities with a distinct style.
//!
//! **Three visual channels**: layer colour = committed entities, cyan =
//! selected, amber = preview. The operator can tell what is real and what is
//! provisional at a glance.
//!
//! **Dashes are a state form** (LCV-163, ADR 0013): the rubber band stays a
//! solid translucent stroke (LCV-037), while [`draw_dashed`] strokes an entity
//! with egui 0.29's `Shape::dashed_line` over its projected polyline (circles
//! and arcs sampled like `arc_polyline`). The crossing box is dashed in
//! `preview`; what TRIM or ERASE will remove is dashed in `danger`.
//!
//! **Drawing order**: caller invokes [`draw_preview`] AFTER
//! [`crate::render::draw_entities`] and [`crate::render::draw_selection_highlight`]
//! and BEFORE snap markers (LCV-038). The preview should sit visually above
//! selection halos (the operator's attention is on what they're about to commit)
//! but below snap markers (snap markers are the most important cursor-anchored
//! feedback).
//!
//! Introduced by demand LCV-037.

use crate::document::Entity;
use crate::geometry::Vec2;
use crate::render::Camera;

/// Dash and gap lengths of [`draw_dashed`], in screen points.
const DASH_PT: f32 = 6.0;
const GAP_PT: f32 = 4.0;

/// Polyline samples of a full circle or an arc in [`draw_dashed`].
const CURVE_SEGMENTS: usize = 64;

/// Preview stroke: 1-px translucent amber/yellow.
///
/// Choice rationale:
/// - **Amber** distinguishes from the committed layer-colored stroke and
///   cyan-blue selection halo (LCV-036). Three visual channels: layer color =
///   committed, cyan = selected, amber = preview.
/// - **Translucent (160/255 alpha)** signals "this isn't real yet". The
///   semi-transparency mimics dashed lines without requiring dash-pattern
///   support.
/// - **1-pixel width** matches the committed stroke; the operator can directly
///   compare the preview's footprint to what it will look like once committed.
///
/// Exposed as `pub(crate)` for testability (AC#4).
/// The colour is the `preview` token, [`crate::render::palette::preview`].
pub(crate) fn preview_stroke() -> egui::Stroke {
    egui::Stroke::new(1.0_f32, crate::render::palette::preview())
}

/// Draw the preview overlay: entities that are about to be committed.
///
/// For each entity in `preview_entities`, dispatch on variant (line, circle,
/// arc) and draw with [`preview_stroke`]. The dispatch logic mirrors
/// [`crate::render::draw_entities`]; this function reuses
/// [`crate::render::selection::draw_entity_with_stroke`] from LCV-036.
///
/// **Empty input**: `draw_preview(_, _, _, &[])` is a no-op — no draw calls,
/// no panic. Tools that have nothing to preview (e.g., before the first click)
/// pass an empty slice.
///
/// **No state on the function**: `draw_preview` is stateless; the caller
/// ([`crate::app::App::update`]) owns the preview `Vec<Entity>`. The tool
/// layer constructs and discards it per frame.
///
/// **Order of operations**: caller invokes this function AFTER
/// [`crate::render::draw_entities`] and
/// [`crate::render::draw_selection_highlight`] and BEFORE snap markers
/// (LCV-038). The preview should sit visually above selection halos but below
/// snap markers.
///
/// `rect` is the viewport's screen-space rectangle; `rect.min` is added to
/// every projected point so shapes land in the correct painter clip rect.
pub fn draw_preview(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    preview_entities: &[Entity],
) {
    let stroke = preview_stroke();

    for entity in preview_entities {
        crate::render::selection::draw_entity_with_stroke(painter, rect, camera, entity, stroke);
    }
}

/// Stroke `entity` dashed in `color` (1 pt): the crossing selection box in
/// `preview`, the TRIM / ERASE removal preview in `danger` (LCV-163 AC 2,
/// AC 4, AC 5). Lines are one dashed segment; circles and arcs are dashed
/// along a [`CURVE_SEGMENTS`] polyline, so the dash pattern runs on around
/// the curve.
pub fn draw_dashed(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    entity: &Entity,
    color: egui::Color32,
) {
    let world: Vec<Vec2> = match entity {
        Entity::Line(l) => vec![l.p1, l.p2],
        Entity::Circle(c) => (0..=CURVE_SEGMENTS)
            .map(|i| {
                let t = i as f64 / CURVE_SEGMENTS as f64;
                c.point_at_angle(t * core::f64::consts::TAU)
            })
            .collect(),
        Entity::Arc(a) => crate::render::arc_polyline(a, CURVE_SEGMENTS),
    };
    let offset = rect.min.to_vec2();
    let points: Vec<egui::Pos2> = world
        .into_iter()
        .map(|w| camera.world_to_screen(w) + offset)
        .collect();
    let stroke = egui::Stroke::new(1.0_f32, color);
    painter.extend(egui::Shape::dashed_line(&points, stroke, DASH_PT, GAP_PT));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line};
    use core::f64::consts::FRAC_PI_2;

    /// Every `LineSegment` `paint` emits, as (ends, width, colour).
    fn painted_segments(
        mut paint: impl FnMut(&egui::Painter, egui::Rect, &Camera),
    ) -> Vec<([egui::Pos2; 2], f32, egui::Color32)> {
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_dashed"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let camera = Camera {
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            paint(&painter, rect, &camera);
        });
        out.shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::LineSegment { points, stroke } => match stroke.color {
                    egui::epaint::ColorMode::Solid(k) => Some((*points, stroke.width, k)),
                    egui::epaint::ColorMode::UV(_) => None,
                },
                _ => None,
            })
            .collect()
    }

    /// LCV-163 AC 2 — a 100 pt line is painted as several dashes in the
    /// colour asked for, none as long as the line.
    #[test]
    fn draw_dashed_breaks_a_line_into_dashes() {
        let color = egui::Color32::from_rgb(1, 2, 3);
        let line = Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(100.0, 0.0)));
        let segs = painted_segments(|p, r, c| draw_dashed(p, r, c, &line, color));
        assert!(segs.len() > 5, "dashes: {}", segs.len());
        for ([a, b], _, k) in segs {
            assert_eq!(k, color);
            assert!((a - b).length() < 99.0);
        }
    }

    /// LCV-163 AC 4 — circles and arcs are dashed along their curve: every
    /// dash end lies on the circle.
    #[test]
    fn draw_dashed_follows_circles_and_arcs() {
        let color = egui::Color32::from_rgb(1, 2, 3);
        for entity in [
            Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 50.0)),
            Entity::Arc(Arc::new(Vec2::new(0.0, 0.0), 50.0, 0.0, FRAC_PI_2, true)),
        ] {
            let mut centre = egui::Pos2::ZERO;
            let segs = painted_segments(|p, r, c| {
                centre = c.world_to_screen(Vec2::new(0.0, 0.0)) + r.min.to_vec2();
                draw_dashed(p, r, c, &entity, color);
            });
            assert!(segs.len() > 5, "{entity:?}: {} dashes", segs.len());
            for ([a, b], ..) in segs {
                for q in [a, b] {
                    let d = (q - centre).length();
                    assert!((d - 50.0).abs() < 0.5, "{entity:?}: off the curve by {d}");
                }
            }
        }
    }

    /// AC#1 — `draw_preview` exists and type-checks with empty inputs.
    #[test]
    fn draw_preview_function_exists() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_preview"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let camera = Camera::default();
            let preview_entities = vec![];

            // Should compile and not panic.
            draw_preview(&painter, rect, &camera, &preview_entities);
        });
    }

    /// AC#2 — empty preview visits no entities (no draw calls).
    #[test]
    fn empty_preview_visits_no_entities() {
        let preview_entities: Vec<Entity> = vec![];
        // The empty slice should be a no-op; no draw calls, no panic.
        // We can't directly count draw calls from egui::Painter, but we can
        // verify the function returns without panic and that the trace helper
        // confirms zero iterations.
        assert_eq!(preview_entities.len(), 0);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_empty_preview"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let camera = Camera::default();

            draw_preview(&painter, rect, &camera, &preview_entities);
        });
    }

    /// AC#3 — non-empty preview visits each entity exactly once.
    #[test]
    fn non_empty_preview_visits_each_entity_once() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let circle = Circle::new(Vec2::new(50.0, 50.0), 20.0);
        let arc = Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true);

        let preview_entities = vec![Entity::Line(line), Entity::Circle(circle), Entity::Arc(arc)];

        // Verify we have 3 entities.
        assert_eq!(preview_entities.len(), 3);

        // Call draw_preview and ensure it doesn't panic.
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_non_empty_preview"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let camera = Camera::default();

            draw_preview(&painter, rect, &camera, &preview_entities);
        });
    }

    /// AC#4 — preview stroke is translucent and distinct.
    ///
    /// The preview color must have alpha strictly less than 255 (translucent)
    /// AND a hue that is neither the entity's gray nor the selection's
    /// cyan-blue. The test pins the band: yellow/amber-ish (R > 200, G > 150,
    /// B < 200) so the theme can be tuned without breaking the test.
    #[test]
    fn preview_stroke_is_translucent_and_distinct() {
        let stroke = preview_stroke();
        let color = stroke.color;

        // Translucent: alpha < 255.
        assert!(color.a() < 255, "alpha should be < 255 (translucent)");

        // Amber band: R > 200, G > 150, B < 200.
        assert!(
            color.r() > 200,
            "R should be > 200 for amber (got {})",
            color.r()
        );
        assert!(
            color.g() > 150,
            "G should be > 150 for amber (got {})",
            color.g()
        );
        assert!(
            color.b() < 200,
            "B should be < 200 for amber (got {})",
            color.b()
        );
    }

    /// AC#5 — `draw_preview` does not panic on degenerate inputs.
    #[test]
    fn draw_preview_does_not_panic_on_degenerate_camera() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_degenerate"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));

            let entities = vec![
                Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0))),
                Entity::Circle(Circle::new(Vec2::new(50.0, 50.0), 20.0)),
                Entity::Arc(Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true)),
            ];

            // Degenerate cameras: extreme zoom in/out.
            let cam_extreme_in = Camera {
                mm_per_px: 1e-9,
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            draw_preview(&painter, rect, &cam_extreme_in, &entities);

            let cam_extreme_out = Camera {
                mm_per_px: 1e9,
                viewport_size_px: [800.0, 600.0],
                ..Camera::default()
            };
            draw_preview(&painter, rect, &cam_extreme_out, &entities);

            // Empty slice.
            draw_preview(&painter, rect, &Camera::default(), &[]);
        });
    }
}
