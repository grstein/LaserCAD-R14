//! Single-branch undo/redo stack — 200-deep, FIFO-evicted on overflow.
//!
//! [`History`] turns the [`Command`] trait (LCV-022) into an interactive
//! editor: tools (Phase 4) and the agent (Phase 7) build a `Box<dyn Command>`
//! and hand it to `App::commit` (LCV-030), which delegates to
//! [`History::commit`]. Ctrl+Z (LCV-070) calls [`History::undo`]; Ctrl+Y
//! calls [`History::redo`]. A new commit after one or more undos clears the
//! redo stack (classic CAD single-branch semantics).
//!
//! One **flat group** may be open beside the undo stack — never in it — and
//! that is how one agent turn of any length becomes one `Ctrl+Z` (ADR 0007
//! §D12, LCV-142). [`History::begin_group`] arms it,
//! [`History::commit_grouped`] applies a command at once and remembers it in
//! the group, and [`History::end_group`] seals the group onto the stack as one
//! entry (bare for one command, one [`CompositeCommand`] for more). `commit`,
//! `undo` and `redo` seal first, so foreign work is never absorbed into a
//! group, and nothing of a group reaches the stack until it is one entry — the
//! depth cap cannot evict step 1 of a long turn.
//!
//! `undo_stack` is a [`VecDeque`] (overflow drops the front, commit/undo work
//! at the back); `redo_stack` is a plain [`Vec`] (pure LIFO). The depth cap
//! is re-enforced inside [`History::redo`] so a long commit/undo/redo chain
//! cannot re-grow `undo_stack` past [`HISTORY_DEPTH`].
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. Introduced by demand LCV-026.

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::document::{Command, CompositeCommand, Document};

/// Maximum number of commands retained for undo. Matches LaserCAD v1 and
/// AGENTS.md §"State and mutation".
pub const HISTORY_DEPTH: usize = 200;

/// The next [`History::id`]; every constructor goes through `with_depth`.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

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
    /// The open flat group, if one is armed (ADR 0007 §D12). Held beside the
    /// stack; see [`History::begin_group`].
    group: Option<Group>,
    /// Process-unique, fixed at construction (ADR 0007 §D16).
    id: u64,
}

/// An armed group: its undo label and the commands applied into it so far,
/// oldest first.
struct Group {
    label: String,
    commands: Vec<Box<dyn Command>>,
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
            group: None,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
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

    /// Process-unique identity of this `History` (ADR 0007 §D16). With
    /// [`History::revision`] it names one state of one document: replacing
    /// the document assigns a fresh `History`, whose revision restarts at 0
    /// but whose id is new. `History` is not `Clone`, so no two live values
    /// share an id.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Run a command against `doc`, remember it for undo, invalidate redo.
    ///
    /// Steps: seal any open group ([`History::end_group`]), `cmd.do_(doc)`,
    /// push onto `undo_stack`, drop the oldest entry if over `max_depth`,
    /// clear `redo_stack`, bump `revision`.
    pub fn commit(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) {
        self.end_group();
        cmd.do_(doc);
        self.undo_stack.push_back(cmd);
        self.enforce_depth_cap();
        self.redo_stack.clear();
        self.revision += 1;
    }

