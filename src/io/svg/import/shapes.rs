//! Basic shapes on import (LCV-174, SVG 2 ch. 10): `<line>`, `<circle>`,
//! `<ellipse>` (LCV-176) and `<rect>` as world entities, one [`Shape`] per
//! element. A `<rect>` becomes its equivalent path data and goes through
//! [`path_entities`], so it maps like a `<path>`.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::conic::{Conic, conic_entity};
use super::path::path_entities;
use super::to_world;
use crate::document::entity::Entity;
use crate::geometry::{Circle, Line, Vec2};
use crate::io::svg::length::{parse_length, to_user};
use crate::io::svg::path_data::{PathData, Segment};
use crate::io::svg::viewport::Ctx;

/// Which viewport side a `%` length refers to (LCV-173 AC 10).
#[derive(Debug, Clone, Copy)]
pub(super) enum Axis {
    /// The viewport width (`x1`, `x2`, `cx`, `x`, `width`, `rx`).
    X,
    /// The viewport height (`y1`, `y2`, `cy`, `y`, `height`, `ry`).
    Y,
    /// The normalized diagonal `√(w² + h²) / √2` (`r`).
    Diag,
}

/// One geometry attribute as read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Attr {
    /// Not given.
    Missing,
    /// A length in user units.
    Value(f64),
    /// Given but not a length.
    Invalid,
}

/// Attribute `name` of `n` as a length in user units, `%` resolved against
/// `ctx.viewport` along `axis` (LCV-173 AC 10).
pub(super) fn attr(n: roxmltree::Node<'_, '_>, name: &str, axis: Axis, ctx: &Ctx) -> Attr {
    let Some(raw) = n.attribute(name) else {
        return Attr::Missing;
    };
    let Some(len) = parse_length(raw) else {
        return Attr::Invalid;
    };
    let [w, h] = ctx.viewport;
    let reference = match axis {
        Axis::X => w,
        Axis::Y => h,
        Axis::Diag => w.hypot(h) / core::f64::consts::SQRT_2,
    };
    Attr::Value(to_user(len, reference))
}

impl Attr {
    /// A position: 0 when missing (AC 1).
    fn pos(self) -> Result<f64, Invalid> {
        match self {
            Attr::Missing => Ok(0.0),
            Attr::Value(v) => Ok(v),
            Attr::Invalid => Err(Invalid),
        }
    }

    /// A size or radius: `None` when missing; negative is invalid (AC 3).
    fn size(self) -> Result<Option<f64>, Invalid> {
        match self {
            Attr::Missing => Ok(None),
            Attr::Value(v) if v >= 0.0 => Ok(Some(v)),
            Attr::Value(_) | Attr::Invalid => Err(Invalid),
        }
    }
}

/// A radius attribute: [`attr`], with `auto` (ASCII case-insensitive) read
/// as [`Attr::Missing`] (SVG 2 §10.4, §10.6).
fn radius(n: roxmltree::Node<'_, '_>, name: &str, axis: Axis, ctx: &Ctx) -> Attr {
    match n.attribute(name) {
        Some(raw) if raw.trim().eq_ignore_ascii_case("auto") => Attr::Missing,
        _ => attr(n, name, axis, ctx),
    }
}

/// A geometry attribute that is negative where SVG 2 forbids it, or does
/// not parse: the element draws nothing and is reported (AC 3).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Invalid;

/// What one shape element adds: its entities and its report labels.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Shape {
    /// World entities, in drawing order.
    pub(super) entities: Vec<Entity>,
    /// Report labels, one per problem.
    pub(super) notes: Vec<&'static str>,
}

/// The shape element `name` (`n`) in context `ctx`, un-mirrored around
/// `bed_h`. A missing position is 0 (AC 1); a missing or zero size draws
/// nothing (AC 2); an invalid attribute draws nothing and adds one
/// `<name> (invalid attribute)` note (AC 3).
pub(super) fn import_shape(name: &str, n: roxmltree::Node<'_, '_>, ctx: &Ctx, bed_h: f64) -> Shape {
    let (drawn, invalid) = match name {
        "line" => (
            line(n, ctx, bed_h).map(|e| vec![e]),
            "line (invalid attribute)",
        ),
        "circle" => (
            circle(n, ctx, bed_h).map(Vec::from_iter),
            "circle (invalid attribute)",
        ),
        "ellipse" => (
            ellipse(n, ctx, bed_h).map(Vec::from_iter),
            "ellipse (invalid attribute)",
        ),
        "rect" => (
            rect(n, ctx).map(|d| path_entities(&d, ctx, bed_h).0),
            "rect (invalid attribute)",
        ),
        _ => return Shape::default(),
    };
    match drawn {
        Ok(entities) => Shape {
            entities,
            notes: Vec::new(),
        },
        Err(Invalid) => Shape {
            entities: Vec::new(),
            notes: vec![invalid],
        },
    }
}

