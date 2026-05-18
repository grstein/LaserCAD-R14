//! Canonical document-level entity enum and document file schema version.
//!
//! [`Entity`] is the single sum type used throughout the document model and
//! every downstream consumer (`Command` trait, history stack, selection,
//! render, tools, SVG export). It wraps the kernel's geometry value types
//! (`Line`, `Circle`, `Arc`) declared in [`crate::geometry`] and never adds
//! its own coordinates — the variants are pure carriers of the existing
//! primitives.
//!
//! [`SCHEMA_VERSION`] stamps the on-disk document envelope. See
//! [`crate::document::schema`] for the bump-rule contract; this module owns
//! only the constant itself.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. The document model is part of
//! the pure-Rust kernel.
//!
//! Introduced by demand LCV-020.

use crate::geometry::{Arc, Circle, Line, Vec2};

/// A document-level entity: a tagged union over the kernel's geometry
/// primitives.
///
/// Variants carry the geometry value types unchanged — `Entity::Line` is a
/// thin wrapper around [`Line`], etc. No identity or document-level state is
/// stored here; the index inside `Document::entities` is the addressable
/// handle until a stable-id demand argues otherwise (out of scope here).
///
/// Equality (`==`) is bit-exact `f64` comparison via the inner variant's
/// `PartialEq`. No `Eq` / `Hash` because every variant transitively carries
/// `f64`. Tolerance-aware comparisons live on the inner types
/// (`Vec2::approx_eq` and friends).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Entity {
    /// A line segment.
    Line(Line),
    /// A full circle.
    Circle(Circle),
    /// A proper arc (sweep `< 2π`).
    Arc(Arc),
}

impl Entity {
    /// Axis-aligned bounding box of the entity, in `(min, max)` mm-space
    /// order.
    ///
    /// Delegates to the variant's own `bbox` method ([`Line::bbox`],
    /// [`Circle::bbox`], [`Arc::bbox`]) — this method adds no geometry of its
    /// own.
    pub fn bbox(&self) -> (Vec2, Vec2) {
        match self {
            Entity::Line(line) => line.bbox(),
            Entity::Circle(circle) => circle.bbox(),
            Entity::Arc(arc) => arc.bbox(),
        }
    }

    /// Short lowercase ASCII tag identifying the variant.
    ///
    /// Returns exactly `"line"`, `"circle"`, or `"arc"`. The strings are safe
    /// for logs, telemetry, and any future `serde` tag without further
    /// mapping.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Entity::Line(_) => "line",
            Entity::Circle(_) => "circle",
            Entity::Arc(_) => "arc",
        }
    }
}

/// Canonical document-format schema version.
///
/// Stamps the on-disk envelope shape (informational sketch:
/// `{ "schema_version": 1, "entities": [...] }`). The integer is bumped only
/// on a serialization change that would prevent an older build from cleanly
/// loading a newer file — adding a new optional field does NOT bump; renaming
/// or removing a field DOES. See [`crate::document::schema`] for the full
/// rationale and bump policy.
///
/// The type is `u32` for headroom and to match typical JSON integer ranges.
pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;
    use core::f64::consts::FRAC_PI_2;

    fn bbox_approx_eq(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
        a.0.approx_eq(b.0, EPSILON) && a.1.approx_eq(b.1, EPSILON)
    }

    /// AC#1 — every variant constructs via a struct literal over the kernel
    /// primitive, proving the enum surface and that each variant's inner type
    /// is reachable.
    #[test]
    fn entity_variants_construct_via_literal() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let circle = Circle::new(Vec2::new(0.0, 0.0), 2.0);
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);

        let _e_line = Entity::Line(line);
        let _e_circle = Entity::Circle(circle);
        let _e_arc = Entity::Arc(arc);

        // PartialEq is derived; round-trip equality on a freshly constructed
        // variant is a sanity check.
        assert_eq!(Entity::Line(line), Entity::Line(line));
    }

    /// AC#2 — the schema version constant is pinned at `1`.
    #[test]
    fn schema_version_is_one() {
        assert_eq!(SCHEMA_VERSION, 1);
    }

    /// AC#3 — `Entity::Line(_).bbox()` matches `Line::bbox()` component-wise.
    #[test]
    fn entity_line_bbox_delegates_to_line_bbox() {
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(3.0, 4.0));
        assert!(bbox_approx_eq(Entity::Line(line).bbox(), line.bbox()));
    }

    /// AC#4 — `Entity::Circle(_).bbox()` matches `Circle::bbox()`
    /// component-wise.
    #[test]
    fn entity_circle_bbox_delegates_to_circle_bbox() {
        let circle = Circle::new(Vec2::new(5.0, 5.0), 3.0);
        assert!(bbox_approx_eq(Entity::Circle(circle).bbox(), circle.bbox()));
    }

    /// AC#5 — `Entity::Arc(_).bbox()` matches `Arc::bbox()` component-wise
    /// for a quarter arc (the canonical LCV-013 fixture).
    #[test]
    fn entity_arc_bbox_delegates_to_arc_bbox() {
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);
        assert!(bbox_approx_eq(Entity::Arc(arc).bbox(), arc.bbox()));
    }

    /// AC#6 — `kind_name` returns the exact lowercase ASCII tag per variant.
    #[test]
    fn kind_name_returns_expected_strings() {
        let line = Line::new(Vec2::default(), Vec2::new(1.0, 0.0));
        let circle = Circle::new(Vec2::default(), 1.0);
        let arc = Arc::new(Vec2::default(), 1.0, 0.0, FRAC_PI_2, true);

        assert_eq!(Entity::Line(line).kind_name(), "line");
        assert_eq!(Entity::Circle(circle).kind_name(), "circle");
        assert_eq!(Entity::Arc(arc).kind_name(), "arc");
    }

    /// AC#7 — `SCHEMA_VERSION` is reachable through the `schema` submodule
    /// and resolves to the same constant.
    #[test]
    fn schema_version_reachable_via_schema_module() {
        assert_eq!(crate::document::schema::SCHEMA_VERSION, SCHEMA_VERSION);
    }
}
