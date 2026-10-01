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

use crate::document::{Document, Entity, Selection};
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
        3.0_f32,
        egui::Color32::from_rgba_unmultiplied(64, 160, 255, 180),
    )
}

/// Draw a selection highlight (halo) over each selected entity.
///
/// Iterates `selection.iter()` and strokes each entity with [`halo_stroke`]
/// through [`crate::render::stroke_entity`], as one shape per entity so the
/// translucent halo has no darker joints (LCV-164 AC 6).
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
            crate::render::stroke_entity(painter, rect, camera, entity, stroke);
        }
    }
}

/// Repaint `doc.entities[index]` in its own layer colour at the `hover`
/// width (LCV-163 AC 3): the entity a click would pick. Painted after the
/// selection halo so a hovered, selected entity still reads as hovered. An
/// out-of-range index paints nothing.
pub fn draw_hover(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    doc: &Document,
    index: usize,
) {
    if let Some(entity) = doc.entities.get(index) {
        let [r, g, b] = doc.layer_color(index);
        let color = egui::Color32::from_rgb(r, g, b);
        let stroke = egui::Stroke::new(crate::render::palette::HOVER_WIDTH_PT, color);
        crate::render::stroke_entity(painter, rect, camera, entity, stroke);
    }
}

/// Iterator over selected entity indices and references that are in range.
///
/// Returns only the valid (in-range) selected entities as `(idx, &Entity)`
/// pairs. Out-of-range indices are silently skipped.
///
/// Exposed as `pub(crate)` for testability (AC#4).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "only the AC#4 unit tests call it")
)]
pub(crate) fn selected_entity_indices<'a>(
    entities: &'a [Entity],
    selection: &'a Selection,
) -> impl Iterator<Item = (usize, &'a Entity)> + 'a {
    selection
        .iter()
        .filter_map(move |idx| entities.get(idx).map(|entity| (idx, entity)))
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

    /// Every `LineSegment` painted by `draw_hover(index)` over a one-line
    /// document, as (width, colour).
    fn hover_segments(index: usize) -> (Vec<(f32, egui::Color32)>, [u8; 3]) {
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
        )));
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test_hover"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
            draw_hover(&painter, rect, &Camera::default(), &doc, index);
        });
        let segs = out
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::LineSegment { stroke, .. } => match stroke.color {
                    egui::epaint::ColorMode::Solid(k) => Some((stroke.width, k)),
                    egui::epaint::ColorMode::UV(_) => None,
                },
                _ => None,
            })
            .collect();
        (segs, doc.layer_color(0))
    }

    /// LCV-163 AC 3 — the hovered entity is painted once, in its layer
    /// colour, at the `hover` width.
    #[test]
    fn draw_hover_paints_the_entity_in_its_layer_colour_and_hover_width() {
        let (segs, [r, g, b]) = hover_segments(0);
        let expected = (
            crate::render::palette::HOVER_WIDTH_PT,
            egui::Color32::from_rgb(r, g, b),
        );
        assert_eq!(segs, vec![expected]);
    }

    /// LCV-163 AC 3 — an out-of-range index paints nothing.
    #[test]
    fn draw_hover_skips_an_out_of_range_index() {
        assert!(hover_segments(1).0.is_empty());
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
