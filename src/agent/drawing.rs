//! LCV-144 — `create_drawing`: one declarative JSON batch of lines, circles and
//! arcs, validated completely before anything is sent (ADR 0010). LCV-196 adds
//! polylines, rects, polygons, text and arrays: [`items`] parses each into a
//! shape, [`expand`] turns the shapes into lines, circles and arcs, and the
//! 1000-entity cap applies to the expanded count.
//!
//! Hand-rolled like the rest of `tools.rs` (ADR 0007 §D2a): every failure names
//! its path, `create_drawing entities[17].r: …`, and stops the parse. The
//! payload is never echoed; an unknown key's name is, cut to 64 characters.
//!
//! A batch of one is exactly the scalar call (ADR 0010 §3): the radius goes
//! through the same `tools::validate_r` `create_circle` / `create_arc` use, and
//! degrees become radians here, once. There is deliberately no bed-bounds,
//! zero-length or magnitude rule — the scalar tools have none.
//!
//! Names no drawing-state type; MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::Value;

use crate::agent::tools::ToolCallError;

mod expand;
mod items;
mod keys;
mod schema;
pub use keys::{ENTITY_TYPES, EntityType, Key, KeyKind};
pub use schema::{layer_schema, schema};

/// One entity of a `create_drawing` batch, in mm; arc angles in radians.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawingItem {
    /// A line between two endpoints, mm.
    Line {
        /// First endpoint X, mm.
        x1: f64,
        /// First endpoint Y, mm.
        y1: f64,
        /// Second endpoint X, mm.
        x2: f64,
        /// Second endpoint Y, mm.
        y2: f64,
    },
    /// A full circle, mm.
    Circle {
        /// Centre X, mm.
        cx: f64,
        /// Centre Y, mm.
        cy: f64,
        /// Radius, mm; positive and finite.
        r: f64,
    },
    /// A circular arc, mm; angles in radians (`0` = +X axis).
    Arc {
        /// Centre X, mm.
        cx: f64,
        /// Centre Y, mm.
        cy: f64,
        /// Radius, mm; positive and finite.
        r: f64,
        /// Start angle, radians.
        start: f64,
        /// End angle, radians.
        end: f64,
        /// `true` for a counter-clockwise sweep (Y up).
        ccw: bool,
    },
}

/// Most entities one `create_drawing` call may carry (ADR 0010 §3).
pub const MAX_DRAWING_ENTITIES: usize = 1000;

/// Longest layer name a creation tool accepts, in characters (ADR 0012 §6).
pub const MAX_LAYER_NAME_CHARS: usize = 64;

/// The tool every refusal here names.
const TOOL: &str = "create_drawing";

/// Longest unknown-key name echoed back in an error, in characters.
const KEY_ECHO_CHARS: usize = 64;

/// Shape check of the optional `layer` argument (ADR 0012 §6): absent is
/// `None`; present must be a string of 1..=[`MAX_LAYER_NAME_CHARS`]
/// characters. Whether the layer exists is the apply site's question.
///
/// # Errors
///
/// The reason, without the tool or field framing.
pub fn layer_arg(args: &Value) -> Result<Option<String>, String> {
    let Some(value) = args.get("layer") else {
        return Ok(None);
    };
    let name = value.as_str().ok_or("not a string")?;
    let chars = name.chars().count();
    if chars == 0 || chars > MAX_LAYER_NAME_CHARS {
        return Err(format!("has {chars} characters"));
    }
    Ok(Some(name.to_owned()))
}

/// Parse the arguments of one `create_drawing` call into its items.
///
/// Pure. Checks, in order: the root is an object whose keys are `version`,
/// `entities` and optionally `layer` (checked by [`layer_arg`]); `version` is
/// the integer 1; `entities` holds 1..=[`MAX_DRAWING_ENTITIES`] items; then
/// each item in turn; then the expanded count, 1..=[`MAX_DRAWING_ENTITIES`].
///
/// # Errors
///
/// The first shape failure, as a [`ToolCallError::Arg`] whose path is a root
/// key, `(root)`, `entities[i]`, `entities[i].key` or deeper (LCV-192).
pub fn parse(args: &Value) -> Result<Vec<DrawingItem>, ToolCallError> {
    let root = |field: &str, reason: &str| ToolCallError::arg(TOOL, field, reason);
    let obj = args
        .as_object()
        .ok_or_else(|| root("(root)", "not an object"))?;
    if let Some(key) = obj
        .keys()
        .find(|k| !["version", "entities", "layer"].contains(&k.as_str()))
    {
        return Err(arg(cut(key), "unknown key", "version, entities or layer"));
    }
    let version = obj
        .get("version")
        .ok_or_else(|| root("version", "missing"))?;
    if version.as_u64() != Some(1) {
        return Err(root("version", "unsupported value"));
    }
    let entities = obj
        .get("entities")
        .ok_or_else(|| root("entities", "missing"))?
        .as_array()
        .ok_or_else(|| root("entities", "not a list"))?;
    if entities.is_empty() || entities.len() > MAX_DRAWING_ENTITIES {
        return Err(root("entities", &format!("has {} items", entities.len())));
    }
    let mut shapes = Vec::with_capacity(entities.len());
    for (i, value) in entities.iter().enumerate() {
        let shape = items::item(i, value, &shapes)?;
        shapes.push(shape);
    }
    expand::expand(&shapes)
}

/// A `create_drawing` refusal at `path` with its own expected form.
fn arg(path: String, reason: &str, expected: &str) -> ToolCallError {
    ToolCallError::Arg {
        tool: TOOL.to_owned(),
        path,
        reason: reason.to_owned(),
        expected: expected.to_owned(),
    }
}

/// An unknown key's name, cut to [`KEY_ECHO_CHARS`] characters.
fn cut(key: &str) -> String {
    key.chars().take(KEY_ECHO_CHARS).collect()
}

#[cfg(test)]
mod tests;
