//! Trim and extend commands: [`TrimEntity`] and [`ExtendEntity`].
//!
//! Both commands replace exactly one entity in place with a modified version,
//! capturing the original in `captured: Option<Entity>` so `undo` is exact.
//! `do_` is a no-op (leaves the document unchanged and `captured = None`)
//! whenever the geometry does not support the requested operation — no
//! intersection, parallel/coincident pair, intersection behind the endpoint
//! being extended, or an out-of-scope entity pair.
//!
//! Supported entity pairs (target x cutter / boundary), LCV-160:
//!
//! | Target | Trim cutter          | Extend boundary      | Helpers    |
//! |--------|----------------------|----------------------|------------|
//! | Line   | Line, Circle, Arc    | Line, Circle, Arc    | [`line`]   |
//! | Circle | Line, Circle, Arc    | — (no-op)            | [`circle`] |
//! | Arc    | Line, Circle, Arc    | Line, Circle, Arc    | [`arc`]    |
//!
//! A *cut point* ([`cut_points`]) lies on both entities; an arc counts only
//! its span. [`trim_step`] and [`extend_reach`] are the pure dispatchers the
//! commands and the TRIM / EXTEND tools share.
//!
//! Split into [`line`], [`circle`] and [`arc`] submodules so each file stays
//! under the AGENTS.md 300-LOC cap; this module holds the public command
//! structs, their [`Command`] impls, the dispatchers and the shared
//! `parametric_t` helper.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-025.

use super::Command;
use crate::document::{Document, Entity};
use crate::geometry::{
    Circle, EPSILON, Line, Vec2, arc_arc, circle_arc, circle_circle, line_arc, line_circle,
    line_line,
};

pub(crate) mod arc;
pub(crate) mod circle;
pub(crate) mod line;

/// Trim a single target [`Entity`] at its intersection(s) with a cutter
/// [`Entity`], keeping the sub-segment / sub-arc that contains
/// `keep_side_point`. Captures the original for undo.
#[derive(Debug)]
pub struct TrimEntity {
    /// Index of the entity to trim.
    pub target_idx: usize,
    /// Index of the cutter entity (unchanged by the command).
    pub cutter_idx: usize,
    /// Point on the side / segment / arc to keep, in mm.
    pub keep_side_point: Vec2,
    captured: Option<Entity>,
}

impl TrimEntity {
    /// Build a [`TrimEntity`]. `captured` starts `None`.
    pub fn new(target_idx: usize, cutter_idx: usize, keep_side_point: Vec2) -> Self {
        Self {
            target_idx,
            cutter_idx,
            keep_side_point,
            captured: None,
        }
    }
}

impl Command for TrimEntity {
    fn do_(&mut self, doc: &mut Document) {
        self.captured = None;
        if self.target_idx >= doc.entities.len() || self.cutter_idx >= doc.entities.len() {
            return;
        }
        let target = doc.entities[self.target_idx];
        let cutter = doc.entities[self.cutter_idx];
        if let Some(new_entity) = trim_step(&target, &cutter, self.keep_side_point) {
            self.captured = Some(target);
            doc.entities[self.target_idx] = new_entity;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(original) = self.captured.take() {
            doc.entities[self.target_idx] = original;
        }
    }

    fn label(&self) -> &str {
        "Trim"
    }
}

/// Extend one endpoint of a target Line or Arc to its nearest cut point with
/// a boundary [`Entity`] (see [`extend_reach`]). Captures the original for
/// undo.
#[derive(Debug)]
pub struct ExtendEntity {
    /// Index of the entity (a [`Line`] or an Arc) to extend.
    pub target_idx: usize,
    /// Index of the boundary entity (unchanged by the command).
    pub boundary_idx: usize,
    /// Which endpoint to extend: `0` = `p1` / arc start, `1` = `p2` / arc end.
    /// Values outside `{0, 1}` are an invariant violation.
    pub extend_endpoint: u8,
    captured: Option<Entity>,
}

impl ExtendEntity {
    /// Build an [`ExtendEntity`]. `captured` starts `None`.
    pub fn new(target_idx: usize, boundary_idx: usize, extend_endpoint: u8) -> Self {
        debug_assert!(
            extend_endpoint <= 1,
            "ExtendEntity::extend_endpoint must be 0 or 1"
        );
        Self {
            target_idx,
            boundary_idx,
            extend_endpoint,
            captured: None,
        }
    }
}

impl Command for ExtendEntity {
    fn do_(&mut self, doc: &mut Document) {
        self.captured = None;
        if self.target_idx >= doc.entities.len() || self.boundary_idx >= doc.entities.len() {
            return;
        }
        let target = doc.entities[self.target_idx];
        let boundary = doc.entities[self.boundary_idx];
        if let Some((extended, _)) = extend_reach(&target, &boundary, self.extend_endpoint) {
            self.captured = Some(target);
            doc.entities[self.target_idx] = extended;
        }
    }

