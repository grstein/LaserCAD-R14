//! The [`Command`] trait — the single mutation contract for the
//! [`crate::document::Document`].
//!
//! AGENTS.md §"State and mutation (hard contract)" requires that every
//! mutation of a `Document` go through a `Command` plus the history stack
//! (LCV-026). This module declares the trait, documents the round-trip
//! invariant, and ships [`NoOpCommand`] — the minimal implementation used as
//! a fixture by the history stack tests and as a copy-paste template by every
//! concrete command demand (LCV-023 onward).
//!
//! ## Method naming: `do_` (with trailing underscore)
//!
//! Rust reserves the bare keyword `do`. The available alternatives are
//! `r#do`, which is uglier than `do_` in every call site, and `apply`, which
//! disagrees with the AGENTS.md hard contract wording. `do_` is chosen so the
//! method name in code matches the contract verbatim; downstream demands
//! (LCV-023..LCV-027) follow this name without translation.
//!
//! ## Round-trip invariant
//!
//! For every `cmd: impl Command` and every `doc: &mut Document` that the
//! command is valid against, the sequence
//!
//! ```ignore
//! cmd.do_(&mut doc);
//! cmd.undo(&mut doc);
//! ```
//!
//! must restore `doc` to a state bit-equivalent to the pre-`do_` state —
//! same entities in the same order, same selection, same byte-for-byte
//! `Entity` payloads (compared via `PartialEq`). Reviewers gate-check this
//! invariant on every concrete command via a `do_ → undo` test on at least
//! one representative input.
//!
//! ## Object safety
//!
//! [`Command`] is intentionally object-safe — no generic methods, no
//! `Self: Sized` bounds, no associated types. The history stack (LCV-026)
//! stores `Box<dyn Command>` and `&dyn Command`; adding a generic method to
//! this trait would break that storage. Reviewers should flag any such
//! addition.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`. The command trait is part of
//! the pure-Rust kernel.
//!
//! Introduced by demand LCV-022.

use crate::document::Document;

/// The single mutation contract for a [`Document`].
///
/// Every operator-visible action — drawing a line (LCV-023), deleting
/// entities (LCV-024), trimming a curve (LCV-025), changing the selection
/// (LCV-027), agent-issued mutations (Phase 7) — is expressed as a value
/// that implements this trait, pushed onto the history stack (LCV-026), and
/// reversible by Ctrl+Z because `undo` perfectly inverts `do_`.
///
/// ## Method naming
///
/// `do_` carries a trailing underscore because `do` is a Rust reserved
/// keyword. See the [module-level docs](self) for the rationale.
///
/// ## Round-trip invariant (load-bearing)
///
/// `cmd.do_(doc)` followed by `cmd.undo(doc)` must leave `doc` in a state
/// bit-equivalent (`PartialEq`-equal on every entity, same length, same
/// order, same selection) to the state observed immediately before `do_`.
/// Reviewers verify this with a dedicated test on every concrete command.
///
/// ## `&mut self` on every method
///
/// All three methods take `&mut self` so a command can capture state inside
/// `do_` (e.g. the index at which an entity was inserted, the previous
/// payload of an edited entity) and consume it inside `undo`. Repeated
/// `do_ → undo → do_ → undo` cycles must remain valid because redo (LCV-026)
/// is implemented by calling `do_` a second time.
///
/// ## Object safety
///
/// The trait is object-safe by construction — no generics, no associated
/// types, no `Self: Sized` bounds. [`Box<dyn Command>`] is the storage shape
/// used by the history stack.
///
/// ## Errors
///
/// `do_` / `undo` do not return [`Result`]. A `Command` is only constructed
/// after the tool or agent has validated its inputs; a runtime failure
/// inside `do_` is an invariant violation, not a recoverable error. If a
/// future demand needs fallible commands, it can wrap them in a separate
/// `Result`-returning facade.
pub trait Command {
    /// Apply this command to `doc`, mutating it in place.
    ///
    /// May capture into `self` whatever state is needed for [`Self::undo`]
    /// to restore the document.
    fn do_(&mut self, doc: &mut Document);