    /// Reverse the most recent commit. Returns `true` if a command was undone,
    /// `false` if the undo stack was empty (document unchanged). The undone
    /// command moves onto the redo stack. Bumps `revision` iff it returns
    /// `true`. Seals any open group first, so an undo mid-group reverses the
    /// group's work so far as one step.
    pub fn undo(&mut self, doc: &mut Document) -> bool {
        self.end_group();
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
    /// `revision` iff it returns `true`. Seals any open group first.
    pub fn redo(&mut self, doc: &mut Document) -> bool {
        self.end_group();
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

    /// Arm an empty flat group labelled `label` (ADR 0007 §D12). A group
    /// already open is sealed first, so groups never nest.
    pub fn begin_group(&mut self, label: &str) {
        self.end_group();
        self.group = Some(Group {
            label: label.to_owned(),
            commands: Vec::new(),
        });
    }

    /// Run `cmd` against `doc` **now** and remember it in the open group.
    ///
    /// Observably the same as [`History::commit`] for the document and the
    /// revision — `do_` runs at once, redo is cleared, `revision` bumps by
    /// one — only *where the command is remembered* differs. With no group
    /// armed it is exactly `commit`.
    pub fn commit_grouped(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) {
        let Some(group) = self.group.as_mut() else {
            self.commit(cmd, doc);
            return;
        };
        cmd.do_(doc);
        group.commands.push(cmd);
        self.redo_stack.clear();
        self.revision += 1;
    }

    /// Seal the open group onto the undo stack and disarm it.
    ///
    /// Zero commands push nothing, one is pushed bare, two or more become one
    /// [`CompositeCommand`] under the group's label; the depth cap runs once.
    /// Nothing is re-run and the revision does not move. Returns `None` when
    /// no group was armed (idempotent: a second call is a no-op) and
    /// `Some(n)` with the number of commands it sealed otherwise.
    pub fn end_group(&mut self) -> Option<usize> {
        let Group {
            label,
            mut commands,
        } = self.group.take()?;
        let sealed = commands.len();
        match sealed {
            0 => {}
            1 => self.undo_stack.extend(commands.pop()),
            _ => self
                .undo_stack
                .push_back(Box::new(CompositeCommand::new(commands, label))),
        }
        self.enforce_depth_cap();
        Some(sealed)
    }

    /// Is a group armed? The fence's second witness (ADR 0007 §D14): every
    /// seal and every document replacement leaves this `false`.
    pub fn group_open(&self) -> bool {
        self.group.is_some()
    }

    /// Does the open group hold any command? It counts as one undo entry.
    fn open_entries(&self) -> usize {
        self.group
            .as_ref()
            .map_or(0, |g| usize::from(!g.commands.is_empty()))
    }

    /// At least one command available for undo?
    pub fn can_undo(&self) -> bool {
        !self.is_empty()
    }

    /// At least one command available for redo?
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Current depth of the undo stack (not the cap — see [`History::max_depth`]),
    /// counting a non-empty open group as the one entry it will seal into.
    /// Never above the cap: sealing evicts the oldest entry to make room.
    pub fn len(&self) -> usize {
        (self.undo_stack.len() + self.open_entries()).min(self.max_depth)
    }

    /// Is the undo stack empty? Paired with [`History::len`] to satisfy
    /// clippy's `len_without_is_empty`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
        let g = self.group.as_ref().map(|g| g.commands.len());
        write!(
            f,
            "History {{ undo: {u}, redo: {r}, max_depth: {d}, group: {g:?} }}"
        )
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

    /// LCV-153 AC 7 — two fresh histories never share an id, so replacing
    /// the document is visible even when both revisions read 0.
    #[test]
    fn fresh_histories_have_distinct_ids() {
        let (a, b) = (History::new(), History::with_depth(3));
        let c = History::default();
        assert_ne!(a.id(), b.id());
        assert_ne!(b.id(), c.id());
        assert_ne!(a.id(), c.id());
        assert_eq!(a.revision(), b.revision());
    }

    /// LCV-153 AC 7 — the id never moves with editing: commit, group,
    /// undo and redo all leave it where it was.
    #[test]
    fn the_id_is_stable_across_edits() {
        let mut doc = Document::default();
        let mut h = History::new();
        let id = h.id();
        h.commit(Box::new(CreateLine::new(line_a())), &mut doc);
        assert_eq!(h.id(), id);
        h.begin_group("g");
        h.commit_grouped(Box::new(CreateLine::new(line_b())), &mut doc);
        assert_eq!(h.id(), id);
        h.end_group();
        assert!(h.undo(&mut doc));
        assert_eq!(h.id(), id);
        assert!(h.redo(&mut doc));
        assert_eq!(h.id(), id);
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

    // --- LCV-142 — the flat group (ADR 0007 §D12) ---------------------------

    fn line_at(y: f64) -> Line {
        Line::new(Vec2::new(0.0, y), Vec2::new(10.0, y))
    }

    /// Four grouped commits, the third of which is a delete, so a composite
    /// that undid its children forward would not restore the document.
    fn four_grouped_commits(doc: &mut Document, h: &mut History) {
        h.commit_grouped(Box::new(CreateLine::new(line_at(0.0))), doc);
        h.commit_grouped(Box::new(CreateLine::new(line_at(1.0))), doc);
        h.commit_grouped(Box::new(DeleteEntities::new(vec![0])), doc);
        h.commit_grouped(Box::new(CreateLine::new(line_at(2.0))), doc);
    }

    /// AC 5 — `commit_grouped` applies at once and bumps the revision by one
    /// per call, exactly like `commit`; it also clears redo.
    #[test]
    fn commit_grouped_applies_immediately_and_bumps_the_revision() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_at(9.0))), &mut doc);
        assert!(h.undo(&mut doc));
        assert!(h.can_redo());
        let start = h.revision();

