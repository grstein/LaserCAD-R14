//! Canonical ellipse value type in millimeter space (ADR 0015).
//!
//! [`Ellipse`] covers both full ellipses and elliptical arcs: a center, two
//! semi-axes, the rotation of the `rx` axis and an optional parametric
//! [`EllipseSpan`]. `point(t) = center + R(rotation)·(rx·cos t, ry·sin t)`;
//! span angles are the parameter `t` (eccentric anomaly), not polar angles.
//! Like [`Arc`], storage is lossless: `rx` may be smaller than `ry` and angles
//! are not wrapped. Sweep and containment reuse [`Arc::sweep_angle`] and
//! [`Arc::contains_angle`] on the unit circle, so the kernel has one
//! wrap-around rule.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

mod conjugate;
mod nearest;

use core::f64::consts::{FRAC_PI_2, PI, TAU};

use serde::{Deserialize, Serialize};

use crate::geometry::{Arc, Vec2};

/// Fewest segments in a painted ellipse polyline (ADR 0015 §8).
const MIN_SEGMENTS: usize = 8;
/// Most segments in a painted ellipse polyline (ADR 0015 §8).
const MAX_SEGMENTS: usize = 4096;

/// The parametric span of an elliptical arc, in radians of the parameter `t`.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EllipseSpan {
    /// Start parameter in radians.
    pub start: f64,
    /// End parameter in radians.
    pub end: f64,
    /// `true` = swept with increasing `t` (counter-clockwise for an
    /// unreflected ellipse), `false` = decreasing.
    pub ccw: bool,
}

impl EllipseSpan {
    /// Construct a span from its raw, non-normalized components.
    pub const fn new(start: f64, end: f64, ccw: bool) -> Self {
        Self { start, end, ccw }
    }
}

/// A full ellipse (`span = None`) or an elliptical arc, in millimeter space.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ellipse {
    /// Center of the ellipse.
    pub center: Vec2,
    /// Semi-axis along the rotated +X direction, in mm.
    pub rx: f64,
    /// Semi-axis along the rotated +Y direction, in mm.
    pub ry: f64,
    /// Angle of the `rx` axis in radians, CCW from +X.
    pub rotation: f64,
    /// Parametric span of an arc; `None` is the full ellipse.
    pub span: Option<EllipseSpan>,
}

impl Ellipse {
    /// Construct an ellipse from its raw components; nothing is validated or
    /// normalized.
    pub const fn new(
        center: Vec2,
        rx: f64,
        ry: f64,
        rotation: f64,
        span: Option<EllipseSpan>,
    ) -> Self {
        Self {
            center,
            rx,
            ry,
            rotation,
            span,
        }
    }

    /// The point at parameter `t`.
    pub fn point(&self, t: f64) -> Vec2 {
        let (s, c) = self.rotation.sin_cos();
        let (x, y) = (self.rx * t.cos(), self.ry * t.sin());
        self.center + Vec2::new(x * c - y * s, x * s + y * c)
    }

    /// The point at the span's start, `None` for a full ellipse.
    pub fn start_point(&self) -> Option<Vec2> {
        self.span.map(|s| self.point(s.start))
    }

    /// The point at the span's end, `None` for a full ellipse.
    pub fn end_point(&self) -> Option<Vec2> {
        self.span.map(|s| self.point(s.end))
    }

    /// The span as an arc on the unit circle, which owns the wrap-around rule.
    fn unit_arc(span: EllipseSpan) -> Arc {
        Arc::new(Vec2::default(), 1.0, span.start, span.end, span.ccw)
    }

    /// Magnitude of the parametric sweep in `[0, 2π]`; `2π` for a full ellipse.
    pub fn sweep(&self) -> f64 {
        self.span.map_or(TAU, |s| Self::unit_arc(s).sweep_angle())
    }

    /// True iff parameter `t` lies within the span (always for a full
    /// ellipse); both ends inclusive within `EPSILON`.
    pub fn contains_param(&self, t: f64) -> bool {
        self.span
            .is_none_or(|s| Self::unit_arc(s).contains_angle(t))
    }

    /// Exact axis-aligned bounding box `(min, max)`: the span ends plus the
    /// X and Y extremes of the curve that lie within the span.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        let (s, c) = self.rotation.sin_cos();
        let tx = (-self.ry * s).atan2(self.rx * c);
        let ty = (self.ry * c).atan2(self.rx * s);
        let mut pts: Vec<Vec2> = [tx, tx + PI, ty, ty + PI]
            .into_iter()
            .filter(|&t| self.contains_param(t))
            .map(|t| self.point(t))
            .collect();
        pts.extend(self.start_point());
        pts.extend(self.end_point());
        let first = pts.first().copied().unwrap_or(self.center);
        pts.iter().fold((first, first), |(lo, hi), p| {
            (
                Vec2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Vec2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        })
    }

    /// The axis vertices at `t ∈ {0, π/2, π, 3π/2}` that lie within the span,
    /// in that order.
    pub fn quadrants(&self) -> Vec<Vec2> {
        [0.0, FRAC_PI_2, PI, 3.0 * FRAC_PI_2]
            .into_iter()
            .filter(|&t| self.contains_param(t))
            .map(|t| self.point(t))
            .collect()
    }

    /// Start parameter and signed sweep: the span's, or `(0, 2π)` for a full
    /// ellipse.
    pub fn signed_range(&self) -> (f64, f64) {
        match self.span {
            None => (0.0, TAU),
            Some(s) if s.ccw => (s.start, self.sweep()),
            Some(s) => (s.start, -self.sweep()),
        }
    }

    /// Vertices sampled evenly in the parameter from the start to the end of
    /// the span (closed for a full ellipse), every vertex on the curve. The
    /// step keeps `max(rx, ry)·(1 − cos(Δt/2)) ≤ tol_mm`, a bound on the chord
    /// deviation; the segment count is clamped to `[8, 4096]`.
    pub fn polyline(&self, tol_mm: f64) -> Vec<Vec2> {
        let (start, sweep) = self.signed_range();
        let r = self.rx.abs().max(self.ry.abs());
        let step = 2.0 * (1.0 - tol_mm / r).clamp(-1.0, 1.0).acos();
        let wanted = (sweep.abs() / step).ceil();
        let n = if wanted.is_finite() {
            // `wanted` is non-negative here; the clamp bounds the cast.
            (wanted.clamp(0.0, MAX_SEGMENTS as f64) as usize).max(MIN_SEGMENTS)
        } else {
            MAX_SEGMENTS
        };
        let mut pts: Vec<Vec2> = (0..=n)
            .map(|i| self.point(start + sweep * (i as f64) / (n as f64)))
            .collect();
        if let (Some(last), Some(end)) = (pts.last_mut(), self.end_point()) {
            *last = end;
        }
        pts
    }
}
