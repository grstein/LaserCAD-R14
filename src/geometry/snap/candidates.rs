//! Internal candidate enumeration for the snap engine.
//!
//! Splits the candidate-collection helpers off [`super`] to keep both files
//! under the 300-LOC kernel cap. Nothing here is `pub` outside the `snap`
//! module — callers go through [`super::snap`].

use crate::geometry::intersect::{circle_circle, line_circle, line_line};
use crate::geometry::vec2::Vec2;

use super::{SnapEntity, SnapKind};

/// Internal candidate record used during enumeration.
pub(super) struct Candidate {
    pub point: Vec2,
    pub kind: SnapKind,
    pub primary_idx: usize,
    pub secondary_idx: Option<usize>,
    /// Filled in by [`super::snap`] after distance filtering.
    pub distance: f64,
}

/// Push endpoint, midpoint, and center candidates produced by individual
/// entities (no pairwise work) into `out`.
pub(super) fn collect_single_entity_candidates(entities: &[SnapEntity], out: &mut Vec<Candidate>) {
    for (idx, e) in entities.iter().enumerate() {
        match e {
            SnapEntity::Line(l) => {
                out.push(make_candidate(l.p1, SnapKind::Endpoint, idx));
                out.push(make_candidate(l.p2, SnapKind::Endpoint, idx));
                out.push(make_candidate(l.midpoint(), SnapKind::Midpoint, idx));
            }
            SnapEntity::Circle(c) => {
                out.push(make_candidate(c.center, SnapKind::Center, idx));
            }
            SnapEntity::Arc(a) => {
                out.push(make_candidate(a.start_point(), SnapKind::Endpoint, idx));
                out.push(make_candidate(a.end_point(), SnapKind::Endpoint, idx));
                out.push(make_candidate(a.center, SnapKind::Center, idx));
            }
        }
    }
}

/// Push intersection candidates for every unordered pair of entities.
/// Arc-involving pairs are skipped (documented limitation in [`super::snap`]).
pub(super) fn collect_intersection_candidates(entities: &[SnapEntity], out: &mut Vec<Candidate>) {
    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            push_pair_intersections(&entities[i], &entities[j], i, j, out);
        }
    }
}

/// Compute and push intersection candidates for one entity pair `(i, j)`
/// with `i < j`. Skips arc-involving pairs.
fn push_pair_intersections(
    a: &SnapEntity,
    b: &SnapEntity,
    i: usize,
    j: usize,
    out: &mut Vec<Candidate>,
) {
    match (a, b) {
        (SnapEntity::Line(la), SnapEntity::Line(lb)) => {
            if let Some(p) = line_line(la, lb) {
                out.push(make_intersection_candidate(p, i, j));
            }
        }
        (SnapEntity::Line(la), SnapEntity::Circle(cb)) => {
            for p in line_circle(la, cb) {
                out.push(make_intersection_candidate(p, i, j));
            }
        }
        (SnapEntity::Circle(ca), SnapEntity::Line(lb)) => {
            for p in line_circle(lb, ca) {
                out.push(make_intersection_candidate(p, i, j));
            }
        }
        (SnapEntity::Circle(ca), SnapEntity::Circle(cb)) => {
            for p in circle_circle(ca, cb) {
                out.push(make_intersection_candidate(p, i, j));
            }
        }
        // Arc-involving pairs: skipped. No arc intersection routines exist yet.
        _ => {}
    }
}

fn make_candidate(point: Vec2, kind: SnapKind, primary_idx: usize) -> Candidate {
    Candidate {
        point,
        kind,
        primary_idx,
        secondary_idx: None,
        distance: 0.0,
    }
}

fn make_intersection_candidate(point: Vec2, i: usize, j: usize) -> Candidate {
    Candidate {
        point,
        kind: SnapKind::Intersection,
        primary_idx: i,
        secondary_idx: Some(j),
        distance: 0.0,
    }
}
