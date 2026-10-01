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

pub(super) fn get_f64(
    args: &Value,
    tool: &'static str,
    field: &'static str,
) -> Result<f64, ToolCallError> {
    args.get(field)
        .and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field })
}

pub(super) fn get_bool(
    args: &Value,
    tool: &'static str,
    field: &'static str,
) -> Result<bool, ToolCallError> {
    args.get(field)
        .and_then(|v| v.as_bool())
        .ok_or(ToolCallError::MissingField { tool, field })
}

/// Shape check only: non-negative, integral, finite. The range check
/// (`index < entities.len()`) lives at the apply site — ADR 0007 §D2a.
#[rustfmt::skip]
pub(super) fn get_index(args: &Value, tool: &'static str) -> Result<usize, ToolCallError> {
    let raw = args.get("index").and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field: "index" })?;
    if raw < 0.0 || raw.fract() != 0.0 || !raw.is_finite() { return Err(
        ToolCallError::InvalidArg { tool, field: "index",
            reason: format!("{raw} is not a non-negative integer") }); }
    Ok(raw as usize)
}

/// The one radius rule, shared by `create_circle`, `create_arc` and
/// `create_drawing` (ADR 0010 §3).
pub(crate) fn validate_r(tool: &'static str, r: f64) -> Result<(), ToolCallError> {
    validate_positive(tool, "r", r)
}

/// `value` must be positive and finite: a radius, or a scale factor
/// (LCV-182).
#[rustfmt::skip]
pub(super) fn validate_positive(tool: &'static str, field: &'static str, value: f64)
    -> Result<(), ToolCallError> {
    if value > 0.0 && value.is_finite() { Ok(()) } else { Err(ToolCallError::InvalidArg {
        tool, field, reason: format!("{value} is not a positive finite number") }) }
}

/// The optional `layer` argument of a scalar creation tool (LCV-156).
pub(super) fn get_layer(args: &Value, tool: &'static str) -> Result<Option<String>, ToolCallError> {
    drawing::layer_arg(args).map_err(|reason| ToolCallError::InvalidArg {
        tool,
        field: "layer",
        reason,
    })
}
