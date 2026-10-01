//! The `create_drawing` schema and the shared `layer` property (LCV-144,
//! LCV-156), split out of `drawing.rs` for the LOC cap (LCV-192); typed from
//! the key table since LCV-196.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Value, json};

use super::{ENTITY_TYPES, KeyKind, MAX_DRAWING_ENTITIES};

/// The `parameters` schema of `create_drawing` (ADR 0010 §2).
///
/// A provider-compatible hint, not the contract: only `type`, `properties`,
/// `required`, `description`, `enum`, `minItems`, `maxItems` and `items`, so
/// no provider drops the tool. Which keys each `type` needs is said in words;
/// [`parse`](super::parse) is what enforces it. Every item property comes
/// from the [`ENTITY_TYPES`] table.
pub fn schema() -> Value {
    let types: Vec<&str> = ENTITY_TYPES.iter().map(|t| t.name).collect();
    let mut props = serde_json::Map::new();
    props.insert("type".into(), json!({"type": "string", "enum": types}));
    for key in ENTITY_TYPES.iter().flat_map(|t| t.keys.iter()) {
        props.insert(key.name.into(), property(key.kind));
    }
    let per_type: Vec<String> = ENTITY_TYPES
        .iter()
        .map(|t| {
            let names: Vec<&str> = t.keys.iter().map(|k| k.name).collect();
            format!("{}: {}.", t.name, names.join(", "))
        })
        .collect();
    let description = format!("{ITEM_INTRO} {} {ITEM_RULES}", per_type.join(" "));
    json!({"type": "object",
      "properties": {
        "version": {"type": "integer", "enum": [1],
          "description": "Format version; always 1."},
        "entities": {"type": "array", "minItems": 1, "maxItems": MAX_DRAWING_ENTITIES,
          "items": {"type": "object",
            "description": description,
            "properties": props,
            "required": ["type"]}},
        "layer": layer_schema()},
      "required": ["version", "entities"]})
}

/// The opening of the item description.
const ITEM_INTRO: &str =
    "One entity: its type and that type's keys; keys of other types may be omitted or null.";

/// What the keys mean, after the per-type key lists.
const ITEM_RULES: &str = "Angles in degrees, 0 = +X; ccw=true is counter-clockwise. \
polyline: 2 to 1000 points {x, y}; closed=true adds the last-to-first line. \
rect: (x, y) is the lower-left corner; corner_radius (optional, 0) from 0 to half the shorter side rounds the corners. \
polygon: regular, inscribed in r, 3 to 64 sides, first vertex at start_deg (optional, 0). \
text: the TEXT command's single-stroke font, baseline start (x, y), cap height in mm, 1 to 256 characters. \
linear_array and polar_array: count-1 copies (count 2 to 1000) of the earlier items listed in of by 0-based batch position; \
copy k is moved by k*(dx, dy), or rotated by k*step_deg counter-clockwise about (cx, cy); \
an array listed in of brings its whole output (a grid), and it may list no array itself. \
The 1000-entity limit counts the entities after expansion.";

/// The JSON schema of one key kind.
fn property(kind: KeyKind) -> Value {
    match kind {
        KeyKind::Num => json!({"type": "number"}),
        KeyKind::Int => json!({"type": "integer"}),
        KeyKind::Bool => json!({"type": "boolean"}),
        KeyKind::Str => json!({"type": "string"}),
        KeyKind::Points => json!({"type": "array", "items": {"type": "object",
            "properties": {"x": {"type": "number"}, "y": {"type": "number"}},
            "required": ["x", "y"]}}),
        KeyKind::IndexList => json!({"type": "array", "items": {"type": "integer"}}),
    }
}

/// The optional `layer` property every creation tool takes (LCV-156).
pub fn layer_schema() -> Value {
    json!({"type": "string",
      "description": "Optional: the name of an existing layer to draw on (see query_entities). Omit to use the current layer."})
}