    fn undo(&mut self, doc: &mut Document) {
        if let Some(original) = self.captured.take() {
            doc.entities[self.target_idx] = original;
        }
    }

    fn label(&self) -> &str {
        "Extend"
    }
}

/// Cut points of `target` with `cutter`: points on both entities, where an
/// arc counts only its span and a line only its segment (LCV-160).
pub(crate) fn cut_points(target: &Entity, cutter: &Entity) -> Vec<Vec2> {
    use Entity::{Arc, Circle as Circ, Line as Seg};
    match (*target, *cutter) {
        (Seg(a), Seg(b)) => line_line(&a, &b).into_iter().collect(),
        (Seg(l), Circ(c)) | (Circ(c), Seg(l)) => line_circle(&l, &c),
        (Seg(l), Arc(a)) | (Arc(a), Seg(l)) => line_arc(&l, &a),
        (Circ(a), Circ(b)) => circle_circle(&a, &b),
        (Circ(c), Arc(a)) | (Arc(a), Circ(c)) => circle_arc(&c, &a),
        (Arc(a), Arc(b)) => arc_arc(&a, &b),
    }
}

/// One trim of `target` by `cutter`, keeping the piece around `keep`.
/// `None` when the pair leaves the target unchanged (no usable cut point).
pub(crate) fn trim_step(target: &Entity, cutter: &Entity, keep: Vec2) -> Option<Entity> {
    let pts = cut_points(target, cutter);
    match *target {
        Entity::Line(t) => line::trim_line_at_points(t, &pts, keep),
        Entity::Circle(t) => circle::trim_circle_at_points(&t, &pts, keep),
        Entity::Arc(t) => arc::trim_arc_at_points(&t, &pts, keep),
    }
}

/// Extend endpoint `ep` (`0` = p1 / start, `1` = p2 / end) of a Line or Arc
/// `target` to its nearest reach on `boundary`. Returns the grown entity and
/// the travel in mm; `None` for a Circle target or no reachable cut point.
/// A Line's extension is infinite; an Arc grows along its parent circle.
pub(crate) fn extend_reach(target: &Entity, boundary: &Entity, ep: u8) -> Option<(Entity, f64)> {
    match *target {
        Entity::Line(t) => {
            let grown = match *boundary {
                Entity::Line(b) => line::extend_line_to_line(t, &b, ep),
                Entity::Circle(b) => line::extend_line_to_circle(t, &b, ep),
                Entity::Arc(b) => line::extend_line_to_arc(t, &b, ep),
            }?;
            let (old, new) = if ep == 0 {
                (t.p1, grown.p1)
            } else {
                (t.p2, grown.p2)
            };
            Some((Entity::Line(grown), (new - old).length()))
        }
        Entity::Arc(t) => {
            let parent = Entity::Circle(Circle::new(t.center, t.r));
            let pts = cut_points(&parent, boundary);
            arc::extend_arc(&t, &pts, ep).map(|(a, mm)| (Entity::Arc(a), mm))
        }
        Entity::Circle(_) => None,
    }
}

/// Parametric `t` of `p` projected onto the infinite line through
/// `(line.p1, line.p2)`. Returns `0.0` on a degenerate line — callers must
/// guard. Shared by [`line`] and [`circle`] submodules.
pub(crate) fn parametric_t(line: &Line, p: Vec2) -> f64 {
    let d = line.p2 - line.p1;
    let len_sq = d.length_squared();
    if len_sq <= EPSILON * EPSILON {
        return 0.0;
    }
    (p - line.p1).dot(d) / len_sq
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC#1 — module surface: both commands construct with the documented
    /// constructors, and are reachable through the public re-exports.
    #[test]
    fn trim_module_defines_trim_and_extend() {
        let _ = TrimEntity::new(0, 1, Vec2::default());
        let _ = ExtendEntity::new(0, 1, 0);
        use crate::document::commands::{ExtendEntity as E, TrimEntity as T};
        let _ = T::new(0, 1, Vec2::default());
        let _ = E::new(0, 1, 1);
    }

    /// AC#11 — exact label strings.
    #[test]
    fn trim_extend_labels_exact() {
        assert_eq!(TrimEntity::new(0, 1, Vec2::default()).label(), "Trim");
        assert_eq!(ExtendEntity::new(0, 1, 0).label(), "Extend");
    }

    /// AC#12 — both commands are object-safe (`Box<dyn Command>`).
    #[test]
    fn trim_extend_commands_are_object_safe() {
        let _: Box<dyn Command> = Box::new(TrimEntity::new(0, 1, Vec2::default()));
        let _: Box<dyn Command> = Box::new(ExtendEntity::new(0, 1, 0));
    }

    /// LCV-160 AC 1 / AC 5 / AC 8 — both commands take an Arc target and
    /// undo restores it exactly.
    #[test]
    fn commands_accept_arc_targets() {
        use crate::geometry::{Arc, Line};
        use core::f64::consts::{FRAC_PI_2, PI};
        let target = Entity::Arc(Arc::new(Vec2::default(), 10.0, 0.0, FRAC_PI_2, true));
        let wall = Line::new(Vec2::new(-5.0, -20.0), Vec2::new(-5.0, 20.0));
        let mut doc = Document::default();
        doc.push_current(target);
        doc.push_current(Entity::Line(wall));
        let mut extend = ExtendEntity::new(0, 1, 1);
        extend.do_(&mut doc);
        let Entity::Arc(grown) = doc.entities[0] else {
            panic!("expected Arc")
        };
        assert!((grown.end_angle - 2.0 * PI / 3.0).abs() < 1e-9);
        extend.undo(&mut doc);
        assert_eq!(doc.entities[0], target);
        let mut doc = Document::default();
        doc.push_current(target);
        doc.push_current(Entity::Line(Line::new(
            Vec2::default(),
            Vec2::new(20.0, 20.0),
        )));
        let mut trim = TrimEntity::new(0, 1, Vec2::new(10.0, 1.0));
        trim.do_(&mut doc);
        let Entity::Arc(kept) = doc.entities[0] else {
            panic!("expected Arc")
        };
        assert_eq!(kept.start_angle, 0.0);
        assert!((kept.end_angle - PI / 4.0).abs() < 1e-9);
        trim.undo(&mut doc);
        assert_eq!(doc.entities[0], target);
    }

    /// `cut_points` is symmetric in its pair and counts arc spans only.
    #[test]
    fn cut_points_span_filtered_and_symmetric() {
        use crate::geometry::{Arc, Circle};
        use core::f64::consts::PI;
        let upper = Entity::Arc(Arc::new(Vec2::default(), 10.0, 0.0, PI, true));
        let circle = Entity::Circle(Circle::new(Vec2::new(10.0, 0.0), 10.0));
        assert_eq!(cut_points(&upper, &circle).len(), 1);
        assert_eq!(cut_points(&circle, &upper), cut_points(&upper, &circle));
        assert!(extend_reach(&circle, &upper, 0).is_none());
    }
}
