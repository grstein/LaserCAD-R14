//! Drawing check (LCV-190): open ends, gaps, duplicates, degenerate and
//! off-bed entities, found once and printed by every consumer.
//!
//! Read-only: nothing here mutates the [`Document`]. Only entities on layers
//! with Output on are checked; indices are document indices, zero-based.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{Document, Entity};
use crate::geometry::{Arc, EPSILON, Vec2};

/// One problem the check found. Indices are zero-based document indices.
#[derive(Clone, Debug, PartialEq)]
pub enum Finding {
    /// A line or arc endpoint that meets no other endpoint.
    OpenEnd {
        /// The entity the endpoint belongs to.
        index: usize,
        /// The endpoint, mm.
        at: Vec2,
    },
    /// Two endpoints closer than [`GAP_MM`] that do not meet.
    Gap {
        /// The lower of the two entity indices.
        a: usize,
        /// The higher of the two entity indices.
        b: usize,
        /// Midpoint between the two endpoints, mm.
        mid: Vec2,
        /// Distance between the two endpoints, mm.
        width: f64,
    },
    /// An entity with the same geometry as a lower-indexed one.
    Duplicate {
        /// The later entity.
        index: usize,
        /// The lowest entity it duplicates.
        of: usize,
        /// The later entity's start point (a circle's centre), mm.
        at: Vec2,
    },
}

/// Endpoints closer than this (mm) that do not meet form a gap.
pub const GAP_MM: f64 = 0.5;

/// Every finding of one check, in kind order and by ascending index.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckReport {
    /// The findings, sorted by kind, then by index.
    pub findings: Vec<Finding>,
}

/// Check the drawing on Output-on layers and report what is wrong with it.
pub fn check_drawing(doc: &Document) -> CheckReport {
    let scope: Vec<usize> = (0..doc.entities.len())
        .filter(|&i| {
            doc.entity_layer(i)
                .and_then(|id| doc.layer(id))
                .is_some_and(|l| l.output)
        })
        .collect();
    let mut chained = Vec::new();
    let mut duplicates = Vec::new();
    for (n, &j) in scope.iter().enumerate() {
        let later = &doc.entities[j];
        match scope[..n]
            .iter()
            .find(|&&i| same_geometry(&doc.entities[i], later))
        {
            Some(&of) => duplicates.push(Finding::Duplicate {
                index: j,
                of,
                at: first_point(later),
            }),
            None => chained.push(j),
        }
    }
    let (open, gaps) = end_findings(doc, &chained);
    let mut findings = open;
    findings.extend(gaps);
    findings.extend(duplicates);
    CheckReport { findings }
}

/// A line's start, an arc's start point, a circle's centre.
fn first_point(entity: &Entity) -> Vec2 {
    match entity {
        Entity::Line(l) => l.p1,
        Entity::Circle(c) => c.center,
        Entity::Arc(a) => a.start_point(),
    }
}

/// Same geometry within [`EPSILON`]: lines as unordered endpoint pairs,
/// circles by centre and radius, arcs by centre, radius and their endpoints
/// once both are read counter-clockwise (so wrap-around needs no case).
fn same_geometry(a: &Entity, b: &Entity) -> bool {
    let near = |p: Vec2, q: Vec2| p.distance(q) <= EPSILON;
    let same_r = |r: f64, s: f64| (r - s).abs() <= EPSILON;
    match (a, b) {
        (Entity::Line(a), Entity::Line(b)) => {
            (near(a.p1, b.p1) && near(a.p2, b.p2)) || (near(a.p1, b.p2) && near(a.p2, b.p1))
        }
        (Entity::Circle(a), Entity::Circle(b)) => near(a.center, b.center) && same_r(a.r, b.r),
        (Entity::Arc(a), Entity::Arc(b)) => {
            let ([a0, a1], [b0, b1]) = (ccw_ends(a), ccw_ends(b));
            near(a.center, b.center) && same_r(a.r, b.r) && near(a0, b0) && near(a1, b1)
        }
        _ => false,
    }
}

/// An arc's endpoints in counter-clockwise order: a CW arc swaps them.
fn ccw_ends(arc: &Arc) -> [Vec2; 2] {
    if arc.ccw {
        [arc.start_point(), arc.end_point()]
    } else {
        [arc.end_point(), arc.start_point()]
    }
}

/// The two endpoints of a line or arc; a circle has none.
fn endpoints(entity: &Entity) -> Option<[Vec2; 2]> {
    match entity {
        Entity::Line(l) => Some([l.p1, l.p2]),
        Entity::Arc(a) => Some([a.start_point(), a.end_point()]),
        Entity::Circle(_) => None,
    }
}

/// Open ends and gaps among the endpoints of the `chained` entities.
///
/// An endpoint *meets* when another endpoint lies within [`EPSILON`]. Ends
/// that meet nothing pair greedily, nearest first, while closer than
/// [`GAP_MM`]; each pair is one gap and the rest are open ends, so a gap never
/// also counts as two open ends.
fn end_findings(doc: &Document, chained: &[usize]) -> (Vec<Finding>, Vec<Finding>) {
    let ends: Vec<(usize, Vec2)> = chained
        .iter()
        .filter_map(|&i| endpoints(&doc.entities[i]).map(|pts| pts.map(|at| (i, at))))
        .flatten()
        .collect();
    let unmet: Vec<(usize, Vec2)> = ends
        .iter()
        .enumerate()
        .filter(|&(k, &(_, at))| {
            !ends
                .iter()
                .enumerate()
                .any(|(m, &(_, other))| m != k && at.distance(other) <= EPSILON)
        })
        .map(|(_, end)| *end)
        .collect();
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for k in 0..unmet.len() {
        for m in k + 1..unmet.len() {
            let width = unmet[k].1.distance(unmet[m].1);
            if width < GAP_MM {
                pairs.push((width, k, m));
            }
        }
    }
    pairs.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut used = vec![false; unmet.len()];
    let mut gaps = Vec::new();
    for (width, k, m) in pairs {
        if used[k] || used[m] {
            continue;
        }
        (used[k], used[m]) = (true, true);
        let (ik, im) = (unmet[k].0, unmet[m].0);
        gaps.push((
            ik.min(im),
            ik.max(im),
            unmet[k].1.lerp(unmet[m].1, 0.5),
            width,
        ));
    }
    gaps.sort_by_key(|&(a, b, _, _)| (a, b));
    let gaps = gaps
        .into_iter()
        .map(|(a, b, mid, width)| Finding::Gap { a, b, mid, width })
        .collect();
    let open = unmet
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|(&(index, at), _)| Finding::OpenEnd { index, at })
        .collect();
    (open, gaps)
}

#[cfg(test)]
mod tests;
