//! Agent tool registry: JSON schemas + dispatch (LCV-078).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{json, Value};
use thiserror::Error;

use crate::document::{
    CreateArc, CreateCircle, CreateLine, DeleteEntities, Document, History, MoveEntities,
};
use crate::geometry::{Arc as GeoArc, Circle, Line, Vec2};

#[derive(Debug, Error)]
pub enum ToolCallError {
    #[error("unknown tool: `{0}`")]
    UnknownTool(String),
    #[error("tool `{tool}` missing required argument `{field}`")]
    MissingField {
        tool: &'static str,
        field: &'static str,
    },
    #[error("tool `{tool}` argument `{field}` is invalid: {reason}")]
    InvalidArg {
        tool: &'static str,
        field: &'static str,
        reason: String,
    },
}

/// OpenAI function-calling schemas. Order: create_line(0) create_circle(1)
/// create_arc(2) delete_entity(3) move_entity(4).
pub fn tool_definitions() -> Value {
    json!([
      {"type":"function","function":{"name":"create_line",
        "description":"Create a straight line segment between two endpoints in mm.",
        "parameters":{"type":"object",
          "properties":{"x1":{"type":"number"},"y1":{"type":"number"},
                        "x2":{"type":"number"},"y2":{"type":"number"}},
          "required":["x1","y1","x2","y2"]}}},
      {"type":"function","function":{"name":"create_circle",
        "description":"Create a full circle with a given center and radius in mm.",
        "parameters":{"type":"object",
          "properties":{"cx":{"type":"number"},"cy":{"type":"number"},
                        "r":{"type":"number"}},
          "required":["cx","cy","r"]}}},
      {"type":"function","function":{"name":"create_arc",
        "description":"Create a circular arc. start_deg and end_deg are in degrees (0 = +X axis). ccw=true means counter-clockwise sweep.",
        "parameters":{"type":"object",
          "properties":{"cx":{"type":"number"},"cy":{"type":"number"},
                        "r":{"type":"number"},"start_deg":{"type":"number"},
                        "end_deg":{"type":"number"},"ccw":{"type":"boolean"}},
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
          "required":["index","dx","dy"]}}}
    ])
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

#[rustfmt::skip]
fn get_index(args: &Value, tool: &'static str, doc_len: usize) -> Result<usize, ToolCallError> {
    let raw = args.get("index").and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field: "index" })?;
    if raw < 0.0 || raw.fract() != 0.0 || !raw.is_finite() { return Err(
        ToolCallError::InvalidArg { tool, field: "index",
            reason: format!("{raw} is not a non-negative integer") }); }
    let idx = raw as usize;
    if idx >= doc_len { return Err(ToolCallError::InvalidArg { tool, field: "index",
        reason: format!("index {idx} is out of range (document has {doc_len} entities)") }); }
    Ok(idx)
}

#[rustfmt::skip]
fn validate_r(tool: &'static str, r: f64) -> Result<(), ToolCallError> {
    if r > 0.0 && r.is_finite() { Ok(()) } else { Err(ToolCallError::InvalidArg {
        tool, field: "r", reason: format!("{r} is not a positive finite number") }) }
}

