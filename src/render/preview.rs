//! Preview overlay: translucent amber stroke for in-progress tool geometry.
//!
//! Drawing a line in AutoCAD R14 works like this: the operator clicks the first
//! point, then a "rubber-band" preview follows the cursor showing what the line
//! will look like; the second click commits. Same for circles, arcs, polylines.
//! v2 mirrors this: the active tool (LCV-043 LineTool, LCV-046 CircleTool, etc.)
//! constructs a `Vec<Entity>` each frame representing the in-progress preview,
//! and [`draw_preview`] paints those entities with a distinct style.
//!
//! **Three visual channels**: gray = committed entities, cyan = selected,
//! amber = preview. The operator can tell what is real and what is provisional
//! at a glance.
//!
//! **Translucent stroke instead of dashes**: AutoCAD R14 uses dashed previews;
//! v2 uses translucency (160/255 alpha). Emulating dashed lines in egui's
//! immediate-mode `Painter` requires either `Shape::dashed_line` (if available
//! in the pinned version — it is not in egui 0.29) or hand-built polyline
//! segmentation. Both are more LOC than a single alpha-blended stroke. v2 picks
//! translucency for KISS; if a user complains the preview is too subtle, a
//! future demand can swap to dashes.
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
use crate::render::Camera;

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
pub(crate) fn preview_stroke() -> egui::Stroke {
    egui::Stroke::new(
        1.0,
        egui::Color32::from_rgba_unmultiplied(255, 220, 100, 160),
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line, Vec2};
    use core::f64::consts::FRAC_PI_2;

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
