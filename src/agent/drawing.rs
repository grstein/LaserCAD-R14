//! LCV-144 — `create_drawing`: one declarative JSON batch of lines, circles and
//! arcs, validated completely before anything is sent (ADR 0010).
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

use crate::agent::tools::{ToolCallError, expected_form, validate_r};

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
/// each item in turn.
///
/// # Errors
///
/// The first shape failure, as a [`ToolCallError::Arg`] whose path is a root
/// key, `(root)`, `entities[i]` or `entities[i].key` (LCV-192).
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
    entities
        .iter()
        .enumerate()
        .map(|(i, v)| item(i, v))
        .collect()
}

/// One entity: an object, a known `type`, that type's keys (another type's
/// key only as `null`), finite numbers, a boolean `ccw`, and the shared
/// radius rule. Every refusal's path is `entities[index].key`.
fn item(index: usize, value: &Value) -> Result<DrawingItem, ToolCallError> {
    let at = |key: &str| format!("entities[{index}].{key}");
    let fail = |key: &str, reason: &str| arg(at(key), reason, expected_form(key));
    let obj = value.as_object().ok_or_else(|| {
        let form = "an object with a type and that type's keys";
        arg(format!("entities[{index}]"), "not an object", form)
    })?;
    let kind = obj.get("type").ok_or_else(|| fail("type", "missing"))?;
    let Some(ty) = kind.as_str().and_then(keys::entity_type) else {
        return Err(fail("type", "unknown type"));
    };
    let (a, kind_name, own_keys) = (ty.article(), ty.name, ty.own_keys());
    // A key of another type is tolerated only as `null` (ADR 0010 §2).
    let own = |k: &str| k == "type" || ty.takes(k);
    for (key, value) in obj.iter().filter(|(k, _)| !own(k)) {
        if !keys::published(key) {
            return Err(arg(at(&cut(key)), "unknown key", &own_keys));
        } else if !value.is_null() {
            let reason = format!("not {a} {kind_name} key");
            return Err(arg(at(key), &reason, &format!("null or {own_keys}")));
        }
    }
    if let Some(key) = ty.keys.iter().find(|k| !obj.contains_key(k.name)) {
        return Err(fail(key.name, "missing"));
    }
    let num = |key: &str| {
        obj.get(key)
            .and_then(Value::as_f64)
            .filter(|n| n.is_finite())
            .ok_or_else(|| fail(key, "not a number"))
    };
    let radius = || {
        let r = num("r")?;
        validate_r(TOOL, r).map_err(|e| match e {
            ToolCallError::Arg { reason, .. } => fail("r", &reason),
            other => other,
        })?;
        Ok(r)
    };
    Ok(match kind.as_str() {
        Some("line") => DrawingItem::Line {
            x1: num("x1")?,
            y1: num("y1")?,
            x2: num("x2")?,
            y2: num("y2")?,
        },
        Some("circle") => DrawingItem::Circle {
            cx: num("cx")?,
            cy: num("cy")?,
            r: radius()?,
        },
        _ => DrawingItem::Arc {
            cx: num("cx")?,
            cy: num("cy")?,
            r: radius()?,
            // The one unit boundary of this file: degrees in, radians out.
            start: num("start_deg")?.to_radians(),
            end: num("end_deg")?.to_radians(),
            ccw: obj
                .get("ccw")
                .and_then(Value::as_bool)
                .ok_or_else(|| fail("ccw", "not a boolean"))?,
        },
    })
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
