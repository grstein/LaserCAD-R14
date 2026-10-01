//! Agent tool registry: JSON schemas + argument parsing (LCV-078 / LCV-122).
//!
//! This file turns one tool call from the model into one
//! [`AgentAction`] — and stops there. It does **not** commit anything: since
//! LCV-122 the background thread that calls into here owns no [`Document`] and
//! no [`History`] (ADR 0007 §D1), so applying the action is the UI thread's job
//! in `src/app/agent_apply.rs`.
//!
//! ## Which checks live here, and which do not (ADR 0007 §D2a)
//!
//! Everything that is a property of the **arguments alone** is checked here,
//! hand-rolled, with a per-field message: missing field, wrong JSON type, a
//! radius that is not positive and finite, an index that is not a non-negative
//! integer. That is deliberately not a `serde` derive — the derive would buy
//! free parsing and throw away every one of those messages (ADR 0007
//! §Alternatives).
//!
//! The one check that is a property of the **document** —
//! `index < entities.len()` — is not here, because the thread cannot know
//! `entities.len()`. It happens at the apply site and answers
//! `AgentOutcome::Refused`, which the model can act on.
//!
//! Angles cross here: the schema speaks `start_deg` / `end_deg` because that is
//! what a human dictates, and [`AgentAction`] speaks radians because that is
//! what the kernel speaks everywhere else.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! [`Document`]: crate::document::Document
//! [`History`]: crate::document::History

use serde_json::Value;

use crate::agent::bridge::AgentAction;
use crate::agent::drawing;

mod args;
pub use args::ToolCallError;
pub(crate) use args::{expected_form, refusal, validate_r};
use args::{get_bool, get_f64, get_index, get_layer, validate_positive};
mod capture;
mod schema;
use schema::base_definitions;
mod transform;

/// OpenAI function-calling schemas. Order: create_line(0) create_circle(1)
/// create_arc(2) delete_entity(3) move_entity(4) copy_entity(5)
/// rotate_entity(6) mirror_entity(7) scale_entity(8) set_layer(9)
/// query_entities(10) query_selection(11) check_drawing(12) create_drawing(13).
///
/// The two queries and `check_drawing` take no arguments at all — an explicitly empty
/// `properties` / `required` pair rather than an absent `parameters`, because
/// some providers reject a function schema without one (LCV-123 AC 13).
///
/// `vision` is the turn-start snapshot of both canvas opt-ins (LCV-145,
/// ADR 0011 item 1): when on, `capture_canvas` is inserted just before
/// `create_drawing`, which stays last.
pub fn tool_definitions(vision: bool) -> Value {
    let mut tools = base_definitions();
    if let (true, Some(list)) = (vision, tools.as_array_mut()) {
        let at = list.len().saturating_sub(1);
        list.insert(at, capture::definition());
    }
    tools
}

