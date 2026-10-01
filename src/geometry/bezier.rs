//! Quadratic and cubic Bézier value type in millimeter space (ADR 0016).
//!
//! [`Bezier`] stores its control points as given: a quadratic is never
//! elevated to a cubic, so it round-trips as the same `Q` segment. The curve
//! is the Bernstein form `Σ Bᵢ,d(t)·Pᵢ` for `t ∈ [0, 1]`; the ends are the
//! first and last control points exactly.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

mod nearest;
mod solve;

use serde::{Deserialize, Serialize};

use crate::geometry::Vec2;

/// Most segments in a painted Bézier polyline (ADR 0016 §4).
const MAX_SEGMENTS: f64 = 4096.0;

/// A quadratic or cubic Bézier curve segment, in millimeter space.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Bezier {
    /// Start, control, end.
    Quadratic([Vec2; 3]),
    /// Start, first control, second control, end.
    Cubic([Vec2; 4]),
}

impl Bezier {
    /// The control points in order, start first and end last.
    pub fn points(&self) -> &[Vec2] {
        match self {
            Self::Quadratic(p) => p,
            Self::Cubic(p) => p,
        }
    }

    /// The first control point, where the curve starts.
    pub fn start(&self) -> Vec2 {
        match self {
            Self::Quadratic([p, ..]) | Self::Cubic([p, ..]) => *p,
        }
    }

    /// The last control point, where the curve ends.
    pub fn end(&self) -> Vec2 {
        match self {
            Self::Quadratic([.., p]) | Self::Cubic([.., p]) => *p,
        }
    }

    /// The curve point at parameter `t` (Bernstein form; exact at 0 and 1).
    pub fn point(&self, t: f64) -> Vec2 {
        let m = 1.0 - t;
        match self {
            Self::Quadratic([a, b, c]) => *a * (m * m) + *b * (2.0 * m * t) + *c * (t * t),
            Self::Cubic([a, b, c, d]) => {
                *a * (m * m * m)
                    + *b * (3.0 * m * m * t)
                    + *c * (3.0 * m * t * t)
                    + *d * (t * t * t)
            }
        }
    }

    /// The same degree with every control point mapped through `f`.
    pub fn map(&self, f: impl Fn(Vec2) -> Vec2) -> Self {
        match self {
            Self::Quadratic(p) => Self::Quadratic(p.map(f)),
            Self::Cubic(p) => Self::Cubic(p.map(f)),
        }
    }

    /// Tight axis-aligned bounding box `(min, max)` of the curve: the ends
    /// plus the curve at each derivative root in `(0, 1)` of either axis.
    /// Never the control polygon's box.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        let xs: Vec<f64> = self.points().iter().map(|p| p.x).collect();
        let ys: Vec<f64> = self.points().iter().map(|p| p.y).collect();
        let mut roots = solve::derivative_roots(&xs);
        roots.extend(solve::derivative_roots(&ys));
        let (s, e) = (self.start(), self.end());
        roots.iter().map(|&t| self.point(t)).fold(
            (
                Vec2::new(s.x.min(e.x), s.y.min(e.y)),
                Vec2::new(s.x.max(e.x), s.y.max(e.y)),
            ),
            |(lo, hi), p| {
                (
                    Vec2::new(lo.x.min(p.x), lo.y.min(p.y)),
                    Vec2::new(hi.x.max(p.x), hi.y.max(p.y)),
                )
            },
        )
    }

    /// Vertices `point(i/n)` for `i = 0..=n`, every vertex on the curve and
    /// the ends exact. `n = ⌈√(M / (8·tol))⌉` with
    /// `M = d(d−1)·max‖Pᵢ − 2Pᵢ₊₁ + Pᵢ₊₂‖` (Wang's bound on the chord
    /// deviation), clamped to `[1, 4096]`.
    pub fn polyline(&self, tol_mm: f64) -> Vec<Vec2> {
        let p = self.points();
        let degree = (p.len() - 1) as f64;
        let second = p
            .windows(3)
            .map(|w| (w[0] - w[1] * 2.0 + w[2]).length())
            .fold(0.0, f64::max);
        let wanted = (degree * (degree - 1.0) * second / (8.0 * tol_mm))
            .sqrt()
            .ceil();
        let n = if wanted.is_nan() {
            1
        } else {
            // The clamp bounds the cast to `[1, 4096]`.
            wanted.clamp(1.0, MAX_SEGMENTS) as usize
        };
        (0..=n).map(|i| self.point(i as f64 / n as f64)).collect()
    }
}
