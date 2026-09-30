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
    fn tool_definitions_array_length_is_nine() {
        assert_eq!(tool_definitions(false).as_array().unwrap().len(), 9);
    }
    /// AC 13 — the order is part of the contract: every other schema test and
    /// `transport.rs`'s wire assertions index into this array.
    #[test]
    fn tool_definitions_names_in_order() {
        let d = tool_definitions(false);
        let n = ["create_line","create_circle","create_arc","delete_entity","move_entity",
                 "copy_entity","query_entities","query_selection","create_drawing"];
        for (i, nm) in n.iter().enumerate() { assert_eq!(d[i]["function"]["name"], *nm); }
        assert_eq!(d[n.len()], Value::Null, "and nothing after them");
    }
    #[test]
    fn tool_definitions_types_are_function() {
        let d = tool_definitions(false);
        for i in 0..9 { assert_eq!(d[i]["type"], "function"); }
    }
    #[test]
    fn tool_definitions_required_fields() {
        let d = tool_definitions(false);
        assert_eq!(req(&d,0), json!(["x1","y1","x2","y2"]));
        assert_eq!(req(&d,1), json!(["cx","cy","r"]));
        assert_eq!(req(&d,2), json!(["cx","cy","r","start_deg","end_deg","ccw"]));
        assert_eq!(req(&d,3), json!(["index"]));
        assert_eq!(req(&d,4), json!(["index","dx","dy"]));
        assert_eq!(req(&d,5), json!(["index","dx","dy"]));
    }
    /// AC 13 — both queries declare an **empty** object, not a missing one:
    /// `properties` is `{}` and `required` is `[]`, both present.
    #[test]
    fn the_two_query_schemas_take_no_parameters() {
        let d = tool_definitions(false);
        for i in [6, 7] {
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
    /// LCV-144 AC 10 — `create_drawing` is last, and its schema speaks only the
    /// keywords every provider accepts (ADR 0010 §2).
    #[test]
    fn create_drawing_is_last_and_its_schema_is_provider_safe() {
        assert_eq!(tool_definitions(true).as_array().unwrap().last().unwrap()["function"]["name"],
            "create_drawing");
        let d = tool_definitions(false);
        let last = d.as_array().unwrap().last().unwrap().clone();
        assert_eq!(last["function"]["name"], "create_drawing");
        let params = &last["function"]["parameters"];
        fn walk(v: &Value, found: &mut Vec<String>) {
            match v {
                Value::Object(m) => for (k, v) in m {
                    if ["oneOf","anyOf","allOf","const","additionalProperties"].contains(&k.as_str()) {
                        found.push(k.clone());
                    }
                    walk(v, found);
                },
                Value::Array(a) => for v in a { walk(v, found); },
                _ => {}
            }
        }
        let mut found = Vec::new();
        walk(params, &mut found);
        assert!(found.is_empty(), "forbidden keywords: {found:?}");
        assert!(params["properties"]["entities"]["items"]["properties"]["type"]["enum"].is_array(),
            "positive control: the walk reaches the item schema");
        assert_eq!(params["properties"]["entities"]["maxItems"], 1000);
        assert_eq!(params["properties"]["entities"]["minItems"], 1);
        assert_eq!(params["properties"]["version"]["enum"], json!([1]));
        assert_eq!(params["required"], json!(["version","entities"]));
    }
    /// LCV-145 AC 2 — `capture_canvas` is advertised only when the turn-start
    /// vision flag is on, with the empty-parameters form, right before
    /// `create_drawing` so that stays last; the rest is unchanged.
    #[test]
    fn capture_canvas_is_advertised_only_with_vision() {
        let names = |d: &Value| -> Vec<String> { d.as_array().unwrap().iter()
            .map(|t| t["function"]["name"].as_str().unwrap().to_owned()).collect() };
        let off = tool_definitions(false);
        assert!(!names(&off).contains(&"capture_canvas".to_owned()));
        let on = tool_definitions(true);
        let mut expected = names(&off);
        expected.insert(8, "capture_canvas".to_owned());
        assert_eq!(names(&on), expected);
        let params = &on[8]["function"]["parameters"];
        assert_eq!(*params, json!({"type":"object","properties":{},"required":[]}));
        assert_eq!(on[9]["function"]["name"], "create_drawing");
        for (i, tool) in off.as_array().unwrap().iter().enumerate() {
            let j = if i < 8 { i } else { i + 1 };
            assert_eq!(on[j], *tool, "tool {i} unchanged");
        }
    }
    /// LCV-145 AC 3 — `capture_canvas` takes no arguments: any object parses.
    #[test]
    fn capture_canvas_parses_with_any_arguments() {
        for args in [json!({}), json!({"frame":"bed"}), Value::Null] {
            assert_eq!(ok("capture_canvas", args.clone()), AgentAction::CaptureCanvas, "{args}");
        }
    }
    /// LCV-144 — the parse arm delegates to `drawing::parse`.
    #[test]
    fn parse_create_drawing_delegates_to_the_drawing_parser() {
        let a = ok("create_drawing", json!({"version":1,"entities":[
            {"type":"circle","cx":1.0,"cy":2.0,"r":3.0}]}));
        assert_eq!(a, AgentAction::CreateDrawing { layer: None, items: vec![
            crate::agent::DrawingItem::Circle { cx: 1.0, cy: 2.0, r: 3.0 }] });
        let e = err("create_drawing", json!({"version":2,"entities":[]}));
        assert_eq!(e.to_string(), "create_drawing version: must be the integer 1");
    }
    #[test]
    fn tool_definitions_arc_ccw_is_boolean() {
        assert_eq!(tool_definitions(false)[2]["function"]["parameters"]["properties"]["ccw"]["type"], "boolean");
    }

    // ── AC 5: the five names produce the right action ────────────────────────

    /// AC 5 — every field lands in its own slot. The four numbers are distinct
    /// so a transposed pair cannot pass.
    #[test]
    fn parse_create_line_happy_path() {
        assert_eq!(ok("create_line", json!({"x1":1.0,"y1":2.0,"x2":3.0,"y2":4.0})),
            AgentAction::CreateLine { layer: None, x1: 1.0, y1: 2.0, x2: 3.0, y2: 4.0 });
    }
    #[test]
    fn parse_create_circle_happy_path() {
        assert_eq!(ok("create_circle", json!({"cx":5.0,"cy":6.0,"r":3.0})),
            AgentAction::CreateCircle { layer: None, cx: 5.0, cy: 6.0, r: 3.0 });
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
    /// LCV-157 AC9 — `copy_entity` parses like `move_entity`; the range check
    /// is the apply site's.
    #[test]
    fn parse_copy_entity() {
        assert_eq!(ok("copy_entity", json!({"index":1,"dx":-2.5,"dy":4.0})),
            AgentAction::Copy { index: 1, dx: -2.5, dy: 4.0 });
        assert_eq!(ok("copy_entity", json!({"index":99,"dx":0.0,"dy":0.0})),
            AgentAction::Copy { index: 99, dx: 0.0, dy: 0.0 });
        let e = err("copy_entity", json!({"index":0,"dx":1.0}));
        assert!(matches!(e, ToolCallError::MissingField { tool: "copy_entity", field: "dy" }), "{e:?}");
        let e = err("copy_entity", json!({"index":-1,"dx":1.0,"dy":1.0}));
        assert!(matches!(e, ToolCallError::InvalidArg { tool: "copy_entity", field: "index", .. }), "{e:?}");
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
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/agent/tools.rs"));
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
