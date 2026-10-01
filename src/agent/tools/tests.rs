    use super::*;
    use super::args::{expected_form, refusal};
    use core::f64::consts::FRAC_PI_2;
    use serde_json::json;

    fn ok(nm: &str, a: Value) -> AgentAction { parse_tool_call(nm, &a).unwrap() }
    fn err(nm: &str, a: Value) -> ToolCallError { parse_tool_call(nm, &a).unwrap_err() }
    fn req(d: &Value, i: usize) -> Value { d[i]["function"]["parameters"]["required"].clone() }
    /// The tool, path and reason of an argument refusal (LCV-192).
    fn at(e: &ToolCallError) -> (&str, &str, &str) {
        let ToolCallError::Arg { tool, path, reason, .. } = e else { panic!("{e:?}") };
        (tool, path, reason)
    }

    // ── Schema (LCV-123 AC 13: five tools became seven) ──────────────────────

    /// AC 13 — the count. Renamed with the number it now pins: a test called
    /// `..._is_five` asserting `7` is a lie a reader has to read the body to
    /// catch.
    #[test]
    fn tool_definitions_array_length_is_fourteen() {
        assert_eq!(tool_definitions(false).as_array().unwrap().len(), 14);
    }
    /// AC 13 — the order is part of the contract: every other schema test and
    /// `transport.rs`'s wire assertions index into this array.
    #[test]
    fn tool_definitions_names_in_order() {
        let d = tool_definitions(false);
        let n = ["create_line","create_circle","create_arc","delete_entity","move_entity",
                 "copy_entity","rotate_entity","mirror_entity","scale_entity","set_layer",
                 "query_entities","query_selection","check_drawing","create_drawing"];
        for (i, nm) in n.iter().enumerate() { assert_eq!(d[i]["function"]["name"], *nm); }
        assert_eq!(d[n.len()], Value::Null, "and nothing after them");
    }
    #[test]
    fn tool_definitions_types_are_function() {
        let d = tool_definitions(false);
        for i in 0..14 { assert_eq!(d[i]["type"], "function"); }
    }
    /// LCV-156 — every creation tool, the batch included, advertises an
    /// optional string `layer` (kills the `layer_schema` mutant, LCV-192).
    #[test]
    fn every_creation_tool_takes_an_optional_string_layer() {
        let d = tool_definitions(false);
        for i in [0, 1, 2, 13] {
            let layer = &d[i]["function"]["parameters"]["properties"]["layer"];
            assert_eq!(layer["type"], "string", "{}", d[i]["function"]["name"]);
            assert!(layer["description"].as_str().is_some_and(|t| t.contains("existing layer")));
            assert!(!req(&d, i).as_array().unwrap().contains(&json!("layer")));
        }
    }
    /// LCV-191 AC 1 — `set_layer` advertises the edit tools' `indices` list
    /// and a required `layer` string, nothing else.
    #[test]
    fn set_layer_takes_the_shared_indices_and_layer_schemas() {
        let d = tool_definitions(false);
        let props = &d[9]["function"]["parameters"]["properties"];
        assert_eq!(props["indices"], d[3]["function"]["parameters"]["properties"]["indices"]);
        assert_eq!(props["layer"]["type"], "string");
        let about = props["layer"]["description"].as_str().unwrap();
        assert!(about.contains("existing layer") && !about.contains("Optional"), "{about}");
        assert_eq!(props.as_object().unwrap().len(), 2);
        let text = d[9]["function"]["description"].as_str().unwrap();
        assert!(text.contains("existing layer"), "{text}");
    }
    #[test]
    fn tool_definitions_required_fields() {
        let d = tool_definitions(false);
        assert_eq!(req(&d,0), json!(["x1","y1","x2","y2"]));
        assert_eq!(req(&d,1), json!(["cx","cy","r"]));
        assert_eq!(req(&d,2), json!(["cx","cy","r","start_deg","end_deg","ccw"]));
        // LCV-186: `index` or `indices`, so neither is required.
        assert_eq!(req(&d,3), json!([]));
        assert_eq!(req(&d,4), json!(["dx","dy"]));
        assert_eq!(req(&d,5), json!(["dx","dy"]));
        assert_eq!(req(&d,6), json!(["x","y","degrees"]));
        assert_eq!(req(&d,7), json!(["x1","y1","x2","y2","erase_source"]));
        assert_eq!(req(&d,8), json!(["x","y","factor"]));
        // LCV-191: `set_layer` has no `index` form; both fields are required.
        assert_eq!(req(&d,9), json!(["indices","layer"]));
    }
    /// AC 13 — both queries, and `check_drawing` (LCV-190 AC 9), declare an
    /// **empty** object, not a missing one: `properties` is `{}` and
    /// `required` is `[]`, both present.
    #[test]
    fn the_argument_free_schemas_take_no_parameters() {
        let d = tool_definitions(false);
        for i in [10, 11, 12] {
            let params = &d[i]["function"]["parameters"];
            assert_eq!(params["type"], "object", "schema {i}");
            assert_eq!(params["properties"], json!({}), "schema {i}");
            assert_eq!(params["required"], json!([]), "schema {i}");
            assert!(params["properties"].is_object(), "schema {i} must be {{}}, not null");
            assert!(params["required"].is_array(), "schema {i} must be [], not null");
        }
    }
    /// AC 14, AC 15 — both queries, and `check_drawing` (LCV-190 AC 9), parse
    /// with **any** arguments object, since there is nothing in it to read:
    /// `{}`, a stray field, or JSON null.
    #[test]
    fn the_argument_free_tools_parse_with_any_arguments() {
        for (nm, expected) in [("query_entities", AgentAction::QueryEntities),
                               ("query_selection", AgentAction::QuerySelection),
                               ("check_drawing", AgentAction::CheckDrawing)] {
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
    /// vision flag is on, right before `create_drawing` so that stays last;
    /// the rest is unchanged. LCV-187 — its parameters are the optional
    /// `frame` enum and four corner numbers, nothing required.
    #[test]
    fn capture_canvas_is_advertised_only_with_vision() {
        let names = |d: &Value| -> Vec<String> { d.as_array().unwrap().iter()
            .map(|t| t["function"]["name"].as_str().unwrap().to_owned()).collect() };
        let off = tool_definitions(false);
        assert!(!names(&off).contains(&"capture_canvas".to_owned()));
        let on = tool_definitions(true);
        let mut expected = names(&off);
        expected.insert(13, "capture_canvas".to_owned());
        assert_eq!(names(&on), expected);
        let params = &on[13]["function"]["parameters"];
        let mm = json!({"type":"number"});
        assert_eq!(*params, json!({"type":"object","properties":{
            "frame":{"type":"string","enum":["view","drawing","region"]},
            "x0":mm,"y0":mm,"x1":mm,"y1":mm},"required":[]}));
        assert_eq!(on[14]["function"]["name"], "create_drawing");
        for (i, tool) in off.as_array().unwrap().iter().enumerate() {
            let j = if i < 13 { i } else { i + 1 };
            assert_eq!(on[j], *tool, "tool {i} unchanged");
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
        assert_eq!(e.to_string(), "create_drawing version: unsupported value; expected the integer 1");
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
        assert_eq!(at(&e), ("create_circle", "r", "-1 is out of range"));
    }
    /// AC 5 — a zero radius is rejected by the same check as a negative one.
    #[test]
    fn parse_create_circle_zero_radius() {
        let e = err("create_circle", json!({"cx":0.0,"cy":0.0,"r":0.0}));
        assert!(at(&e).1 == "r", "{e:?}");
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
        assert!((at(&e).0, at(&e).1) == ("create_arc", "r"), "{e:?}");
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
        assert!(at(&e) == ("copy_entity", "dy", "missing"), "{e:?}");
        let e = err("copy_entity", json!({"index":-1,"dx":1.0,"dy":1.0}));
        assert!((at(&e).0, at(&e).1) == ("copy_entity", "index"), "{e:?}");
    }
    /// LCV-158 AC9 — `rotate_entity` takes degrees and parses to radians; the
    /// range check is the apply site's.
    #[test]
    fn parse_rotate_entity() {
        assert_eq!(ok("rotate_entity", json!({"index":2,"x":1.0,"y":-3.0,"degrees":90.0})),
            AgentAction::Rotate { index: 2, x: 1.0, y: -3.0, angle: FRAC_PI_2 });
        let e = err("rotate_entity", json!({"index":0,"x":1.0,"y":1.0}));
        assert!(at(&e) == ("rotate_entity", "degrees", "missing"), "{e:?}");
        let e = err("rotate_entity", json!({"index":-1,"x":0.0,"y":0.0,"degrees":1.0}));
        assert!((at(&e).0, at(&e).1) == ("rotate_entity", "index"), "{e:?}");
    }
    /// LCV-181 AC10 — `mirror_entity` takes two line points and
    /// `erase_source`; the range and distinct-points checks are the apply site's.
    #[test]
    fn parse_mirror_entity() {
        let args = json!({"index":1,"x1":0.0,"y1":-1.0,"x2":2.0,"y2":3.0,"erase_source":true});
        assert_eq!(ok("mirror_entity", args),
            AgentAction::Mirror { index: 1, x1: 0.0, y1: -1.0, x2: 2.0, y2: 3.0, erase_source: true });
        let e = err("mirror_entity", json!({"index":0,"x1":0.0,"y1":0.0,"x2":1.0,"y2":1.0}));
        assert!(at(&e) == ("mirror_entity", "erase_source", "missing"), "{e:?}");
        let e = err("mirror_entity",
            json!({"index":0,"x1":0.0,"y1":0.0,"x2":1.0,"y2":1.0,"erase_source":"no"}));
        assert!(at(&e) == ("mirror_entity", "erase_source", "not a boolean"), "{e:?}");
        let e = err("mirror_entity",
            json!({"index":-1,"x1":0.0,"y1":0.0,"x2":1.0,"y2":1.0,"erase_source":false}));
        assert!((at(&e).0, at(&e).1) == ("mirror_entity", "index"), "{e:?}");
    }
    /// LCV-182 AC8 — `scale_entity` takes a base point and a factor; a zero,
    /// negative or non-finite factor is refused at parse time, like `r`.
    #[test]
    fn parse_scale_entity() {
        assert_eq!(ok("scale_entity", json!({"index":1,"x":2.0,"y":-3.0,"factor":0.5})),
            AgentAction::Scale { index: 1, x: 2.0, y: -3.0, factor: 0.5 });
        let e = err("scale_entity", json!({"index":0,"x":0.0,"y":0.0}));
        assert!(at(&e) == ("scale_entity", "factor", "missing"), "{e:?}");
        for factor in [0.0, -2.0] {
            let e = err("scale_entity", json!({"index":0,"x":0.0,"y":0.0,"factor":factor}));
            assert!((at(&e).0, at(&e).1) == ("scale_entity", "factor"), "{e:?}");
        }
        let e = err("scale_entity", json!({"index":-1,"x":0.0,"y":0.0,"factor":2.0}));
        assert!((at(&e).0, at(&e).1) == ("scale_entity", "index"), "{e:?}");
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
        assert!(at(&e) == ("create_line", "y1", "missing"), "{e:?}");
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
        assert!((at(&e).1, at(&e).2) == ("x1", "missing"), "{e:?}");
        let e = err("create_circle", json!({"cx":0.0,"cy":0.0,"r":f64::INFINITY}));
        assert!((at(&e).1, at(&e).2) == ("r", "missing"), "{e:?}");
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
    /// AC 5 — a negative index and a fractional index are both shape errors
    /// (LCV-192 wording).
    #[test]
    fn parse_index_must_be_a_non_negative_integer() {
        for (raw, text) in [(json!(-1), "-1 is not an index"), (json!(1.5), "1.5 is not an index")] {
            let e = err("delete_entity", json!({"index": raw}));
            assert_eq!(at(&e), ("delete_entity", "index", text));
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
    fn tool_call_error_display() {
        assert_eq!(ToolCallError::UnknownTool("x".into()).to_string(), "unknown tool: `x`");
        assert_eq!(ToolCallError::arg("t", "f", "bad").to_string(),
            "t f: bad; expected a value the tool's schema allows");
        assert_eq!(refusal("t", "p", "why", "form"), "t p: why; expected form");
    }
    /// LCV-192 AC 1 — every row of the expected-form table, so a row cannot
    /// drift or fall through to the default unnoticed.
    #[test]
    fn expected_form_names_each_field() {
        for f in ["x1","y1","x2","y2","cx","cy","dx","dy","x","y","x0","y0"] {
            assert_eq!(expected_form(f), "a number in mm", "{f}");
        }
        for f in ["start_deg","end_deg","degrees"] { assert_eq!(expected_form(f), "a number in degrees"); }
        for f in ["ccw","erase_source"] { assert_eq!(expected_form(f), "true or false"); }
        for (f, form) in [("r", "a positive number in mm"), ("factor", "a positive number"),
            ("index", "a non-negative integer (an index from query_entities)"),
            ("indices", "a list of 1 to 1000 distinct entity indices"),
            ("layer", "the name of an existing layer, 1 to 64 characters"),
            ("frame", r#""view", "drawing" or "region""#), ("version", "the integer 1"),
            ("entities", "a list of 1 to 1000 entity objects"),
            ("type", r#""line", "circle" or "arc""#), ("(root)", "a JSON object"),
            ("bogus", "a value the tool's schema allows")] {
            assert_eq!(expected_form(f), form, "{f}");
        }
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
        // LCV-192: the getters and validators moved to `tools/args.rs`.
        let args = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/agent/tools/args.rs"));
        let implementation = &format!("{}{args}", &src[..at]);

        assert!(implementation.contains(concat!("pub fn parse_", "tool_call")),
            "positive control: the parser must be declared in this file");
        assert!(implementation.contains(concat!("fn validate", "_r")),
            "positive control: the radius check must still be in tools.rs or tools/args.rs");

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

    // ── LCV-192 AC 1: every argument refusal has one shape ───────────────────

    /// LCV-192 AC 1 — `<tool> <field>: <reason>; expected <form>`, pinned
    /// exactly: absent, wrong type and out of domain are distinct reasons.
    #[test]
    fn scalar_refusals_name_tool_field_reason_and_form() {
        let mm_r = "expected a positive number in mm";
        let index = "expected a non-negative integer (an index from query_entities)";
        let layer = "expected the name of an existing layer, 1 to 64 characters";
        let cases = [
            ("create_circle", json!({"cx":0,"cy":0}), format!("create_circle r: missing; {mm_r}")),
            ("create_circle", json!({"cx":0,"cy":0,"r":"5"}),
                format!("create_circle r: not a number; {mm_r}")),
            ("create_circle", json!({"cx":0,"cy":0,"r":-3}),
                format!("create_circle r: -3 is out of range; {mm_r}")),
            ("scale_entity", json!({"index":0,"x":0,"y":0,"factor":0}),
                "scale_entity factor: 0 is out of range; expected a positive number".to_owned()),
            ("delete_entity", json!({"index":-1}), format!("delete_entity index: -1 is not an index; {index}")),
            ("delete_entity", json!({"index":1.5}), format!("delete_entity index: 1.5 is not an index; {index}")),
            ("delete_entity", json!({"index":"2"}), format!("delete_entity index: not a number; {index}")),
            ("create_line", json!({"x1":0,"y1":0,"x2":1,"y2":1,"layer":"x".repeat(65)}),
                format!("create_line layer: has 65 characters; {layer}")),
            ("create_line", json!({"x1":0,"y1":0,"x2":1,"y2":1,"layer":5}),
                format!("create_line layer: not a string; {layer}")),
            ("create_line", json!({"x1":0,"y1":0,"y2":1}),
                "create_line x2: missing; expected a number in mm".to_owned()),
            ("create_arc", json!({"cx":0,"cy":0,"r":1,"start_deg":0,"end_deg":9,"ccw":"yes"}),
                "create_arc ccw: not a boolean; expected true or false".to_owned()),
            ("rotate_entity", json!({"index":0,"x":0,"y":0}),
                "rotate_entity degrees: missing; expected a number in degrees".to_owned()),
        ];
        for (tool, args, want) in cases {
            assert_eq!(err(tool, args.clone()).to_string(), want, "{tool} {args}");
        }
    }