/// Dispatch an LLM `tool_call` to the matching CAD command via `history`.
#[rustfmt::skip]
pub fn dispatch_tool_call(
    name: &str, args: &Value, doc: &mut Document, history: &mut History,
) -> Result<String, ToolCallError> {
    match name {
        "create_line" => {
            let x1 = get_f64(args, "create_line", "x1")?;
            let y1 = get_f64(args, "create_line", "y1")?;
            let x2 = get_f64(args, "create_line", "x2")?;
            let y2 = get_f64(args, "create_line", "y2")?;
            let ln = Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2));
            history.commit(Box::new(CreateLine::new(ln)), doc);
            Ok(format!("Line created: ({x1:.3}, {y1:.3}) → ({x2:.3}, {y2:.3}) mm."))
        }
        "create_circle" => {
            let cx = get_f64(args, "create_circle", "cx")?;
            let cy = get_f64(args, "create_circle", "cy")?;
            let r = get_f64(args, "create_circle", "r")?;
            validate_r("create_circle", r)?;
            let circ = Circle::new(Vec2::new(cx, cy), r);
            history.commit(Box::new(CreateCircle::new(circ)), doc);
            Ok(format!("Circle created: center ({cx:.3}, {cy:.3}) mm, r = {r:.3} mm."))
        }
        "create_arc" => {
            let cx = get_f64(args, "create_arc", "cx")?;
            let cy = get_f64(args, "create_arc", "cy")?;
            let r = get_f64(args, "create_arc", "r")?;
            let start_deg = get_f64(args, "create_arc", "start_deg")?;
            let end_deg = get_f64(args, "create_arc", "end_deg")?;
            let ccw = get_bool(args, "create_arc", "ccw")?;
            validate_r("create_arc", r)?;
            let (sa, ea) = (start_deg.to_radians(), end_deg.to_radians());
            let arc = GeoArc::new(Vec2::new(cx, cy), r, sa, ea, ccw);
            history.commit(Box::new(CreateArc::new(arc)), doc);
            let dir = if ccw { "ccw" } else { "cw" };
            Ok(format!("Arc created: center ({cx:.3}, {cy:.3}) mm, r = {r:.3} mm, {start_deg:.1}°→{end_deg:.1}° {dir}."))
        }
        "delete_entity" => {
            let idx = get_index(args, "delete_entity", doc.entities.len())?;
            history.commit(Box::new(DeleteEntities::new(vec![idx])), doc);
            Ok(format!("Entity {idx} deleted."))
        }
        "move_entity" => {
            let dx = get_f64(args, "move_entity", "dx")?;
            let dy = get_f64(args, "move_entity", "dy")?;
            let idx = get_index(args, "move_entity", doc.entities.len())?;
            history.commit(Box::new(MoveEntities::new(vec![idx], Vec2::new(dx, dy))), doc);
            Ok(format!("Entity {idx} moved by ({dx:.3}, {dy:.3}) mm."))
        }
        _ => Err(ToolCallError::UnknownTool(name.to_owned())),
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use crate::document::Entity;
    use crate::geometry::{Circle, Line, Vec2, EPSILON};
    use core::f64::consts::FRAC_PI_2;
    use serde_json::json;

    struct Ctx { doc: Document, h: History }
    impl Ctx {
        fn new() -> Self { Self { doc: Document::default(), h: History::new() } }
        fn ok(&mut self, nm: &str, a: Value) -> String {
            dispatch_tool_call(nm, &a, &mut self.doc, &mut self.h).unwrap()
        }
        fn err(&mut self, nm: &str, a: Value) -> ToolCallError {
            dispatch_tool_call(nm, &a, &mut self.doc, &mut self.h).unwrap_err()
        }
    }
    fn req(d: &Value, i: usize) -> Value { d[i]["function"]["parameters"]["required"].clone() }

    #[test]
    fn tool_definitions_array_length_is_five() {
        assert_eq!(tool_definitions().as_array().unwrap().len(), 5);
    }
    #[test]
    fn tool_definitions_names_in_order() {
        let d = tool_definitions();
        let n = ["create_line","create_circle","create_arc","delete_entity","move_entity"];
        for (i, nm) in n.iter().enumerate() { assert_eq!(d[i]["function"]["name"], *nm); }
    }
    #[test]
    fn tool_definitions_types_are_function() {
        let d = tool_definitions();
        for i in 0..5 { assert_eq!(d[i]["type"], "function"); }
    }
    #[test]
    fn tool_definitions_required_fields() {
        let d = tool_definitions();
        assert_eq!(req(&d,0), json!(["x1","y1","x2","y2"]));
        assert_eq!(req(&d,1), json!(["cx","cy","r"]));
        assert_eq!(req(&d,2), json!(["cx","cy","r","start_deg","end_deg","ccw"]));
        assert_eq!(req(&d,3), json!(["index"]));
        assert_eq!(req(&d,4), json!(["index","dx","dy"]));
    }
    #[test]
    fn tool_definitions_arc_ccw_is_boolean() {
        assert_eq!(tool_definitions()[2]["function"]["parameters"]["properties"]["ccw"]["type"], "boolean");
    }
    #[test]
    fn dispatch_create_line_happy_path() {
        let mut c = Ctx::new();
        let r = c.ok("create_line", json!({"x1":0.0,"y1":0.0,"x2":10.0,"y2":0.0}));
        assert!(r.contains("Line created") && c.doc.entities.len() == 1);
        let exp = Entity::Line(Line::new(Vec2::new(0.0,0.0), Vec2::new(10.0,0.0)));
        assert_eq!(c.doc.entities[0], exp);
    }
    #[test]
    fn dispatch_create_circle_happy_path() {
        let mut c = Ctx::new();
        let r = c.ok("create_circle", json!({"cx":5.0,"cy":5.0,"r":3.0}));
        assert!(r.contains("Circle created"));
        assert_eq!(c.doc.entities[0], Entity::Circle(Circle::new(Vec2::new(5.0,5.0), 3.0)));
    }
    #[test]
    fn dispatch_create_circle_negative_radius() {
        let mut c = Ctx::new();
        let e = c.err("create_circle", json!({"cx":0.0,"cy":0.0,"r":-1.0}));
        assert!(matches!(e, ToolCallError::InvalidArg { field: "r", .. }) && c.doc.entities.is_empty());
    }
    #[test]
    fn dispatch_create_arc_degree_to_radian_conversion() {
        let mut c = Ctx::new();
        let r = c.ok("create_arc", json!({"cx":0.0,"cy":0.0,"r":1.0,"start_deg":0.0,"end_deg":90.0,"ccw":true}));
        assert!(r.contains("Arc created") && r.contains("0.0°→90.0°") && r.contains("ccw"));
        let Entity::Arc(arc) = c.doc.entities[0] else { panic!() };
        assert!((arc.start_angle).abs() < EPSILON && (arc.end_angle - FRAC_PI_2).abs() < EPSILON);
    }
    #[test]
    fn dispatch_create_arc_clockwise_label() {
        let mut c = Ctx::new();
        let r = c.ok("create_arc", json!({"cx":0.0,"cy":0.0,"r":1.0,"start_deg":0.0,"end_deg":90.0,"ccw":false}));
        assert!(r.ends_with("cw.") && !r.contains("ccw"));
    }
    #[test]
    fn dispatch_delete_entity_happy_path() {
        let mut c = Ctx::new();
        c.ok("create_line", json!({"x1":0.0,"y1":0.0,"x2":1.0,"y2":0.0}));
        let r = c.ok("delete_entity", json!({"index":0}));
        assert!(r.contains("Entity 0 deleted") && c.doc.entities.is_empty());
        assert!(c.h.undo(&mut c.doc) && c.doc.entities.len() == 1);
    }
    #[test]
    fn dispatch_delete_entity_out_of_range() {
        let mut c = Ctx::new();
        c.ok("create_line", json!({"x1":0.0,"y1":0.0,"x2":1.0,"y2":0.0}));
        let e = c.err("delete_entity", json!({"index":5}));
        assert!(matches!(e, ToolCallError::InvalidArg { field: "index", .. }));
    }
    #[test]
    fn dispatch_move_entity_happy_path() {
        let mut c = Ctx::new();
        c.ok("create_line", json!({"x1":0.0,"y1":0.0,"x2":10.0,"y2":0.0}));
        let r = c.ok("move_entity", json!({"index":0,"dx":3.0,"dy":4.0}));
        assert!(r.contains("Entity 0 moved"));
        let Entity::Line(l) = c.doc.entities[0] else { panic!() };
        assert!((l.p1.x-3.0).abs() < EPSILON && (l.p1.y-4.0).abs() < EPSILON);
        assert!((l.p2.x-13.0).abs() < EPSILON && (l.p2.y-4.0).abs() < EPSILON);
        assert!(c.h.undo(&mut c.doc));
        let Entity::Line(l) = c.doc.entities[0] else { panic!() };
        assert!(l.p1.x.abs() < EPSILON && l.p1.y.abs() < EPSILON);
    }
    #[test]
    fn dispatch_unknown_tool() {
        let mut c = Ctx::new();
        match c.err("frobnicate", json!({})) {
            ToolCallError::UnknownTool(s) => assert_eq!(s, "frobnicate"),
            _ => panic!(),
        }
    }
    #[test]
    fn dispatch_missing_field() {
        let mut c = Ctx::new();
        let e = c.err("create_line", json!({"x1":0.0}));
        assert!(matches!(e, ToolCallError::MissingField { tool: "create_line", .. }));
    }
    #[test]
    fn dispatch_missing_field_no_commit() {
        let mut c = Ctx::new();
        let _ = c.err("create_line", json!({"x1":0.0}));
        assert!(c.doc.entities.is_empty() && !c.h.undo(&mut c.doc));
    }
    #[test]
    fn tool_call_error_display_non_empty() {
        let e1 = ToolCallError::UnknownTool("x".into());
        let e2 = ToolCallError::MissingField { tool: "t", field: "f" };
        let e3 = ToolCallError::InvalidArg { tool: "t", field: "f", reason: "bad".into() };
        assert!(!e1.to_string().is_empty() && !e2.to_string().is_empty() && !e3.to_string().is_empty());
    }
}
