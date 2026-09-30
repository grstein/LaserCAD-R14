//! Snap resolve helper for the pointer pipeline (LCV-041).
//!
//! [`resolve_snap`] is a pure free function called once per frame from
//! [`super::App::update`]. It converts the cursor screen position to world
//! space, builds a tolerance in mm from the pixel threshold, and queries the
//! geometry snap engine over the current document entities.
//!
//! [`suppress_snap_if_disabled`] is the matching frame-level guard: it clears
//! any stale snap while snap mode is off.
//!
//! Lives in `src/app/snap.rs` (a sub-module of `app`) to keep `mod.rs` under
//! the 300-LOC hard cap; both functions are re-exported as
//! `lasercad::app::…`.

use crate::document::Entity;
use crate::geometry::{SnapEntity, SnapKinds, SnapResult, Vec2, snap_query};
use crate::render::Camera;

/// Pixel radius within which a snap candidate beats the raw cursor position.
const SNAP_TOLERANCE_PX: f64 = 12.0;

/// Compute the best snap candidate for the cursor at `cursor_screen`.
///
/// - Converts `cursor_screen` (absolute screen space) to viewport-local
///   coordinates by subtracting `rect.min`, then unprojects via
///   [`Camera::screen_to_world`].
/// - Converts `SNAP_TOLERANCE_PX` to mm using `camera.mm_per_px`.
/// - Delegates to the geometry snap engine with the active tool's `anchor`
///   (Perpendicular / Tangent) and the enabled `kinds` (LCV-161).
///
/// Returns `None` when `entities` is empty or no candidate lies within the
/// tolerance.
pub fn resolve_snap(
    cursor_screen: egui::Pos2,
    rect: egui::Rect,
    camera: &Camera,
    entities: &[Entity],
    anchor: Option<Vec2>,
    kinds: SnapKinds,
) -> Option<SnapResult> {
    let local = cursor_screen - rect.min.to_vec2();
    let world_pos = camera.screen_to_world(local);
    let tolerance_mm = SNAP_TOLERANCE_PX * camera.mm_per_px;
    let snappables: Vec<SnapEntity> = entities.iter().map(to_snap_entity).collect();
    snap_query(world_pos, tolerance_mm, &snappables, anchor, kinds)
}

/// Clear `active_snap` when snap is disabled.
///
/// Called at the top of each frame (before panel rendering) from
/// [`App::update_ui`](crate::app::App::update_ui) so that no snap marker is
/// rendered while snap is turned off, even if `active_snap` was set by a
/// previous frame. Extracted for testability (LCV-070 AC#16).
pub fn suppress_snap_if_disabled(snap_enabled: bool, active_snap: &mut Option<SnapResult>) {
    if !snap_enabled {
        *active_snap = None;
    }
}

fn to_snap_entity(e: &Entity) -> SnapEntity {
    match e {
        Entity::Line(l) => SnapEntity::Line(*l),
        Entity::Circle(c) => SnapEntity::Circle(*c),
        Entity::Arc(a) => SnapEntity::Arc(*a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle, EPSILON, Line};
    use crate::render::Camera;

    fn cam_1px() -> Camera {
        Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        }
    }

    fn full_rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0))
    }

    /// Empty entity slice → None.
    #[test]
    fn resolve_snap_empty_returns_none() {
        let result = resolve_snap(
            egui::Pos2::new(400.0, 300.0),
            full_rect(),
            &cam_1px(),
            &[],
            None,
            SnapKinds::default(),
        );
        assert!(result.is_none());
    }

    /// Cursor exactly on a line endpoint at world origin snaps to it.
    ///
    /// With mm_per_px=1 and viewport 800×600, world (0,0) → screen (400,300).
    #[test]
    fn resolve_snap_finds_endpoint_at_origin() {
        let line = Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)));
        // cursor at screen (400, 300) = world (0, 0)
        let result = resolve_snap(
            egui::Pos2::new(400.0, 300.0),
            full_rect(),
            &cam_1px(),
            &[line],
            None,
            SnapKinds::default(),
        );
        let snap = result.expect("should find endpoint at origin");
        assert!(
            snap.point.approx_eq(Vec2::new(0.0, 0.0), EPSILON),
            "snap point should be at origin, got {:?}",
            snap.point
        );
    }

    /// Cursor far from all entities (> SNAP_TOLERANCE_PX away) → None.
    #[test]
    fn resolve_snap_out_of_tolerance_returns_none() {
        // Circle center at (100, 100) world — screen (500, 200).
        // Cursor at (400, 300) = world (0, 0), ~141 mm away.
        let circle = Entity::Circle(Circle::new(Vec2::new(100.0, 100.0), 5.0));
        let result = resolve_snap(
            egui::Pos2::new(400.0, 300.0),
            full_rect(),
            &cam_1px(),
            &[circle],
            None,
            SnapKinds::default(),
        );
        assert!(result.is_none());
    }

    /// LCV-070 AC#16 — `suppress_snap_if_disabled` clears `active_snap` when
    /// snap mode is off, and leaves it alone when snap is on.
    #[test]
    fn suppress_snap_clears_when_disabled() {
        use crate::geometry::{SnapKind, SnapResult};
        let snap = Some(SnapResult {
            point: Vec2::new(1.0, 2.0),
            kind: SnapKind::Endpoint,
            primary_idx: 0,
            secondary_idx: None,
        });

        let mut active = snap;
        suppress_snap_if_disabled(false, &mut active);
        assert!(active.is_none());

        let mut active2 = snap;
        suppress_snap_if_disabled(true, &mut active2);
        assert!(active2.is_some());
    }

    /// rect.min offset is applied: non-zero rect origin shifts the conversion.
    #[test]
    fn resolve_snap_accounts_for_rect_min_offset() {
        // rect starts at (50, 30) — typical of a window with panel offsets.
        let rect =
            egui::Rect::from_min_size(egui::Pos2::new(50.0, 30.0), egui::Vec2::new(800.0, 600.0));
        let line = Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)));
        // World (0,0) maps to viewport-local (400, 300), absolute screen (450, 330).
        let result = resolve_snap(
            egui::Pos2::new(450.0, 330.0),
            rect,
            &cam_1px(),
            &[line],
            None,
            SnapKinds::default(),
        );
        let snap = result.expect("should find endpoint with rect offset");
        assert!(
            snap.point.approx_eq(Vec2::new(0.0, 0.0), EPSILON),
            "snap point should be at origin"
        );
    }
}
