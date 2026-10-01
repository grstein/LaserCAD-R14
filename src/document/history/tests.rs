use super::*;
use crate::document::{CreateLine, DeleteEntities, Entity, MoveEntities, NoOpCommand};
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
    doc.push_current(Entity::Line(line_a()));
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

// --- LCV-198 — rewinding the open group to a mark (ADR 0007 §D12) --------

/// Entities and ids of `doc`, in order: the state a checkpoint names.
fn snapshot(doc: &Document) -> Vec<(Entity, Option<crate::document::EntityId>)> {
    (0..doc.entities.len())
        .map(|i| (doc.entities[i], doc.entity_id(i)))
        .collect()
}

/// A human line, then a group holding one create (the mark) and then a
/// create → move → delete that only a reverse-order undo can take back.
fn group_with_mark(doc: &mut Document, h: &mut History) -> usize {
    h.commit(Box::new(CreateLine::new(line_at(9.0))), doc);
    h.begin_group("Agent: try");
    h.commit_grouped(Box::new(CreateLine::new(line_at(0.0))), doc);
    h.group_len()
}

fn risky_steps(doc: &mut Document, h: &mut History) {
    h.commit_grouped(Box::new(CreateLine::new(line_at(1.0))), doc);
    let delta = Vec2::new(5.0, 5.0);
    h.commit_grouped(Box::new(MoveEntities::new(vec![1, 2], delta)), doc);
    h.commit_grouped(Box::new(DeleteEntities::new(vec![1])), doc);
}

/// LCV-198 AC 2 — `group_len` counts the open group; the rewind undoes
/// every command past the mark in reverse order, drops them, leaves redo
/// empty and bumps the revision exactly once.
#[test]
fn rewind_group_undoes_past_the_mark_in_reverse_order() {
    let (mut doc, mut h) = (Document::default(), History::new());
    assert_eq!(h.group_len(), 0, "no group open");
    let mark = group_with_mark(&mut doc, &mut h);
    assert_eq!(mark, 1);
    let at_mark = snapshot(&doc);
    risky_steps(&mut doc, &mut h);
    assert_eq!(h.group_len(), 4);
    let rev = h.revision();

    assert_eq!(h.rewind_group(mark, &mut doc), 3);
    assert_eq!(snapshot(&doc), at_mark, "entities and ids as at the mark");
    assert_eq!(h.revision(), rev + 1, "one bump for the whole rewind");
    assert_eq!(h.group_len(), mark, "the rewound commands are dropped");
    assert!(h.group_open(), "a rewind does not seal");
    assert!(!h.can_redo(), "no redo of a rewind");
}

/// LCV-198 — a rewind with no group, or a mark at or past `group_len`,
/// changes nothing and does not move the revision.
#[test]
fn rewind_group_is_a_no_op_without_a_group_or_past_its_length() {
    let (mut doc, mut h) = (Document::default(), History::new());
    h.commit(Box::new(CreateLine::new(line_at(0.0))), &mut doc);
    assert_eq!(h.rewind_group(0, &mut doc), 0, "no group open");
    assert_eq!((h.revision(), doc.entities.len()), (1, 1));

    let mark = group_with_mark(&mut doc, &mut h);
    let rev = h.revision();
    for m in [mark, mark + 1, usize::MAX] {
        assert_eq!(h.rewind_group(m, &mut doc), 0);
    }
    assert_eq!(h.revision(), rev);
    assert_eq!((h.group_len(), doc.entities.len()), (1, 3));
}

/// LCV-198 AC 6 — `end_group` after a rewind seals only the survivors: one
/// undo entry for them, and none when the rewind went back to 0.
#[test]
fn end_group_after_a_rewind_seals_only_the_survivors() {
    let (mut doc, mut h) = (Document::default(), History::new());
    let mark = group_with_mark(&mut doc, &mut h);
    risky_steps(&mut doc, &mut h);
    h.rewind_group(mark, &mut doc);
    assert_eq!(h.end_group(), Some(1));
    assert_eq!(h.len(), 2, "the human line plus the one survivor");
    assert!(h.undo(&mut doc));
    assert_eq!(doc.entities.len(), 1, "only the human line is left");

    let (mut doc, mut h) = (Document::default(), History::new());
    group_with_mark(&mut doc, &mut h);
    risky_steps(&mut doc, &mut h);
    assert_eq!(h.rewind_group(0, &mut doc), 4);
    assert_eq!(h.end_group(), Some(0));
    assert_eq!(h.len(), 1, "nothing survived: no entry for the turn");
    assert_eq!(doc.entities.len(), 1);
}
