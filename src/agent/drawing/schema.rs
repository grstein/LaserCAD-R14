//! The `create_drawing` schema and the shared `layer` property (LCV-144,
//! LCV-156), split out of `drawing.rs` for the LOC cap (LCV-192).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Value, json};

use super::{ENTITY_TYPES, KeyKind, MAX_DRAWING_ENTITIES};

/// The `parameters` schema of `create_drawing` (ADR 0010 §2).
///
/// A provider-compatible hint, not the contract: only `type`, `properties`,
/// `required`, `description`, `enum`, `minItems`, `maxItems` and `items`, so
/// no provider drops the tool. Which keys each `type` needs is said in words;
/// [`parse`](super::parse) is what enforces it.
pub fn schema() -> Value {
    let types: Vec<&str> = ENTITY_TYPES.iter().map(|t| t.name).collect();
    let mut props = serde_json::Map::new();
    props.insert("type".into(), json!({"type": "string", "enum": types}));
    for key in ENTITY_TYPES.iter().flat_map(|t| t.keys.iter()) {
        let kind = if key.kind == KeyKind::Bool {
            "boolean"
        } else {
            "number"
        };
        props.insert(key.name.into(), json!({"type": kind}));
    }
    json!({"type": "object",
      "properties": {
        "version": {"type": "integer", "enum": [1],
          "description": "Format version; always 1."},
        "entities": {"type": "array", "minItems": 1, "maxItems": MAX_DRAWING_ENTITIES,
          "items": {"type": "object",
            "description": "One entity: its type and that type's keys; keys of other types may be omitted or null. line: x1, y1, x2, y2. circle: cx, cy, r. arc: cx, cy, r, start_deg, end_deg, ccw (degrees, 0 = +X; ccw=true is counter-clockwise).",
            "properties": props,
            "required": ["type"]}},
        "layer": layer_schema()},
      "required": ["version", "entities"]})
}

/// The optional `layer` property every creation tool takes (LCV-156).
pub fn layer_schema() -> Value {
    json!({"type": "string",
      "description": "Optional: the name of an existing layer to draw on (see query_entities). Omit to use the current layer."})
}