        h.begin_group("Agent: draw");
        assert!(h.group_open());
        for (i, y) in [0.0, 1.0, 2.0].into_iter().enumerate() {
            h.commit_grouped(Box::new(CreateLine::new(line_at(y))), &mut doc);
            assert_eq!(doc.entities.len(), i + 1, "applied at once");
            assert_eq!(h.revision(), start + i as u64 + 1, "one bump per call");
        }
        assert!(!h.can_redo(), "a grouped commit clears redo like commit");
    }

    /// AC 5 — sealing 0 / 1 / n commands: nothing, the bare command, one
    /// composite under the group's label. The revision never moves on seal,
    /// the report says how many were sealed, and a second call is a no-op.
    #[test]
    fn end_group_seals_zero_one_or_many_and_is_idempotent() {
        let mut doc = Document::default();
        let mut h = History::new();
        assert_eq!(h.end_group(), None, "no group armed");

        h.begin_group("Agent: nothing");
        assert_eq!(h.end_group(), Some(0));
        assert_eq!(h.len(), 0, "an empty group pushes nothing");

        h.begin_group("Agent: one");
        h.commit_grouped(Box::new(CreateLine::new(line_at(0.0))), &mut doc);
        let revision = h.revision();
        assert_eq!(h.end_group(), Some(1));
        assert_eq!(h.revision(), revision, "sealing runs nothing");
        assert_eq!(h.len(), 1);
        assert_eq!(
            h.undo_stack.back().map(|c| c.label()),
            Some("Create Line"),
            "a group of one is pushed bare"
        );

        for label in ["Agent: draw a square", "Agent: delete that circle"] {
            h.begin_group(label);
            four_grouped_commits(&mut doc, &mut h);
            let applied = doc.entities.clone();
            let before = h.len();
            assert_eq!(h.end_group(), Some(4));
            assert!(!h.group_open());
            assert_eq!(h.len(), before, "the open group already counted as one");
            assert_eq!(h.undo_stack.back().map(|c| c.label()), Some(label));
            assert_eq!(doc.entities, applied, "sealing must not re-run do_");
            assert_eq!(h.end_group(), None, "idempotent");
            assert_eq!(h.len(), before);
        }
    }

    /// AC 5 — one undo reverses a sealed group of four in reverse order, one
    /// redo re-applies it in the original order.
    #[test]
    fn a_sealed_group_undoes_and_redoes_as_one_entry() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.begin_group("Agent: four");
        four_grouped_commits(&mut doc, &mut h);
        let applied = doc.entities.clone();
        assert_eq!(applied.len(), 2, "line(1.0) and line(2.0) survive");
        h.end_group();

        assert!(h.undo(&mut doc));
        assert!(doc.entities.is_empty(), "got {:?}", doc.entities);
        assert!(!h.can_undo(), "the group was one entry");
        assert!(h.redo(&mut doc));
        assert_eq!(doc.entities, applied);
    }

    /// AC 5 — `len` / `can_undo` / `is_empty` see an open non-empty group as
    /// one entry, and an open empty one as nothing.
    #[test]
    fn an_open_group_counts_as_one_entry_once_it_holds_a_command() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.begin_group("Agent: x");
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());
        assert!(!h.can_undo());

        h.commit_grouped(Box::new(NoOpCommand), &mut doc);
        assert_eq!(h.len(), 1);
        assert!(!h.is_empty());
        assert!(h.can_undo());
        h.commit_grouped(Box::new(NoOpCommand), &mut doc);
        assert_eq!(h.len(), 1, "still one entry");
    }

    /// AC 5 — `commit`, `undo` and `redo` each seal the group before they
    /// touch the stack, so foreign work is never absorbed into it.
    #[test]
    fn commit_undo_and_redo_seal_the_group_first() {
        // commit: the foreign entry sits above the sealed group.
        let mut doc = Document::default();
        let mut h = History::new();
        h.begin_group("Agent: two");
        h.commit_grouped(Box::new(CreateLine::new(line_at(0.0))), &mut doc);
        h.commit_grouped(Box::new(CreateLine::new(line_at(1.0))), &mut doc);
        h.commit(Box::new(CreateLine::new(line_at(5.0))), &mut doc);
        assert!(!h.group_open());
        assert_eq!(h.len(), 2);
        assert!(h.undo(&mut doc));
        assert_eq!(doc.entities.len(), 2, "the foreign line went first, alone");
        assert!(h.undo(&mut doc));
        assert!(doc.entities.is_empty(), "then the whole group");

        // undo: seals, then reverses the group so far as one step.
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit(Box::new(CreateLine::new(line_at(9.0))), &mut doc);
        h.begin_group("Agent: two");
        h.commit_grouped(Box::new(CreateLine::new(line_at(0.0))), &mut doc);
        h.commit_grouped(Box::new(CreateLine::new(line_at(1.0))), &mut doc);
        assert!(h.undo(&mut doc));
        assert!(!h.group_open());
        assert_eq!(doc.entities.len(), 1, "only the human line is left");

        // redo: seals too, even with nothing to redo.
        h.begin_group("Agent: again");
        h.commit_grouped(Box::new(CreateLine::new(line_at(3.0))), &mut doc);
        assert!(!h.redo(&mut doc), "the grouped commit cleared redo");
        assert!(!h.group_open());
        assert_eq!(h.len(), 2);
    }

    /// AC 5 — `begin_group` seals a group already open; groups never nest.
    #[test]
    fn begin_group_seals_an_open_group_first() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.begin_group("first");
        h.commit_grouped(Box::new(NoOpCommand), &mut doc);
        h.commit_grouped(Box::new(NoOpCommand), &mut doc);
        h.begin_group("second");
        h.commit_grouped(Box::new(NoOpCommand), &mut doc);
        h.commit_grouped(Box::new(NoOpCommand), &mut doc);
        assert_eq!(h.end_group(), Some(2));
        assert_eq!(h.len(), 2);
        let labels: Vec<&str> = h.undo_stack.iter().map(|c| c.label()).collect();
        assert_eq!(labels, ["first", "second"]);
    }

    /// AC 5 — with no group armed `commit_grouped` is `commit`.
    #[test]
    fn commit_grouped_without_a_group_is_commit() {
        let mut doc = Document::default();
        let mut h = History::new();
        h.commit_grouped(Box::new(CreateLine::new(line_at(0.0))), &mut doc);
        h.commit_grouped(Box::new(CreateLine::new(line_at(1.0))), &mut doc);
        assert_eq!(h.len(), 2, "two separate entries");
        assert_eq!(h.revision(), 2);
        assert!(!h.group_open());
    }

    /// AC 6 at the unit level — a group far longer than the cap never has an
    /// entry evicted from inside it; sealing displaces one oldest entry.
    #[test]
    fn a_group_longer_than_the_cap_seals_into_one_entry() {
        let mut doc = Document::default();
        let mut h = History::with_depth(5);
        for y in 0..5 {
            h.commit(Box::new(CreateLine::new(line_at(f64::from(y)))), &mut doc);
        }
        h.begin_group("Agent: many");
        for y in 10..40 {
            h.commit_grouped(Box::new(CreateLine::new(line_at(f64::from(y)))), &mut doc);
        }
        assert_eq!(h.len(), 5, "never above the cap");
        assert_eq!(h.end_group(), Some(30));
        assert_eq!(h.len(), 5);
        assert!(h.undo(&mut doc));
        assert_eq!(doc.entities.len(), 5, "all thirty went in one undo");
        for _ in 0..4 {
            assert!(h.undo(&mut doc));
        }
        assert!(!h.undo(&mut doc), "the oldest human entry was evicted");
        assert_eq!(doc.entities.len(), 1);
    }
}
