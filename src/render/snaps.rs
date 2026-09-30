//! Snap marker rendering: distinct shape per [`SnapKind`] at the snapped point.
//!
//! [`draw_snap_marker`] dispatches on [`SnapResult::kind`] and draws a
//! high-contrast orange marker — square for endpoint, triangle for midpoint,
//! unfilled circle for center, X for intersection, and (LCV-161) diamond for
//! quadrant, right-angle mark for perpendicular, circle with a tangent bar for
//! tangent, hourglass for nearest. The visual language matches AutoCAD R14's
//! default OSNAP markers; muscle memory carries over.
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
    /// Hollow diamond — quadrant snap.
    Diamond,
    /// L with a corner box — perpendicular snap.
    RightAngle,
    /// Unfilled circle with a bar across its top — tangent snap.
    Tangent,
    /// Closed hourglass — nearest snap.
    Hourglass,
}

/// Map snap kind to marker shape.
pub(crate) fn marker_shape_for(kind: SnapKind) -> MarkerShape {
    match kind {
        SnapKind::Endpoint => MarkerShape::Square,
        SnapKind::Midpoint => MarkerShape::Triangle,
        SnapKind::Center => MarkerShape::Circle,
        SnapKind::Intersection => MarkerShape::X,
        SnapKind::Quadrant => MarkerShape::Diamond,
        SnapKind::Perpendicular => MarkerShape::RightAngle,
        SnapKind::Tangent => MarkerShape::Tangent,
        SnapKind::Nearest => MarkerShape::Hourglass,
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

/// Quadrant diamond vertices: `[top, right, bottom, left]`.
pub(crate) fn quadrant_marker_corners(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 4] {
    let h = size_px / 2.0;
    [
        egui::Pos2::new(center.x, center.y - h),
        egui::Pos2::new(center.x + h, center.y),
        egui::Pos2::new(center.x, center.y + h),
        egui::Pos2::new(center.x - h, center.y),
    ]
}

/// Perpendicular mark: the L `[top_left, bottom_left, bottom_right]` and the
/// corner box `[left_mid, center, bottom_mid]` closing on its two legs.
pub(crate) fn perpendicular_marker_paths(center: egui::Pos2, size_px: f32) -> [[egui::Pos2; 3]; 2] {
    let h = size_px / 2.0;
    let (l, r, t, b) = (center.x - h, center.x + h, center.y - h, center.y + h);
    [
        [
            egui::Pos2::new(l, t),
            egui::Pos2::new(l, b),
            egui::Pos2::new(r, b),
        ],
        [
            egui::Pos2::new(l, center.y),
            center,
            egui::Pos2::new(center.x, b),
        ],
    ]
}

/// Tangent bar: a horizontal segment touching the top of the
/// [`center_marker_radius`] circle, `size_px` long.
pub(crate) fn tangent_marker_bar(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 2] {
    let h = size_px / 2.0;
    let y = center.y - center_marker_radius(size_px);
    [
        egui::Pos2::new(center.x - h, y),
        egui::Pos2::new(center.x + h, y),
    ]
}

/// Hourglass vertices in path order: `[top_left, top_right, bottom_left,
/// bottom_right]`; closing the path draws both diagonals.
pub(crate) fn nearest_marker_corners(center: egui::Pos2, size_px: f32) -> [egui::Pos2; 4] {
    let h = size_px / 2.0;
    [
        egui::Pos2::new(center.x - h, center.y - h),
        egui::Pos2::new(center.x + h, center.y - h),
        egui::Pos2::new(center.x - h, center.y + h),
        egui::Pos2::new(center.x + h, center.y + h),
    ]
}

/// Stroke width, in points, of a stroked glyph.
const GLYPH_STROKE_PT: f32 = 1.5;

/// How much wider, in points, the dark edge under a glyph is: 1 pt each side
/// (LCV-164 AC 2).
const EDGE_EXTRA_PT: f32 = 2.0;

/// The shapes of one pass of `shape`'s glyph at `pos`, in `color`, every
/// stroke `widen` points wider than the glyph's own: `0` for the glyph,
/// [`EDGE_EXTRA_PT`] for the edge under it. A filled glyph gets an outline
/// of width `widen`, so the glyph pass has none.
pub(crate) fn glyph_shapes(
    shape: MarkerShape,
    pos: egui::Pos2,
    color: egui::Color32,
    widen: f32,
) -> Vec<egui::Shape> {
    use egui::Shape;
    let size = MARKER_SIZE_PX;
    let stroke = egui::Stroke::new(GLYPH_STROKE_PT + widen, color);
    let outline = egui::Stroke::new(widen, color);
    match shape {
        MarkerShape::Square => {
            let c = endpoint_marker_corners(pos, size);
            let rect = egui::Rect::from_min_max(c[0], c[2]);
            vec![Shape::Rect(egui::epaint::RectShape::new(
                rect, 0.0, color, outline,
            ))]
        }
        MarkerShape::Triangle => {
            let corners = midpoint_marker_corners(pos, size).to_vec();
            vec![Shape::convex_polygon(corners, color, outline)]
        }
        MarkerShape::Circle => vec![Shape::circle_stroke(
            pos,
            center_marker_radius(size),
            stroke,
        )],
        MarkerShape::X => intersection_marker_segments(pos, size)
            .into_iter()
            .map(|seg| Shape::line_segment(seg, stroke))
            .collect(),
        MarkerShape::Diamond => {
            vec![Shape::closed_line(
                quadrant_marker_corners(pos, size).to_vec(),
                stroke,
            )]
        }
        MarkerShape::RightAngle => perpendicular_marker_paths(pos, size)
            .into_iter()
            .map(|path| Shape::line(path.to_vec(), stroke))
            .collect(),
        MarkerShape::Tangent => vec![
            Shape::circle_stroke(pos, center_marker_radius(size), stroke),
            Shape::line_segment(tangent_marker_bar(pos, size), stroke),
        ],
        MarkerShape::Hourglass => {
            vec![Shape::closed_line(
                nearest_marker_corners(pos, size).to_vec(),
                stroke,
            )]
        }
    }
}

/// Draw a snap marker at the given [`SnapResult`] location.
///
/// Converts `snap.point` (world space) to screen space and paints the kind's
/// glyph ([`glyph_shapes`]) twice: first the edge in
/// [`crate::render::palette::SNAP_EDGE`], [`EDGE_EXTRA_PT`] wider, then the
/// glyph in [`marker_color`] (LCV-164 AC 2):
///
/// - `Endpoint` → filled square; `Midpoint` → filled upward triangle;
///   `Center` → unfilled circle; `Intersection` → X.
/// - `Quadrant` → diamond; `Perpendicular` → right-angle mark; `Tangent` →
///   circle with a tangent bar; `Nearest` → hourglass (all stroked).
///
/// Markers are sized in screen points (constant size regardless of zoom).
pub fn draw_snap_marker(
    painter: &egui::Painter,
    rect: egui::Rect,
    camera: &Camera,
    snap: &SnapResult,
) {
    let pos = camera.world_to_screen(snap.point) + rect.min.to_vec2();
    let shape = marker_shape_for(snap.kind);
    let edge = crate::render::palette::SNAP_EDGE;
    painter.extend(glyph_shapes(shape, pos, edge, EDGE_EXTRA_PT));
    painter.extend(glyph_shapes(shape, pos, marker_color(), 0.0));
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
        assert!(
            (s[0][1].x - 104.0).abs() < f32::EPSILON && (s[0][1].y - 104.0).abs() < f32::EPSILON
        );
        assert!(
            (s[1][0].x - 96.0).abs() < f32::EPSILON && (s[1][0].y - 104.0).abs() < f32::EPSILON
        );
        assert!(
            (s[1][1].x - 104.0).abs() < f32::EPSILON && (s[1][1].y - 96.0).abs() < f32::EPSILON
        );
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
        assert!(
            s.iter()
                .flat_map(|a| a.iter())
                .all(|p| p.x.is_finite() && p.y.is_finite())
        );
        assert!(center_marker_radius(1e-9).is_finite());
    }

    /// LCV-038 AC#9 — marker shape dispatch.
    #[test]
    fn marker_shape_dispatch() {
        assert_eq!(marker_shape_for(SnapKind::Endpoint), MarkerShape::Square);
        assert_eq!(marker_shape_for(SnapKind::Midpoint), MarkerShape::Triangle);
        assert_eq!(marker_shape_for(SnapKind::Center), MarkerShape::Circle);
        assert_eq!(marker_shape_for(SnapKind::Intersection), MarkerShape::X);
        assert_eq!(marker_shape_for(SnapKind::Quadrant), MarkerShape::Diamond);
        assert_eq!(
            marker_shape_for(SnapKind::Perpendicular),
            MarkerShape::RightAngle
        );
        assert_eq!(marker_shape_for(SnapKind::Tangent), MarkerShape::Tangent);
        assert_eq!(marker_shape_for(SnapKind::Nearest), MarkerShape::Hourglass);
    }

    // ── LCV-161 AC7: painted glyphs per kind ─────────────────────────────

    /// The glyph pass of [`paint_all`]: every shape in the marker colour
    /// (LCV-164 adds an edge pass under it and a label over it).
    fn paint(kind: SnapKind) -> (Vec<egui::Shape>, egui::Pos2) {
        let (shapes, c) = paint_all(kind);
        let glyph = shapes
            .into_iter()
            .filter(|s| ink(s).is_some_and(|(color, _)| color == marker_color()))
            .collect();
        (glyph, c)
    }

    /// The colour and stroke width a marker shape paints with: its stroke
    /// when it has one, else its fill at width 0. `None` for text.
    fn ink(shape: &egui::Shape) -> Option<(egui::Color32, f32)> {
        let pick = |fill: egui::Color32, color: egui::Color32, width: f32| {
            if width > 0.0 {
                (color, width)
            } else {
                (fill, 0.0)
            }
        };
        match shape {
            egui::Shape::Rect(r) => Some(pick(r.fill, r.stroke.color, r.stroke.width)),
            egui::Shape::Circle(c) => Some(pick(c.fill, c.stroke.color, c.stroke.width)),
            egui::Shape::Path(p) => Some(pick(p.fill, solid(&p.stroke.color), p.stroke.width)),
            egui::Shape::LineSegment { stroke, .. } => Some((solid(&stroke.color), stroke.width)),
            _ => None,
        }
    }

    /// Paint `kind` at world (0, 0) through `draw_snap_marker` and return
    /// the flattened shapes plus the marker's screen centre.
    fn paint_all(kind: SnapKind) -> (Vec<egui::Shape>, egui::Pos2) {
        let camera = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));
        let snap = SnapResult {
            point: Vec2::new(0.0, 0.0),
            kind,
            primary_idx: 0,
            secondary_idx: None,
        };
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("lcv161-snap"),
            ));
            draw_snap_marker(&painter, rect, &camera, &snap);
        });
        fn flatten(shape: egui::Shape, out: &mut Vec<egui::Shape>) {
            match shape {
                egui::Shape::Vec(v) => v.into_iter().for_each(|s| flatten(s, out)),
                egui::Shape::Noop => {}
                s => out.push(s),
            }
        }
        let mut shapes = Vec::new();
        for clipped in out.shapes {
            flatten(clipped.shape, &mut shapes);
        }
        (shapes, camera.world_to_screen(Vec2::new(0.0, 0.0)))
    }

    fn solid(c: &egui::epaint::ColorMode) -> egui::Color32 {
        match c {
            egui::epaint::ColorMode::Solid(c) => *c,
            egui::epaint::ColorMode::UV(_) => panic!("marker strokes are solid"),
        }
    }

    fn near(a: egui::Pos2, b: egui::Pos2) -> bool {
        (a - b).length() < 1e-3
    }

    /// The single stroked path of `shapes`, asserting the marker colour.
    fn only_path(shapes: &[egui::Shape]) -> egui::epaint::PathShape {
        assert_eq!(shapes.len(), 1, "one path expected, got {shapes:?}");
        match &shapes[0] {
            egui::Shape::Path(p) => {
                assert_eq!(solid(&p.stroke.color), marker_color());
                p.clone()
            }
            other => panic!("expected a path, got {other:?}"),
        }
    }

    /// AC7 — Quadrant: a hollow 4-vertex diamond, one vertex on each axis.
    #[test]
    fn quadrant_paints_a_diamond() {
        let (shapes, c) = paint(SnapKind::Quadrant);
        let p = only_path(&shapes);
        assert!(p.closed);
        assert_eq!(p.points.len(), 4);
        let h = MARKER_SIZE_PX / 2.0;
        for v in [
            egui::pos2(c.x, c.y - h),
            egui::pos2(c.x + h, c.y),
            egui::pos2(c.x, c.y + h),
            egui::pos2(c.x - h, c.y),
        ] {
            assert!(p.points.iter().any(|q| near(*q, v)), "missing vertex {v:?}");
        }
    }

    /// AC7 — Perpendicular: an L (two perpendicular legs) plus the inner
    /// right-angle box, all in the marker colour.
    #[test]
    fn perpendicular_paints_a_right_angle_mark() {
        let (shapes, _) = paint(SnapKind::Perpendicular);
        assert_eq!(shapes.len(), 2, "L plus corner box, got {shapes:?}");
        for s in &shapes {
            let egui::Shape::Path(p) = s else {
                panic!("expected paths, got {s:?}")
            };
            assert_eq!(solid(&p.stroke.color), marker_color());
            assert!(!p.closed);
            assert_eq!(p.points.len(), 3);
            let (a, b) = (p.points[0] - p.points[1], p.points[2] - p.points[1]);
            assert!(a.dot(b).abs() < 1e-3, "legs must meet at a right angle");
            assert!(a.length() > 0.0 && b.length() > 0.0);
        }
        let (egui::Shape::Path(l), egui::Shape::Path(corner)) = (&shapes[0], &shapes[1]) else {
            unreachable!()
        };
        // The corner box closes on the L's two legs.
        let knee = l.points[1];
        assert!(
            (corner.points[0].x - knee.x).abs() < 1e-3,
            "box starts on one leg"
        );
        assert!(
            (corner.points[2].y - knee.y).abs() < 1e-3,
            "box ends on the other"
        );
    }

    /// AC7 — Tangent: a stroked circle plus a bar touching its top.
    #[test]
    fn tangent_paints_a_circle_with_a_tangent_bar() {
        let (shapes, c) = paint(SnapKind::Tangent);
        assert_eq!(shapes.len(), 2, "circle plus bar, got {shapes:?}");
        let circle = shapes.iter().find_map(|s| match s {
            egui::Shape::Circle(ci) => Some(*ci),
            _ => None,
        });
        let circle = circle.expect("a circle");
        assert_eq!(circle.stroke.color, marker_color());
        assert!(near(circle.center, c));
        let bar = shapes.iter().find_map(|s| match s {
            egui::Shape::LineSegment { points, stroke } => Some((*points, stroke.clone())),
            _ => None,
        });
        let (bar, stroke) = bar.expect("a tangent bar");
        assert_eq!(solid(&stroke.color), marker_color());
        let top = c.y - circle.radius;
        assert!((bar[0].y - top).abs() < 1e-3 && (bar[1].y - top).abs() < 1e-3);
        assert!(
            bar[0].x < c.x && bar[1].x > c.x,
            "the bar spans the circle's top"
        );
    }

    /// AC7 — Nearest: a closed hourglass (top edge, diagonal, bottom edge,
    /// diagonal).
    #[test]
    fn nearest_paints_an_hourglass() {
        let (shapes, c) = paint(SnapKind::Nearest);
        let p = only_path(&shapes);
        assert!(p.closed);
        assert_eq!(p.points.len(), 4);
        let [tl, tr, bl, br] = [p.points[0], p.points[1], p.points[2], p.points[3]];
        assert!((tl.y - tr.y).abs() < 1e-3 && tl.y < c.y, "top edge");
        assert!((bl.y - br.y).abs() < 1e-3 && bl.y > c.y, "bottom edge");
        assert!((tl.x - bl.x).abs() < 1e-3 && tl.x < c.x, "left side");
        assert!((tr.x - br.x).abs() < 1e-3 && tr.x > c.x, "right side");
    }

    /// AC7 — the four LCV-016 kinds keep their shapes.
    #[test]
    fn existing_kinds_keep_their_shapes() {
        let (s, _) = paint(SnapKind::Endpoint);
        assert!(matches!(&s[..], [egui::Shape::Rect(r)] if r.fill == marker_color()));
        let (s, _) = paint(SnapKind::Midpoint);
        assert!(
            matches!(&s[..], [egui::Shape::Path(p)] if p.fill == marker_color() && p.points.len() == 3)
        );
        let (s, _) = paint(SnapKind::Center);
        assert!(matches!(&s[..], [egui::Shape::Circle(c)] if c.stroke.color == marker_color()));
        let (s, _) = paint(SnapKind::Intersection);
        assert_eq!(s.len(), 2);
        assert!(
            s.iter()
                .all(|x| matches!(x, egui::Shape::LineSegment { .. }))
        );
    }

    // ── LCV-164 AC 2, AC 3: dark edge and kind label ─────────────────────

    const ALL_KINDS: [SnapKind; 8] = [
        SnapKind::Endpoint,
        SnapKind::Midpoint,
        SnapKind::Center,
        SnapKind::Intersection,
        SnapKind::Quadrant,
        SnapKind::Perpendicular,
        SnapKind::Tangent,
        SnapKind::Nearest,
    ];

    /// LCV-164 AC 2 — every glyph shape has an edge twin of the same kind in
    /// `SNAP_EDGE` (the canvas background), ≥1 pt wider, and every edge is
    /// painted before the first glyph shape.
    #[test]
    fn ac2_every_glyph_sits_on_a_wider_canvas_coloured_edge() {
        use crate::render::palette::SNAP_EDGE;
        assert_eq!(SNAP_EDGE, crate::ui::CANVAS_BG);
        for kind in ALL_KINDS {
            let (shapes, _) = paint_all(kind);
            let inked: Vec<(usize, &egui::Shape, egui::Color32, f32)> = shapes
                .iter()
                .enumerate()
                .filter_map(|(i, s)| ink(s).map(|(c, w)| (i, s, c, w)))
                .collect();
            let edges: Vec<_> = inked.iter().filter(|x| x.2 == SNAP_EDGE).collect();
            let glyph: Vec<_> = inked.iter().filter(|x| x.2 == marker_color()).collect();
            assert!(!glyph.is_empty(), "{kind:?}: control, the glyph paints");
            assert_eq!(
                edges.len(),
                glyph.len(),
                "{kind:?}: one edge per glyph shape"
            );
            let last_edge = edges.iter().map(|x| x.0).max().unwrap_or(usize::MAX);
            let first_glyph = glyph.iter().map(|x| x.0).min().unwrap_or(0);
            assert!(last_edge < first_glyph, "{kind:?}: the edge paints first");
            for (edge, glyph) in edges.iter().zip(&glyph) {
                assert_eq!(
                    core::mem::discriminant(edge.1),
                    core::mem::discriminant(glyph.1),
                    "{kind:?}: the edge has the glyph's shape"
                );
                assert!(
                    edge.3 >= glyph.3 + 1.0,
                    "{kind:?}: edge {} pt vs glyph {} pt",
                    edge.3,
                    glyph.3
                );
            }
        }
    }

    /// LCV-164 AC 3 — every glyph is followed by one text shape with the
    /// kind's lower-case name, in the `snap` colour, whose rect does not
    /// cover the snap point.
    #[test]
    fn ac3_every_glyph_is_labelled_clear_of_the_point() {
        let names = [
            "endpoint",
            "midpoint",
            "center",
            "intersection",
            "quadrant",
            "perpendicular",
            "tangent",
            "nearest",
        ];
        for (kind, name) in ALL_KINDS.into_iter().zip(names) {
            let (shapes, point) = paint_all(kind);
            let texts: Vec<(usize, &egui::epaint::TextShape)> = shapes
                .iter()
                .enumerate()
                .filter_map(|(i, s)| match s {
                    egui::Shape::Text(t) => Some((i, t)),
                    _ => None,
                })
                .collect();
            let [(at, label)] = texts[..] else {
                panic!("{kind:?}: one label expected, got {texts:?}");
            };
            assert_eq!(label.galley.text(), name, "{kind:?}");
            assert!(at == shapes.len() - 1, "{kind:?}: the label paints last");
            let rect = label.visual_bounding_rect();
            assert!(
                !rect.expand(1.0).contains(point),
                "{kind:?}: label {rect:?} covers the point {point:?}"
            );
            let color = label.fallback_color;
            assert!(
                color == marker_color() || label.override_text_color == Some(marker_color()),
                "{kind:?}: label in the snap colour, got {color:?}"
            );
        }
    }
}