/// Turn one LLM `tool_call` into the [`AgentAction`] it asks for.
///
/// Pure: no document, no history, no side effect. The caller sends the action
/// over the bridge and the UI thread decides what really happens to it.
///
/// # Errors
///
/// [`ToolCallError`] when the tool name is unknown or an argument is missing,
/// of the wrong type, or outside its domain.
#[rustfmt::skip]
pub fn parse_tool_call(name: &str, args: &Value) -> Result<AgentAction, ToolCallError> {
    // LCV-186: `indices` on an edit tool is a set call; else as before.
    if let Some(set) = transform::parse_set(name, args) { return set; }
    match name {
        "create_line" => Ok(AgentAction::CreateLine {
            x1: get_f64(args, "create_line", "x1")?,
            y1: get_f64(args, "create_line", "y1")?,
            x2: get_f64(args, "create_line", "x2")?,
            y2: get_f64(args, "create_line", "y2")?,
            layer: get_layer(args, "create_line")?,
        }),
        "create_circle" => {
            let cx = get_f64(args, "create_circle", "cx")?;
            let cy = get_f64(args, "create_circle", "cy")?;
            let r = get_f64(args, "create_circle", "r")?;
            validate_r("create_circle", r)?;
            Ok(AgentAction::CreateCircle { cx, cy, r, layer: get_layer(args, "create_circle")? })
        }
        "create_arc" => {
            let cx = get_f64(args, "create_arc", "cx")?;
            let cy = get_f64(args, "create_arc", "cy")?;
            let r = get_f64(args, "create_arc", "r")?;
            let start_deg = get_f64(args, "create_arc", "start_deg")?;
            let end_deg = get_f64(args, "create_arc", "end_deg")?;
            let ccw = get_bool(args, "create_arc", "ccw")?;
            validate_r("create_arc", r)?;
            // A unit boundary, like `rotate_entity`: degrees in, radians out.
            Ok(AgentAction::CreateArc {
                cx, cy, r, start: start_deg.to_radians(), end: end_deg.to_radians(), ccw,
                layer: get_layer(args, "create_arc")?,
            })
        }
        "delete_entity" => Ok(AgentAction::Delete {
            index: get_index(args, "delete_entity")?,
        }),
        "move_entity" => {
            // Read order preserved from LCV-078: a call missing both `dx` and
            // `index` still reports `dx` first, as it always has.
            let dx = get_f64(args, "move_entity", "dx")?;
            let dy = get_f64(args, "move_entity", "dy")?;
            let index = get_index(args, "move_entity")?;
            Ok(AgentAction::Move { index, dx, dy })
        }
        "copy_entity" => {
            let index = get_index(args, "copy_entity")?;
            let dx = get_f64(args, "copy_entity", "dx")?;
            let dy = get_f64(args, "copy_entity", "dy")?;
            Ok(AgentAction::Copy { index, dx, dy })
        }
        "rotate_entity" => {
            let index = get_index(args, "rotate_entity")?;
            let x = get_f64(args, "rotate_entity", "x")?;
            let y = get_f64(args, "rotate_entity", "y")?;
            let degrees = get_f64(args, "rotate_entity", "degrees")?;
            Ok(AgentAction::Rotate { index, x, y, angle: degrees.to_radians() })
        }
        "mirror_entity" => {
            let index = get_index(args, "mirror_entity")?;
            let x1 = get_f64(args, "mirror_entity", "x1")?;
            let y1 = get_f64(args, "mirror_entity", "y1")?;
            let x2 = get_f64(args, "mirror_entity", "x2")?;
            let y2 = get_f64(args, "mirror_entity", "y2")?;
            let erase_source = get_bool(args, "mirror_entity", "erase_source")?;
            Ok(AgentAction::Mirror { index, x1, y1, x2, y2, erase_source })
        }
        "scale_entity" => {
            let index = get_index(args, "scale_entity")?;
            let x = get_f64(args, "scale_entity", "x")?;
            let y = get_f64(args, "scale_entity", "y")?;
            let factor = get_f64(args, "scale_entity", "factor")?;
            validate_positive("scale_entity", "factor", factor)?;
            Ok(AgentAction::Scale { index, x, y, factor })
        }
        "set_layer" => transform::parse_set_layer(args),
        // Read-only, argument-free: whatever the model sends as arguments —
        // `{}`, a stray field, or nothing at all — the answer is the same, so
        // there is no shape to check and nothing to refuse (AC 14, AC 15).
        "query_entities" => Ok(AgentAction::QueryEntities),
        "query_selection" => Ok(AgentAction::QuerySelection),
        "check_drawing" => Ok(AgentAction::CheckDrawing),
        // Shape only (LCV-187); permission and the frame's area are checked
        // live at the apply site, never here (LCV-145 AC 2).
        "capture_canvas" => Ok(AgentAction::CaptureCanvas(capture::parse(args)?)),
        "create_drawing" => Ok(AgentAction::CreateDrawing {
            items: drawing::parse(args)?,
            layer: drawing::layer_arg(args)
                .map_err(|reason| ToolCallError::arg("create_drawing", "layer", reason))?,
        }),
        _ => Err(ToolCallError::UnknownTool(name.to_owned())),
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests;
