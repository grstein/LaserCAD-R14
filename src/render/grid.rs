//! Responsive grid renderer.
//!
//! Draws a world-coordinate aligned orthogonal grid whose minor spacing
//! snaps to a 1-2-5 decade ladder so the screen spacing stays readable at
//! every zoom level.

use crate::geometry::Vec2;
use crate::render::Camera;

/// Minimum desired screen spacing between minor grid lines (pixels).
const MIN_SCREEN_PX: f64 = 8.0;

/// Clamp exponent for the spacing ladder (`10^k` where `k` is clamped).
const K_MIN: i32 = -6;
const K_MAX: i32 = 6;

/// Return the minor grid spacing in millimetres for the current zoom.
///
/// Chooses the smallest member of `{10^k, 2·10^k, 5·10^k}` such that
/// `spacing_mm / mm_per_px >= MIN_SCREEN_PX`.  The exponent `k` is
/// clamped to `[K_MIN, K_MAX]`.
pub(crate) fn pick_minor_spacing_mm(mm_per_px: f64) -> f64 {
    debug_assert!(mm_per_px > 0.0, "mm_per_px must be > 0");
    let mut k = K_MIN;
    loop {
        let decade = 10_f64.powi(k);
        for m in [1.0, 2.0, 5.0] {
            let spacing = m * decade;
            if spacing / mm_per_px >= MIN_SCREEN_PX {
                return spacing;
            }
        }
        if k >= K_MAX {
            return 5.0 * decade; // largest clamped value
        }
        k += 1;
    }
}

/// Fixed major step: every 10th minor line is major.
pub(crate) fn pick_major_step(_minor_spacing_mm: f64) -> u32 {
    10
}

