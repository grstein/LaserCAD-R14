//! Single-branch undo/redo stack — 200-deep, FIFO-evicted on overflow.
//!
//! [`History`] turns the [`Command`] trait (LCV-022) into an interactive
//! editor: tools (Phase 4) and the agent (Phase 7) build a `Box<dyn Command>`
//! and hand it to `App::commit` (LCV-030), which delegates to
//! [`History::commit`]. Ctrl+Z (LCV-070) calls [`History::undo`]; Ctrl+Y
//! calls [`History::redo`]. A new commit after one or more undos clears the
//! redo stack (classic CAD single-branch semantics).
//!
//! `undo_stack` is a [`VecDeque`] (overflow drops the front, commit/undo work
//! at the back); `redo_stack` is a plain [`Vec`] (pure LIFO). The depth cap
//! is re-enforced inside [`History::redo`] so a long commit/undo/redo chain
//! cannot re-grow `undo_stack` past [`HISTORY_DEPTH`].
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-026.

use std::collections::VecDeque;
use std::fmt;

use crate::document::{Command, Document};

/// Maximum number of commands retained for undo. Matches LaserCAD v1 and
/// AGENTS.md §"State and mutation".
pub const HISTORY_DEPTH: usize = 200;

/// Single-branch undo/redo history stack. See the [module docs](self) for the
/// API surface and shape rationale. No `Debug` derive — `Box<dyn Command>`
/// is not `Debug`; a manual impl prints stack lengths instead.
pub struct History {
    /// Commands available for undo, oldest at the front. Capped at `max_depth`.
    undo_stack: VecDeque<Box<dyn Command>>,
    /// Commands available for redo, LIFO. Cleared on every new commit.
    redo_stack: Vec<Box<dyn Command>>,
    /// Cap on `undo_stack.len()`. Exposed via [`History::max_depth`].
    max_depth: usize,
    /// Monotonic counter, bumped on every `commit` and on every `undo`/`redo`
    /// that actually did work (ADR 0002 §B). This is the document-dirty
    /// signal: `App::sync_dirty` compares it against a last-synced value
    /// instead of sampling `len()`, which is not monotonic across an
    /// undo-then-commit in the same frame.
    revision: u64,
}

impl History {
    /// Empty history with `max_depth == HISTORY_DEPTH`.
    pub fn new() -> Self {
        Self::with_depth(HISTORY_DEPTH)
    }

