//! Snap engine: pick the best geometric feature within tolerance of a cursor.
//!
//! Given a world-space cursor position and a slice of [`SnapEntity`] values,
//! [`snap`] enumerates candidate snap points across four kinds — endpoint,
//! midpoint, center, intersection — and returns the nearest candidate within
//! the supplied tolerance. The selection rule is documented on [`snap`].
//!
//! **Transitional `SnapEntity`.** This module defines a stripped-down
//! [`SnapEntity`] enum (`Line` | `Circle` | `Arc`) as a stand-in for the
//! `document::Entity` enum that will ship with LCV-020. Once LCV-020 lands, a
//! follow-up demand replaces `SnapEntity` with `document::Entity` and removes
//! this enum. Until then, callers convert their entities to `SnapEntity` at
//! the call site (a short match).
//!
//! Kernel-purity contract: no `egui`, `eframe`, or `rfd` imports here. The
//! snap engine must remain testable as a pure library.
//!
//! Split across two files to honor the kernel's 300-LOC-per-file cap:
//!
//! - This file — public types, [`snap`] entry point, picker, tests.
//! - [`candidates`] — candidate enumeration helpers.
//!
//! Frozen by demand LCV-016.

mod candidates;

use crate::geometry::arc::Arc;
use crate::geometry::circle::Circle;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;

use candidates::{Candidate, collect_intersection_candidates, collect_single_entity_candidates};

/// Discrete classification of a snap candidate.
///
/// The ordering of the variants is **not** the tie-break priority — see
/// [`snap`] for the precedence rule. `Eq` and `Hash` are derived because the
/// enum carries no floating-point data.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SnapKind {
    /// Endpoint of a line segment or arc.
    Endpoint,
    /// Midpoint of a line segment.
    Midpoint,
    /// Center of a circle or arc.
    Center,
    /// Intersection point between two entities.
    Intersection,
}

/// Transitional entity type for the snap engine.
///
/// Stripped-down stand-in for `document::Entity` (LCV-020). A follow-up demand
/// replaces this enum with the document enum once LCV-020 ships.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SnapEntity {
    /// A line segment.
    Line(Line),
    /// A full circle.
    Circle(Circle),
    /// A proper arc (sweep < 2π).
    Arc(Arc),
}

/// Outcome of a successful snap query.
///
/// `primary_idx` is the index (into the input slice passed to [`snap`]) of the
/// entity that produced this candidate. For [`SnapKind::Intersection`],
/// `secondary_idx` carries the index of the other participant; for every
/// other kind it is `None`.
///
/// No `Eq` / `Hash` derive because [`Vec2`] carries `f64`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SnapResult {
    /// World-space coordinate of the snapped feature.
    pub point: Vec2,
    /// Kind of feature that produced the snap.
    pub kind: SnapKind,
    /// Index of the primary participant in the input entities slice. For
    /// intersections this is the lower of the two indices.
    pub primary_idx: usize,
    /// Index of the secondary participant; `Some(idx)` only for
    /// [`SnapKind::Intersection`].
    pub secondary_idx: Option<usize>,
}

/// Find the best snap candidate within `tolerance` of `world`.
///
/// Returns `None` when `entities` is empty or when no candidate point lies
/// within `tolerance` (`world`-space units, millimeters).
///
/// # Selection rule
///
/// Among all candidates within `tolerance` of `world`:
///
/// 1. Smallest [`Vec2`] distance wins.
/// 2. On ties (equal distance within [`EPSILON`]), kind priority breaks the
///    tie: `Endpoint > Intersection > Midpoint > Center`. This matches
///    AutoCAD's default OSNAP precedence.
/// 3. On further ties, the smaller `primary_idx` wins.
///
/// # Candidate enumeration
///
/// - **Endpoint**: each [`SnapEntity::Line`] contributes both endpoints; each
///   [`SnapEntity::Arc`] contributes [`Arc::start_point`] and
///   [`Arc::end_point`]. Circles contribute no endpoints.
/// - **Midpoint**: each [`SnapEntity::Line`] contributes [`Line::midpoint`].
///   Arcs and circles do not contribute midpoints in this demand.
/// - **Center**: each [`SnapEntity::Circle`] contributes its center; each
///   [`SnapEntity::Arc`] contributes its center.
/// - **Intersection**: for every unordered pair `(i, j)` with `i < j`,
///   line-line, line-circle, and circle-circle pairs run through
///   [`crate::geometry::intersect`]. Arc-involving pairs are **skipped** —
///   no arc intersection routines exist yet (documented limitation).
///
/// The intersection enumeration is `O(n²)`; for the v2 document size cap
/// (~10k entities) at pointer-event rate this is acceptable.
pub fn snap(world: Vec2, tolerance: f64, entities: &[SnapEntity]) -> Option<SnapResult> {
    let mut candidates: Vec<Candidate> = Vec::new();
    collect_single_entity_candidates(entities, &mut candidates);
    collect_intersection_candidates(entities, &mut candidates);

    let in_range: Vec<Candidate> = candidates
        .into_iter()
        .filter_map(|mut c| {
            let d = (c.point - world).length();
            if d <= tolerance {
                c.distance = d;
                Some(c)
            } else {
                None
            }
        })
        .collect();

    pick_best(in_range).map(|c| SnapResult {
        point: c.point,
        kind: c.kind,
        primary_idx: c.primary_idx,
        secondary_idx: c.secondary_idx,
    })
}

/// Priority weight for tie-break: lower number wins.
fn priority(kind: SnapKind) -> u8 {
    match kind {
        SnapKind::Endpoint => 0,
        SnapKind::Intersection => 1,
        SnapKind::Midpoint => 2,
        SnapKind::Center => 3,
    }
}

/// Pick the winning candidate using the rule documented on [`snap`].
fn pick_best(candidates: Vec<Candidate>) -> Option<Candidate> {
    let mut best: Option<Candidate> = None;
    for c in candidates {
        best = Some(match best {
            None => c,
            Some(b) => {
                let delta = c.distance - b.distance;
                if delta < -EPSILON {
                    c
                } else if delta > EPSILON {
                    b
                } else {
                    // Distances equal within EPSILON: kind priority, then primary_idx.
                    let pc = priority(c.kind);
                    let pb = priority(b.kind);
                    if pc < pb {
                        c
                    } else if pc > pb {
                        b
                    } else if c.primary_idx < b.primary_idx {
                        c
                    } else {
                        b
                    }
                }
            }
        });
    }
    best
}

#[cfg(test)]
mod tests;
