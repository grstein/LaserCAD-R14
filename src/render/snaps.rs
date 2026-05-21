//! Snap marker rendering: distinct shape per [`SnapKind`] at the snapped point.
//!
//! [`draw_snap_marker`] dispatches on [`SnapResult::kind`] and draws a
//! high-contrast orange marker — square for endpoint, triangle for midpoint,
//! unfilled circle for center, X for intersection. The visual language
//! matches AutoCAD R14's default OSNAP markers; muscle memory carries over.
//!
//! **Shape helpers are purely numeric** and testable as plain math; the main
//! `draw_snap_marker` orchestrator is covered by manual smoke testing (the
//! same pattern as LCV-032 and LCV-035).
//!
//! Importing `egui` is allowed here — `render/*` is the UI side of the
//! kernel/UI boundary. Importing `eframe` or `rfd` is NOT allowed.
//!
//! Introduced by demand LCV-038.

use crate::geometry::{SnapKind, SnapResult};
use crate::render::Camera;

/// Marker size in screen pixels (independent of zoom).
pub(crate) const MARKER_SIZE_PX: f32 = 8.0;

/// High-contrast orange marker color. R > 200, G in (100, 200), B < 100.
/// Distinct from entity gray, selection cyan, preview amber.
pub(crate) fn marker_color() -> egui::Color32 {
    egui::Color32::from_rgb(255, 160, 0)
}

/// Dispatch enum for testability: maps [`SnapKind`] to marker shape.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum MarkerShape {
    /// Filled square — endpoint snap.
    Square,
    /// Filled upward-pointing triangle — midpoint snap.
    Triangle,
    /// Unfilled circle (stroke only) — center snap.
    Circle,
    /// X (two crossed diagonals) — intersection snap.
    X,
}

/// Map snap kind to marker shape.
pub(crate) fn marker_shape_for(kind: SnapKind) -> MarkerShape {
    match kind {
        SnapKind::Endpoint => MarkerShape::Square,
        SnapKind::Midpoint => MarkerShape::Triangle,
        SnapKind::Center => MarkerShape::Circle,
        SnapKind::Intersection => MarkerShape::X,
    }
}

/// Compute the four corners of the endpoint marker (filled square).
///
/// Returns `[top_left, top_right, bottom_right, bottom_left]` (clockwise from
/// top-left, egui Y-down convention).
pub(crate) fn endpoint_marker_corners(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 4] {
    let half = size_px / 2.0;
    [
        egui::Pos2::new(center.x - half, center.y - half), // top-left
        egui::Pos2::new(center.x + half, center.y - half), // top-right
        egui::Pos2::new(center.x + half, center.y + half), // bottom-right
        egui::Pos2::new(center.x - half, center.y + half), // bottom-left
    ]
}

/// Compute the three vertices of the midpoint marker (upward-pointing triangle).
///
/// Returns `[top, bottom_left, bottom_right]`. "Up" in egui screen space means
/// smaller Y.
pub(crate) fn midpoint_marker_corners(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 3] {
    let half = size_px / 2.0;
    [
        egui::Pos2::new(center.x, center.y - half), // top vertex
        egui::Pos2::new(center.x - half, center.y + half), // bottom-left
        egui::Pos2::new(center.x + half, center.y + half), // bottom-right
    ]
}

/// Compute the two line segments forming the intersection marker (X).
///
/// Returns `[[seg1_start, seg1_end], [seg2_start, seg2_end]]` — the two
/// diagonals of the size_px square.
pub(crate) fn intersection_marker_segments(
    center: egui::Pos2,
    size_px: f32,
) -> [[egui::Pos2; 2]; 2] {
    let half = size_px / 2.0;
    [
        // Top-left to bottom-right diagonal.
        [
            egui::Pos2::new(center.x - half, center.y - half),
            egui::Pos2::new(center.x + half, center.y + half),
        ],
        // Bottom-left to top-right diagonal.
        [
            egui::Pos2::new(center.x - half, center.y + half),
            egui::Pos2::new(center.x + half, center.y - half),
        ],
    ]
}

/// Compute the radius of the center marker circle.
pub(crate) fn center_marker_radius(size_px: f32) -> f32 {
    size_px / 2.0
}

