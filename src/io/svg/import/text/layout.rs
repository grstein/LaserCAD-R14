//! Text layout for SVG `<text>` import (LCV-179): per-character positions
//! from `x`/`y`/`dx`/`dy` lists, the pen advance and `text-anchor` chunk
//! shifts, in user units (Y down, before the element transform).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::geometry::Vec2;

/// A laid-out element's characters `first..end` and its `x`, `y`, `dx`,
/// `dy` lists in user units.
pub(super) type Span = (usize, usize, [Vec<f64>; 4]);

/// Per character of `n`, its `x`, `y`, `dx`, `dy`: the i-th value of an
/// element's list goes to its i-th character, a later span (an inner
/// element) overriding an earlier one (AC 6).
pub(super) fn positions(n: usize, spans: &[Span]) -> Vec<[Option<f64>; 4]> {
    let mut pos = vec![[None; 4]; n];
    for (first, end, lists) in spans {
        let chars = pos.get_mut(*first..(*end).min(n)).unwrap_or_default();
        for (k, values) in lists.iter().enumerate() {
            for (p, v) in chars.iter_mut().zip(values) {
                p[k] = Some(*v);
            }
        }
    }
    pos
}

/// Each character's glyph origin: the pen starts at (0, 0), moves to a
/// given `x`/`y`, then by `dx`/`dy`, and after each character advances by
/// its `advances` entry (AC 1, AC 4, AC 6). A character with an `x` or `y`
/// starts a chunk; each chunk then shifts left by its first character's
/// `anchors` fraction of its advance width (AC 5).
pub(super) fn layout(advances: &[f64], anchors: &[f64], pos: &[[Option<f64>; 4]]) -> Vec<Vec2> {
    let mut pen = Vec2::new(0.0, 0.0);
    let mut origins: Vec<Vec2> = Vec::with_capacity(advances.len());
    let mut start = 0;
    for (i, (advance, [x, y, dx, dy])) in advances.iter().zip(pos).enumerate() {
        if i > 0 && (x.is_some() || y.is_some()) {
            shift(&mut origins[start..], anchors[start], pen.x);
            start = i;
        }
        pen.x = x.unwrap_or(pen.x) + dx.unwrap_or(0.0);
        pen.y = y.unwrap_or(pen.y) + dy.unwrap_or(0.0);
        origins.push(pen);
        pen.x += advance;
    }
    if let Some(&anchor) = anchors.get(start) {
        shift(&mut origins[start..], anchor, pen.x);
    }
    origins
}

/// Shift `chunk` left by `anchor` times its width, from its first origin
/// to the pen's `end`.
fn shift(chunk: &mut [Vec2], anchor: f64, end: f64) {
    let Some(first) = chunk.first() else {
        return;
    };
    let dx = anchor * (end - first.x);
    for o in chunk {
        o.x -= dx;
    }
}
