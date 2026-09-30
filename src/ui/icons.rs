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

/// Side of a rail button, in points (LCV-183 AC 1).
pub(crate) const BUTTON_SIZE: f32 = 32.0;

/// A square selectable button painting `icon` in the text colour, centred in
/// an [`ICON_SIZE`] square. No text.
pub(crate) fn icon_button(ui: &mut egui::Ui, selected: bool, icon: IconFn) -> egui::Response {
    let (rect, response, color) = square_button(ui, selected);
    let inner = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(ICON_SIZE));
    icon(ui.painter(), inner, egui::Stroke::new(ICON_STROKE, color));
    response
}

/// A square selectable button showing a short `text` (the `AI` toggle).
pub(crate) fn text_button(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let (rect, response, color) = square_button(ui, selected);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let centre = egui::Align2::CENTER_CENTER;
    ui.painter().text(rect.center(), centre, text, font, color);
    response
}

/// Allocate one [`BUTTON_SIZE`] square, paint its background the way
/// `egui::SelectableLabel` does (selected fill while `selected`, hover fill
/// while hovered) and return its rect, response and foreground colour.
fn square_button(ui: &mut egui::Ui, selected: bool) -> (egui::Rect, egui::Response, egui::Color32) {
    let size = egui::Vec2::splat(BUTTON_SIZE);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let visuals = ui.style().interact_selectable(&response, selected);
    let lit = selected || response.hovered() || response.highlighted() || response.has_focus();
    if lit && ui.is_rect_visible(rect) {
        let bg = rect.expand(visuals.expansion);
        let painter = ui.painter();
        painter.rect(
            bg,
            visuals.rounding,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
        );
    }
    (rect, response, visuals.text_color())
}

/// Draw-group icons (Select … Text).
pub(crate) mod draw;
/// Modify-group icons (Move … Dist).
pub(crate) mod modify;

/// A point on the 20-unit authoring grid, `[x, y]` with `y` down.
pub(crate) type P = [f32; 2];

/// Side of a vertex marker, in grid units (2.5 pt at the 20 pt size).
const MARKER: f32 = 2.5;

/// Dash and gap lengths of [`dashed`], in grid units.
const DASH: [f32; 2] = [2.5, 1.5];

/// Length and half-angle (radians) of an [`arrow_head`]'s two barbs.
const ARROW: (f32, f32) = (3.0, 0.55);

/// Map grid point `p` into `rect`.
pub(crate) fn at(rect: egui::Rect, p: P) -> egui::Pos2 {
    rect.min + egui::vec2(p[0], p[1]) * (rect.width() / ICON_SIZE)
}

/// An open (or `closed`) stroked path through `pts`.
pub(crate) fn path(
    painter: &egui::Painter,
    rect: egui::Rect,
    pts: &[P],
    closed: bool,
    stroke: egui::Stroke,
) {
    let points: Vec<egui::Pos2> = pts.iter().map(|p| at(rect, *p)).collect();
    if closed {
        painter.add(egui::Shape::closed_line(points, stroke));
    } else {
        painter.add(egui::Shape::line(points, stroke));
    }
}

/// A filled square vertex marker centred on `p`, in the stroke's colour.
pub(crate) fn marker(painter: &egui::Painter, rect: egui::Rect, p: P, stroke: egui::Stroke) {
    let side = MARKER * rect.width() / ICON_SIZE;
    let square = egui::Rect::from_center_size(at(rect, p), egui::Vec2::splat(side));
    painter.rect_filled(square, 0.0, stroke.color);
}

/// Two barbs at `tip`, pointing away from `from`.
pub(crate) fn arrow_head(
    painter: &egui::Painter,
    rect: egui::Rect,
    tip: P,
    from: P,
    stroke: egui::Stroke,
) {
    let back = egui::vec2(from[0] - tip[0], from[1] - tip[1]).normalized();
    let (len, half) = ARROW;
    let barb = |angle: f32| {
        let v = egui::Vec2::angled(back.angle() + angle) * len;
        [tip[0] + v.x, tip[1] + v.y]
    };
    path(
        painter,
        rect,
        &[barb(half), tip, barb(-half)],
        false,
        stroke,
    );
}

/// A dashed line from `a` to `b`, starting with a dash.
pub(crate) fn dashed(painter: &egui::Painter, rect: egui::Rect, a: P, b: P, stroke: egui::Stroke) {
    let d = egui::vec2(b[0] - a[0], b[1] - a[1]);
    let total = d.length();
    let dir = d / total;
    let mut t = 0.0;
    while t < total {
        let end = (t + DASH[0]).min(total);
        let p = |s: f32| at(rect, [a[0] + dir.x * s, a[1] + dir.y * s]);
        painter.line_segment([p(t), p(end)], stroke);
        t = end + DASH[1];
    }
}

/// Points of a circular arc about `c` with radius `r`, from angle `a0` to
/// `a1` (radians, screen orientation), in sixteen steps.
pub(crate) fn arc_points(c: P, r: f32, a0: f32, a1: f32) -> Vec<P> {
    (0..=16)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / 16.0;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
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
