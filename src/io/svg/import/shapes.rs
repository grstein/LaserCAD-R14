//! Basic shapes on import (LCV-174, SVG 2 ch. 10): `<line>`, `<circle>`
//! and `<ellipse>` (LCV-176) as world entities, one [`Shape`] per element.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::conic::{Conic, conic_entity};
use super::{SvgImportError, malformed, to_world};
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

/// What one shape element adds: its entities and its report labels.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Shape {
    /// World entities, in drawing order.
    pub(super) entities: Vec<Entity>,
    /// Report labels, one per problem.
    pub(super) notes: Vec<&'static str>,
}

impl Shape {
    fn of(entity: Option<Entity>) -> Self {
        Self {
            entities: entity.into_iter().collect(),
            notes: Vec::new(),
        }
    }
}

/// The report label of an `<ellipse>` without a positive radius (LCV-176).
const INVALID_ELLIPSE: &str = "ellipse (invalid radius)";

/// The shape element `name` (`n`) in context `ctx`, un-mirrored around
/// `bed_h`.
pub(super) fn import_shape(
    name: &str,
    n: roxmltree::Node<'_, '_>,
    ctx: &Ctx,
    bed_h: f64,
) -> Result<Shape, SvgImportError> {
    Ok(match name {
        "line" => Shape::of(Some(line(n, ctx, bed_h)?)),
        "circle" => Shape::of(circle(n, ctx, bed_h)?),
        "ellipse" => match ellipse(n, ctx, bed_h)? {
            None => Shape {
                entities: Vec::new(),
                notes: vec![INVALID_ELLIPSE],
            },
            some => Shape::of(some),
        },
        _ => Shape::default(),
    })
}

/// Attribute `a` of `n` (element `el`), which must be a length.
fn required(
    n: roxmltree::Node<'_, '_>,
    el: &'static str,
    a: &'static str,
    axis: Axis,
    ctx: &Ctx,
) -> Result<f64, SvgImportError> {
    match attr(n, a, axis, ctx) {
        Attr::Value(v) => Ok(v),
        _ => Err(malformed(el, a, n.attribute(a).unwrap_or("").to_string())),
    }
}

fn line(n: roxmltree::Node<'_, '_>, ctx: &Ctx, bed_h: f64) -> Result<Entity, SvgImportError> {
    let len = |a, axis| required(n, "line", a, axis, ctx);
    let p1 = Vec2::new(len("x1", Axis::X)?, len("y1", Axis::Y)?);
    let p2 = Vec2::new(len("x2", Axis::X)?, len("y2", Axis::Y)?);
    Ok(Entity::Line(Line::new(
        to_world(ctx, p1, bed_h),
        to_world(ctx, p2, bed_h),
    )))
}

/// A `<circle>`: a [`Circle`] under a similarity, else the exact image
/// ellipse (LCV-176 AC 3); `Ok(None)` only when that image is degenerate.
fn circle(
    n: roxmltree::Node<'_, '_>,
    ctx: &Ctx,
    bed_h: f64,
) -> Result<Option<Entity>, SvgImportError> {
    let len = |a, axis| required(n, "circle", a, axis, ctx);
    let c = Vec2::new(len("cx", Axis::X)?, len("cy", Axis::Y)?);
    let r = len("r", Axis::Diag)?;
    if r <= 0.0 {
        let v = n.attribute("r").unwrap_or("").to_string();
        return Err(malformed("circle", "r", v));
    }
    if let Some(s) = ctx.ctm.similarity_scale() {
        let circle = Circle::new(to_world(ctx, c, bed_h), r * s);
        return Ok(Some(Entity::Circle(circle)));
    }
    let k = Conic {
        center: c,
        u: Vec2::new(r, 0.0),
        v: Vec2::new(0.0, r),
        span: None,
    };
    Ok(conic_entity(ctx, k, bed_h))
}

/// An `<ellipse>` as a world entity (LCV-176 AC 2): a missing or `auto`
/// radius takes the other's value. `Ok(None)` when neither is given, either
/// is ≤ 0, or the mapped ellipse is degenerate.
fn ellipse(
    n: roxmltree::Node<'_, '_>,
    ctx: &Ctx,
    bed_h: f64,
) -> Result<Option<Entity>, SvgImportError> {
    let len = |a, axis| required(n, "ellipse", a, axis, ctx);
    let center = Vec2::new(len("cx", Axis::X)?, len("cy", Axis::Y)?);
    let radius = |a, axis| match n.attribute(a).map(str::trim) {
        None | Some("auto") => Ok(None),
        Some(_) => len(a, axis).map(Some),
    };
    let (rx, ry) = match (radius("rx", Axis::X)?, radius("ry", Axis::Y)?) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(r), None) | (None, Some(r)) => (r, r),
        (None, None) => return Ok(None),
    };
    if !(rx > 0.0 && ry > 0.0) {
        return Ok(None);
    }
    let k = Conic {
        center,
        u: Vec2::new(rx, 0.0),
        v: Vec2::new(0.0, ry),
        span: None,
    };
    Ok(conic_entity(ctx, k, bed_h))
}