    /// Empty history with a caller-supplied cap. Handy for tests of the
    /// eviction path; production callers use [`History::new`].
    pub fn with_depth(depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_depth: depth,
            revision: 0,
        }
    }

    /// Monotonic revision counter (ADR 0002 §B). Starts at `0` and
    /// increments by exactly one on every `commit`, and on every `undo` /
    /// `redo` that returns `true`. Never decreases and never resets on its
    /// own — replacing the whole `History` (e.g. `action_new`) is the only
    /// way it goes back to `0`.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Run a command against `doc`, remember it for undo, invalidate redo.
    ///
    /// Steps: `cmd.do_(doc)`, push onto `undo_stack`, drop the oldest entry
    /// if over `max_depth`, clear `redo_stack`, bump `revision`.
    pub fn commit(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) {
        cmd.do_(doc);
        self.undo_stack.push_back(cmd);
        self.enforce_depth_cap();
        self.redo_stack.clear();
        self.revision += 1;
    }

    /// Reverse the most recent commit. Returns `true` if a command was undone,
    /// `false` if the undo stack was empty (document unchanged). The undone
    /// command moves onto the redo stack. Bumps `revision` iff it returns
    /// `true`.
    pub fn undo(&mut self, doc: &mut Document) -> bool {
        match self.undo_stack.pop_back() {
            Some(mut cmd) => {
                cmd.undo(doc);
                self.redo_stack.push(cmd);
                self.revision += 1;
                true
            }
            None => false,
        }
    }

    /// Replay the most recently undone command. Returns `true` on success,
    /// `false` if the redo stack was empty. The redone command moves back
    /// onto the undo stack and the depth cap is re-checked there. Bumps
    /// `revision` iff it returns `true`.
    pub fn redo(&mut self, doc: &mut Document) -> bool {
        match self.redo_stack.pop() {
            Some(mut cmd) => {
                cmd.do_(doc);
                self.undo_stack.push_back(cmd);
                self.enforce_depth_cap();
                self.revision += 1;
                true
            }
            None => false,
        }
    }

    /// At least one command available for undo?
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// At least one command available for redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Current depth of the undo stack (not the cap — see [`History::max_depth`]).
    pub fn len(&self) -> usize {
        self.undo_stack.len()
    }

    /// Is the undo stack empty? Paired with [`History::len`] to satisfy
    /// clippy's `len_without_is_empty`.
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }

    /// Configured maximum undo depth.
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    fn enforce_depth_cap(&mut self) {
        while self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (u, r, d) = (self.undo_stack.len(), self.redo_stack.len(), self.max_depth);
        write!(f, "History {{ undo: {u}, redo: {r}, max_depth: {d} }}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{CreateLine, DeleteEntities, Entity, NoOpCommand};
    use crate::geometry::{Line, Vec2};

    fn line_a() -> Line {
        Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
    }
    fn line_b() -> Line {
        Line::new(Vec2::new(0.0, 5.0), Vec2::new(10.0, 5.0))
    }

    /// AC#1, AC#9 — `HISTORY_DEPTH == 200` and every constructor matches it.
    #[test]
    fn history_constructors_match_depth_constant() {
        assert_eq!(HISTORY_DEPTH, 200);
        assert_eq!(History::new().max_depth(), 200);
        assert_eq!(History::default().max_depth(), HISTORY_DEPTH);
        assert_eq!(History::with_depth(7).max_depth(), 7);
    }

    /// AC#2 — a fresh history has nothing to undo or redo.
    #[test]
    fn fresh_history_cannot_undo_or_redo() {
        let h = History::new();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());
    }

    /// AC#3 — commit runs `do_` and pushes onto the undo stack.
    #[test]
    fn commit_runs_do_and_enables_undo() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line_a()));
        assert!(h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.len(), 1);
    }

    /// AC#4 — undo reverses the document and moves the command to redo.
    #[test]
    fn undo_reverses_and_enables_redo() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        assert!(h.undo(&mut doc));
        assert!(doc.entities.is_empty());
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    /// AC#5 — redo replays the previously undone command.
    #[test]
    fn redo_replays_and_re_enables_undo() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        h.undo(&mut doc);
        assert!(h.redo(&mut doc));
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line_a()));
        assert!(h.can_undo());
        assert!(!h.can_redo());
    }

    /// AC#6 — undo/redo on empty return `false` and never mutate `doc`.
    #[test]
    fn undo_and_redo_on_empty_return_false() {
        let mut doc = Document::default();
        doc.entities.push(Entity::Line(line_a()));
        let snapshot = doc.entities.clone();
        let mut h = History::new();
        assert!(!h.undo(&mut doc));
        assert!(!h.redo(&mut doc));
        assert_eq!(doc.entities, snapshot);
    }

    /// AC#7 — committing after an undo clears the redo stack.
    #[test]
    fn new_commit_after_undo_clears_redo_stack() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        assert!(h.undo(&mut doc));
        assert!(h.can_redo());
        h.commit(Box::new(CreateLine::new(line_b())), &mut doc);
        assert!(!h.can_redo());
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line_b()));
    }

    /// AC#8 — the depth cap evicts the oldest commands.
    #[test]
    fn depth_cap_evicts_oldest() {
        let mut doc = Document::default();
        let mut h = History::with_depth(3);
        for _ in 0..4 {
            h.commit(Box::new(NoOpCommand), &mut doc);
        }
        assert_eq!(h.len(), 3);
        assert!(h.undo(&mut doc));
        assert!(h.undo(&mut doc));
        assert!(h.undo(&mut doc));
        assert!(!h.undo(&mut doc));
    }

    /// AC#10 — full chain with state-changing commands: two creates plus a
    /// delete, three undos roll the document back to empty.
    #[test]
    fn commit_undo_redo_chain_three_commands() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        h.commit(Box::new(CreateLine::new(line_b())), &mut doc);
        h.commit(Box::new(DeleteEntities::new(vec![0])), &mut doc);
        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line_b()));
        assert!(h.undo(&mut doc));
        assert_eq!(doc.entity_count(), 2);
        assert!(h.undo(&mut doc));
        assert_eq!(doc.entity_count(), 1);
        assert!(h.undo(&mut doc));
        assert_eq!(doc.entity_count(), 0);
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    /// Depth-cap symmetry: a long commit/undo/redo chain cannot re-grow the
    /// undo stack past the cap (LCV-026 Notes §"Depth-cap symmetry").
    #[test]
    fn redo_path_re_enforces_depth_cap() {
        let mut doc = Document::default();
        let mut h = History::with_depth(2);
        h.commit(Box::new(NoOpCommand), &mut doc);
        h.commit(Box::new(NoOpCommand), &mut doc);
        h.undo(&mut doc);
        h.undo(&mut doc);
        assert!(h.redo(&mut doc));
        assert!(h.redo(&mut doc));
        assert_eq!(h.len(), 2);
    }

    /// `Box<dyn Command>` is the storage shape — guards object-safety at the
    /// commit boundary.
    #[test]
    fn history_accepts_boxed_dyn_command() {
        let mut doc = Document::default();
        let mut h = History::new();
        let cmd: Box<dyn Command> = Box::new(CreateLine::new(line_a()));
        h.commit(cmd, &mut doc);
        assert_eq!(h.len(), 1);
    }

    // --- LCV-102 — revision counter (ADR 0002 §B) ---------------------------

    /// AC 1 — every constructor starts at revision `0`.
    #[test]
    fn revision_starts_at_zero() {
        assert_eq!(History::new().revision(), 0);
        assert_eq!(History::default().revision(), 0);
        assert_eq!(History::with_depth(3).revision(), 0);
    }

    /// AC 2 — commit increments the revision by exactly one per call,
    /// including commits that evict the oldest entry at the depth cap.
    #[test]
    fn commit_increments_revision() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        h.commit(Box::new(CreateLine::new(line_b())), &mut doc);
        h.commit(Box::new(NoOpCommand), &mut doc);
        assert_eq!(h.revision(), 3);

        let mut doc2 = Document::default();
        let mut capped = History::with_depth(2);
        for _ in 0..4 {
            capped.commit(Box::new(NoOpCommand), &mut doc2);
        }
        assert_eq!(capped.len(), 2, "depth cap evicted the oldest entries");
        assert_eq!(capped.revision(), 4, "eviction still counts as a commit");
    }

    /// AC 3 — undo/redo bump the revision only on the branch that did work.
    #[test]
    fn undo_and_redo_increment_revision_only_when_work_is_done() {
        let mut doc = Document::default();
        let mut h = History::new();

        // No-op undo/redo on an empty history: no revision change.
        assert!(!h.undo(&mut doc));
        assert_eq!(h.revision(), 0);
        assert!(!h.redo(&mut doc));
        assert_eq!(h.revision(), 0);

        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        assert_eq!(h.revision(), 1);

        assert!(h.undo(&mut doc));
        assert_eq!(h.revision(), 2, "a successful undo bumps the revision");
        assert!(!h.undo(&mut doc), "undo stack is empty now");
        assert_eq!(h.revision(), 2, "a no-op undo must not bump the revision");

        assert!(h.redo(&mut doc));
        assert_eq!(h.revision(), 3, "a successful redo bumps the revision");
        assert!(!h.redo(&mut doc), "redo stack is empty now");
        assert_eq!(h.revision(), 3, "a no-op redo must not bump the revision");
    }

    /// AC 4 — the revision is monotonically non-decreasing across any
    /// commit/undo/redo sequence: a commit-undo-commit sequence yields three
    /// distinct increasing values, a property `history.len()` does not have
    /// (`len()` goes 1 -> 0 -> 1, i.e. it repeats).
    #[test]
    fn revision_is_monotonic_across_commit_undo_commit() {
        let mut doc = Document::default();
        let mut h = History::new();

        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        let after_first_commit = h.revision();
        assert_eq!(h.len(), 1);

        assert!(h.undo(&mut doc));
        let after_undo = h.revision();
        assert_eq!(h.len(), 0);

        h.commit(Box::new(CreateLine::new(line_b())), &mut doc);
        let after_second_commit = h.revision();
        assert_eq!(h.len(), 1, "history.len() repeats: 1 -> 0 -> 1");

        assert_eq!(
            (after_first_commit, after_undo, after_second_commit),
            (1, 2, 3)
        );
        assert!(after_first_commit < after_undo);
        assert!(after_undo < after_second_commit);
    }
}
