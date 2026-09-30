//! Draw-group icons (LCV-183): R14 metaphors for Select … Text, authored on
//! the 20-unit grid of [`super::at`].

use super::{arc_points, marker, path};
use egui::{Painter, Rect, Stroke};

/// Select: a pointer arrow.
pub(crate) fn select(p: &Painter, r: Rect, s: Stroke) {
    let arrow = [
        [5.0, 2.0],
        [5.0, 16.0],
        [8.5, 12.5],
        [11.0, 18.0],
        [13.0, 17.0],
        [10.5, 11.5],
        [15.0, 11.5],
    ];
    path(p, r, &arrow, true, s);
}

/// Line: a segment with a marker on each end.
pub(crate) fn line(p: &Painter, r: Rect, s: Stroke) {
    let ends = [[4.0, 16.0], [16.0, 4.0]];
    path(p, r, &ends, false, s);
    ends.iter().for_each(|e| marker(p, r, *e, s));
}

/// Polyline: a zigzag with a marker on each vertex.
pub(crate) fn polyline(p: &Painter, r: Rect, s: Stroke) {
    let vertices = [[3.0, 16.0], [8.0, 5.0], [12.0, 14.0], [17.0, 4.0]];
    path(p, r, &vertices, false, s);
    vertices.iter().for_each(|v| marker(p, r, *v, s));
}

/// Rect: a rectangle with markers on its two defining corners.
pub(crate) fn rect(p: &Painter, r: Rect, s: Stroke) {
    let corners = [[3.0, 5.0], [17.0, 5.0], [17.0, 15.0], [3.0, 15.0]];
    path(p, r, &corners, true, s);
    marker(p, r, [3.0, 15.0], s);
    marker(p, r, [17.0, 5.0], s);
}

/// Circle: a circle with a marker at its centre.
pub(crate) fn circle(p: &Painter, r: Rect, s: Stroke) {
    let scale = r.width() / super::ICON_SIZE;
    p.circle_stroke(super::at(r, [10.0, 10.0]), 7.0 * scale, s);
    marker(p, r, [10.0, 10.0], s);
}

/// Arc: an arc over the top through three markers.
pub(crate) fn arc(p: &Painter, r: Rect, s: Stroke) {
    // The circle through (3,15), (10,5) and (17,15): centre (10, 12.45).
    let (c, radius) = ([10.0, 12.45], 7.45);
    let a0 = f32::atan2(15.0 - c[1], 3.0 - c[0]);
    let a1 = f32::atan2(15.0 - c[1], 17.0 - c[0]) + std::f32::consts::TAU;
    path(p, r, &arc_points(c, radius, a0, a1), false, s);
    for m in [[3.0, 15.0], [10.0, 5.0], [17.0, 15.0]] {
        marker(p, r, m, s);
    }
}

/// Text: a stroked capital `A`.
pub(crate) fn text(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[4.0, 17.0], [10.0, 3.0], [16.0, 17.0]], false, s);
    path(p, r, &[[6.6, 11.0], [13.4, 11.0]], false, s);
}
