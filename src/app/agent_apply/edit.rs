//! The edit arms of `agent_apply.rs::plan`: actions that address an existing
//! entity by index (LCV-078; `copy_entity` LCV-157). Split out for the LOC
//! cap (LCV-157); the range check lives here because only these arms need it
//! (ADR 0007 §D2a).
//!
//! Imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere.

use super::Planned;
use crate::agent::AgentOutcome;
use crate::app::agent_narrate::{describe, pt};
use crate::document::{CopyEntities, DeleteEntities, Document, Entity, MoveEntities};
use crate::geometry::Vec2;

/// `delete_entity`: refuse an out-of-range index, else name what goes and
/// which indices shift.
pub(super) fn delete(index: usize, doc: &Document) -> Planned {
    match in_range(index, doc) {
        Err(refusal) => Planned::Answer(refusal),
        Ok(entity) => Planned::Commit(
            Box::new(DeleteEntities::new(vec![index])),
            format!(
                "Deleted entity {index} ({}).{}",
                describe(entity),
                shift_note(index, doc.entity_count())
            ),
        ),
    }
}

/// `move_entity`: refuse an out-of-range index, else translate by `(dx, dy)`.
pub(super) fn move_(index: usize, dx: f64, dy: f64, doc: &Document) -> Planned {
    match in_range(index, doc) {
        Err(refusal) => Planned::Answer(refusal),
        Ok(entity) => Planned::Commit(
            Box::new(MoveEntities::new(vec![index], Vec2::new(dx, dy))),
            format!(
                "Moved entity {index} ({}) by {} mm.",
                describe(entity),
                pt(dx, dy)
            ),
        ),
    }
}

/// `copy_entity`: refuse an out-of-range index, else append a translated
/// copy on the source's layer and name its index (LCV-157 AC9).
pub(super) fn copy(index: usize, dx: f64, dy: f64, doc: &Document) -> Planned {
    match in_range(index, doc) {
        Err(refusal) => Planned::Answer(refusal),
        Ok(entity) => Planned::Commit(
            Box::new(CopyEntities::new(vec![index], Vec2::new(dx, dy))),
            format!(
                "Copied entity {index} ({}) by {} mm as entity {}.",
                describe(entity),
                pt(dx, dy),
                doc.entity_count()
            ),
        ),
    }
}

/// The check the worker thread cannot make: is `index` a real entity?
///
/// ADR 0007 §D2a — the wording of the refusal is the wording `get_index` used
/// to produce, re-homed where `entities.len()` is actually knowable.
fn in_range(index: usize, doc: &Document) -> Result<&Entity, AgentOutcome> {
    doc.entities.get(index).ok_or_else(|| {
        AgentOutcome::Refused(format!(
            "index {index} is out of range (the drawing has {} entities)",
            doc.entity_count()
        ))
    })
}

/// Which indices a delete at `index` renumbered, given the count *before* it.
fn shift_note(index: usize, count_before: usize) -> String {
    if index + 1 < count_before {
        format!(
            " Indices {}..{} are now {}..{}.",
            index + 1,
            count_before - 1,
            index,
            count_before - 2
        )
    } else {
        " No indices shifted.".to_string()
    }
}
