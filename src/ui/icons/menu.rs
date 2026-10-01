//! Menu-row icons (LCV-166): File, Edit and View glyphs plus the check mark,
//! authored on the 20-unit grid of [`super::at`] like the rail icons.

use super::{P, arc_points, arrow_head, at, marker, path};
use egui::{Painter, Rect, Stroke};

/// New: a page with a folded corner.
pub(crate) fn new_file(p: &Painter, r: Rect, s: Stroke) {
    let page = [
        [4.0, 2.0],
        [12.0, 2.0],
        [16.0, 6.0],
        [16.0, 18.0],
        [4.0, 18.0],
    ];
    path(p, r, &page, true, s);
    path(p, r, &[[12.0, 2.0], [12.0, 6.0], [16.0, 6.0]], false, s);
}

/// Open: a folder with its flap lifted.
pub(crate) fn open(p: &Painter, r: Rect, s: Stroke) {
    let back = [
        [2.0, 16.0],
        [2.0, 4.0],
        [8.0, 4.0],
        [10.0, 6.0],
        [16.0, 6.0],
        [16.0, 9.0],
    ];
    path(p, r, &back, false, s);
    path(
        p,
        r,
        &[[2.0, 16.0], [5.0, 9.0], [18.0, 9.0], [15.0, 16.0]],
        true,
        s,
    );
}

/// Save: a floppy disk with its label and shutter.
pub(crate) fn save(p: &Painter, r: Rect, s: Stroke) {
    let body = [
        [3.0, 3.0],
        [15.0, 3.0],
        [17.0, 5.0],
        [17.0, 17.0],
        [3.0, 17.0],
    ];
    path(p, r, &body, true, s);
    path(
        p,
        r,
        &[[6.0, 3.0], [6.0, 8.0], [14.0, 8.0], [14.0, 3.0]],
        false,
        s,
    );
    path(
        p,
        r,
        &[[6.0, 17.0], [6.0, 12.0], [14.0, 12.0], [14.0, 17.0]],
        false,
        s,
    );
}

/// A half-turn arrow over the top, from angle `a0` to `a1`.
fn turn(p: &Painter, r: Rect, a0: f32, a1: f32, s: Stroke) {
    let pts = arc_points([10.0, 12.0], 6.0, a0, a1);
    path(p, r, &pts, false, s);
    let n = pts.len();
    arrow_head(p, r, pts[n - 1], pts[n - 2], s);
}

/// Undo: an arrow turning back to the left.
pub(crate) fn undo(p: &Painter, r: Rect, s: Stroke) {
    turn(p, r, 0.0, -std::f32::consts::PI, s);
}

/// Redo: an arrow turning on to the right.
pub(crate) fn redo(p: &Painter, r: Rect, s: Stroke) {
    turn(p, r, -std::f32::consts::PI, 0.0, s);
}

/// A magnifier: lens and handle.
fn lens(p: &Painter, r: Rect, s: Stroke) {
    let scale = r.width() / super::ICON_SIZE;
    p.circle_stroke(at(r, [8.0, 8.0]), 5.5 * scale, s);
    path(p, r, &[[12.0, 12.0], [18.0, 18.0]], false, s);
}

/// Zoom In: a magnifier with a plus.
pub(crate) fn zoom_in(p: &Painter, r: Rect, s: Stroke) {
    lens(p, r, s);
    path(p, r, &[[5.5, 8.0], [10.5, 8.0]], false, s);
    path(p, r, &[[8.0, 5.5], [8.0, 10.5]], false, s);
}

/// Zoom Out: a magnifier with a minus.
pub(crate) fn zoom_out(p: &Painter, r: Rect, s: Stroke) {
    lens(p, r, s);
    path(p, r, &[[5.5, 8.0], [10.5, 8.0]], false, s);
}

/// Four corner brackets framing the square.
fn corners(p: &Painter, r: Rect, s: Stroke) {
    let brackets: [[P; 3]; 4] = [
        [[2.0, 6.0], [2.0, 2.0], [6.0, 2.0]],
        [[14.0, 2.0], [18.0, 2.0], [18.0, 6.0]],
        [[18.0, 14.0], [18.0, 18.0], [14.0, 18.0]],
        [[6.0, 18.0], [2.0, 18.0], [2.0, 14.0]],
    ];
    brackets.iter().for_each(|b| path(p, r, b, false, s));
}

/// Zoom Extents: the drawing (a circle and a line) framed by brackets.
pub(crate) fn zoom_extents(p: &Painter, r: Rect, s: Stroke) {
    corners(p, r, s);
    let scale = r.width() / super::ICON_SIZE;
    p.circle_stroke(at(r, [11.5, 8.5]), 3.0 * scale, s);
    path(p, r, &[[6.0, 14.0], [10.0, 10.0]], false, s);
}

/// Fit to Bed: the bed rectangle with its origin marked.
pub(crate) fn fit_bed(p: &Painter, r: Rect, s: Stroke) {
    let bed = [[3.0, 5.0], [17.0, 5.0], [17.0, 15.0], [3.0, 15.0]];
    path(p, r, &bed, true, s);
    marker(p, r, [3.0, 15.0], s);
    path(p, r, &[[3.0, 11.0], [7.0, 15.0]], false, s);
}

/// Check mark: a two-segment tick.
pub(crate) fn check(p: &Painter, r: Rect, s: Stroke) {
    path(p, r, &[[4.0, 10.0], [8.0, 14.0], [16.0, 5.0]], false, s);
}
