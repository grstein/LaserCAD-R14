//! The set arm of `agent_apply.rs::plan` (LCV-186): one edit over a list of
//! entity indices, committed as ONE existing document command, so it is one
//! step and one entry of the turn's flat undo group (ADR 0007 §D12).
//!
//! The list arrives shape-checked (1..=1000, unique); the range check against
//! the live drawing is here (ADR 0007 §D2a). Indices are sorted ascending
//! before the command is built, so copies and keep-source mirrors append in
//! ascending source order whatever order the model listed them in.
//!
//! Imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere.

use super::Planned;
use super::edit::{out_of_range, same_point};
use crate::agent::{AgentOutcome, SetOp};
use crate::app::agent_narrate::pt;
use crate::document::{
    Command, CopyEntities, DeleteEntities, Document, MoveEntities, TransformEntities,
};
use crate::geometry::{Transform, Vec2};

/// Refuse the first out-of-range entry as `<tool> indices[k]: …` (LCV-192
/// AC 2), else plan `op` over the sorted set.
pub(super) fn plan(tool: &str, indices: &[usize], op: &SetOp, doc: &Document) -> Planned {
    let count = doc.entity_count();
    if let Some((at, &index)) = indices.iter().enumerate().find(|(_, i)| **i >= count) {
        let path = format!("indices[{at}]");
        return Planned::Answer(out_of_range(tool, &path, index, count));
    }
    let mut sorted = indices.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let what = entities(n);
    let (sentence, command): (String, Box<dyn Command>) = match *op {
        SetOp::Delete => (
            delete_sentence(&sorted, count),
            Box::new(DeleteEntities::new(sorted)),
        ),
        SetOp::Move { dx, dy } => (
            format!("Moved {what} by {} mm.", pt(dx, dy)),
            Box::new(MoveEntities::new(sorted, Vec2::new(dx, dy))),
        ),
        SetOp::Copy { dx, dy } => (
            format!(
                "Copied {what} by {} mm as {}.",
                pt(dx, dy),
                appended(count, n)
            ),
            Box::new(CopyEntities::new(sorted, Vec2::new(dx, dy))),
        ),
        SetOp::Rotate { x, y, angle } => {
            let transform = Transform::Rotate {
                base: Vec2::new(x, y),
                angle,
            };
            if transform.is_identity() {
                return unchanged(format!("A whole-turn rotation leaves {what} unchanged"));
            }
            let about = pt(x, y);
            let sentence = format!(
                "Rotated {what} by {:.3}° about {about} mm.",
                angle.to_degrees()
            );
            (
                sentence,
                Box::new(TransformEntities::new(sorted, transform)),
            )
        }
        SetOp::Mirror {
            x1,
            y1,
            x2,
            y2,
            erase_source,
        } => {
            let (a, b) = (Vec2::new(x1, y1), Vec2::new(x2, y2));
            let transform = Transform::Mirror { a, b };
            if transform.is_identity() {
                return Planned::Answer(same_point(a));
            }
            let across = format!(
                "Mirrored {what} across the line {}–{} mm",
                pt(x1, y1),
                pt(x2, y2)
            );
            let sentence = if erase_source {
                format!("{across}.")
            } else {
                format!("{across} as {}.", appended(count, n))
            };
            let command = TransformEntities::new(sorted, transform).with_keep_source(!erase_source);
            (sentence, Box::new(command))
        }
        SetOp::Scale { x, y, factor } => {
            let transform = Transform::Scale {
                base: Vec2::new(x, y),
                factor,
            };
            if transform.is_identity() {
                return unchanged(format!("A factor of 1 leaves {what} unchanged"));
            }
            let sentence = format!("Scaled {what} by {factor:.3} about {} mm.", pt(x, y));
            (
                sentence,
                Box::new(TransformEntities::new(sorted, transform)),
            )
        }
    };
    Planned::Commit(command, sentence)
}

/// An identity transform: answered, nothing committed (LCV-158 AC8, LCV-182
/// AC6).
fn unchanged(what: String) -> Planned {
    Planned::Answer(AgentOutcome::Ok(format!("{what}; nothing committed.")))
}

/// `1 entity` / `n entities`.
fn entities(n: usize) -> String {
    if n == 1 {
        "1 entity".to_owned()
    } else {
        format!("{n} entities")
    }
}

/// The indices `n` appended entities take after `count`: `entity c` or
/// `entities c..c+n-1`.
fn appended(count: usize, n: usize) -> String {
    if n == 1 {
        format!("entity {count}")
    } else {
        format!("entities {count}..{}", count + n - 1)
    }
}

/// What a set delete removes and whether later indices shifted (AC4);
/// `sorted` is ascending and non-empty, `count` is the count before it.
fn delete_sentence(sorted: &[usize], count: usize) -> String {
    let list: Vec<String> = sorted.iter().map(usize::to_string).collect();
    let label = if sorted.len() == 1 {
        "index"
    } else {
        "indices"
    };
    let shift = if sorted
        .first()
        .is_some_and(|&first| first + sorted.len() == count)
    {
        "No indices shifted."
    } else {
        "Every later index moved down by the number of deleted entities before it."
    };
    format!(
        "Deleted {} ({label} {}). {shift}",
        entities(sorted.len()),
        list.join(", ")
    )
}
