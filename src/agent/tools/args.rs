//! The argument half of `tools.rs` (LCV-192): [`ToolCallError`] and the
//! per-field getters and validators every tool parser shares. Split out of
//! `tools.rs` for the LOC cap; `tools.rs` re-exports what callers name.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::Value;
use thiserror::Error;

use crate::agent::drawing;

/// Why a tool call could not be turned into an
/// [`AgentAction`](crate::agent::AgentAction).
///
/// Every variant describes the *call*, never the drawing: a tool call that is
/// well-formed but asks for something the document cannot satisfy is not an
/// error here, it is an `AgentOutcome::Refused` at the apply site.
#[derive(Debug, Error)]
pub enum ToolCallError {
    /// The model named a tool that is not in
    /// [`tool_definitions`](super::tool_definitions).
    #[error("unknown tool: `{0}`")]
    UnknownTool(String),
    /// An argument refusal (LCV-192 AC 1), shown as
    /// `<tool> <path>: <reason>; expected <form>` by [`refusal`]. Names the
    /// field and its accepted form; never echoes a string payload.
    #[error("{}", refusal(tool, path, reason, expected))]
    Arg {
        /// The tool that was called.
        tool: String,
        /// The failing field: `r`, `indices[2]`, `entities[3].r`, `(root)`.
        path: String,
        /// What is wrong with it: `missing`, `not a number`, `-3 is out of range`.
        reason: String,
        /// The accepted type and range, usually [`expected_form`] of the key.
        expected: String,
    },
}

impl ToolCallError {
    /// The refusal of `field` itself: the path is the field and the expected
    /// form is [`expected_form`] of it.
    pub(crate) fn arg(tool: &str, field: &str, reason: impl Into<String>) -> Self {
        Self::Arg {
            tool: tool.to_owned(),
            path: field.to_owned(),
            reason: reason.into(),
            expected: expected_form(field).to_owned(),
        }
    }
}

/// The one refusal shape of LCV-192 (AC 1, AC 2, AC 3), shared by argument
/// and document refusals: `<tool> <path>: <reason>; expected <form>`.
pub(crate) fn refusal(tool: &str, path: &str, reason: &str, expected: &str) -> String {
    format!("{tool} {path}: {reason}; expected {expected}")
}

/// The accepted form of an argument, by its key, in the words the model
/// reads after `expected` (LCV-192 AC 1).
pub(crate) fn expected_form(field: &str) -> &'static str {
    match field {
        "x1" | "y1" | "x2" | "y2" | "cx" | "cy" | "dx" | "dy" | "x" | "y" | "x0" | "y0" => {
            "a number in mm"
        }
        "r" => "a positive number in mm",
        "start_deg" | "end_deg" | "degrees" => "a number in degrees",
        "ccw" | "erase_source" => "true or false",
        "factor" => "a positive number",
        "index" => "a non-negative integer (an index from query_entities)",
        "indices" => "a list of 1 to 1000 distinct entity indices",
        "id" => r#"an entity id such as "e7" (from query_entities)"#,
        "ids" => r#"a list of 1 to 1000 distinct entity ids such as "e7""#,
        "layer" => "the name of an existing layer, 1 to 64 characters",
        "frame" => r#""view", "drawing" or "region""#,
        "version" => "the integer 1",
        "entities" => "a list of 1 to 1000 entity objects",
        "type" => r#""line", "circle" or "arc""#,
        "(root)" => "a JSON object",
        "query" => r#""distance", "length", "bbox", "intersections" or "angle""#,
        "points" => "a list of points {x, y} in mm",
        "point" => "a point {x, y} in mm",
        _ => "a value the tool's schema allows",
    }
}

/// `args[field]` unless absent or JSON `null`: both read as `missing`.
fn present<'a>(args: &'a Value, tool: &str, field: &str) -> Result<&'a Value, ToolCallError> {
    args.get(field)
        .filter(|v| !v.is_null())
        .ok_or_else(|| ToolCallError::arg(tool, field, "missing"))
}

/// A number argument: absent is `missing`, any other type `not a number`.
pub(super) fn get_f64(args: &Value, tool: &str, field: &str) -> Result<f64, ToolCallError> {
    present(args, tool, field)?
        .as_f64()
        .ok_or_else(|| ToolCallError::arg(tool, field, "not a number"))
}

/// A boolean argument: absent is `missing`, any other type `not a boolean`.
pub(super) fn get_bool(args: &Value, tool: &str, field: &str) -> Result<bool, ToolCallError> {
    present(args, tool, field)?
        .as_bool()
        .ok_or_else(|| ToolCallError::arg(tool, field, "not a boolean"))
}

/// Shape check only: non-negative, integral, finite. The range check
/// (`index < entities.len()`) lives at the apply site — ADR 0007 §D2a.
pub(super) fn get_index(args: &Value, tool: &str) -> Result<usize, ToolCallError> {
    let raw = get_f64(args, tool, "index")?;
    if raw < 0.0 || raw.fract() != 0.0 || !raw.is_finite() {
        return Err(ToolCallError::arg(
            tool,
            "index",
            format!("{raw} is not an index"),
        ));
    }
    Ok(raw as usize)
}

/// The one radius rule, shared by `create_circle`, `create_arc` and
/// `create_drawing` (ADR 0010 §3).
pub(crate) fn validate_r(tool: &str, r: f64) -> Result<(), ToolCallError> {
    validate_positive(tool, "r", r)
}

/// `value` must be positive and finite: a radius, or a scale factor
/// (LCV-182).
pub(super) fn validate_positive(tool: &str, field: &str, value: f64) -> Result<(), ToolCallError> {
    if value > 0.0 && value.is_finite() {
        Ok(())
    } else {
        Err(ToolCallError::arg(
            tool,
            field,
            format!("{value} is out of range"),
        ))
    }
}

/// The optional `layer` argument of a scalar creation tool (LCV-156).
pub(super) fn get_layer(args: &Value, tool: &str) -> Result<Option<String>, ToolCallError> {
    drawing::layer_arg(args).map_err(|reason| ToolCallError::arg(tool, "layer", reason))
}