/// The point `(x, y)` of `n`, each 0 when missing.
fn point(n: roxmltree::Node<'_, '_>, x: &str, y: &str, ctx: &Ctx) -> Result<Vec2, Invalid> {
    let x = attr(n, x, Axis::X, ctx).pos()?;
    Ok(Vec2::new(x, attr(n, y, Axis::Y, ctx).pos()?))
}

fn line(n: roxmltree::Node<'_, '_>, ctx: &Ctx, bed_h: f64) -> Result<Entity, Invalid> {
    let p1 = point(n, "x1", "y1", ctx)?;
    let p2 = point(n, "x2", "y2", ctx)?;
    Ok(Entity::Line(Line::new(
        to_world(ctx, p1, bed_h),
        to_world(ctx, p2, bed_h),
    )))
}

/// A `<circle>`: a [`Circle`] under a similarity, else the exact image
/// ellipse (LCV-176 AC 3); `Ok(None)` when `r` is missing or 0, or the
/// image is degenerate.
fn circle(n: roxmltree::Node<'_, '_>, ctx: &Ctx, bed_h: f64) -> Result<Option<Entity>, Invalid> {
    let c = point(n, "cx", "cy", ctx)?;
    let r = attr(n, "r", Axis::Diag, ctx).size()?.unwrap_or(0.0);
    if r == 0.0 {
        return Ok(None);
    }
    if let Some(s) = ctx.ctm.similarity_scale() {
        let circle = Circle::new(to_world(ctx, c, bed_h), r * s);
        return Ok(Some(Entity::Circle(circle)));
    }
    Ok(conic(ctx, c, (r, r), bed_h))
}

/// An `<ellipse>` as a world entity (LCV-176 AC 2): a missing or `auto`
/// radius takes the other's value. `Ok(None)` when neither is given,
/// either is 0, or the mapped ellipse is degenerate.
fn ellipse(n: roxmltree::Node<'_, '_>, ctx: &Ctx, bed_h: f64) -> Result<Option<Entity>, Invalid> {
    let center = point(n, "cx", "cy", ctx)?;
    let rx = radius(n, "rx", Axis::X, ctx).size()?;
    let ry = radius(n, "ry", Axis::Y, ctx).size()?;
    let (rx, ry) = match (rx, ry) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(r), None) | (None, Some(r)) => (r, r),
        (None, None) => return Ok(None),
    };
    if rx == 0.0 || ry == 0.0 {
        return Ok(None);
    }
    Ok(conic(ctx, center, (rx, ry), bed_h))
}

/// A `<rect>` as its SVG 2 §10.2 equivalent path; empty when `width` or
/// `height` is missing or 0 (AC 2).
fn rect(n: roxmltree::Node<'_, '_>, ctx: &Ctx) -> Result<PathData, Invalid> {
    let at = point(n, "x", "y", ctx)?;
    let w = attr(n, "width", Axis::X, ctx).size()?.unwrap_or(0.0);
    let h = attr(n, "height", Axis::Y, ctx).size()?.unwrap_or(0.0);
    let radii = radii(n, ctx, w, h)?;
    if w == 0.0 || h == 0.0 {
        return Ok(PathData::default());
    }
    Ok(rect_path(at, w, h, radii))
}

/// The used corner radii of a `w` × `h` `<rect>` (AC 5): `rx` and `ry`
/// read with `auto` as missing, then [`resolve_radii`].
fn radii(n: roxmltree::Node<'_, '_>, ctx: &Ctx, w: f64, h: f64) -> Result<(f64, f64), Invalid> {
    let rx = radius(n, "rx", Axis::X, ctx).size()?;
    let ry = radius(n, "ry", Axis::Y, ctx).size()?;
    Ok(resolve_radii(rx, ry, w, h))
}

/// SVG 2 §10.2: a missing (or `auto`) radius is the other's, both missing
/// is 0, then each clamps to half its side. Either at 0 gives `(0, 0)`, a
/// sharp rect.
fn resolve_radii(rx: Option<f64>, ry: Option<f64>, w: f64, h: f64) -> (f64, f64) {
    let (rx, ry) = match (rx, ry) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(r), None) | (None, Some(r)) => (r, r),
        (None, None) => (0.0, 0.0),
    };
    let (rx, ry) = (rx.min(w / 2.0), ry.min(h / 2.0));
    if rx == 0.0 || ry == 0.0 {
        (0.0, 0.0)
    } else {
        (rx, ry)
    }
}

