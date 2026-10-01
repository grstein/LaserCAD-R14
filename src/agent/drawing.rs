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

mod schema;
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

/// Each entity `type` with the keys it takes besides `type`, in reading
/// order; the one source of the item schema and of the item key check.
pub const ENTITY_KEYS: [(&str, &[&str]); 3] = [
    ("line", &["x1", "y1", "x2", "y2"]),
    ("circle", &["cx", "cy", "r"]),
    ("arc", &["cx", "cy", "r", "start_deg", "end_deg", "ccw"]),
];

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
    let Some(&(kind_name, keys)) = ENTITY_KEYS.iter().find(|(t, _)| Some(*t) == kind.as_str())
    else {
        return Err(fail("type", "unknown type"));
    };
    let a = if kind_name == "arc" { "an" } else { "a" };
    let own_keys = format!("{a} {kind_name} key ({})", keys.join(", "));
    // A key of another type is tolerated only as `null` (ADR 0010 §2).
    let own = |k: &str| k == "type" || keys.contains(&k);
    for (key, value) in obj.iter().filter(|(k, _)| !own(k)) {
        if !ENTITY_KEYS.iter().any(|(_, k)| k.contains(&key.as_str())) {
            return Err(arg(at(&cut(key)), "unknown key", &own_keys));
        } else if !value.is_null() {
            let reason = format!("not {a} {kind_name} key");
            return Err(arg(at(key), &reason, &format!("null or {own_keys}")));
        }
    }
    if let Some(key) = keys.iter().find(|k| !obj.contains_key(**k)) {
        return Err(fail(key, "missing"));
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
mod tests {
    use super::*;
    use crate::agent::bridge::AgentAction;
    use crate::agent::tools::parse_tool_call;
    use core::f64::consts::{FRAC_PI_2, PI};
    use serde_json::json;

    fn batch(entities: Value) -> Value {
        json!({"version": 1, "entities": entities})
    }
    fn err(args: Value) -> String {
        parse(&args).unwrap_err().to_string()
    }
    fn circle(i: usize) -> Value {
        json!({"type": "circle", "cx": i as f64, "cy": 0.0, "r": 1.0})
    }
    fn circles(n: usize) -> Value {
        Value::Array((0..n).map(circle).collect())
    }

    // ── AC 2: limits and root shape ──────────────────────────────────────────

    #[test]
    fn one_and_a_thousand_entities_are_accepted() {
        assert_eq!(parse(&batch(circles(1))).unwrap().len(), 1);
        let items = parse(&batch(circles(1000))).unwrap();
        assert_eq!(items.len(), 1000);
        assert_eq!(
            items[999],
            DrawingItem::Circle {
                cx: 999.0,
                cy: 0.0,
                r: 1.0
            }
        );
    }

    #[test]
    fn zero_and_a_thousand_and_one_entities_are_refused() {
        assert_eq!(
            err(batch(circles(0))),
            "create_drawing entities: has 0 items; expected a list of 1 to 1000 entity objects"
        );
        assert_eq!(
            err(batch(circles(1001))),
            "create_drawing entities: has 1001 items; expected a list of 1 to 1000 entity objects"
        );
    }

    #[test]
    fn the_root_key_set_is_exactly_version_and_entities() {
        assert_eq!(
            err(json!({"entities": circles(1)})),
            "create_drawing version: missing; expected the integer 1"
        );
        assert_eq!(
            err(json!({"version": 1})),
            "create_drawing entities: missing; expected a list of 1 to 1000 entity objects"
        );
        assert_eq!(
            err(json!({"version": 1, "entities": circles(1), "extra": 0})),
            "create_drawing extra: unknown key; expected version, entities or layer"
        );
        assert_eq!(
            err(json!([1, 2])),
            "create_drawing (root): not an object; expected a JSON object"
        );
        assert_eq!(
            err(json!({"version": 1, "entities": {}})),
            "create_drawing entities: not a list; expected a list of 1 to 1000 entity objects"
        );
    }

    #[test]
    fn version_must_be_the_integer_one() {
        for bad in [json!(2), json!("1"), json!(1.0), json!(0), json!(null)] {
            assert_eq!(
                err(json!({"version": bad, "entities": circles(1)})),
                "create_drawing version: unsupported value; expected the integer 1",
                "version {bad}"
            );
        }
    }

    // ── AC 3: per-entity validation and error reporting ──────────────────────

    #[test]
    fn an_unknown_key_is_named_and_cut_to_64_characters() {
        let mut item = circle(0);
        item["bogus"] = json!(1);
        assert_eq!(
            err(batch(json!([item]))),
            "create_drawing entities[0].bogus: unknown key; expected a circle key (cx, cy, r)"
        );
        let long = "k".repeat(200);
        let mut item = circle(0);
        item[long.as_str()] = json!(1);
        let e = err(batch(json!([item])));
        assert_eq!(
            e,
            format!(
                "create_drawing entities[0].{}: unknown key; expected a circle key (cx, cy, r)",
                "k".repeat(64)
            )
        );
        assert!(!e.contains(&"k".repeat(65)));
    }

    #[test]
    fn a_missing_key_a_wrong_type_and_a_non_boolean_ccw_are_refused() {
        assert_eq!(
            err(batch(json!([{"type": "circle", "cx": 0, "cy": 0}]))),
            "create_drawing entities[0].r: missing; expected a positive number in mm"
        );
        assert_eq!(
            err(batch(
                json!([{"type": "line", "x1": 0, "y1": "0", "x2": 1, "y2": 1}])
            )),
            "create_drawing entities[0].y1: not a number; expected a number in mm"
        );
        assert_eq!(
            err(batch(json!([{"type": "arc", "cx": 0, "cy": 0, "r": 1,
                "start_deg": 0, "end_deg": 90, "ccw": "true"}]))),
            "create_drawing entities[0].ccw: not a boolean; expected true or false"
        );
        assert_eq!(
            err(batch(json!([{"cx": 0, "cy": 0, "r": 1}]))),
            r#"create_drawing entities[0].type: missing; expected "line", "circle" or "arc""#
        );
        assert_eq!(
            err(batch(json!([{"type": "polyline"}]))),
            r#"create_drawing entities[0].type: unknown type; expected "line", "circle" or "arc""#
        );
        assert_eq!(
            err(batch(json!([7]))),
            "create_drawing entities[0]: not an object; expected an object with a type and that type's keys"
        );
    }

    #[test]
    fn a_zero_or_negative_radius_is_refused_with_the_scalar_wording() {
        assert_eq!(
            err(batch(json!([{"type": "circle", "cx": 0, "cy": 0, "r": 0}]))),
            "create_drawing entities[0].r: 0 is out of range; expected a positive number in mm"
        );
        assert_eq!(
            err(batch(json!([{"type": "arc", "cx": 0, "cy": 0, "r": -3,
                "start_deg": 0, "end_deg": 90, "ccw": true}]))),
            "create_drawing entities[0].r: -3 is out of range; expected a positive number in mm"
        );
    }

    #[test]
    fn a_bad_first_middle_or_last_item_reports_its_index() {
        for bad in [0usize, 17, 99] {
            let mut items: Vec<Value> = (0..100).map(circle).collect();
            items[bad]["r"] = json!(-3);
            assert_eq!(
                err(batch(Value::Array(items))),
                format!(
                    "create_drawing entities[{bad}].r: -3 is out of range; expected a positive number in mm"
                )
            );
        }
    }

    #[test]
    fn the_error_never_echoes_the_payload() {
        let e = err(batch(json!([
            {"type": "circle", "cx": 987654.321, "cy": 0, "r": 1},
            {"type": "SENTINEL_TYPE", "cx": 0, "cy": 0, "r": 1}
        ])));
        assert!(!e.contains("987654") && !e.contains("SENTINEL"), "{e}");
        let e = err(batch(json!([
            {"type": "line", "x1": "SENTINEL_VALUE", "y1": 0, "x2": 1, "y2": 1}
        ])));
        assert!(!e.contains("SENTINEL"), "{e}");
    }

    #[test]
    fn an_entity_far_off_the_bed_is_accepted() {
        let items = parse(&batch(json!([
            {"type": "line", "x1": -10000, "y1": -10000, "x2": -10000, "y2": -10000}
        ])))
        .unwrap();
        assert_eq!(
            items,
            vec![DrawingItem::Line {
                x1: -10000.0,
                y1: -10000.0,
                x2: -10000.0,
                y2: -10000.0
            }]
        );
    }

    // ── AC 4: units, and a batch of one is the scalar call ───────────────────

    #[test]
    fn degrees_become_radians_for_both_directions() {
        for ccw in [true, false] {
            let items = parse(&batch(json!([{"type": "arc", "cx": 1, "cy": 2, "r": 3,
                "start_deg": 90, "end_deg": 180, "ccw": ccw}])))
            .unwrap();
            let DrawingItem::Arc {
                cx,
                cy,
                r,
                start,
                end,
                ccw: got,
            } = items[0]
            else {
                panic!("{items:?}")
            };
            assert_eq!((cx, cy, r, got), (1.0, 2.0, 3.0, ccw));
            assert!((start - FRAC_PI_2).abs() < 1e-12, "start {start}");
            assert!((end - PI).abs() < 1e-12, "end {end}");
        }
    }

    /// The scalar action a one-item batch must reproduce, field for field.
    fn as_action(item: &DrawingItem) -> AgentAction {
        match *item {
            DrawingItem::Line { x1, y1, x2, y2 } => AgentAction::CreateLine {
                x1,
                y1,
                x2,
                y2,
                layer: None,
            },
            DrawingItem::Circle { cx, cy, r } => AgentAction::CreateCircle {
                cx,
                cy,
                r,
                layer: None,
            },
            DrawingItem::Arc {
                cx,
                cy,
                r,
                start,
                end,
                ccw,
            } => AgentAction::CreateArc {
                layer: None,
                cx,
                cy,
                r,
                start,
                end,
                ccw,
            },
        }
    }

    #[test]
    fn a_batch_of_one_equals_the_scalar_tool() {
        let cases = [
            (
                "create_line",
                json!({"x1": 1.5, "y1": 2, "x2": -3, "y2": 4.25}),
            ),
            ("create_circle", json!({"cx": 5, "cy": 6.5, "r": 0.75})),
            (
                "create_arc",
                json!({"cx": 7, "cy": 8, "r": 9,
                "start_deg": 33.3, "end_deg": -45, "ccw": false}),
            ),
        ];
        for (tool, args) in cases {
            let mut item = args.clone();
            item["type"] = json!(tool.trim_start_matches("create_"));
            let items = parse(&batch(json!([item]))).unwrap();
            assert_eq!(
                as_action(&items[0]),
                parse_tool_call(tool, &args).unwrap(),
                "{tool}"
            );
        }
    }

    /// Breaks if the radius helper is copied and drifts: the batch and
    /// `create_circle` must accept and reject exactly the same values.
    #[test]
    fn the_radius_rule_is_the_scalar_rule() {
        for r in [
            json!(-3),
            json!(-0.0),
            json!(0),
            json!(1e-300),
            json!(0.5),
            json!(5),
            json!(1e300),
            json!("5"),
            json!(null),
        ] {
            let scalar = parse_tool_call("create_circle", &json!({"cx": 0, "cy": 0, "r": r}));
            let batched = parse(&batch(
                json!([{"type": "circle", "cx": 0, "cy": 0, "r": r}]),
            ));
            assert_eq!(scalar.is_ok(), batched.is_ok(), "r = {r}");
        }
    }
}
