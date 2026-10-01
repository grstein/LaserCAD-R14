//! Modify-group icons (LCV-183): R14 metaphors for Move … Dist, authored on
//! the 20-unit grid of [`super::at`].

use super::{arc_points, arrow_head, dashed, path};
use egui::{Painter, Rect, Stroke};

/// Move: four-way arrows.
pub(crate) fn move_(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[2.0, 10.0], [18.0, 10.0]], false, s);
    path(p, r, &[[10.0, 2.0], [10.0, 18.0]], false, s);
    let c = [10.0, 10.0];
    for tip in [[2.0, 10.0], [18.0, 10.0], [10.0, 2.0], [10.0, 18.0]] {
        arrow_head(p, r, tip, c, s);
    }
}

/// Copy: two offset squares.
pub(crate) fn copy(p: &Painter, r: Rect, s: Stroke) {
    square(p, r, [3.0, 3.0], 9.0, s);
    square(p, r, [8.0, 8.0], 9.0, s);
}

/// Rotate: a square under a curved arrow.
pub(crate) fn rotate(p: &Painter, r: Rect, s: Stroke) {
    square(p, r, [6.0, 9.0], 8.0, s);
    let turn = arc_points(
        [10.0, 10.0],
        8.0,
        (-160.0f32).to_radians(),
        (-20.0f32).to_radians(),
    );
    path(p, r, &turn, false, s);
    let n = turn.len();
    arrow_head(p, r, turn[n - 1], turn[n - 2], s);
}

/// Mirror: a shape, a dashed axis and its reflection.
pub(crate) fn mirror(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[3.0, 4.0], [7.0, 16.0], [3.0, 16.0]], true, s);
    dashed(p, r, [10.0, 2.0], [10.0, 18.0], s);
    path(p, r, &[[17.0, 4.0], [13.0, 16.0], [17.0, 16.0]], true, s);
}

/// Scale: a small square in a large one, with a diagonal arrow.
pub(crate) fn scale(p: &Painter, r: Rect, s: Stroke) {
    square(p, r, [3.0, 3.0], 14.0, s);
    square(p, r, [3.0, 11.0], 6.0, s);
    path(p, r, &[[9.0, 11.0], [15.0, 5.0]], false, s);
    arrow_head(p, r, [15.0, 5.0], [9.0, 11.0], s);
}

/// Trim: a cutting edge, the kept part solid and the cut part dashed.
pub(crate) fn trim(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[10.0, 2.0], [10.0, 18.0]], false, s);
    path(p, r, &[[2.0, 12.0], [10.0, 12.0]], false, s);
    dashed(p, r, [11.5, 12.0], [18.0, 12.0], s);
}

/// Extend: a segment with an arrow reaching a boundary.
pub(crate) fn extend(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[17.0, 2.0], [17.0, 18.0]], false, s);
    path(p, r, &[[3.0, 13.0], [10.0, 13.0]], false, s);
    dashed(p, r, [10.0, 13.0], [15.5, 13.0], s);
    arrow_head(p, r, [15.5, 13.0], [10.0, 13.0], s);
}

/// Delete: a tilted eraser with its band.
pub(crate) fn delete(p: &Painter, r: Rect, s: Stroke) {
    path(
        p,
        r,
        &[[3.0, 12.0], [10.0, 5.0], [16.0, 11.0], [9.0, 18.0]],
        true,
        s,
    );
    path(p, r, &[[6.5, 8.5], [12.5, 14.5]], false, s);
}

/// Dist: a dimension line `|<->|`.
pub(crate) fn dist(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[2.0, 6.0], [2.0, 14.0]], false, s);
    path(p, r, &[[18.0, 6.0], [18.0, 14.0]], false, s);
    path(p, r, &[[2.0, 10.0], [18.0, 10.0]], false, s);
    arrow_head(p, r, [2.0, 10.0], [18.0, 10.0], s);
    arrow_head(p, r, [18.0, 10.0], [2.0, 10.0], s);
}

/// Check (LCV-190): a contour under a magnifying glass.
pub(crate) fn check_drawing(p: &Painter, r: Rect, s: Stroke) {
    square(p, r, [2.0, 2.0], 10.0, s);
    let lens = super::at(r, [12.0, 12.0]);
    p.circle_stroke(lens, lens.distance(super::at(r, [16.5, 12.0])), s);
    path(p, r, &[[15.5, 15.5], [18.5, 18.5]], false, s);
}

/// An axis-aligned square with top-left corner `min` and side `side`.
fn square(p: &Painter, r: Rect, min: super::P, side: f32, s: Stroke) {
    let [x, y] = min;
    let corners = [[x, y], [x + side, y], [x + side, y + side], [x, y + side]];
    path(p, r, &corners, true, s);
}