    /// Reverse the effect of the most recent [`Self::do_`] call.
    ///
    /// After this returns, `doc` must be bit-equivalent (via `PartialEq` on
    /// every entity) to its state immediately before `do_` ran.
    fn undo(&mut self, doc: &mut Document);

    /// Short, human-readable label used by the history-aware UI (Phase 6,
    /// e.g. `Edit > Undo Create Line`).
    ///
    /// Returns a borrowed `&str` so commands can return string literals
    /// (`"Create Line"`, `"Delete"`) without allocating.
    fn label(&self) -> &str;
}

/// A [`Command`] that does nothing.
///
/// Two reasons it exists in v2:
///
/// 1. **History-stack fixture.** LCV-026 exercises push / pop / undo / redo
///    semantics; `NoOpCommand` is the smallest value that satisfies the
///    trait without depending on `Entity` mutation, which keeps the
///    history-stack tests independent of the concrete-command demands.
/// 2. **Copy-paste template.** Future command authors (LCV-023 onward) use
///    this as the starting shape — minimal `impl Command`, literal label,
///    no state.
///
/// Round-trip is trivially satisfied: both `do_` and `undo` are no-ops, so
/// the document is unchanged by either, and therefore the pre-`do_` and
/// post-`undo` states are identical.
#[derive(Debug, Default)]
pub struct NoOpCommand;

impl Command for NoOpCommand {
    fn do_(&mut self, _doc: &mut Document) {}

    fn undo(&mut self, _doc: &mut Document) {}

    fn label(&self) -> &str {
        "No-op"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Entity;
    use crate::geometry::{Line, Vec2};

    /// AC#1 — the trait surface has exactly the three methods, callable on
    /// a concrete impl. Compile-checking this guards against accidental
    /// signature drift on the contract.
    #[test]
    fn command_trait_has_three_methods() {
        let mut cmd = NoOpCommand;
        let mut doc = Document::default();
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);
        let _: &str = cmd.label();
    }

    /// AC#2 — the trait is object-safe; both `Box<dyn Command>` and
    /// `&dyn Command` compile.
    #[test]
    fn command_is_object_safe() {
        let _: Box<dyn Command> = Box::new(NoOpCommand);
        let cmd = NoOpCommand;
        let _: &dyn Command = &cmd;
    }

    /// AC#3 — `NoOpCommand::label()` returns the exact string `"No-op"`.
    #[test]
    fn noop_label_exact() {
        assert_eq!(NoOpCommand.label(), "No-op");
    }

    /// AC#4 — round-trip on an empty document leaves it empty with no
    /// bounds.
    #[test]
    fn noop_roundtrip_on_empty_document() {
        let mut doc = Document::default();
        let mut cmd = NoOpCommand;
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);
        assert_eq!(doc.entity_count(), 0);
        assert!(doc.bounds().is_none());
    }

    /// AC#5 — round-trip on a one-line document preserves the entity
    /// bit-equal via `PartialEq`.
    #[test]
    fn noop_roundtrip_preserves_existing_entity() {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
        doc.entities.push(Entity::Line(line));

        let mut cmd = NoOpCommand;
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);

        assert_eq!(doc.entity_count(), 1);
        assert_eq!(doc.entities[0], Entity::Line(line));
    }

    /// AC#6 — `label()` returns a non-empty string for `NoOpCommand`.
    #[test]
    fn noop_label_non_empty() {
        let cmd = NoOpCommand;
        assert!(!cmd.label().is_empty());
    }

    /// Calling the trait through a `&mut dyn Command` reference exercises
    /// the dynamic-dispatch path the history stack will use.
    #[test]
    fn noop_roundtrip_via_dyn_dispatch() {
        let mut doc = Document::default();
        let line = Line::new(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0));
        doc.entities.push(Entity::Line(line));

        let mut cmd: Box<dyn Command> = Box::new(NoOpCommand);
        cmd.do_(&mut doc);
        cmd.undo(&mut doc);

        assert_eq!(doc.entities[0], Entity::Line(line));
        assert_eq!(cmd.label(), "No-op");
    }
}
