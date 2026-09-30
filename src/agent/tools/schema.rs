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
        "description":"Delete the entity at the given zero-based document index.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"}},
          "required":["index"]}}},
      {"type":"function","function":{"name":"move_entity",
        "description":"Translate a single entity by (dx, dy) mm.",
        "parameters":{"type":"object",
          "properties":{"index":{"type":"integer"},"dx":{"type":"number"},
                        "dy":{"type":"number"}},
          "required":["index","dx","dy"]}}},
      {"type":"function","function":{"name":"query_entities",
        "description":"List every entity in the drawing with its zero-based index, kind, mm geometry and layer, plus the bed size and the layers. Call this before deleting or moving an entity you did not create in this turn.",
        "parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"query_selection",
        "description":"List the zero-based indices of the entities the operator currently has selected.",
        "parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"create_drawing",
        "description":"Append many lines, circles and arcs (mm, degrees) in one atomic call. The whole batch is validated first; any error draws nothing.",
        "parameters":drawing::schema()}}
    ])
}
