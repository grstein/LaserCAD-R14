//! The edit arms of `agent_apply.rs::plan`: actions that address an existing
//! entity by index (LCV-078; `copy_entity` LCV-157; `rotate_entity` LCV-158;
//! `mirror_entity` LCV-181; `scale_entity` LCV-182). Split out for the LOC
//! cap (LCV-157); the range check lives here because only these arms need it
//! (ADR 0007 §D2a).
//!
//! Imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere.

use super::Planned;
use crate::agent::AgentOutcome;
use crate::agent::tools::refusal;
use crate::app::agent_narrate::{describe, pt};
use crate::document::{
    CopyEntities, DeleteEntities, Document, Entity, MoveEntities, TransformEntities,
};
use crate::geometry::{Transform, Vec2};

/// `delete_entity`: refuse an out-of-range index, else name what goes and
/// which indices shift.
pub(super) fn delete(index: usize, doc: &Document) -> Planned {
    match in_range("delete_entity", index, doc) {
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
    match in_range("move_entity", index, doc) {
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
    match in_range("copy_entity", index, doc) {
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

/// `rotate_entity`: refuse an out-of-range index; a whole turn commits
/// nothing (LCV-158 AC8); else rotate about `(x, y)` by `angle` radians.
pub(super) fn rotate(index: usize, x: f64, y: f64, angle: f64, doc: &Document) -> Planned {
    let entity = match in_range("rotate_entity", index, doc) {
        Err(refusal) => return Planned::Answer(refusal),
        Ok(entity) => entity,
    };
    let base = Vec2::new(x, y);
    let transform = Transform::Rotate { base, angle };
    let what = format!("entity {index} ({})", describe(entity));
    if transform.is_identity() {
        return Planned::Answer(AgentOutcome::Ok(format!(
            "A whole-turn rotation leaves {what} unchanged; nothing committed."
        )));
    }
    Planned::Commit(
        Box::new(TransformEntities::new(vec![index], transform)),
        format!(
            "Rotated {what} by {:.3}° about {} mm.",
            angle.to_degrees(),
            pt(x, y)
        ),
    )
}

/// `mirror_entity`: refuse an out-of-range index or a mirror line whose two
/// points coincide; else mirror across it, replacing the entity when
/// `erase_source`, else appending the image on its layer (LCV-181 AC10).
pub(super) fn mirror(index: usize, line: (Vec2, Vec2), erase: bool, doc: &Document) -> Planned {
    let entity = match in_range("mirror_entity", index, doc) {
        Err(refusal) => return Planned::Answer(refusal),
        Ok(entity) => entity,
    };
    let (a, b) = line;
    let transform = Transform::Mirror { a, b };
    if transform.is_identity() {
        return Planned::Answer(same_point(a));
    }
    let what = format!("entity {index} ({})", describe(entity));
    let across = format!("the line {}–{} mm", pt(a.x, a.y), pt(b.x, b.y));
    let cmd = TransformEntities::new(vec![index], transform).with_keep_source(!erase);
    let summary = if erase {
        format!("Mirrored {what} across {across}.")
    } else {
        format!(
            "Mirrored {what} across {across} as entity {}.",
            doc.entity_count()
        )
    };
    Planned::Commit(Box::new(cmd), summary)
}

/// `scale_entity`: refuse an out-of-range index; a factor of 1 commits
/// nothing (LCV-182 AC6); else scale about `base` by `factor`, already
/// checked positive at parse time.
pub(super) fn scale(index: usize, base: Vec2, factor: f64, doc: &Document) -> Planned {
    let entity = match in_range("scale_entity", index, doc) {
        Err(refusal) => return Planned::Answer(refusal),
        Ok(entity) => entity,
    };
    let transform = Transform::Scale { base, factor };
    let what = format!("entity {index} ({})", describe(entity));
    if transform.is_identity() {
        return Planned::Answer(AgentOutcome::Ok(format!(
            "A factor of 1 leaves {what} unchanged; nothing committed."
        )));
    }
    Planned::Commit(
        Box::new(TransformEntities::new(vec![index], transform)),
        format!(
            "Scaled {what} by {factor:.3} about {} mm.",
            pt(base.x, base.y)
        ),
    )
}

/// The check the worker thread cannot make: is `index` a real entity?
///
/// ADR 0007 §D2a — re-homed where `entities.len()` is actually knowable;
/// refused in the LCV-192 shape naming `tool`.
fn in_range<'d>(tool: &str, index: usize, doc: &'d Document) -> Result<&'d Entity, AgentOutcome> {
    doc.entities
        .get(index)
        .ok_or_else(|| out_of_range(tool, "index", index, doc.entity_count()))
}

/// `<tool> <path>: <index> is out of range; expected <the valid range>`
/// (LCV-192 AC 2); an empty drawing has no range, so the form says when one
/// exists.
pub(super) fn out_of_range(tool: &str, path: &str, index: usize, count: usize) -> AgentOutcome {
    let expected = match count {
        0 => "an index once the drawing has entities (it has 0)".to_owned(),
        n => format!("0..={} (the drawing has {n} entities)", n - 1),
    };
    let reason = format!("{index} is out of range");
    AgentOutcome::Refused(refusal(tool, path, &reason, &expected))
}

/// A mirror line whose two points coincide at `a` (LCV-192 AC 2).
pub(super) fn same_point(a: Vec2) -> AgentOutcome {
    let reason = format!("same point as x1, y1 at {} mm", pt(a.x, a.y));
    let expected = "a second point distinct from x1, y1";
    AgentOutcome::Refused(refusal("mirror_entity", "x2, y2", &reason, expected))
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
