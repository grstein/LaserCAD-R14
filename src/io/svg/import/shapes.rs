//! Basic shapes on import (LCV-174, SVG 2 ch. 10): `<line>`, `<circle>`
//! and `<ellipse>` (LCV-176) as world entities, one [`Shape`] per element.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::conic::{Conic, conic_entity};
use super::to_world;
use crate::document::entity::Entity;
use crate::geometry::{Circle, Line, Vec2};
use crate::io::svg::length::{parse_length, to_user};
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
        "line" => (line(n, ctx, bed_h).map(Some), "line (invalid attribute)"),
        "circle" => (circle(n, ctx, bed_h), "circle (invalid attribute)"),
        "ellipse" => (ellipse(n, ctx, bed_h), "ellipse (invalid attribute)"),
        _ => return Shape::default(),
    };
    match drawn {
        Ok(entity) => Shape {
            entities: entity.into_iter().collect(),
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