/// Draw the grid into the viewport.
///
/// `rect` is the viewport's screen-space rectangle — in *global* (window)
/// screen coordinates, since that is what every caller in this tree holds.
/// `Camera::screen_to_world` / `Camera::world_to_screen` operate on
/// *viewport-local* coordinates (origin at the canvas's own top-left), so
/// every global position is converted by subtracting `rect.min` exactly once
/// before it reaches the camera, and every local result the camera hands
/// back is converted the other way — `+ rect.min` — exactly once before it
/// reaches the painter. LCV-137 AC 3.
pub fn draw_grid(painter: &egui::Painter, rect: egui::Rect, camera: &Camera) {
    let minor_mm = pick_minor_spacing_mm(camera.mm_per_px);
    let major_step = pick_major_step(minor_mm) as i64;

    // Visible world bounds. `rect.min`/`rect.max` are global; subtract the
    // viewport's own origin once to get the local corners `screen_to_world`
    // expects (LCV-137 AC 3 — before this fix this line passed the global
    // rect straight through, drifting the visible grid by the toolbar width
    // and menubar height whenever the viewport did not start at the
    // window's top-left corner).
    let local_tl = rect.min - rect.min.to_vec2();
    let local_br = rect.max - rect.min.to_vec2();
    let world_tl = camera.screen_to_world(local_tl);
    let world_br = camera.screen_to_world(local_br);
    let world_left = world_tl.x.min(world_br.x);
    let world_right = world_tl.x.max(world_br.x);
    let world_bottom = world_tl.y.min(world_br.y);
    let world_top = world_tl.y.max(world_br.y);

    let offset = rect.min.to_vec2();
    let minor_stroke = egui::Stroke::new(0.5, egui::Color32::from_gray(48));
    let major_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(96));

    // Vertical lines (constant world X).
    let mut x = (world_left / minor_mm).floor() * minor_mm;
    while x <= world_right + minor_mm * 0.5 {
        let idx = (x / minor_mm).round() as i64;
        let stroke = if idx % major_step == 0 {
            major_stroke
        } else {
            minor_stroke
        };
        let p0 = camera.world_to_screen(Vec2::new(x, world_bottom)) + offset;
        let p1 = camera.world_to_screen(Vec2::new(x, world_top)) + offset;
        painter.line_segment([p0, p1], stroke);
        x += minor_mm;
    }

    // Horizontal lines (constant world Y).
    let mut y = (world_bottom / minor_mm).floor() * minor_mm;
    while y <= world_top + minor_mm * 0.5 {
        let idx = (y / minor_mm).round() as i64;
        let stroke = if idx % major_step == 0 {
            major_stroke
        } else {
            minor_stroke
        };
        let p0 = camera.world_to_screen(Vec2::new(world_left, y)) + offset;
        let p1 = camera.world_to_screen(Vec2::new(world_right, y)) + offset;
        painter.line_segment([p0, p1], stroke);
        y += minor_mm;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_public_api_exists() {
        let s = pick_minor_spacing_mm(1.0);
        assert!(s.is_finite() && s > 0.0);
        assert_eq!(pick_major_step(s), 10);
    }

    #[test]
    fn minor_spacing_at_unit_zoom_is_10mm() {
        assert_eq!(pick_minor_spacing_mm(1.0), 10.0);
    }

    #[test]
    fn minor_spacing_at_zoom_in_is_1mm() {
        assert_eq!(pick_minor_spacing_mm(0.1), 1.0);
    }

    #[test]
    fn minor_spacing_at_zoom_out_is_100mm() {
        assert_eq!(pick_minor_spacing_mm(10.0), 100.0);
    }

    #[test]
    fn minor_spacing_at_half_unit_zoom_is_5mm() {
        assert_eq!(pick_minor_spacing_mm(0.5), 5.0);
    }

    #[test]
    fn minor_spacing_at_micro_zoom() {
        assert_eq!(pick_minor_spacing_mm(0.001), 0.01);
    }

    #[test]
    fn minor_spacing_clamped_at_extreme_zoom() {
        let small = pick_minor_spacing_mm(1e-9);
        assert!(small >= 1e-6);
        assert!(small.is_finite());
        let large = pick_minor_spacing_mm(1e9);
        assert!(large >= 1e6);
        assert!(large.is_finite());
    }

    #[test]
    fn major_step_is_ten() {
        assert_eq!(pick_major_step(1.0), 10);
        assert_eq!(pick_major_step(100.0), 10);
    }

    /// LCV-137 AC 3 — `draw_grid`'s world-bounds computation subtracts the
    /// viewport's own origin from the global `rect.min`/`rect.max` exactly
    /// once, before either reaches `Camera::screen_to_world`. Mirrors the
    /// scan style of `src/app/viewport.rs`'s
    /// `the_live_predicate_has_exactly_three_terms`.
    #[test]
    fn ac3_bounds_computation_subtracts_the_viewport_origin_source_scan() {
        let src = include_str!("grid.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("grid.rs must have a bare #[cfg(test)] to bound the scan");
        let implementation = &src[..at];

        let local_tl_at = implementation
            .find("let local_tl = rect.min - rect.min.to_vec2();")
            .expect("AC 3: draw_grid must subtract rect.min from rect.min (local top-left)");
        let local_br_at = implementation
            .find("let local_br = rect.max - rect.min.to_vec2();")
            .expect("AC 3: draw_grid must subtract rect.min from rect.max (local bottom-right)");
        let tl_call_at = implementation
            .find("let world_tl = camera.screen_to_world(local_tl);")
            .expect("world_tl must be computed from the local point, not the global one");
        let br_call_at = implementation
            .find("let world_br = camera.screen_to_world(local_br);")
            .expect("world_br must be computed from the local point, not the global one");
        assert!(
            local_tl_at < tl_call_at && local_br_at < br_call_at,
            "AC 3: the subtraction must precede the camera call it feeds"
        );
        assert!(
            !implementation.contains("camera.screen_to_world(rect.min)"),
            "AC 3: the bounds computation must never pass the global rect.min \
             straight to the camera"
        );
        assert!(
            !implementation.contains("camera.screen_to_world(rect.max)"),
            "AC 3: the bounds computation must never pass the global rect.max \
             straight to the camera"
        );
    }

    /// LCV-137 AC 5 — a grid line is painted through the screen position of
    /// a known world-space point, across three zoom levels and both a
    /// zero-origin and a nonzero-origin viewport rect.
    ///
    /// `(0.0, 0.0)` is always a grid intersection — every minor spacing
    /// divides it evenly — so one world point, read back from a real
    /// committed [`crate::document::Entity::Line`] rather than a bare
    /// literal, covers every zoom level tested. Checked against
    /// `draw_grid`'s own line-position formula: `Camera::world_to_screen`
    /// plus the viewport's origin, added exactly once — the same `+ offset`
    /// every projected point in this file already carries. A mutation that
    /// drops, doubles, or mis-signs that offset moves every painted grid
    /// line away from `expected` by the whole `rect.min` (tens of points),
    /// far outside the 0.5-point tolerance, in the nonzero-origin case.
    #[test]
    fn ac5_a_grid_line_passes_through_a_known_world_point_at_every_zoom_and_origin() {
        use crate::document::{CreateLine, Document, Entity, History};
        use crate::geometry::Line;

        let mut document = Document::default();
        let mut history = History::default();
        history.commit(
            Box::new(CreateLine::new(Line::new(
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 10.0),
            ))),
            &mut document,
        );
        let known_point = match &document.entities[0] {
            Entity::Line(l) => l.p1,
            other => panic!("expected a Line entity, got {other:?}"),
        };

        for mm_per_px in [0.1_f64, 1.0, 10.0] {
            for rect_min in [egui::Pos2::ZERO, egui::Pos2::new(37.0, 52.0)] {
                let camera = Camera {
                    center_world: Vec2::new(0.0, 0.0),
                    mm_per_px,
                    viewport_size_px: [800.0, 600.0],
                };
                let rect = egui::Rect::from_min_size(rect_min, egui::Vec2::new(800.0, 600.0));
                let expected = camera.world_to_screen(known_point) + rect.min.to_vec2();

                let ctx = egui::Context::default();
                let out = ctx.run(egui::RawInput::default(), |ctx| {
                    let painter = ctx.layer_painter(egui::LayerId::new(
                        egui::Order::Background,
                        egui::Id::new("ac5"),
                    ));
                    draw_grid(&painter, rect, &camera);
                });

                let hit = out.shapes.iter().any(|clipped| {
                    matches!(&clipped.shape, egui::Shape::LineSegment { points, .. }
                        if line_passes_through(points[0], points[1], expected, 0.5))
                });
                assert!(
                    hit,
                    "mm_per_px={mm_per_px}, rect_min={rect_min:?}: no grid line \
                     painted through {expected:?} (world point {known_point:?})"
                );
            }
        }
    }

    /// Whether `expected` lies within `tolerance` logical points of the
    /// axis-aligned segment `a`-`b` — the grid only ever draws vertical or
    /// horizontal segments, so this checks the perpendicular offset from
    /// whichever axis the segment is constant on, plus containment along
    /// the other, rather than a general point-to-segment distance.
    fn line_passes_through(
        a: egui::Pos2,
        b: egui::Pos2,
        expected: egui::Pos2,
        tolerance: f32,
    ) -> bool {
        if (a.x - b.x).abs() <= tolerance {
            (a.x - expected.x).abs() <= tolerance
                && expected.y >= a.y.min(b.y) - tolerance
                && expected.y <= a.y.max(b.y) + tolerance
        } else if (a.y - b.y).abs() <= tolerance {
            (a.y - expected.y).abs() <= tolerance
                && expected.x >= a.x.min(b.x) - tolerance
                && expected.x <= a.x.max(b.x) + tolerance
        } else {
            false
        }
    }

    #[test]
    fn draw_grid_does_not_panic_on_degenerate_cameras() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Background,
                egui::Id::new("test"),
            ));
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(800.0, 600.0));

            let cams = [
                Camera {
                    viewport_size_px: [0.0, 0.0],
                    ..Camera::default()
                },
                Camera {
                    mm_per_px: 1e-9,
                    viewport_size_px: [800.0, 600.0],
                    ..Camera::default()
                },
                Camera {
                    mm_per_px: 1e9,
                    viewport_size_px: [800.0, 600.0],
                    ..Camera::default()
                },
                Camera {
                    center_world: Vec2::new(1e6, 1e6),
                    viewport_size_px: [800.0, 600.0],
                    ..Camera::default()
                },
            ];
            for cam in &cams {
                draw_grid(&painter, rect, cam);
            }
        });
    }
}
