//! Derivative roots of one Bézier coordinate (ADR 0016 §4).
//!
//! The derivative of a degree-`d` Bézier coordinate is, up to the factor
//! `d`, the degree-`d−1` Bernstein polynomial of the differences
//! `Δᵢ = cᵢ₊₁ − cᵢ`; its roots in `(0, 1)` split `[0, 1]` into monotone pieces.

use crate::geometry::EPSILON;

/// Roots in the open interval `(0, 1)` of the derivative of the Bézier
/// coordinate with control values `c` (3 or 4 values), in ascending order.
pub(super) fn derivative_roots(c: &[f64]) -> Vec<f64> {
    let d: Vec<f64> = c.windows(2).map(|w| w[1] - w[0]).collect();
    // Power form `a·t² + b·t + k` of the Bernstein polynomial of `d`.
    let (a, b, k) = match d.as_slice() {
        [d0, d1] => (0.0, d1 - d0, *d0),
        [d0, d1, d2] => (d0 - 2.0 * d1 + d2, 2.0 * (d1 - d0), *d0),
        _ => return Vec::new(),
    };
    let mut roots = quadratic_roots(a, b, k);
    roots.retain(|t| *t > 0.0 && *t < 1.0);
    roots.sort_by(f64::total_cmp);
    roots
}

/// Real roots of `a·t² + b·t + k`; a leading coefficient within `EPSILON`
/// of zero falls back to the linear root. Uses the cancellation-free form.
fn quadratic_roots(a: f64, b: f64, k: f64) -> Vec<f64> {
    if a.abs() <= EPSILON {
        return if b == 0.0 { Vec::new() } else { vec![-k / b] };
    }
    let disc = b * b - 4.0 * a * k;
    if disc < 0.0 {
        return Vec::new();
    }
    let q = -0.5 * (b + disc.sqrt().copysign(b));
    if q == 0.0 {
        return vec![0.0];
    }
    vec![q / a, k / q]
}
