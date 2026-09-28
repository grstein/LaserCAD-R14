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

use serde_json::{json, Value};
use thiserror::Error;

use crate::agent::bridge::AgentAction;

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
/// query_selection(6).
///
/// The two queries take no arguments at all — an explicitly empty
/// `properties` / `required` pair rather than an absent `parameters`, because
/// some providers reject a function schema without one (LCV-123 AC 13).
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
          "required":["index","dx","dy"]}}},
      {"type":"function","function":{"name":"query_entities",
        "description":"List every entity in the drawing with its zero-based index, kind and mm geometry, plus the bed size. Call this before deleting or moving an entity you did not create in this turn.",
        "parameters":{"type":"object","properties":{},"required":[]}}},
      {"type":"function","function":{"name":"query_selection",
        "description":"List the zero-based indices of the entities the operator currently has selected.",
        "parameters":{"type":"object","properties":{},"required":[]}}}
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
        }),
        "create_circle" => {
            let cx = get_f64(args, "create_circle", "cx")?;
            let cy = get_f64(args, "create_circle", "cy")?;
            let r = get_f64(args, "create_circle", "r")?;
            validate_r("create_circle", r)?;
            Ok(AgentAction::CreateCircle { cx, cy, r })
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
        _ => Err(ToolCallError::UnknownTool(name.to_owned())),
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_PI_2;
    use serde_json::json;

    fn ok(nm: &str, a: Value) -> AgentAction { parse_tool_call(nm, &a).unwrap() }
    fn err(nm: &str, a: Value) -> ToolCallError { parse_tool_call(nm, &a).unwrap_err() }
    fn req(d: &Value, i: usize) -> Value { d[i]["function"]["parameters"]["required"].clone() }

    // ── Schema (LCV-123 AC 13: five tools became seven) ──────────────────────

    /// AC 13 — the count. Renamed with the number it now pins: a test called
    /// `..._is_five` asserting `7` is a lie a reader has to read the body to
    /// catch.
    #[test]
    fn tool_definitions_array_length_is_seven() {
        assert_eq!(tool_definitions().as_array().unwrap().len(), 7);
    }
    /// AC 13 — the order is part of the contract: every other schema test and
    /// `transport.rs`'s wire assertions index into this array.
    #[test]
    fn tool_definitions_names_in_order() {
        let d = tool_definitions();
        let n = ["create_line","create_circle","create_arc","delete_entity","move_entity",
                 "query_entities","query_selection"];
        for (i, nm) in n.iter().enumerate() { assert_eq!(d[i]["function"]["name"], *nm); }
        assert_eq!(d[n.len()], Value::Null, "and nothing after them");
    }
    #[test]
    fn tool_definitions_types_are_function() {
        let d = tool_definitions();
        for i in 0..7 { assert_eq!(d[i]["type"], "function"); }
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
    /// AC 13 — both queries declare an **empty** object, not a missing one:
    /// `properties` is `{}` and `required` is `[]`, both present.
    #[test]
    fn the_two_query_schemas_take_no_parameters() {
        let d = tool_definitions();
        for i in [5, 6] {
            let params = &d[i]["function"]["parameters"];
            assert_eq!(params["type"], "object", "schema {i}");
            assert_eq!(params["properties"], json!({}), "schema {i}");
            assert_eq!(params["required"], json!([]), "schema {i}");
            assert!(params["properties"].is_object(), "schema {i} must be {{}}, not null");
            assert!(params["required"].is_array(), "schema {i} must be [], not null");
        }
    }
    /// AC 14, AC 15 — both queries parse with **any** arguments object, since
    /// there is nothing in it to read: `{}`, a stray field, or JSON null.
    #[test]
    fn the_two_queries_parse_with_any_arguments() {
        for (nm, expected) in [("query_entities", AgentAction::QueryEntities),
                               ("query_selection", AgentAction::QuerySelection)] {
            for args in [json!({}), json!({"ignored":1}), Value::Null] {
                assert_eq!(ok(nm, args.clone()), expected, "{nm} with {args}");
            }
        }
    }
    #[test]
    fn tool_definitions_arc_ccw_is_boolean() {
        assert_eq!(tool_definitions()[2]["function"]["parameters"]["properties"]["ccw"]["type"], "boolean");
    }

    // ── AC 5: the five names produce the right action ────────────────────────

    /// AC 5 — every field lands in its own slot. The four numbers are distinct
    /// so a transposed pair cannot pass.
    #[test]
    fn parse_create_line_happy_path() {
        assert_eq!(ok("create_line", json!({"x1":1.0,"y1":2.0,"x2":3.0,"y2":4.0})),
            AgentAction::CreateLine { x1: 1.0, y1: 2.0, x2: 3.0, y2: 4.0 });
    }
    #[test]
    fn parse_create_circle_happy_path() {
        assert_eq!(ok("create_circle", json!({"cx":5.0,"cy":6.0,"r":3.0})),
            AgentAction::CreateCircle { cx: 5.0, cy: 6.0, r: 3.0 });
    }
    #[test]
    fn parse_create_circle_negative_radius() {
        let e = err("create_circle", json!({"cx":0.0,"cy":0.0,"r":-1.0}));
        let ToolCallError::InvalidArg { field, reason, .. } = e else { panic!("{e:?}") };
        assert_eq!(field, "r");
        assert_eq!(reason, "-1 is not a positive finite number");
    }
    /// AC 5 — a zero radius is rejected by the same check as a negative one.
    #[test]
    fn parse_create_circle_zero_radius() {
        let e = err("create_circle", json!({"cx":0.0,"cy":0.0,"r":0.0}));
        assert!(matches!(e, ToolCallError::InvalidArg { field: "r", .. }), "{e:?}");
    }
    /// AC 5 — degrees in, radians out, to within 1e-12.
    #[test]
    fn parse_create_arc_degree_to_radian_conversion() {
        let a = ok("create_arc", json!({"cx":0.0,"cy":0.0,"r":1.0,
            "start_deg":0.0,"end_deg":90.0,"ccw":true}));
        let AgentAction::CreateArc { start, end, ccw, r, .. } = a else { panic!("{a:?}") };
        assert!(start.abs() < 1e-12, "start was {start}");
        assert!((end - FRAC_PI_2).abs() < 1e-12, "end was {end}");
        assert!(ccw);
        assert_eq!(r, 1.0);
    }
    /// AC 5 — `ccw: false` survives the parse. The `cw` wording moved to the
    /// apply site with the rest of the outcome strings.
    #[test]
    fn parse_create_arc_keeps_clockwise() {
        let a = ok("create_arc", json!({"cx":0.0,"cy":0.0,"r":1.0,
            "start_deg":0.0,"end_deg":90.0,"ccw":false}));
        assert!(matches!(a, AgentAction::CreateArc { ccw: false, .. }), "{a:?}");
    }
    /// AC 5 — a negative arc radius is rejected before the angles matter.
    #[test]
    fn parse_create_arc_rejects_bad_radius() {
        let e = err("create_arc", json!({"cx":0.0,"cy":0.0,"r":0.0,
            "start_deg":0.0,"end_deg":90.0,"ccw":true}));
        assert!(matches!(e, ToolCallError::InvalidArg { tool: "create_arc", field: "r", .. }), "{e:?}");
    }
    #[test]
    fn parse_delete_entity_happy_path() {
        assert_eq!(ok("delete_entity", json!({"index":2})), AgentAction::Delete { index: 2 });
    }
    #[test]
    fn parse_move_entity_happy_path() {
        assert_eq!(ok("move_entity", json!({"index":0,"dx":3.0,"dy":4.0})),
            AgentAction::Move { index: 0, dx: 3.0, dy: 4.0 });
    }
    #[test]
    fn parse_unknown_tool() {
        match err("frobnicate", json!({})) {
            ToolCallError::UnknownTool(s) => assert_eq!(s, "frobnicate"),
            other => panic!("{other:?}"),
        }
    }
    #[test]
    fn parse_missing_field() {
        let e = err("create_line", json!({"x1":0.0}));
        assert!(matches!(e, ToolCallError::MissingField { tool: "create_line", field: "y1" }), "{e:?}");
    }
    /// AC 5 — a rejected call yields no action at all. This is what
    /// `dispatch_missing_field_no_commit` asserted before there was an action
    /// type: nothing partial escapes.
    #[test]
    fn parse_missing_field_produces_no_action() {
        assert!(parse_tool_call("create_line", &json!({"x1":0.0})).is_err());
        assert!(parse_tool_call("move_entity", &json!({"dx":1.0,"dy":2.0})).is_err());
    }
    /// AC 5 — a non-finite coordinate cannot survive JSON: `serde_json` has no
    /// NaN or infinity, so `json!` stores `null` and the field reads as
    /// missing. Pinned here so the behaviour is a decision, not a surprise.
    #[test]
    fn parse_nan_coordinate_is_reported_as_a_missing_field() {
        assert_eq!(json!({"x1": f64::NAN})["x1"], Value::Null);
        let e = err("create_line", json!({"x1":f64::NAN,"y1":0.0,"x2":1.0,"y2":1.0}));
        assert!(matches!(e, ToolCallError::MissingField { field: "x1", .. }), "{e:?}");
        let e = err("create_circle", json!({"cx":0.0,"cy":0.0,"r":f64::INFINITY}));
        assert!(matches!(e, ToolCallError::MissingField { field: "r", .. }), "{e:?}");
    }
    /// AC 5 — the finite half of `validate_r` is unreachable through JSON, so
    /// it is pinned directly. Deleting `r.is_finite()` turns this red.
    #[test]
    fn validate_r_rejects_infinity_and_nan() {
        for bad in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.0, -1.0] {
            assert!(validate_r("create_circle", bad).is_err(), "{bad} must be rejected");
        }
        for good in [f64::MIN_POSITIVE, 0.5, 1e9] {
            assert!(validate_r("create_circle", good).is_ok(), "{good} must be accepted");
        }
    }
    /// AC 5 — a negative index and a fractional index are both shape errors,
    /// with the message they have always had.
    #[test]
    fn parse_index_must_be_a_non_negative_integer() {
        for (raw, text) in [(json!(-1), "-1 is not a non-negative integer"),
                            (json!(1.5), "1.5 is not a non-negative integer")] {
            let e = err("delete_entity", json!({"index": raw}));
            let ToolCallError::InvalidArg { field, reason, .. } = e else { panic!("{e:?}") };
            assert_eq!(field, "index");
            assert_eq!(reason, text);
        }
    }

    // ── AC 6: the range check is not the agent's business ────────────────────

    /// AC 6, agent side — an index far past the end of any drawing parses
    /// happily, because there is no drawing in sight. The refusal is produced
    /// later, in `crate::app::agent_apply`, where the document is.
    #[test]
    fn parse_delete_entity_out_of_range_is_not_rejected_here() {
        assert_eq!(ok("delete_entity", json!({"index":7})), AgentAction::Delete { index: 7 });
        assert_eq!(ok("move_entity", json!({"index":99,"dx":0.0,"dy":0.0})),
            AgentAction::Move { index: 99, dx: 0.0, dy: 0.0 });
    }

    #[test]
    fn tool_call_error_display_non_empty() {
        let e1 = ToolCallError::UnknownTool("x".into());
        let e2 = ToolCallError::MissingField { tool: "t", field: "f" };
        let e3 = ToolCallError::InvalidArg { tool: "t", field: "f", reason: "bad".into() };
        assert!(!e1.to_string().is_empty() && !e2.to_string().is_empty() && !e3.to_string().is_empty());
    }

    // ── AC 7 / AC 3: what this file must never become ────────────────────────

    /// AC 7 — validation stays hand-rolled: no `serde` shortcut deserialises
    /// an `AgentAction` here. AC 3 — and no document state reaches this file.
    ///
    /// Bounded at the bare `#[cfg(test)]` at column 0, needles built with
    /// `concat!`, so the scan cannot match the literals in this very test.
    #[test]
    fn tools_parses_by_hand_and_holds_no_document() {
        let src = include_str!("tools.rs");
        let at = src.find("\n#[cfg(test)]").expect("tools.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(implementation.contains(concat!("pub fn parse_", "tool_call")),
            "positive control: the parser must be declared in this file");
        assert!(implementation.contains(concat!("fn validate", "_r")),
            "positive control: the radius check must still be here");

        for forbidden in [
            concat!("from_value::<", "AgentAction>"),
            concat!("Deser", "ialize"),
            concat!("crate::", "document"),
            concat!("Doc", "ument"),
            concat!("His", "tory"),
        ] {
            let hit = implementation.lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//")
                       && !l.trim_start().starts_with("//!"));
            assert!(hit.is_none(), "tools.rs must not name `{forbidden}` in code: {hit:?}");
        }
    }
}
