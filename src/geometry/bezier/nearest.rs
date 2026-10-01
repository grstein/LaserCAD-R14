//! Nearest point on a Bézier (ADR 0016 §4).
//!
//! 64 even parameter samples pick the best neighbourhood; golden-section
//! search on the best sample's two neighbouring intervals refines it. The
//! result is always on the curve; it can miss the global foot only when two
//! local minima tie within one sample step, which ranks, never corrupts, a
//! pick or a snap.

use crate::geometry::{Bezier, Vec2};

/// Even parameter samples before refinement.
const SAMPLES: usize = 64;
/// Golden-section steps; `0.618^80 · 2/64` is far below `f64` resolution.
const GOLDEN_STEPS: usize = 80;

impl Bezier {
    /// The parameter and point of the curve nearest to `p`.
    pub fn nearest(&self, p: Vec2) -> (f64, Vec2) {
        let d2 = |t: f64| self.point(t).distance_squared(p);
        let step = 1.0 / SAMPLES as f64;
        let best = (0..=SAMPLES)
            .map(|i| i as f64 * step)
            .min_by(|&a, &b| d2(a).total_cmp(&d2(b)))
            .unwrap_or(0.0);
        let (mut a, mut b) = ((best - step).max(0.0), (best + step).min(1.0));
        let r = (5.0f64.sqrt() - 1.0) / 2.0;
        for _ in 0..GOLDEN_STEPS {
            let (c, d) = (b - r * (b - a), a + r * (b - a));
            if d2(c) < d2(d) {
                b = d;
            } else {
                a = c;
            }
        }
        let mid = 0.5 * (a + b);
        let t = if d2(mid) < d2(best) { mid } else { best };
        (t, self.point(t))
    }

    /// Unsigned distance from `p` to the curve.
    pub fn distance_to_point(&self, p: Vec2) -> f64 {
        p.distance(self.nearest(p).1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s_curve() -> Bezier {
        Bezier::Cubic([
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 20.0),
            Vec2::new(20.0, -20.0),
            Vec2::new(30.0, 0.0),
        ])
    }

    /// Brute-force distance over a dense parameter grid.
    fn brute(b: &Bezier, p: Vec2) -> f64 {
        (0..=100_000)
            .map(|i| b.point(f64::from(i) / 100_000.0).distance(p))
            .fold(f64::INFINITY, f64::min)
    }

    /// A point pushed off the S-curve along its normal finds the point it
    /// was pushed from, on both sides of the curve.
    #[test]
    fn nearest_on_s_curve_is_the_normal_foot() {
        let b = s_curve();
        for k in 1..20 {
            let t = f64::from(k) / 20.0;
            let on = b.point(t);
            let tangent = b.point(t + 1e-6) - b.point(t - 1e-6);
            let n = Vec2::new(-tangent.y, tangent.x).normalize().expect("n");
            for d in [-0.05, 0.05] {
                let (ft, f) = b.nearest(on + n * d);
                assert!((ft - t).abs() <= 1e-6, "{t} {ft}");
                assert!(f.approx_eq(on, 1e-5), "{t} {d} {f:?} {on:?}");
                assert!(f.approx_eq(b.point(ft), 0.0));
                assert!((b.distance_to_point(on + n * d) - 0.05).abs() <= 1e-9);
            }
        }
    }

    /// Beyond an end the end itself is the foot, exactly.
    #[test]
    fn nearest_beyond_an_end_is_the_end() {
        let b = s_curve();
        assert_eq!(b.nearest(Vec2::new(-3.0, -3.0)), (0.0, b.start()));
        assert_eq!(b.nearest(Vec2::new(33.0, 3.0)), (1.0, b.end()));
        let q = Bezier::Quadratic([
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 20.0),
            Vec2::new(10.0, 0.0),
        ]);
        assert_eq!(q.nearest(Vec2::new(11.0, -1.0)), (1.0, q.end()));
    }

    /// Where the distance has several local minima (a self-crossing cubic,
    /// probed on a grid), the sampled start still finds the global foot.
    #[test]
    fn nearest_is_global_on_a_loop() {
        let b = Bezier::Cubic([
            Vec2::new(0.0, 0.0),
            Vec2::new(40.0, 30.0),
            Vec2::new(-10.0, 30.0),
            Vec2::new(30.0, 0.0),
        ]);
        for i in 0..9 {
            for j in 0..8 {
                let p = Vec2::new(f64::from(i) * 5.0 - 5.0, f64::from(j) * 5.0 - 5.0);
                let (d, want) = (b.distance_to_point(p), brute(&b, p));
                // Brute force samples the curve: the global foot is never farther.
                assert!(d <= want + 1e-9, "{p:?}: {d} vs {want}");
            }
        }
    }

    /// An off-curve control point is far from the curve: its distance is
    /// the true (brute-force) distance, not zero.
    #[test]
    fn control_point_is_off_the_curve() {
        let b = s_curve();
        for c in [Vec2::new(10.0, 20.0), Vec2::new(20.0, -20.0)] {
            let d = b.distance_to_point(c);
            assert!(d > 5.0, "{d}");
            assert!((d - brute(&b, c)).abs() <= 1e-6, "{d} {}", brute(&b, c));
        }
    }
}
