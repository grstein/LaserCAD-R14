//! Painted tool-rail icons (LCV-183).
//!
//! Every icon is a plain [`IconFn`] that paints flat line primitives —
//! `LineSegment`, `Path`, `Circle`, `Rect` — into a square it is handed, in the
//! stroke it is handed. No image, texture or icon font is involved
//! (DESIGN.md §1, §12). Icon coordinates are authored on a 20-unit grid and
//! scaled to the square, so the same function paints at any size.

/// One tool icon: paints into `rect` (a square) with `stroke`.
pub(crate) type IconFn = fn(&egui::Painter, egui::Rect, egui::Stroke);

/// Side of the square every icon is painted into, in points.
pub(crate) const ICON_SIZE: f32 = 20.0;

/// Width of every icon line, in points.
pub(crate) const ICON_STROKE: f32 = 1.5;

/// Draw-group icons (Select … Text).
pub(crate) mod draw {
    /// Stub.
    pub(crate) fn select(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn line(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn polyline(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn rect(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn circle(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn arc(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn text(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
}

/// Modify-group icons (Move … Dist).
pub(crate) mod modify {
    /// Stub.
    pub(crate) fn move_(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn copy(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn rotate(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn mirror(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn scale(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn trim(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn extend(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn delete(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
    /// Stub.
    pub(crate) fn dist(_: &egui::Painter, _: egui::Rect, _: egui::Stroke) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sixteen icons in `TOOLS` order, named for failure messages.
    const ICONS: [(&str, IconFn); 16] = [
        ("select", draw::select),
        ("line", draw::line),
        ("polyline", draw::polyline),
        ("rect", draw::rect),
        ("circle", draw::circle),
        ("arc", draw::arc),
        ("text", draw::text),
        ("move", modify::move_),
        ("copy", modify::copy),
        ("rotate", modify::rotate),
        ("mirror", modify::mirror),
        ("scale", modify::scale),
        ("trim", modify::trim),
        ("extend", modify::extend),
        ("delete", modify::delete),
        ("dist", modify::dist),
    ];

    /// A colour no theme token uses, so a shape painted in it came from the
    /// stroke the test handed in.
    const INK: egui::Color32 = egui::Color32::from_rgb(1, 2, 3);

    /// The square the icons are painted into: off the origin, so an icon that
    /// ignores `rect.min` lands outside it.
    fn square() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(40.0, 60.0), egui::Vec2::splat(ICON_SIZE))
    }

    /// Run `icon` on a real `Painter` and return every leaf shape it added.
    fn paint(icon: IconFn) -> Vec<egui::Shape> {
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            let layer = egui::LayerId::new(egui::Order::Background, egui::Id::new("icon"));
            icon(
                &ctx.layer_painter(layer),
                square(),
                egui::Stroke::new(ICON_STROKE, INK),
            );
        });
        let mut leaves = Vec::new();
        let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
        while let Some(shape) = stack.pop() {
            match shape {
                egui::Shape::Vec(inner) => stack.extend(inner),
                other => leaves.push(other),
            }
        }
        leaves
    }

    /// `true` when a line stroke is exactly the icon stroke.
    fn is_icon_stroke(width: f32, color: egui::Color32) -> bool {
        width == ICON_STROKE && color == INK
    }

    /// `true` for a path stroke that is the icon stroke or absent.
    fn path_stroke_ok(stroke: &egui::epaint::PathStroke) -> bool {
        match stroke.color {
            egui::epaint::ColorMode::Solid(c) => {
                stroke.width == 0.0 || is_icon_stroke(stroke.width, c)
            }
            egui::epaint::ColorMode::UV(_) => false,
        }
    }

    /// `true` for a solid stroke that is the icon stroke or absent.
    fn stroke_ok(stroke: egui::Stroke) -> bool {
        stroke.is_empty() || is_icon_stroke(stroke.width, stroke.color)
    }

    /// `true` for a fill that is the ink colour or none.
    fn fill_ok(fill: egui::Color32) -> bool {
        fill == egui::Color32::TRANSPARENT || fill == INK
    }

    /// AC 1 — each icon paints something, and only line-art vector shapes:
    /// no text, mesh (image/texture) or callback.
    #[test]
    fn ac1_icons_paint_only_vector_shapes() {
        for (name, icon) in ICONS {
            let shapes = paint(icon);
            assert!(!shapes.is_empty(), "{name}: the icon must paint something");
            for shape in &shapes {
                assert!(
                    matches!(
                        shape,
                        egui::Shape::LineSegment { .. }
                            | egui::Shape::Path(_)
                            | egui::Shape::Circle(_)
                            | egui::Shape::Rect(_)
                    ),
                    "{name}: only line-art vector shapes are allowed, got {shape:?}"
                );
                if let egui::Shape::Rect(r) = shape {
                    assert_eq!(
                        r.fill_texture_id,
                        egui::TextureId::default(),
                        "{name}: no texture"
                    );
                }
            }
        }
    }

    /// AC 1 — every shape is drawn in the handed stroke (1.5 pt, ink) or is a
    /// solid ink fill (markers), and stays inside the 20 pt square.
    #[test]
    fn ac1_icons_use_the_stroke_and_stay_inside_the_square() {
        let bounds = square().expand(ICON_STROKE / 2.0 + 0.01);
        for (name, icon) in ICONS {
            for shape in paint(icon) {
                let ok = match &shape {
                    egui::Shape::LineSegment { stroke, .. } => {
                        stroke.width > 0.0 && path_stroke_ok(stroke)
                    }
                    egui::Shape::Path(p) => path_stroke_ok(&p.stroke) && fill_ok(p.fill),
                    egui::Shape::Circle(c) => stroke_ok(c.stroke) && fill_ok(c.fill),
                    egui::Shape::Rect(r) => stroke_ok(r.stroke) && fill_ok(r.fill),
                    _ => false,
                };
                assert!(
                    ok,
                    "{name}: stroke must be 1.5 pt ink, fill ink or none: {shape:?}"
                );
                let vis = shape.visual_bounding_rect();
                assert!(
                    bounds.contains_rect(vis),
                    "{name}: {vis:?} leaves the icon square {bounds:?}"
                );
            }
        }
    }

    /// A shape's kind and its geometry relative to the square, rounded to a
    /// tenth of a point.
    fn signature(shapes: &[egui::Shape]) -> Vec<String> {
        let origin = square().min.to_vec2();
        let fmt = |pts: &[egui::Pos2]| -> String {
            pts.iter()
                .map(|p| {
                    let q = *p - origin;
                    format!("({:.1},{:.1})", q.x, q.y)
                })
                .collect()
        };
        let mut sig: Vec<String> = shapes
            .iter()
            .map(|s| match s {
                egui::Shape::LineSegment { points, .. } => format!("seg{}", fmt(points)),
                egui::Shape::Path(p) => format!("path{}{}", p.closed, fmt(&p.points)),
                egui::Shape::Circle(c) => format!("circle{}{:.1}", fmt(&[c.center]), c.radius),
                egui::Shape::Rect(r) => format!("rect{}", fmt(&[r.rect.min, r.rect.max])),
                other => format!("{other:?}"),
            })
            .collect();
        sig.sort();
        sig
    }

    /// AC 2 — no two tools paint the same icon.
    #[test]
    fn ac2_every_icon_is_distinct() {
        let sigs: Vec<(&str, Vec<String>)> = ICONS
            .iter()
            .map(|(name, icon)| (*name, signature(&paint(*icon))))
            .collect();
        for a in 0..sigs.len() {
            for b in (a + 1)..sigs.len() {
                assert_ne!(
                    sigs[a].1, sigs[b].1,
                    "{} and {} paint the same icon",
                    sigs[a].0, sigs[b].0
                );
            }
        }
    }
}
