//! SVG 2 viewports (LCV-173 AC 4, AC 9): `viewBox`, `preserveAspectRatio`
//! and the context the import walk carries down the tree.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::length::{parse_length, to_user};
use super::matrix::Matrix;

/// What the import walk knows at one element.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Ctx {
    /// Current user units → bed millimetres, Y-down (before the world mirror).
    pub(super) ctm: Matrix,
    /// The nearest viewport's `[width, height]` in current user units, the
    /// reference of `%` lengths.
    pub(super) viewport: [f64; 2],
}

/// A parsed `preserveAspectRatio`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Par {
    /// The `[x, y]` alignment as a fraction of the free space (0 min,
    /// 0.5 mid, 1 max); `None` for `none` (non-uniform scale).
    pub(super) align: Option<[f64; 2]>,
    /// `slice` (cover the viewport) rather than `meet` (fit inside).
    pub(super) slice: bool,
}

/// The default `xMidYMid meet`.
const DEFAULT_PAR: Par = Par {
    align: Some([0.5, 0.5]),
    slice: false,
};

/// Parse `preserveAspectRatio`; absent or invalid is `xMidYMid meet`, and a
/// leading `defer` is ignored.
pub(super) fn par(raw: Option<&str>) -> Par {
    let mut tok = raw.unwrap_or("").split_ascii_whitespace().peekable();
    tok.next_if_eq(&"defer");
    let align = match tok.next() {
        Some("none") => None,
        Some(a) if a.len() == 8 => match (fraction(&a[..4], 'x'), fraction(&a[4..], 'Y')) {
            (Some(x), Some(y)) => Some([x, y]),
            _ => return DEFAULT_PAR,
        },
        _ => return DEFAULT_PAR,
    };
    let slice = match tok.next() {
        None | Some("meet") => false,
        Some("slice") => true,
        Some(_) => return DEFAULT_PAR,
    };
    if tok.next().is_some() {
        return DEFAULT_PAR;
    }
    Par { align, slice }
}

/// `xMin`/`xMid`/`xMax` (or `Y…`) as 0, 0.5, 1.
fn fraction(part: &str, axis: char) -> Option<f64> {
    let rest = part.strip_prefix(axis)?;
    match rest {
        "Min" => Some(0.0),
        "Mid" => Some(0.5),
        "Max" => Some(1.0),
        _ => None,
    }
}

/// A `viewBox`: four finite numbers `[x, y, width, height]` with positive
/// size, comma and/or whitespace separated; `None` otherwise.
pub(super) fn parse_view_box(raw: &str) -> Option<[f64; 4]> {
    let mut nums = raw
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<f64>().ok().filter(|v| v.is_finite()));
    let vb = [nums.next()??, nums.next()??, nums.next()??, nums.next()??];
    (nums.next().is_none() && vb[2] > 0.0 && vb[3] > 0.0).then_some(vb)
}

/// The SVG 2 §8.2 viewBox-to-viewport map: `vb` user units onto the
/// viewport `rect = [x, y, width, height]` per `par`.
pub(super) fn view_box_map(vb: [f64; 4], rect: [f64; 4], par: Par) -> Matrix {
    let (mut sx, mut sy) = (rect[2] / vb[2], rect[3] / vb[3]);
    if par.align.is_some() {
        let s = if par.slice { sx.max(sy) } else { sx.min(sy) };
        (sx, sy) = (s, s);
    }
    let (mut tx, mut ty) = (rect[0] - vb[0] * sx, rect[1] - vb[1] * sy);
    if let Some([ax, ay]) = par.align {
        tx += (rect[2] - vb[2] * sx) * ax;
        ty += (rect[3] - vb[3] * sy) * ay;
    }
    Matrix {
        a: sx,
        b: 0.0,
        c: 0.0,
        d: sy,
        e: tx,
        f: ty,
    }
}

