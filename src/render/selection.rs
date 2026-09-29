//! Selection highlight rendering: draw a colored halo over selected entities.
//!
//! Phase-4 tools (SelectTool, MoveTool, DeleteTool, TrimTool) operate on the
//! document's [`Selection`]. The operator needs to see which entities are
//! selected before confirming an action ("the next delete will remove the right
//! things"). AutoCAD R14 uses colored halos plus grip handles; v2 takes the
//! lightweight route — a thick, semi-transparent blue/cyan stroke drawn over
//! each selected entity.
//!
//! Drawing order: caller invokes [`draw_selection_highlight`] AFTER
//! [`crate::render::draw_entities`] so the halo overlays the normal entity
//! stroke. The semi-transparent halo lets the underlying layer-colored stroke
//! show through, producing a "double-stroke" visual that reads clearly as
//! "selected".
//!
//! Out-of-range indices (stale selection after a `DeleteEntities` command) are
//! silently skipped rather than causing a panic.
//!
//! Introduced by demand LCV-036.

use crate::document::{Entity, Selection};
use crate::render::Camera;

/// Halo stroke: 3-px cyan-blue at ~70% alpha.
///
/// Thicker than the normal entity stroke (1 px, layer color) and translucent
/// so the underlying stroke remains visible through the halo, producing a
/// "double-stroke" visual.
///
/// Exposed as `pub(crate)` for testability (AC#5, AC#6).
pub(crate) fn halo_stroke() -> egui::Stroke {
    egui::Stroke::new(
        3.0,
        egui::Color32::from_rgba_unmultiplied(64, 160, 255, 180),
    )
}

/// Draw a selection highlight (halo) over each selected entity.
///
/// Iterates `selection.iter()` and draws a halo using the same dispatch as
/// [`crate::render::draw_entities`] (line → line_segment, circle →
/// circle_stroke, arc → arc_polyline + line segments) but using
/// [`halo_stroke`] instead of the default stroke.
///
/// **Out-of-range indices** (stale selection after entity deletion) are
/// silently skipped; no panic.
///
/// **Order of operations**: caller must invoke this function AFTER
/// [`crate::render::draw_entities`] so the halo paints over the normal entity
/// stroke.
///
/// `rect` is the viewport's screen-space rectangle; `rect.min` is added to
/// every projected point so shapes land in the correct painter clip rect.
pub fn draw_selection_highlight(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    entities: &[Entity],
    selection: &Selection,
) {
    let stroke = halo_stroke();

    for idx in selection.iter() {
        // Out-of-range indices are silently skipped (AC#3).
        if let Some(entity) = entities.get(idx) {
            draw_entity_with_stroke(painter, rect, camera, entity, stroke);
        }
    }
}

/// Draw a single entity with the specified stroke.
///
/// Shared dispatch logic between normal entity rendering and selection
/// highlight rendering. Exposed as `pub(crate)` for testability and potential
/// reuse.
pub(crate) fn draw_entity_with_stroke(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    entity: &Entity,
    stroke: egui::Stroke,
) {
    match entity {
        Entity::Line(line) => {
            let p1 = world_to_screen_offset(rect, camera, line.p1);
            let p2 = world_to_screen_offset(rect, camera, line.p2);
            painter.line_segment([p1, p2], stroke);
        }
        Entity::Circle(circle) => {
            let center = world_to_screen_offset(rect, camera, circle.center);
            let radius_px = (circle.r / camera.mm_per_px) as f32;
            painter.circle_stroke(center, radius_px, stroke);
        }
        Entity::Arc(arc) => {
            // Use the same arc_polyline function from entities.rs
            let points = crate::render::arc_polyline(arc, 64);
            for i in 0..points.len().saturating_sub(1) {
                let p1 = world_to_screen_offset(rect, camera, points[i]);
                let p2 = world_to_screen_offset(rect, camera, points[i + 1]);
                painter.line_segment([p1, p2], stroke);
            }
        }
    }
}

