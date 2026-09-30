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

use serde_json::{Value, json};
use thiserror::Error;

use crate::agent::bridge::AgentAction;
use crate::agent::drawing;

mod schema;
use schema::base_definitions;

/// Why a tool call could not be turned into an [`AgentAction`].
///
/// Every variant describes the *call*, never the drawing: a tool call that is
/// well-formed but asks for something the document cannot satisfy is not an
/// error here, it is an `AgentOutcome::Refused` at the apply site.
#[derive(Debug, Error)]
pub enum ToolCallError {
    /// The model named a tool that is not in [`tool_definitions`].
    #[error("unknown tool: `{0}`")]
    UnknownTool(String),
    /// A required argument was absent, or present with the wrong JSON type.
    #[error("tool `{tool}` missing required argument `{field}`")]
    MissingField {
        /// The tool that was called.
        tool: &'static str,
        /// The argument that was missing.
        field: &'static str,
    },
    /// An argument was present and of the right type, but out of its domain.
    #[error("tool `{tool}` argument `{field}` is invalid: {reason}")]
    InvalidArg {
        /// The tool that was called.
        tool: &'static str,
        /// The argument that was rejected.
        field: &'static str,
        /// Human-readable explanation, read by the model as the tool result.
        reason: String,
    },
    /// A `create_drawing` root-level failure (ADR 0010 §3). `field` is a root
    /// key, `arguments`, or `entities[i]` when the item is not an object.
    #[error("create_drawing {field}: {reason}")]
    DrawingRoot {
        /// The failing path, never a payload value.
        field: String,
        /// Human-readable explanation, read by the model as the tool result.
        reason: String,
    },
    /// A `create_drawing` entity failure (ADR 0010 §3).
    #[error("create_drawing entities[{index}].{field}: {reason}")]
    DrawingItem {
        /// Zero-based index of the failing entity.
        index: usize,
        /// The failing key; an unknown one is cut to 64 characters.
        field: String,
        /// Human-readable explanation, read by the model as the tool result.
        reason: String,
    },
}

/// OpenAI function-calling schemas. Order: create_line(0) create_circle(1)
/// create_arc(2) delete_entity(3) move_entity(4) query_entities(5)
/// query_selection(6) create_drawing(7).
///
/// The two queries take no arguments at all — an explicitly empty
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
        list.insert(at, capture_canvas_definition());
    }
    tools
}

/// `capture_canvas`: no arguments, the empty-properties form.
fn capture_canvas_definition() -> Value {
    json!({"type":"function","function":{"name":"capture_canvas",
      "description":"Look at the drawing: returns a grayscale picture of the bed outline (grey) and every entity (black) as framed in the operator's viewport, with its mm mapping. No grid, selection or UI. Use query_entities for exact numbers.",
      "parameters":{"type":"object","properties":{},"required":[]}}})
}

fn get_f64(args: &Value, tool: &'static str, field: &'static str) -> Result<f64, ToolCallError> {
    args.get(field)
        .and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field })
}

fn get_bool(args: &Value, tool: &'static str, field: &'static str) -> Result<bool, ToolCallError> {
    args.get(field)
        .and_then(|v| v.as_bool())
        .ok_or(ToolCallError::MissingField { tool, field })
}

/// Shape check only: non-negative, integral, finite. The range check
/// (`index < entities.len()`) lives at the apply site — ADR 0007 §D2a.
#[rustfmt::skip]
fn get_index(args: &Value, tool: &'static str) -> Result<usize, ToolCallError> {
    let raw = args.get("index").and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field: "index" })?;
    if raw < 0.0 || raw.fract() != 0.0 || !raw.is_finite() { return Err(
        ToolCallError::InvalidArg { tool, field: "index",
            reason: format!("{raw} is not a non-negative integer") }); }
    Ok(raw as usize)
}

/// The one radius rule, shared by `create_circle`, `create_arc` and
/// `create_drawing` (ADR 0010 §3).
#[rustfmt::skip]
pub(crate) fn validate_r(tool: &'static str, r: f64) -> Result<(), ToolCallError> {
    if r > 0.0 && r.is_finite() { Ok(()) } else { Err(ToolCallError::InvalidArg {
        tool, field: "r", reason: format!("{r} is not a positive finite number") }) }
}

/// The optional `layer` argument of a scalar creation tool (LCV-156).
fn get_layer(args: &Value, tool: &'static str) -> Result<Option<String>, ToolCallError> {
    drawing::layer_arg(args).map_err(|reason| ToolCallError::InvalidArg {
        tool,
        field: "layer",
        reason,
    })
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
            // The one unit boundary in this file: degrees in, radians out.
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
        // Read-only, argument-free: whatever the model sends as arguments —
        // `{}`, a stray field, or nothing at all — the answer is the same, so
        // there is no shape to check and nothing to refuse (AC 14, AC 15).
        "query_entities" => Ok(AgentAction::QueryEntities),
        "query_selection" => Ok(AgentAction::QuerySelection),
        // Argument-free like the queries; permission is checked live at the
        // apply site, never here (LCV-145 AC 2).
        "capture_canvas" => Ok(AgentAction::CaptureCanvas),
        "create_drawing" => Ok(AgentAction::CreateDrawing {
            items: drawing::parse(args)?,
            layer: drawing::layer_arg(args).map_err(|reason| ToolCallError::DrawingRoot {
                field: "layer".to_owned(), reason })?,
        }),
        _ => Err(ToolCallError::UnknownTool(name.to_owned())),
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests;
