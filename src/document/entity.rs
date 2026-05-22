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

use serde::{Deserialize, Serialize};

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
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    /// Translate the entity in place by `delta` (mm).
    ///
    /// Pure rigid translation: line endpoints both shift by `delta`, circle
    /// and arc centers shift by `delta`, radii and arc angles are unchanged.
    /// Translation is exactly invertible at floating-point precision —
    /// `e.translate(d); e.translate(-d);` returns the entity to its original
    /// state within `EPSILON` (used by [`super::commands::MoveEntities`] for
    /// undo).
    ///
    /// Introduced by demand LCV-024.
    pub fn translate(&mut self, delta: Vec2) {
        match self {
            Entity::Line(line) => {
                line.p1 = line.p1 + delta;
                line.p2 = line.p2 + delta;
            }
            Entity::Circle(circle) => {
                circle.center = circle.center + delta;
            }
            Entity::Arc(arc) => {
                arc.center = arc.center + delta;
            }
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

    /// LCV-024 — `translate` shifts the line endpoints, circle center, and
    /// arc center by `delta`; radii and arc angles are unchanged.
    #[test]
    fn entity_translate_line_circle_arc() {
        let delta = Vec2::new(3.0, -4.0);

        let mut e_line = Entity::Line(Line::new(Vec2::new(1.0, 1.0), Vec2::new(5.0, 2.0)));
        e_line.translate(delta);
        if let Entity::Line(l) = e_line {
            assert!(l.p1.approx_eq(Vec2::new(4.0, -3.0), EPSILON));
            assert!(l.p2.approx_eq(Vec2::new(8.0, -2.0), EPSILON));
        } else {
            panic!("variant changed under translate");
        }

        let mut e_circle = Entity::Circle(Circle::new(Vec2::new(10.0, 10.0), 2.5));
        e_circle.translate(delta);
        if let Entity::Circle(c) = e_circle {
            assert!(c.center.approx_eq(Vec2::new(13.0, 6.0), EPSILON));
            assert!((c.r - 2.5).abs() < EPSILON);
        } else {
            panic!("variant changed under translate");
        }

        let mut e_arc = Entity::Arc(Arc::new(Vec2::new(0.0, 0.0), 1.0, 0.0, FRAC_PI_2, true));
        e_arc.translate(delta);
        if let Entity::Arc(a) = e_arc {
            assert!(a.center.approx_eq(Vec2::new(3.0, -4.0), EPSILON));
            assert!((a.r - 1.0).abs() < EPSILON);
            assert!((a.start_angle - 0.0).abs() < EPSILON);
            assert!((a.end_angle - FRAC_PI_2).abs() < EPSILON);
            assert!(a.ccw);
        } else {
            panic!("variant changed under translate");
        }
    }

    /// LCV-024 — translate-then-translate-by-negation restores the entity to
    /// its original state within `EPSILON`. This is the property
    /// [`super::commands::MoveEntities::undo`] relies on.
    #[test]
    fn entity_translate_roundtrip_with_negation() {
        let delta = Vec2::new(7.5, -2.25);
        let original = Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)));
        let mut moved = original;
        moved.translate(delta);
        moved.translate(-delta);
        if let (Entity::Line(o), Entity::Line(m)) = (original, moved) {
            assert!(o.p1.approx_eq(m.p1, EPSILON));
            assert!(o.p2.approx_eq(m.p2, EPSILON));
        } else {
            panic!("variant changed under translate");
        }
    }
}
