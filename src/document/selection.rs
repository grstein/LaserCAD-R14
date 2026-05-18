//! Document [`Selection`]: the set of entity indices currently highlighted
//! by the operator.
//!
//! Stored as a [`HashSet<usize>`] over indices into [`Document::entities`].
//! Sparse selections (a handful of entities out of hundreds) are the common
//! case in laser-cutting CAD; `HashSet` gives O(1) `add` / `remove` /
//! `is_selected` and a small default footprint. Iteration order is not
//! guaranteed — callers that need deterministic order sort the iterator
//! themselves.
//!
//! Index stability is the caller's responsibility. After a `DeleteEntities`
//! command shifts the [`Document::entities`] vec, the indices in `Selection`
//! may now refer to different entities. The Phase-4 `DeleteTool` (LCV-052)
//! issues a follow-up `SelectionCommand` to clear or remap; this type does
//! **not** auto-clean.
//!
//! External callers mutate the selection through [`Selection::add`],
//! [`Selection::remove`], [`Selection::set`], [`Selection::clear`], or by
//! committing a [`crate::document::SelectionCommand`] through the history
//! stack so the change participates in Ctrl+Z. The inner [`HashSet`] is
//! visible to the rest of the crate via [`Selection::replace_indices`] so
//! `SelectionCommand` can swap it atomically without going through the
//! public mutator surface.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. The selection model is part
//! of the pure-Rust kernel.
//!
//! Introduced by demand LCV-027 (replaces the LCV-021 placeholder in
//! `state.rs`).
//!
//! [`Document::entities`]: crate::document::Document::entities

use std::collections::HashSet;

/// The set of entity indices currently selected in the active [`Document`].
///
/// Internal storage is a [`HashSet<usize>`] kept private so the only mutation
/// paths are this type's public methods plus the crate-private
/// [`Selection::replace_indices`] used by
/// [`crate::document::SelectionCommand`]. Two `Selection` values are equal
/// iff they hold the same indices (set equality, order-independent).
///
/// `Default` yields an empty selection.
///
/// [`Document`]: crate::document::Document
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Selection {
    indices: HashSet<usize>,
}

impl Selection {
    /// Is `idx` currently selected? O(1).
    pub fn is_selected(&self, idx: usize) -> bool {
        self.indices.contains(&idx)
    }

