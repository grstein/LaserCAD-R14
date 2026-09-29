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
mod tests;