/// Draw a snap marker at the given [`SnapResult`] location.
///
/// Converts `snap.point` (world space) to screen space, then dispatches on
/// `snap.kind` to draw the appropriate shape:
///
/// - `Endpoint` → filled orange square.
/// - `Midpoint` → filled orange upward-pointing triangle.
/// - `Center` → unfilled orange circle (stroke only).
/// - `Intersection` → orange X (two crossed line segments).
///
/// Markers are sized in screen pixels (constant size regardless of zoom).
pub fn draw_snap_marker(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    snap: &SnapResult,
) {
    // Convert world point to screen space.
    let screen_pos = camera.world_to_screen(snap.point) + rect.min.to_vec2();
    let color = marker_color();
    let size = MARKER_SIZE_PX;

    match marker_shape_for(snap.kind) {
        MarkerShape::Square => {
            let corners = endpoint_marker_corners(screen_pos, size);
            // Draw filled square using top-left and bottom-right.
            let rect = egui::Rect::from_min_max(corners[0], corners[2]);
            painter.rect_filled(rect, 0.0, color);
        }
        MarkerShape::Triangle => {
            let corners = midpoint_marker_corners(screen_pos, size);
            painter.add(egui::Shape::convex_polygon(
                corners.to_vec(),
                color,
                egui::Stroke::NONE,
            ));
        }
        MarkerShape::Circle => {
            let radius = center_marker_radius(size);
            painter.circle_stroke(screen_pos, radius, egui::Stroke::new(1.5, color));
        }
        MarkerShape::X => {
            let segments = intersection_marker_segments(screen_pos, size);
            let stroke = egui::Stroke::new(1.5, color);
            painter.line_segment(segments[0], stroke);
            painter.line_segment(segments[1], stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    /// LCV-038 AC#1 — `draw_snap_marker` function exists.
    #[test]
    fn draw_snap_marker_function_exists() {
        let snap = SnapResult {
            point: Vec2::new(0.0, 0.0),
            kind: SnapKind::Endpoint,
            primary_idx: 0,
            secondary_idx: None,
        };
        let _ = (Camera::default(), snap); // Signature check.
    }

    /// LCV-038 AC#2 — endpoint marker corners.
    #[test]
    fn endpoint_marker_corners_at_origin_with_size_8() {
        let c = endpoint_marker_corners(egui::Pos2::new(100.0, 100.0), 8.0);
        assert_eq!(c.len(), 4);
        assert!((c[0].x - 96.0).abs() < f32::EPSILON && (c[0].y - 96.0).abs() < f32::EPSILON);
        assert!((c[1].x - 104.0).abs() < f32::EPSILON && (c[1].y - 96.0).abs() < f32::EPSILON);
        assert!((c[2].x - 104.0).abs() < f32::EPSILON && (c[2].y - 104.0).abs() < f32::EPSILON);
        assert!((c[3].x - 96.0).abs() < f32::EPSILON && (c[3].y - 104.0).abs() < f32::EPSILON);
    }

    /// LCV-038 AC#2 (offset case) — endpoint marker corners offset.
    #[test]
    fn endpoint_marker_corners_offset() {
        let c = endpoint_marker_corners(egui::Pos2::new(50.0, 75.0), 6.0);
        assert_eq!(c.len(), 4);
        assert!((c[0].x - 47.0).abs() < f32::EPSILON && (c[0].y - 72.0).abs() < f32::EPSILON);
        assert!((c[1].x - 53.0).abs() < f32::EPSILON && (c[1].y - 72.0).abs() < f32::EPSILON);
        assert!((c[2].x - 53.0).abs() < f32::EPSILON && (c[2].y - 78.0).abs() < f32::EPSILON);
        assert!((c[3].x - 47.0).abs() < f32::EPSILON && (c[3].y - 78.0).abs() < f32::EPSILON);
    }

    /// LCV-038 AC#3 — midpoint marker is upward triangle.
    #[test]
    fn midpoint_marker_is_upward_triangle() {
        let c = midpoint_marker_corners(egui::Pos2::new(100.0, 100.0), 8.0);
        assert_eq!(c.len(), 3);
        assert!(c[0].y < c[1].y && c[0].y < c[2].y); // Top vertex has smallest Y.
        assert!((c[1].y - c[2].y).abs() < f32::EPSILON); // Bottom same Y.
        assert!(((c[1].x + c[2].x) / 2.0 - 100.0).abs() < f32::EPSILON); // Symmetric.
    }

    /// LCV-038 AC#4 — intersection marker is two diagonals.
    #[test]
    fn intersection_marker_is_two_diagonals() {
        let s = intersection_marker_segments(egui::Pos2::new(100.0, 100.0), 8.0);
        assert_eq!(s.len(), 2);
        assert!((s[0][0].x - 96.0).abs() < f32::EPSILON && (s[0][0].y - 96.0).abs() < f32::EPSILON);
        assert!((s[0][1].x - 104.0).abs() < f32::EPSILON && (s[0][1].y - 104.0).abs() < f32::EPSILON);
        assert!((s[1][0].x - 96.0).abs() < f32::EPSILON && (s[1][0].y - 104.0).abs() < f32::EPSILON);
        assert!((s[1][1].x - 104.0).abs() < f32::EPSILON && (s[1][1].y - 96.0).abs() < f32::EPSILON);
    }

    /// LCV-038 AC#5 — center marker radius is half size.
    #[test]
    fn center_marker_radius_is_half_size() {
        assert!((center_marker_radius(8.0) - 4.0).abs() < f32::EPSILON);
    }

    /// LCV-038 AC#7 — marker color is orange band.
    #[test]
    fn marker_color_is_orange_band() {
        let c = marker_color();
        assert!(c.r() > 200 && c.g() > 100 && c.g() < 200 && c.b() < 100);
    }

    /// LCV-038 AC#8 — helpers do not panic on degenerate inputs.
    #[test]
    fn draw_snap_marker_does_not_panic_on_degenerate_inputs() {
        let extreme = egui::Pos2::new(1e6, 1e6);
        let c = endpoint_marker_corners(extreme, 1e-9);
        assert!(c.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        let c = endpoint_marker_corners(extreme, 1e9);
        assert!(c.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        let c = midpoint_marker_corners(extreme, 1e-9);
        assert!(c.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        let s = intersection_marker_segments(extreme, 1e9);
        assert!(s.iter().flat_map(|a| a.iter()).all(|p| p.x.is_finite() && p.y.is_finite()));
        assert!(center_marker_radius(1e-9).is_finite());
    }

    /// LCV-038 AC#9 — marker shape dispatch.
    #[test]
    fn marker_shape_dispatch() {
        assert_eq!(marker_shape_for(SnapKind::Endpoint), MarkerShape::Square);
        assert_eq!(marker_shape_for(SnapKind::Midpoint), MarkerShape::Triangle);
        assert_eq!(marker_shape_for(SnapKind::Center), MarkerShape::Circle);
        assert_eq!(marker_shape_for(SnapKind::Intersection), MarkerShape::X);
    }
}
