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

/// LCV-137 AC 3 — `draw_grid`'s world-bounds computation feeds
/// `Camera::screen_to_world` the viewport-local corners (`(0, 0)` and the
/// rect's own size), never the global `rect.min`/`rect.max` directly.
/// Mirrors the scan style of `src/app/viewport.rs`'s
/// `the_live_predicate_has_exactly_three_terms`.
///
/// This is intent, not effect: it pins the *shape* of the fix so a
/// future edit cannot silently reintroduce the direct global-to-camera
/// call. It does not by itself prove the culling is correct at the
/// viewport's edges — that is
/// `ac3_grid_lines_cover_up_to_each_edge_of_a_nonzero_origin_viewport`,
/// added alongside this fix, which is the one that actually fails if the
/// fix is reverted.
#[test]
fn ac3_bounds_computation_uses_local_corners_source_scan() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/render/grid.rs"));
    let at = src
        .find("\n#[cfg(test)]")
        .expect("grid.rs must have a bare #[cfg(test)] to bound the scan");
    let implementation = &src[..at];

    let local_tl_at = implementation
        .find("let local_tl = egui::Pos2::ZERO;")
        .expect("AC 3: draw_grid's local top-left must be the viewport origin");
    let local_br_at = implementation
        .find("let local_br = (rect.max - rect.min).to_pos2();")
        .expect("AC 3: draw_grid's local bottom-right must be the rect's own size");
    let tl_call_at = implementation
        .find("let world_tl = camera.screen_to_world(local_tl);")
        .expect("world_tl must be computed from the local point, not the global one");
    let br_call_at = implementation
        .find("let world_br = camera.screen_to_world(local_br);")
        .expect("world_br must be computed from the local point, not the global one");
    assert!(
        local_tl_at < tl_call_at && local_br_at < br_call_at,
        "AC 3: the local corners must be computed before the camera call they feed"
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

/// LCV-137 AC 3 (behavioural half — required by review) — grid lines are
/// painted all the way to each edge of the viewport, not just somewhere
/// inside it, when the viewport rect's origin is nonzero: the normal
/// case, since the menubar and toolbar always claim screen space before
/// the `CentralPanel` starts, and the agent panel claims more.
///
/// Before the AC 3 fix, `draw_grid`'s bounds computation passed the
/// global `rect.min`/`rect.max` straight into `Camera::screen_to_world`.
/// `screen_to_world`'s formula (`world = center + (screen - half) *
/// mm_per_px`, flipped on Y) turns that into a fixed pixel-space shift of
/// the *whole* world range the loop walks: `+rect.min.x` on the X bounds,
/// `-rect.min.y` on the Y bounds. Because every painted point still adds
/// `rect.min` back before reaching the painter, that shift is not
/// undone by the reprojection — it leaves a real strip near the panel's
/// near edges with no grid lines, and extra lines painted needlessly
/// past the far edges. The shift's magnitude in screen pixels tracks
/// `rect.min` itself, not `mm_per_px`, which is why a `rect_min` far
/// larger than any tested grid spacing (`220`/`96` here, vs. a worst-case
/// spacing under 50px across the zooms tested) reliably swamps the
/// per-line floor/ceil quantization noise the tolerance below exists to
/// absorb.
///
/// This differs from `ac5_a_grid_line_passes_through_a_known_world_point…`:
/// that test targets one point deep inside the visible range, where the
/// bug's uniform shift does not matter — every world point in range still
/// gets a line, just at a shifted position. This test instead reads back
/// every painted line and checks the extremes against the rect's own
/// edges, which is where the shift actually costs a visible line.
#[test]
fn ac3_grid_lines_cover_up_to_each_edge_of_a_nonzero_origin_viewport() {
    for mm_per_px in [0.5_f64, 1.0, 4.0] {
        for rect_min in [egui::Pos2::ZERO, egui::Pos2::new(220.0, 96.0)] {
            let camera = Camera {
                center_world: Vec2::new(0.0, 0.0),
                mm_per_px,
                viewport_size_px: [800.0, 600.0],
            };
            let rect = egui::Rect::from_min_size(rect_min, egui::Vec2::new(800.0, 600.0));

            let ctx = egui::Context::default();
            let out = ctx.run(egui::RawInput::default(), |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Background,
                    egui::Id::new("ac3-edges"),
                ));
                draw_grid(&painter, rect, &camera);
            });

            let mut xs: Vec<f32> = Vec::new();
            let mut ys: Vec<f32> = Vec::new();
            for clipped in &out.shapes {
                if let egui::Shape::LineSegment { points, .. } = &clipped.shape {
                    let [a, b] = *points;
                    if (a.x - b.x).abs() < 0.01 {
                        xs.push(a.x);
                    } else if (a.y - b.y).abs() < 0.01 {
                        ys.push(a.y);
                    }
                }
            }
            assert!(
                !xs.is_empty() && !ys.is_empty(),
                "positive control: draw_grid must paint at least one \
                     vertical and one horizontal line"
            );

            let min_x = xs.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_x = xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
            let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

            let minor_mm = pick_minor_spacing_mm(mm_per_px);
            let spacing_px = (minor_mm / mm_per_px) as f32;

            assert!(
                (min_x - rect.min.x).abs() <= spacing_px,
                "mm_per_px={mm_per_px}, rect_min={rect_min:?}: leftmost \
                     vertical line at x={min_x} is more than one grid spacing \
                     ({spacing_px}) from the rect's left edge ({})",
                rect.min.x
            );
            assert!(
                (max_x - rect.max.x).abs() <= spacing_px,
                "mm_per_px={mm_per_px}, rect_min={rect_min:?}: rightmost \
                     vertical line at x={max_x} is more than one grid spacing \
                     ({spacing_px}) from the rect's right edge ({})",
                rect.max.x
            );
            assert!(
                (min_y - rect.min.y).abs() <= spacing_px,
                "mm_per_px={mm_per_px}, rect_min={rect_min:?}: topmost \
                     horizontal line at y={min_y} is more than one grid \
                     spacing ({spacing_px}) from the rect's top edge ({})",
                rect.min.y
            );
            assert!(
                (max_y - rect.max.y).abs() <= spacing_px,
                "mm_per_px={mm_per_px}, rect_min={rect_min:?}: bottommost \
                     horizontal line at y={max_y} is more than one grid \
                     spacing ({spacing_px}) from the rect's bottom edge ({})",
                rect.max.y
            );
        }
    }
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
fn line_passes_through(a: egui::Pos2, b: egui::Pos2, expected: egui::Pos2, tolerance: f32) -> bool {
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
