//! Snap engine: pick the best geometric feature within tolerance of a cursor.
//!
//! Given a world-space cursor position and a slice of [`SnapEntity`] values,
//! [`snap_query`] enumerates candidate snap points of the kinds enabled in a
//! [`SnapKinds`] set — endpoint, midpoint, center, intersection, quadrant,
//! perpendicular, tangent, nearest — and returns the best candidate within
//! the supplied tolerance. [`snap`] is the anchor-less, default-kinds
//! wrapper. The selection rule is documented on [`snap_query`].
//!
//! [`SnapEntity`] is the snap engine's own entity view (`Line` | `Circle` |
//! `Arc`); callers convert at the call site. Kernel-pure: no `egui`,
//! `eframe` or `rfd`. Split to honor the 300-LOC cap: [`candidates`]
//! (endpoint, midpoint, center, intersection) and [`anchored`] (quadrant,
//! perpendicular, tangent, nearest). Introduced by LCV-016; extended by LCV-161.

mod anchored;
mod candidates;

use crate::geometry::arc::Arc;
use crate::geometry::circle::Circle;
use crate::geometry::epsilon::EPSILON;
use crate::geometry::line::Line;
use crate::geometry::vec2::Vec2;
use serde::{Deserialize, Serialize};

use candidates::{Candidate, collect_intersection_candidates, collect_single_entity_candidates};

/// Discrete classification of a snap candidate.
///
/// The ordering of the variants is **not** the tie-break priority — see
/// [`snap_query`] for the precedence rule. `Eq` and `Hash` are derived because the
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
    /// Point of a circle or arc at 0°, 90°, 180° or 270° (world axes).
    Quadrant,
    /// Foot of the perpendicular from the tool's anchor onto an entity.
    Perpendicular,
    /// Tangent point on a circle or arc from the tool's anchor.
    Tangent,
    /// Closest point of the nearest line, circle or arc.
    Nearest,
}

/// Set of enabled [`SnapKind`]s, one flag per kind (persisted in settings).
///
/// `Default` enables every kind except [`SnapKind::Nearest`]. Missing fields
/// deserialize to their default so older settings files keep loading.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SnapKinds {
    /// [`SnapKind::Endpoint`] enabled.
    pub endpoint: bool,
    /// [`SnapKind::Midpoint`] enabled.
    pub midpoint: bool,
    /// [`SnapKind::Center`] enabled.
    pub center: bool,
    /// [`SnapKind::Intersection`] enabled.
    pub intersection: bool,
    /// [`SnapKind::Quadrant`] enabled.
    pub quadrant: bool,
    /// [`SnapKind::Perpendicular`] enabled.
    pub perpendicular: bool,
    /// [`SnapKind::Tangent`] enabled.
    pub tangent: bool,
    /// [`SnapKind::Nearest`] enabled.
    pub nearest: bool,
}

impl Default for SnapKinds {
    fn default() -> Self {
        Self {
            endpoint: true,
            midpoint: true,
            center: true,
            intersection: true,
            quadrant: true,
            perpendicular: true,
            tangent: true,
            nearest: false,
        }
    }
}

impl SnapKinds {
    /// True iff `kind` is enabled.
    pub fn contains(&self, kind: SnapKind) -> bool {
        let mut copy = *self;
        *copy.flag_mut(kind)
    }

    /// Enable (`on = true`) or disable `kind`.
    pub fn set(&mut self, kind: SnapKind, on: bool) {
        *self.flag_mut(kind) = on;
    }

    fn flag_mut(&mut self, kind: SnapKind) -> &mut bool {
        match kind {
            SnapKind::Endpoint => &mut self.endpoint,
            SnapKind::Midpoint => &mut self.midpoint,
            SnapKind::Center => &mut self.center,
            SnapKind::Intersection => &mut self.intersection,
            SnapKind::Quadrant => &mut self.quadrant,
            SnapKind::Perpendicular => &mut self.perpendicular,
            SnapKind::Tangent => &mut self.tangent,
            SnapKind::Nearest => &mut self.nearest,
        }
    }
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
/// `primary_idx` is the index (into the input slice passed to [`snap_query`]) of the
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

/// Find the best snap candidate within `tolerance` of `world`, with no
/// anchor and [`SnapKinds::default`]. Thin wrapper over [`snap_query`].
pub fn snap(world: Vec2, tolerance: f64, entities: &[SnapEntity]) -> Option<SnapResult> {
    snap_query(world, tolerance, entities, None, SnapKinds::default())
}

/// Find the best candidate of an enabled kind within `tolerance` of `world`.
///
/// Returns `None` when no candidate of a kind in `kinds` lies within
/// `tolerance` (`world`-space units, millimeters). `anchor` is the active
/// tool's anchor point; without one, Perpendicular and Tangent give nothing.
///
/// # Selection rule
///
/// The closest candidate within `tolerance` wins. Distance ties (within
/// [`EPSILON`]) break by kind, `Endpoint > Intersection > Midpoint > Center >
/// Quadrant > Perpendicular > Tangent` (AutoCAD's OSNAP precedence), then by
/// the smaller `primary_idx`.
///
/// Nearest is computed only when no other candidate is in range, so it never
/// shadows a real feature.
///
/// Intersections cover line-line, line-circle and circle-circle pairs
/// (arc-involving pairs are skipped) in `O(n²)`, acceptable for the ~10k
/// entity cap at pointer-event rate. Arc points (quadrant, foot, tangent,
/// nearest) count only inside the sweep; segment feet only on the segment.
pub fn snap_query(
    world: Vec2,
    tolerance: f64,
    entities: &[SnapEntity],
    anchor: Option<Vec2>,
    kinds: SnapKinds,
) -> Option<SnapResult> {
    let mut candidates: Vec<Candidate> = Vec::new();
    collect_single_entity_candidates(entities, &mut candidates);
    if kinds.intersection {
        collect_intersection_candidates(entities, &mut candidates);
    }
    anchored::collect_quadrants(entities, &mut candidates);
    if let Some(anchor) = anchor {
        if kinds.perpendicular {
            anchored::collect_perpendicular(anchor, entities, &mut candidates);
        }
        if kinds.tangent {
            anchored::collect_tangent(anchor, entities, &mut candidates);
        }
    }

    let mut in_range: Vec<Candidate> = candidates
        .into_iter()
        .filter(|c| kinds.contains(c.kind))
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
    if in_range.is_empty() && kinds.nearest {
        anchored::collect_nearest(world, tolerance, entities, &mut in_range);
    }

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
        SnapKind::Quadrant => 4,
        SnapKind::Perpendicular => 5,
        SnapKind::Tangent => 6,
        SnapKind::Nearest => 7,
    }
}

/// Pick the winning candidate using the rule documented on [`snap_query`].
fn pick_best(candidates: Vec<Candidate>) -> Option<Candidate> {
    candidates
        .into_iter()
        .reduce(|best, c| if beats(&c, &best) { c } else { best })
}

/// True iff `c` strictly beats `b`: closer, else (distances equal within
/// [`EPSILON`]) higher kind priority, else lower `primary_idx`.
fn beats(c: &Candidate, b: &Candidate) -> bool {
    let delta = c.distance - b.distance;
    if delta.abs() > EPSILON {
        return delta < 0.0;
    }
    (priority(c.kind), c.primary_idx) < (priority(b.kind), b.primary_idx)
}

#[cfg(test)]
mod tests;
