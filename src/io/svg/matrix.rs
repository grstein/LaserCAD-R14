//! 2D affine matrices and the SVG `transform` attribute (LCV-173 AC 5).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::geometry::Vec2;

/// Relative tolerance of [`Matrix::similarity_scale`] and the singular test.
const REL_TOL: f64 = 1e-9;

/// The affine map `(x, y) → (a x + c y + e, b x + d y + f)`, SVG's
/// `matrix(a b c d e f)`, in f64.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Matrix {
    pub(super) a: f64,
    pub(super) b: f64,
    pub(super) c: f64,
    pub(super) d: f64,
    pub(super) e: f64,
    pub(super) f: f64,
}

impl Matrix {
    /// The identity map.
    pub(super) const IDENTITY: Matrix = Matrix::scale(1.0, 1.0);

    /// `translate(tx ty)`.
    pub(super) const fn translate(tx: f64, ty: f64) -> Matrix {
        Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    /// `scale(sx sy)`.
    pub(super) const fn scale(sx: f64, sy: f64) -> Matrix {
        Matrix {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    /// The map that applies `self` first, then `next`.
    pub(super) fn then(self, next: Matrix) -> Matrix {
        let n = next;
        Matrix {
            a: n.a * self.a + n.c * self.b,
            b: n.b * self.a + n.d * self.b,
            c: n.a * self.c + n.c * self.d,
            d: n.b * self.c + n.d * self.d,
            e: n.a * self.e + n.c * self.f + n.e,
            f: n.b * self.e + n.d * self.f + n.f,
        }
    }

    /// `p` mapped. The identity returns `p` bit for bit (LCV-173 AC 11).
    pub(super) fn apply(&self, p: Vec2) -> Vec2 {
        if *self == Matrix::IDENTITY {
            return p;
        }
        Vec2::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// The determinant; negative for a reflection.
    pub(super) fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// Whether the map collapses the plane: `|det|` below 1e-12 of the squared
    /// norm, or not finite (SVG 2: the element is not rendered).
    pub(super) fn is_singular(&self) -> bool {
        let norm2 = self.a * self.a + self.b * self.b + self.c * self.c + self.d * self.d;
        let regular = self.det().abs() > 1e-12 * norm2 && self.e.is_finite() && self.f.is_finite();
        !regular
    }

    /// The uniform scale factor when the map is a similarity (rotation,
    /// uniform scale, translation, reflection): columns of equal length and
    /// orthogonal within a 1e-9 relative tolerance. `None` otherwise.
    pub(super) fn similarity_scale(&self) -> Option<f64> {
        let (c1, c2) = (Vec2::new(self.a, self.b), Vec2::new(self.c, self.d));
        let (l1, l2) = (c1.length_squared(), c2.length_squared());
        let tol = REL_TOL * l1.max(l2);
        let similar = (l1 - l2).abs() <= tol && c1.dot(c2).abs() <= tol && l1 > 0.0;
        similar.then(|| self.det().abs().sqrt())
    }
}

/// Parse a `transform` list: `matrix translate scale rotate skewX skewY`,
/// arguments and functions separated by commas and/or whitespace, composed
/// left to right (the rightmost applies first). Angles are degrees. The empty
/// list is the identity; any syntax error is `None`.
pub(super) fn parse_transform(raw: &str) -> Option<Matrix> {
    let mut m = Matrix::IDENTITY;
    let mut rest = raw.trim_start_matches(separator);
    while !rest.is_empty() {
        let open = rest.find('(')?;
        let close = open + rest[open..].find(')')?;
        let name = rest[..open].trim_end_matches(|c: char| c.is_ascii_whitespace());
        let args = numbers(&rest[open + 1..close])?;
        m = function(name, &args)?.then(m);
        rest = rest[close + 1..].trim_start_matches(separator);
    }
    Some(m)
}

fn separator(c: char) -> bool {
    c == ',' || c.is_ascii_whitespace()
}

/// One transform function's matrix; `None` for an unknown name or a wrong
/// argument count.
fn function(name: &str, args: &[f64]) -> Option<Matrix> {
    let rad = |deg: f64| deg.to_radians();
    Some(match (name, args) {
        ("matrix", &[a, b, c, d, e, f]) => Matrix { a, b, c, d, e, f },
        ("translate", &[tx]) => Matrix::translate(tx, 0.0),
        ("translate", &[tx, ty]) => Matrix::translate(tx, ty),
        ("scale", &[s]) => Matrix::scale(s, s),
        ("scale", &[sx, sy]) => Matrix::scale(sx, sy),
        ("rotate", &[a]) => rotate(rad(a)),
        ("rotate", &[a, cx, cy]) => Matrix::translate(-cx, -cy)
            .then(rotate(rad(a)))
            .then(Matrix::translate(cx, cy)),
        ("skewX", &[a]) => Matrix {
            c: rad(a).tan(),
            ..Matrix::IDENTITY
        },
        ("skewY", &[a]) => Matrix {
            b: rad(a).tan(),
            ..Matrix::IDENTITY
        },
        _ => return None,
    })
}

fn rotate(rad: f64) -> Matrix {
    let (s, c) = rad.sin_cos();
    Matrix {
        a: c,
        b: s,
        c: -s,
        d: c,
        e: 0.0,
        f: 0.0,
    }
}

/// The argument list: SVG numbers separated by commas and/or whitespace, or
/// by a sign that starts the next number (`10-5`). `None` at any junk.
fn numbers(src: &str) -> Option<Vec<f64>> {
    let bytes = src.as_bytes();
    let (mut out, mut i) = (Vec::new(), 0);
    loop {
        while i < bytes.len() && separator(char::from(bytes[i])) {
            i += 1;
        }
        if i == bytes.len() {
            return Some(out);
        }
        let start = i;
        let mut seen_dot = false;
        let mut prev = b' ';
        while let Some(&ch) = bytes.get(i) {
            let sign_ok = matches!(ch, b'+' | b'-') && (i == start || matches!(prev, b'e' | b'E'));
            let dot_ok = ch == b'.' && !seen_dot;
            if !(ch.is_ascii_digit() || matches!(ch, b'e' | b'E') || sign_ok || dot_ok) {
                break;
            }
            seen_dot |= ch == b'.';
            prev = ch;
            i += 1;
        }
        let value = src.get(start..i)?.parse::<f64>().ok()?;
        out.push(value.is_finite().then_some(value)?);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(m: Matrix, want: [f64; 6]) -> bool {
        let got = [m.a, m.b, m.c, m.d, m.e, m.f];
        got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-12)
    }

    fn parsed(raw: &str) -> Matrix {
        parse_transform(raw).unwrap_or_else(|| panic!("{raw:?} must parse"))
    }

    /// `then` applies the receiver first; `apply` maps points.
    #[test]
    fn then_applies_the_receiver_first() {
        let m = Matrix::translate(10.0, 0.0).then(Matrix::scale(2.0, 3.0));
        assert_eq!(m.apply(Vec2::new(1.0, 1.0)), Vec2::new(22.0, 3.0));
        let m = Matrix::scale(2.0, 3.0).then(Matrix::translate(10.0, 0.0));
        assert_eq!(m.apply(Vec2::new(1.0, 1.0)), Vec2::new(12.0, 3.0));
    }

    /// AC 11 — the identity returns its input bit for bit, `-0.0` included.
    #[test]
    fn identity_apply_is_bitwise() {
        let p = Vec2::new(-0.0, 123.456_789);
        let q = Matrix::IDENTITY.apply(p);
        assert_eq!(q.x.to_bits(), p.x.to_bits());
        assert_eq!(q.y.to_bits(), p.y.to_bits());
    }

    /// `det` signs a reflection; `similarity_scale` accepts rotation,
    /// uniform scale and reflection, refuses non-uniform scale and skew.
    #[test]
    fn det_and_similarity_scale() {
        assert!(Matrix::scale(1.0, -1.0).det() < 0.0);
        assert_eq!(
            parsed("rotate(30) scale(2)")
                .similarity_scale()
                .map(|s| (s - 2.0).abs() < 1e-12),
            Some(true)
        );
        assert_eq!(Matrix::scale(-3.0, 3.0).similarity_scale(), Some(3.0));
        assert_eq!(Matrix::IDENTITY.similarity_scale(), Some(1.0));
        assert_eq!(Matrix::scale(2.0, 3.0).similarity_scale(), None);
        assert_eq!(parsed("skewX(10)").similarity_scale(), None);
        assert_eq!(Matrix::scale(0.0, 0.0).similarity_scale(), None);
    }

    /// A collapsed or non-finite map is singular; a tiny but regular one is not.
    #[test]
    fn singular_maps() {
        assert!(Matrix::scale(0.0, 1.0).is_singular());
        assert!(parsed("matrix(1 2 2 4 0 0)").is_singular());
        assert!(Matrix::scale(f64::NAN, 1.0).is_singular());
        assert!(!Matrix::scale(1e-6, 1e-6).is_singular());
        assert!(!Matrix::IDENTITY.is_singular());
    }

    /// AC 5 — the six functions, with their optional arguments.
    #[test]
    fn the_six_functions() {
        let (s30, c30) = 30_f64.to_radians().sin_cos();
        let t30 = 30_f64.to_radians().tan();
        for (raw, want) in [
            ("matrix(1 2 3 4 5 6)", [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
            ("translate(5)", [1.0, 0.0, 0.0, 1.0, 5.0, 0.0]),
            ("translate(5 -7)", [1.0, 0.0, 0.0, 1.0, 5.0, -7.0]),
            ("scale(2)", [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]),
            ("scale(2 3)", [2.0, 0.0, 0.0, 3.0, 0.0, 0.0]),
            ("rotate(30)", [c30, s30, -s30, c30, 0.0, 0.0]),
            ("skewX(30)", [1.0, 0.0, t30, 1.0, 0.0, 0.0]),
            ("skewY(30)", [1.0, t30, 0.0, 1.0, 0.0, 0.0]),
        ] {
            assert!(close(parsed(raw), want), "{raw}: {:?}", parsed(raw));
        }
        let about = parsed("rotate(90 10 20)");
        let p = about.apply(Vec2::new(10.0, 20.0));
        assert!(
            (p.x - 10.0).abs() < 1e-12 && (p.y - 20.0).abs() < 1e-12,
            "centre fixed"
        );
        let q = about.apply(Vec2::new(11.0, 20.0));
        assert!(
            (q.x - 10.0).abs() < 1e-12 && (q.y - 21.0).abs() < 1e-12,
            "{q:?}"
        );
    }

    /// AC 5 — commas and/or whitespace anywhere, glued signs, exponents.
    #[test]
    fn separators() {
        let want = [1.0, 0.0, 0.0, 1.0, 10.0, -5.0];
        for raw in [
            "translate(10,-5)",
            "translate(10-5)",
            " translate ( 10 , -5 ) ",
            "translate(1e1\n-.5e1)",
            "translate(10, -5),",
        ] {
            assert!(close(parsed(raw), want), "{raw}");
        }
        assert_eq!(parse_transform(""), Some(Matrix::IDENTITY));
        assert_eq!(parse_transform("  "), Some(Matrix::IDENTITY));
    }

    /// AC 5 — a list composes left to right: the rightmost applies first.
    #[test]
    fn a_list_composes_left_to_right() {
        let p = Vec2::new(1.0, 0.0);
        for raw in [
            "translate(10) scale(2)",
            "translate(10),scale(2)",
            "translate(10)scale(2)",
        ] {
            assert_eq!(parsed(raw).apply(p), Vec2::new(12.0, 0.0), "{raw}");
        }
        assert_eq!(
            parsed("scale(2) translate(10)").apply(p),
            Vec2::new(22.0, 0.0)
        );
    }

    /// AC 8 — anything else does not parse.
    #[test]
    fn invalid_lists_are_none() {
        for raw in [
            "red",
            "none",
            "translate",
            "translate(",
            "translate()",
            "translate(1 2 3)",
            "matrix(1 2 3 4 5)",
            "rotate(1 2)",
            "skewX(1 2)",
            "Scale(2)",
            "scale(2) junk",
            "scale(2))",
            "scale(a)",
            "scale(1e)",
            "scale(inf)",
            "scale(1e999)",
            "translate(1,,2)x",
        ] {
            assert_eq!(parse_transform(raw), None, "{raw:?}");
        }
    }
}
