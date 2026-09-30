//! LCV-164 — canvas legibility and bed framing, proven on painted shapes.
//!
//! Every test paints through the public render API (or a real `App` frame)
//! into a bare `egui::Context` at 1 pixel per point and reads `out.shapes`
//! back: the shape kind, its point count and where the points landed.

use egui::epaint::Shape;
use lasercad::document::{Document, Entity};
use lasercad::geometry::{Arc, Circle, Vec2};
use lasercad::render::{Camera, PaintOptions, draw_entities};

/// The viewport every paint test uses: off the window origin, like the real
/// canvas under the menubar and rail.
fn viewport() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::new(40.0, 30.0), egui::Vec2::new(800.0, 600.0))
}

fn camera(mm_per_px: f64, center: Vec2) -> Camera {
    Camera {
        center_world: center,
        mm_per_px,
        viewport_size_px: [800.0, 600.0],
    }
}

/// Every shape painted by `paint`, `Shape::Vec`s flattened.
fn painted(paint: impl FnOnce(&egui::Painter)) -> Vec<Shape> {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut paint = Some(paint);
    let out = ctx.run(egui::RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            egui::Id::new("lcv164"),
        ));
        if let Some(paint) = paint.take() {
            paint(&painter);
        }
    });
    let mut shapes = Vec::new();
    for clipped in out.shapes {
        flatten(clipped.shape, &mut shapes);
    }
    shapes
}

fn flatten(shape: Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(v) => v.into_iter().for_each(|s| flatten(s, out)),
        Shape::Noop => {}
        s => out.push(s),
    }
}

/// A one-entity document on the default layer.
fn doc_of(entity: Entity) -> Document {
    let mut doc = Document::default();
    doc.push_current(entity);
    doc
}

/// The one path `draw_entities` paints for `entity`.
fn entity_path(entity: Entity, cam: &Camera) -> egui::epaint::PathShape {
    let doc = doc_of(entity);
    let shapes = painted(|p| draw_entities(p, viewport(), cam, &doc, PaintOptions::default()));
    match &shapes[..] {
        [Shape::Path(path)] => path.clone(),
        other => panic!("one path per curve expected, got {other:?}"),
    }
}

/// Largest on-screen sagitta of the chords `points[i]`–`points[i + 1]` (plus
/// the closing chord when `closed`) of a curve of radius `r_pt` around `c`,
/// less the rounding of the `f32` screen points themselves (two ulps at the
/// points' magnitude — 0.002 pt for a 10 000 pt circle).
fn max_sagitta(points: &[egui::Pos2], closed: bool, c: egui::Pos2, r_pt: f64) -> f64 {
    let magnitude = r_pt + f64::from(c.x.abs().max(c.y.abs()));
    let f32_noise = 2.0 * f64::from(f32::EPSILON) * magnitude;
    let mut chords: Vec<(egui::Pos2, egui::Pos2)> =
        points.windows(2).map(|w| (w[0], w[1])).collect();
    if closed {
        chords.push((points[points.len() - 1], points[0]));
    }
    chords
        .into_iter()
        .map(|(a, b)| {
            let mid = egui::Pos2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let (dx, dy) = (f64::from(mid.x - c.x), f64::from(mid.y - c.y));
            r_pt - dx.hypot(dy) - f32_noise
        })
        .fold(0.0, f64::max)
}

/// LCV-164 AC 5 — a circle is one closed path whose chord count lies in
/// [8, 1024] and whose every chord's sagitta is ≤ 0.25 pt, from 0.01 to
/// 100 mm/pt, down to a circle smaller than a point.
#[test]
fn ac5_circles_use_enough_chords_for_a_quarter_point_sagitta() {
    for (mm_per_px, r) in [(0.01, 100.0), (1.0, 100.0), (100.0, 100.0), (1.0, 0.2)] {
        let center = Vec2::new(12.5, -7.25);
        let cam = camera(mm_per_px, center);
        let path = entity_path(Entity::Circle(Circle::new(center, r)), &cam);
        let chords = path.points.len();
        assert!(
            path.closed,
            "mm/pt={mm_per_px}, r={r}: the circle path is closed"
        );
        assert!(
            (8..=1024).contains(&chords),
            "mm/pt={mm_per_px}, r={r}: {chords} chords"
        );
        let c = cam.world_to_screen(center) + viewport().min.to_vec2();
        let s = max_sagitta(&path.points, true, c, r / mm_per_px);
        assert!(s <= 0.25, "mm/pt={mm_per_px}, r={r}: sagitta {s:.4} pt");
    }
}

/// LCV-164 AC 5 — an arc is one open path whose chords, scaled to a full
/// turn, lie in [8, 1024], each with a sagitta ≤ 0.25 pt, at every zoom.
#[test]
fn ac5_arcs_use_enough_chords_for_a_quarter_point_sagitta() {
    use core::f64::consts::{FRAC_PI_2, TAU};
    for (mm_per_px, r) in [(0.01, 100.0), (1.0, 100.0), (100.0, 100.0), (1.0, 0.2)] {
        let center = Vec2::new(-3.0, 4.0);
        let cam = camera(mm_per_px, center);
        let arc = Arc::new(center, r, FRAC_PI_2, 0.0, true); // 270° CCW
        let sweep = arc.sweep_angle();
        let path = entity_path(Entity::Arc(arc), &cam);
        let chords = path.points.len() - 1;
        assert!(
            !path.closed,
            "mm/pt={mm_per_px}, r={r}: the arc path is open"
        );
        let (lo, hi) = ((8.0 * sweep / TAU).ceil(), (1024.0 * sweep / TAU).ceil());
        assert!(
            (lo as usize..=hi as usize).contains(&chords) && chords >= 2,
            "mm/pt={mm_per_px}, r={r}: {chords} chords over {sweep:.3} rad"
        );
        let c = cam.world_to_screen(center) + viewport().min.to_vec2();
        let s = max_sagitta(&path.points, false, c, r / mm_per_px);
        assert!(s <= 0.25, "mm/pt={mm_per_px}, r={r}: sagitta {s:.4} pt");
    }
}