/// Iterator over selected entity indices and references that are in range.
///
/// Returns only the valid (in-range) selected entities as `(idx, &Entity)`
/// pairs. Out-of-range indices are silently skipped.
///
/// Exposed as `pub(crate)` for testability (AC#4).
#[allow(dead_code)]
pub(crate) fn selected_entity_indices<'a>(
    entities: &'a [Entity],
    selection: &'a Selection,
) -> impl Iterator<Item = (usize, &'a Entity)> + 'a {
    selection
        .iter()
        .filter_map(move |idx| entities.get(idx).map(|entity| (idx, entity)))
}

/// Convert a world-space point to screen space, offset by `rect.min`.
///
/// Private helper: `camera.world_to_screen(w) + rect.min.to_vec2()`.
fn world_to_screen_offset(
    rect: egui::Rect,
    camera: &Camera,
    w: crate::geometry::Vec2,
) -> egui::Pos2 {
    camera.world_to_screen(w) + rect.min.to_vec2()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line, Vec2};
    use core::f64::consts::FRAC_PI_2;

    /// AC#1 — `draw_selection_highlight` exists and type-checks with empty
    /// inputs.
    #[test]
    fn draw_selection_highlight_function_exists() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_selection"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let camera = Camera::default();
            let entities = vec![];
            let selection = Selection::default();

            // Should compile and not panic.
            draw_selection_highlight(&painter, rect, &camera, &entities, &selection);
        });
    }

    /// AC#2 — empty selection visits no entities.
    #[test]
    fn empty_selection_visits_no_entities() {
        let entities = vec![Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
        ))];
        let selection = Selection::default();

        let visited: Vec<_> = selected_entity_indices(&entities, &selection).collect();
        assert_eq!(visited.len(), 0);
    }

    /// AC#3 — out-of-range indices are skipped without panic.
    #[test]
    fn out_of_range_indices_are_skipped() {
        let entities = vec![Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
        ))];
        let mut selection = Selection::default();
        selection.add(5); // Out of range.

        let visited: Vec<_> = selected_entity_indices(&entities, &selection).collect();
        assert_eq!(visited.len(), 0);

        // Also verify draw_selection_highlight doesn't panic.
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_out_of_range"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            let camera = Camera::default();

            draw_selection_highlight(&painter, rect, &camera, &entities, &selection);
        });
    }

    /// AC#4 — `selected_entity_indices` iterator yields correct pairs.
    #[test]
    fn selected_indices_iterator_yields_correct_pairs() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let circle = Circle::new(Vec2::new(50.0, 50.0), 20.0);
        let arc = Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true);

        let entities = vec![Entity::Line(line), Entity::Circle(circle), Entity::Arc(arc)];

        let mut selection = Selection::default();
        selection.add(0);
        selection.add(2);

        let mut visited: Vec<_> = selected_entity_indices(&entities, &selection).collect();
        // Sort by index because HashSet iteration order is not guaranteed.
        visited.sort_by_key(|(idx, _)| *idx);

        assert_eq!(visited.len(), 2);
        assert_eq!(visited[0].0, 0);
        assert_eq!(visited[1].0, 2);

        // Check the entity variants match.
        if let Entity::Line(l) = visited[0].1 {
            assert_eq!(l.p1, line.p1);
            assert_eq!(l.p2, line.p2);
        } else {
            panic!("Expected Line at index 0");
        }

        if let Entity::Arc(a) = visited[1].1 {
            assert_eq!(a.center, arc.center);
            assert_eq!(a.r, arc.r);
        } else {
            panic!("Expected Arc at index 2");
        }
    }

    /// AC#5 — halo stroke is thicker than entity stroke.
    #[test]
    fn halo_stroke_is_thicker_than_entity_stroke() {
        let halo = halo_stroke();
        let entity_width = crate::render::PaintOptions::default().stroke_width;

        assert!(
            halo.width > entity_width,
            "halo.width={}, entity_width={entity_width}",
            halo.width,
        );
        assert!(halo.width > 1.0);
    }

    /// AC#6 — halo color is translucent and colored (not gray).
    #[test]
    fn halo_color_is_translucent_and_colored() {
        let halo = halo_stroke();
        let color = halo.color;

        // Translucent: alpha in (0, 255).
        assert!(color.a() > 0, "alpha should be > 0");
        assert!(color.a() < 255, "alpha should be < 255 (translucent)");

        // Not gray: R ≠ G or G ≠ B.
        let not_gray = color.r() != color.g() || color.g() != color.b();
        assert!(
            not_gray,
            "halo color should not be gray (r={}, g={}, b={})",
            color.r(),
            color.g(),
            color.b()
        );
    }
}