/// The context inside a nested `<svg>` (LCV-173 AC 9), given `ctx` (its
/// parent's, with its own `transform` already composed): its `viewBox`
/// mapped per `preserveAspectRatio` onto `x y width height`, or a plain
/// translation by `x y` without one. `x`, `y` default to 0 and `width`,
/// `height` to 100%; `%` refers to the parent viewport; an invalid length or
/// viewBox counts as absent. Nothing is clipped.
pub(super) fn nested(node: roxmltree::Node<'_, '_>, ctx: &Ctx) -> Ctx {
    let [pw, ph] = ctx.viewport;
    let len = |attr: &str, reference: f64, default: f64| {
        node.attribute(attr)
            .and_then(parse_length)
            .map_or(default, |l| to_user(l, reference))
    };
    let rect = [
        len("x", pw, 0.0),
        len("y", ph, 0.0),
        len("width", pw, pw),
        len("height", ph, ph),
    ];
    let (map, viewport) = match node.attribute("viewBox").and_then(parse_view_box) {
        Some(vb) => {
            let par = par(node.attribute("preserveAspectRatio"));
            (view_box_map(vb, rect, par), [vb[2], vb[3]])
        }
        None => (Matrix::translate(rect[0], rect[1]), [rect[2], rect[3]]),
    };
    Ctx {
        ctm: map.then(ctx.ctm),
        viewport,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    /// AC 4 — `preserveAspectRatio` values, `defer` and the default.
    #[test]
    fn par_parses_every_align_and_falls_back_to_the_default() {
        let p = |raw: &str| par(Some(raw));
        assert_eq!(par(None), DEFAULT_PAR);
        assert_eq!(p("xMinYMax slice").align, Some([0.0, 1.0]));
        assert!(p("xMinYMax slice").slice);
        assert_eq!(p("defer xMaxYMid meet").align, Some([1.0, 0.5]));
        assert_eq!(p("none").align, None);
        assert!(p(" none  slice ").slice);
        for bad in [
            "",
            "xmidymid",
            "xMidYMid cover",
            "xMidYMid meet x",
            "yMidxMid",
            "xMidYMidd",
        ] {
            assert_eq!(p(bad), DEFAULT_PAR, "{bad:?}");
        }
    }

    /// The viewBox grammar; a bad one is `None`.
    #[test]
    fn view_box_grammar() {
        assert_eq!(
            parse_view_box("0 0 300 180"),
            Some([0.0, 0.0, 300.0, 180.0])
        );
        assert_eq!(
            parse_view_box("-5,10, 30\n20"),
            Some([-5.0, 10.0, 30.0, 20.0])
        );
        for bad in [
            "0 0 300",
            "0 0 300 180 1",
            "0 0 0 180",
            "0 0 300 -1",
            "a b c d",
            "0 0 inf 1",
            "",
        ] {
            assert_eq!(parse_view_box(bad), None, "{bad:?}");
        }
    }

    /// `vb`'s corners mapped onto the viewport by `par`.
    fn corners(par_raw: &str) -> (Vec2, Vec2) {
        // A 100 × 50 viewBox at (-10, 20) onto a 400 × 400 viewport at (5, 7).
        let m = view_box_map(
            [-10.0, 20.0, 100.0, 50.0],
            [5.0, 7.0, 400.0, 400.0],
            par(Some(par_raw)),
        );
        (
            m.apply(Vec2::new(-10.0, 20.0)),
            m.apply(Vec2::new(90.0, 70.0)),
        )
    }

    fn near(p: Vec2, x: f64, y: f64) -> bool {
        (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9
    }

    /// AC 4 — all nine aligns under `meet` (scale 4, the content is 400 × 200,
    /// free space is vertical) and `slice` (scale 8, 800 × 400, free space is
    /// horizontal and negative), with an offset origin.
    #[test]
    fn nine_aligns_meet_and_slice() {
        for (ax, fx) in [("xMin", 0.0), ("xMid", 0.5), ("xMax", 1.0)] {
            for (ay, fy) in [("YMin", 0.0), ("YMid", 0.5), ("YMax", 1.0)] {
                let (lo, hi) = corners(&format!("{ax}{ay} meet"));
                let y0 = 7.0 + 200.0 * fy;
                assert!(
                    near(lo, 5.0, y0) && near(hi, 405.0, y0 + 200.0),
                    "{ax}{ay} meet {lo:?} {hi:?}"
                );
                let (lo, hi) = corners(&format!("{ax}{ay} slice"));
                let x0 = 5.0 - 400.0 * fx;
                assert!(
                    near(lo, x0, 7.0) && near(hi, x0 + 800.0, 407.0),
                    "{ax}{ay} slice {lo:?} {hi:?}"
                );
            }
        }
    }

    /// AC 4 — `none` stretches each axis on its own.
    #[test]
    fn none_scales_each_axis() {
        let (lo, hi) = corners("none");
        assert!(
            near(lo, 5.0, 7.0) && near(hi, 405.0, 407.0),
            "{lo:?} {hi:?}"
        );
    }

    /// AC 11 — LaserCAD's own `0 0 W H` onto a `W × H` bed is the identity.
    #[test]
    fn own_view_box_maps_to_the_identity() {
        let m = view_box_map(
            [0.0, 0.0, 300.5, 180.25],
            [0.0, 0.0, 300.5, 180.25],
            DEFAULT_PAR,
        );
        assert_eq!(m, Matrix::IDENTITY);
    }
}