/// LCV-164 AC 6 — the selection halo, hover and preview each stroke a
/// selected, hovered arc and circle as one path per entity: a closed path for
/// the circle, an open one for the arc, never one shape per chord.
#[test]
fn ac6_translucent_overlays_paint_one_path_per_entity() {
    use lasercad::render::{draw_hover, draw_preview, draw_selection_highlight};
    let mut doc = Document::default();
    doc.push_current(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        50.0,
        0.0,
        2.0,
        true,
    )));
    doc.push_current(Entity::Circle(Circle::new(Vec2::new(20.0, 10.0), 40.0)));
    doc.selection.set([0, 1]);
    let cam = camera(0.5, Vec2::new(0.0, 0.0));
    let rect = viewport();

    let halo = painted(|p| draw_selection_highlight(p, rect, &cam, &doc.entities, &doc.selection));
    let hover = painted(|p| {
        draw_hover(p, rect, &cam, &doc, 0);
        draw_hover(p, rect, &cam, &doc, 1);
    });
    let preview = painted(|p| draw_preview(p, rect, &cam, &doc.entities));

    for (overlay, shapes) in [("halo", halo), ("hover", hover), ("preview", preview)] {
        let closed: Vec<bool> = shapes
            .iter()
            .map(|s| match s {
                Shape::Path(p) if p.points.len() > 8 => p.closed,
                other => panic!("{overlay}: one path per entity, got {other:?}"),
            })
            .collect();
        assert_eq!(
            closed,
            [false, true],
            "{overlay}: arc then circle, one shape each"
        );
    }
}

/// The origin marker's path in `shapes`: an open three-point path in the
/// `ORIGIN` token, and its index.
fn origin_path(shapes: &[Shape]) -> Option<(usize, egui::epaint::PathShape)> {
    use lasercad::render::palette::ORIGIN;
    shapes.iter().enumerate().find_map(|(i, s)| match s {
        Shape::Path(p)
            if !p.closed
                && p.points.len() == 3
                && p.stroke.color == egui::epaint::ColorMode::Solid(ORIGIN) =>
        {
            Some((i, p.clone()))
        }
        _ => None,
    })
}

/// LCV-164 AC 4 — the origin marker is one open path from 12 pt along +X,
/// through world (0,0), to 12 pt along +Y (up the screen), at every zoom.
#[test]
fn ac4_origin_marker_is_two_twelve_point_arms_at_world_zero() {
    use lasercad::render::draw_origin;
    use lasercad::render::palette::{ORIGIN_ARM_PT, ORIGIN_WIDTH_PT};
    for mm_per_px in [0.05, 1.0, 20.0] {
        let cam = camera(mm_per_px, Vec2::new(30.0, 20.0));
        let rect = viewport();
        let shapes = painted(|p| draw_origin(p, rect, &cam));
        assert_eq!(
            shapes.len(),
            1,
            "mm/pt={mm_per_px}: one shape, got {shapes:?}"
        );
        let (_, path) = origin_path(&shapes).expect("the origin path");
        let o = cam.world_to_screen(Vec2::new(0.0, 0.0)) + rect.min.to_vec2();
        let expected = [
            o + egui::Vec2::new(ORIGIN_ARM_PT, 0.0),
            o,
            o + egui::Vec2::new(0.0, -ORIGIN_ARM_PT),
        ];
        for (got, want) in path.points.iter().zip(expected) {
            assert!(
                (*got - want).length() < 1e-3,
                "mm/pt={mm_per_px}: {got:?} vs {want:?}"
            );
        }
        assert_eq!(ORIGIN_ARM_PT, 12.0);
        assert_eq!(path.stroke.width, ORIGIN_WIDTH_PT);
    }
}

/// LCV-164 AC 4 — in a real frame the origin marker paints before the
/// entities: its path comes before the circle's.
#[test]
fn ac4_origin_marker_paints_before_the_entities() {
    let ctx = egui::Context::default();
    let mut app = lasercad::app::App::default();
    app.document
        .push_current(Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 30.0)));
    let mut shapes = Vec::new();
    for _ in 0..2 {
        let out = ctx.run(crate::harness::raw_input(Vec::new()), |ctx| {
            app.update_ui(ctx)
        });
        shapes.clear();
        for clipped in out.shapes {
            flatten(clipped.shape, &mut shapes);
        }
    }
    let (origin_at, _) = origin_path(&shapes).expect("AC 4: the origin marker paints");
    let circle_at = shapes
        .iter()
        .position(|s| matches!(s, Shape::Path(p) if p.closed && p.points.len() >= 8))
        .expect("control: the circle paints as a closed path");
    assert!(
        origin_at < circle_at,
        "AC 4: origin {origin_at} after entity {circle_at}"
    );
}
