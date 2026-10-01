//! The base agent tool schemas, in registry order (see
//! `tools.rs::tool_definitions`). Split out of `tools.rs` for the LOC cap
//! (LCV-157); `capture_canvas` stays there because it is inserted by opt-in.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Value, json};

use crate::agent::drawing::{self, layer_schema};

/// The always-present OpenAI function-calling schemas, `create_drawing` last.
pub(super) fn base_definitions() -> Value {
    let layer = layer_schema();
    // LCV-186: the set form of the six edit tools; exactly one of `index`
    // and `indices` is given, which `tools/transform.rs` enforces.
    let indices = json!({"type":"array","items":{"type":"integer"},"minItems":1,"maxItems":1000});
    json!([
      {"type":"function","function":{"name":"create_line",
        "description":"Create a straight line segment between two endpoints in mm.",
        "parameters":{"type":"object",
          "properties":{"x1":{"type":"number"},"y1":{"type":"number"},
                        "x2":{"type":"number"},"y2":{"type":"number"},"layer":layer},
          "required":["x1","y1","x2","y2"]}}},
      {"type":"function","function":{"name":"create_circle",
        "description":"Create a full circle with a given center and radius in mm.",
        "parameters":{"type":"object",
          "properties":{"cx":{"type":"number"},"cy":{"type":"number"},
                        "r":{"type":"number"},"layer":layer},
          "required":["cx","cy","r"]}}},
      {"type":"function","function":{"name":"create_arc",
        "description":"Create a circular arc. start_deg and end_deg are in degrees (0 = +X axis). ccw=true means counter-clockwise sweep.",
        "parameters":{"type":"object",
          "properties":{"cx":{"type":"number"},"cy":{"type":"number"},
                        "r":{"type":"number"},"start_deg":{"type":"number"},
                        "end_deg":{"type":"number"},"ccw":{"type":"boolean"},
                        "layer":layer},
          "required":["cx","cy","r","start_deg","end_deg","ccw"]}}},
      {"type":"function","function":{"name":"delete_entity",
        "description":"Delete the entity at the zero-based document index, or every entity listed in indices in one call. Give index or indices, not both.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"indices":indices},
          "required":[]}}},
      {"type":"function","function":{"name":"move_entity",
        "description":"Translate the entity at index, or every entity listed in indices, by (dx, dy) mm.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"indices":indices,"dx":{"type":"number"},
                        "dy":{"type":"number"}},
          "required":["dx","dy"]}}},
      {"type":"function","function":{"name":"copy_entity",
        "description":"Append a copy of the entity at index, or of every entity listed in indices in ascending index order, translated by (dx, dy) mm, on the source entity's layer.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"indices":indices,"dx":{"type":"number"},
                        "dy":{"type":"number"}},
          "required":["dx","dy"]}}},
      {"type":"function","function":{"name":"rotate_entity",
        "description":"Rotate the entity at index, or every entity listed in indices, about the one base point (x, y) mm by `degrees`, counter-clockwise positive.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"indices":indices,"x":{"type":"number"},
                        "y":{"type":"number"},"degrees":{"type":"number"}},
          "required":["x","y","degrees"]}}},
      {"type":"function","function":{"name":"mirror_entity",
        "description":"Mirror the entity at index, or every entity listed in indices, across the line through (x1, y1) and (x2, y2) mm. erase_source true replaces each entity; false keeps it and appends the mirrored copy on its layer, in ascending index order.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"indices":indices,"x1":{"type":"number"},
                        "y1":{"type":"number"},"x2":{"type":"number"},
                        "y2":{"type":"number"},"erase_source":{"type":"boolean"}},
          "required":["x1","y1","x2","y2","erase_source"]}}},
      {"type":"function","function":{"name":"scale_entity",
        "description":"Scale the entity at index, or every entity listed in indices, uniformly about the one base point (x, y) mm by `factor` (> 0): positions and radii scale, arc angles are kept.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"indices":indices,"x":{"type":"number"},
                        "y":{"type":"number"},"factor":{"type":"number"}},
          "required":["x","y","factor"]}}},
      {"type":"function","function":{"name":"set_layer",
        "description":"Move every entity listed in indices onto an existing layer, named as in query_entities; entities already on it stay. Layers cannot be created, renamed or deleted.",
        "parameters":{"type":"object",
          "properties":{"indices":indices,"layer":{"type":"string",
            "description":"The name of an existing layer (see query_entities)."}},
          "required":["indices","layer"]}}},
      {"type":"function","function":{"name":"query_entities",
        "description":"List every entity in the drawing with its zero-based index, kind, mm geometry and layer, plus the bed size and the layers. Call this before deleting or moving an entity you did not create in this turn.",
        "parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"query_selection",
        "description":"List the zero-based indices of the entities the operator currently has selected.",
        "parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"check_drawing",
        "description":"Check the Output-on layers for open ends, gaps under 0.5 mm, duplicates, degenerate entities and entities off the bed; returns one line per finding with entity indices and mm positions. Commits nothing.",
        "parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"create_drawing",
        "description":"Append many lines, circles and arcs (mm, degrees) in one atomic call. The whole batch is validated first; any error draws nothing.",
        "parameters":drawing::schema()}}
    ])
}