/// The equivalent path of the rect at `at`, `w` × `h`, corner radii
/// `(rx, ry)` (SVG 2 §10.2): from the top side clockwise in SVG space, each
/// side followed by its quarter-arc corner when `rx > 0` (AC 4, AC 6). A
/// side of length 0 is dropped later by [`path_entities`].
fn rect_path(at: Vec2, w: f64, h: f64, (rx, ry): (f64, f64)) -> PathData {
    let (x0, y0, x1, y1) = (at.x, at.y, at.x + w, at.y + h);
    let sides = [
        (Vec2::new(x0 + rx, y0), Vec2::new(x1 - rx, y0)),
        (Vec2::new(x1, y0 + ry), Vec2::new(x1, y1 - ry)),
        (Vec2::new(x1 - rx, y1), Vec2::new(x0 + rx, y1)),
        (Vec2::new(x0, y1 - ry), Vec2::new(x0, y0 + ry)),
    ];
    let mut segments = Vec::with_capacity(8);
    for (i, &(a, b)) in sides.iter().enumerate() {
        segments.push(Segment::Line(a, b));
        if rx > 0.0 {
            segments.push(Segment::Arc {
                from: b,
                to: sides[(i + 1) % 4].0,
                rx,
                ry,
                phi: 0.0,
                large: false,
                sweep: true,
            });
        }
    }
    PathData {
        segments,
        error: false,
    }
}

/// The axis-aligned ellipse `center`, `(rx, ry)` (user space) as a world
/// entity through [`conic_entity`].
fn conic(ctx: &Ctx, center: Vec2, (rx, ry): (f64, f64), bed_h: f64) -> Option<Entity> {
    let k = Conic {
        center,
        u: Vec2::new(rx, 0.0),
        v: Vec2::new(0.0, ry),
        span: None,
    };
    conic_entity(ctx, k, bed_h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::svg::matrix::Matrix;

    /// The resolved radii of `<rect {attrs}/>`, `w` × `h`, in a 200 × 100
    /// viewport.
    fn radii_of(attrs: &str, w: f64, h: f64) -> Result<(f64, f64), Invalid> {
        let src = format!("<rect {attrs}/>");
        let doc = roxmltree::Document::parse(&src).expect("test XML");
        let ctx = Ctx {
            ctm: Matrix::IDENTITY,
            viewport: [200.0, 100.0],
        };
        radii(doc.root_element(), &ctx, w, h)
    }

    /// AC 5 — SVG 2 §10.2 radius resolution: a missing or `auto` radius is
    /// the other, both absent is 0, each clamps to half its side, and either
    /// at 0 makes the rect sharp.
    #[test]
    fn rect_radii_resolve_per_svg_2() {
        let table = [
            (r#"rx="3""#, (3.0, 3.0)),
            (r#"ry="4""#, (4.0, 4.0)),
            (r#"rx="auto" ry="4""#, (4.0, 4.0)),
            (r#"rx="3" ry=" AUTO ""#, (3.0, 3.0)),
            ("", (0.0, 0.0)),
            (r#"rx="auto" ry="auto""#, (0.0, 0.0)),
            (r#"rx="3" ry="2""#, (3.0, 2.0)),
            (r#"rx="0" ry="5""#, (0.0, 0.0)),
            (r#"rx="15""#, (10.0, 15.0)),
            (r#"rx="50" ry="50""#, (10.0, 20.0)),
            (r#"rx="5%" ry="5%""#, (10.0, 5.0)),
        ];
        for (attrs, want) in table {
            assert_eq!(radii_of(attrs, 20.0, 40.0), Ok(want), "{attrs}");
        }
        assert_eq!(resolve_radii(Some(30.0), None, 20.0, 40.0), (10.0, 20.0));
    }

    /// AC 3 — a negative or unparseable radius is invalid, `auto` or not.
    #[test]
    fn a_negative_or_unparseable_radius_is_invalid() {
        for attrs in [
            r#"rx="-1""#,
            r#"ry="-1" rx="auto""#,
            r#"rx="x""#,
            r#"ry="1q2""#,
        ] {
            assert_eq!(radii_of(attrs, 20.0, 40.0), Err(Invalid), "{attrs}");
        }
    }
}