    /// Iterate over the selected indices. Order is **not guaranteed** — it is
    /// `HashSet` iteration order. Callers that need deterministic order
    /// collect into a `Vec<usize>` and sort.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.indices.iter().copied()
    }

    /// Number of selected indices.
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// `true` when no index is selected.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Drop every selected index. After this call, [`Selection::is_empty`]
    /// returns `true`.
    pub fn clear(&mut self) {
        self.indices.clear();
    }

    /// Add `idx` to the selection. Idempotent — adding an already-selected
    /// index is a no-op and [`Selection::len`] does not grow.
    pub fn add(&mut self, idx: usize) {
        self.indices.insert(idx);
    }

    /// Remove `idx` from the selection. Idempotent — removing a non-selected
    /// index is a no-op.
    pub fn remove(&mut self, idx: usize) {
        self.indices.remove(&idx);
    }

    /// Replace the selection with `indices`. Equivalent to
    /// [`Selection::clear`] followed by [`Selection::add`] for each supplied
    /// index, but a single allocation. Duplicate indices in the input
    /// collapse (it's a set).
    pub fn set(&mut self, indices: impl IntoIterator<Item = usize>) {
        self.indices.clear();
        self.indices.extend(indices);
    }

    /// Swap the underlying set with `new`, returning the previous set.
    ///
    /// Crate-private so only [`crate::document::SelectionCommand`] reaches
    /// it — external callers must go through `add` / `remove` / `set` or
    /// commit a `SelectionCommand`. Cloning is cheap: real-world selections
    /// hold a handful of indices.
    pub(crate) fn replace_indices(&mut self, new: HashSet<usize>) -> HashSet<usize> {
        std::mem::replace(&mut self.indices, new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC#1 — `Selection::default()` constructs and the value carries a
    /// non-empty `Debug` representation (proves the `Debug` derive).
    #[test]
    fn selection_struct_constructs_via_default() {
        let sel = Selection::default();
        assert!(!format!("{sel:?}").is_empty());
    }

    /// AC#2 — a default `Selection` reports empty, len 0, and yields nothing.
    #[test]
    fn selection_default_is_empty() {
        let sel = Selection::default();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
        assert_eq!(sel.iter().count(), 0);
    }

    /// AC#3 — `add` then `is_selected` returns true; `remove` then
    /// `is_selected` returns false.
    #[test]
    fn selection_add_remove_is_selected_roundtrip() {
        let mut sel = Selection::default();
        sel.add(3);
        assert!(sel.is_selected(3));
        sel.remove(3);
        assert!(!sel.is_selected(3));
        assert!(sel.is_empty());
    }

    /// AC#4 — `add` is idempotent.
    #[test]
    fn selection_add_is_idempotent() {
        let mut sel = Selection::default();
        sel.add(3);
        sel.add(3);
        assert_eq!(sel.len(), 1);
        assert!(sel.is_selected(3));
    }

    /// AC#5 — `remove` of a non-member is a no-op.
    #[test]
    fn selection_remove_non_member_is_noop() {
        let mut sel = Selection::default();
        sel.remove(99);
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
    }

    /// AC#6 — `set` replaces the prior contents.
    #[test]
    fn selection_set_replaces_existing() {
        let mut sel = Selection::default();
        sel.add(1);
        sel.add(2);
        sel.set([3usize, 4, 5]);
        assert_eq!(sel.len(), 3);
        assert!(!sel.is_selected(1));
        assert!(!sel.is_selected(2));
        assert!(sel.is_selected(3));
        assert!(sel.is_selected(4));
        assert!(sel.is_selected(5));
    }

    /// AC#7 — `iter` yields each member exactly once. Order is not asserted;
    /// the test collects into a sorted `Vec<usize>`.
    #[test]
    fn selection_iter_yields_each_member_once() {
        let mut sel = Selection::default();
        sel.set([2usize, 4, 6, 8]);
        let mut got: Vec<usize> = sel.iter().collect();
        got.sort_unstable();
        assert_eq!(got, vec![2usize, 4, 6, 8]);
    }

    /// AC#8 — `clear` empties the selection.
    #[test]
    fn selection_clear_empties() {
        let mut sel = Selection::default();
        sel.add(7);
        sel.add(11);
        sel.clear();
        assert!(sel.is_empty());
        assert_eq!(sel.len(), 0);
    }

    /// `set` collapses duplicate inputs (it's a set).
    #[test]
    fn selection_set_dedups_input() {
        let mut sel = Selection::default();
        sel.set([1usize, 1, 1, 2]);
        assert_eq!(sel.len(), 2);
        assert!(sel.is_selected(1));
        assert!(sel.is_selected(2));
    }

    /// `replace_indices` swaps in the new set and returns the old one. Used
    /// by `SelectionCommand` for atomic do_/undo capture; tested here to
    /// guard the contract.
    #[test]
    fn replace_indices_swaps_and_returns_previous() {
        let mut sel = Selection::default();
        sel.add(1);
        sel.add(2);
        let mut new_set = HashSet::new();
        new_set.insert(7);
        new_set.insert(8);
        let prev = sel.replace_indices(new_set);
        let mut prev_sorted: Vec<usize> = prev.into_iter().collect();
        prev_sorted.sort_unstable();
        assert_eq!(prev_sorted, vec![1, 2]);
        assert!(sel.is_selected(7));
        assert!(sel.is_selected(8));
        assert_eq!(sel.len(), 2);
    }

    /// `Clone` produces an equal but independent value.
    #[test]
    fn selection_clone_is_independent() {
        let mut a = Selection::default();
        a.add(1);
        a.add(2);
        let mut b = a.clone();
        assert_eq!(a, b);
        b.add(99);
        assert_ne!(a, b);
        assert!(!a.is_selected(99));
    }
}
