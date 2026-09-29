//! Trim and extend commands: [`TrimEntity`] and [`ExtendEntity`].
//!
//! Both commands replace exactly one entity in place with a modified version,
//! capturing the original in `captured: Option<Entity>` so `undo` is exact.
//! `do_` is a no-op (leaves the document unchanged and `captured = None`)
//! whenever the geometry does not support the requested operation — no
//! intersection, parallel/coincident pair, intersection behind the endpoint
//! being extended, or an out-of-scope entity pair.
//!
//! Supported entity pairs (target x cutter / boundary):
//!
//! - **Trim**: Line x Line, Line x Circle (in [`line`]); Circle x Line (in
//!   [`circle`]).
//! - **Extend**: Line x Line, Line x Circle (in [`line`]).
//!
//! Arc targets and Circle-as-extend-target are out of scope and silently
//! treated as no-ops (the Phase-4 tools guard against the case).
//!
//! Split into [`line`] and [`circle`] submodules so each file stays under the
//! AGENTS.md 300-LOC cap; this module holds the public command structs and
//! their [`Command`] impls plus the shared `parametric_t` helper.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-025.

use super::Command;
use crate::document::{Document, Entity};
use crate::geometry::{EPSILON, Line, Vec2};

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
        let trimmed = match (target, cutter) {
            (Entity::Line(t), Entity::Line(c)) => {
                line::trim_line_by_line(t, &c, self.keep_side_point)
            }
            (Entity::Line(t), Entity::Circle(c)) => {
                line::trim_line_by_circle(t, &c, self.keep_side_point)
            }
            (Entity::Circle(t), Entity::Line(c)) => {
                circle::trim_circle_by_line(&t, &c, self.keep_side_point)
            }
            _ => None,
        };
        if let Some(new_entity) = trimmed {
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

/// Extend one endpoint of a target [`Line`] to its intersection with a
/// boundary [`Entity`]. Captures the original for undo.
#[derive(Debug)]
pub struct ExtendEntity {
    /// Index of the entity (must be a [`Line`]) to extend.
    pub target_idx: usize,
    /// Index of the boundary entity (unchanged by the command).
    pub boundary_idx: usize,
    /// Which endpoint of the target line to extend: `0` = `p1`, `1` = `p2`.
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
        let target_line = match target {
            Entity::Line(l) => l,
            _ => return, // Only Line targets supported (see module docs).
        };
        let extended = match boundary {
            Entity::Line(b) => line::extend_line_to_line(target_line, &b, self.extend_endpoint),
            Entity::Circle(b) => line::extend_line_to_circle(target_line, &b, self.extend_endpoint),
            _ => None,
        };
        if let Some(new_line) = extended {
            self.captured = Some(target);
            doc.entities[self.target_idx] = Entity::Line(new_line);
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
}
