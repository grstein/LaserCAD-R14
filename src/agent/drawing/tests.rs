//! Unit tests of the `create_drawing` parser (LCV-144, LCV-185, LCV-192,
//! LCV-196).

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
        format!("create_drawing entities[0].type: missing; expected {TYPES}")
    );
    assert_eq!(
        err(batch(json!([{"type": "spline"}]))),
        format!("create_drawing entities[0].type: unknown type; expected {TYPES}")
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

// ── LCV-196: shapes ──────────────────────────────────────────────────────────

/// The expected form of `type`: every type, quoted, in table order.
const TYPES: &str = r#""line", "circle", "arc", "polyline", "rect", "polygon", "text", "linear_array" or "polar_array""#;

fn items(entities: Value) -> Vec<DrawingItem> {
    parse(&batch(entities)).unwrap_or_else(|e| panic!("{e}"))
}
fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> DrawingItem {
    DrawingItem::Line { x1, y1, x2, y2 }
}
fn arc(cx: f64, cy: f64, r: f64, start: f64, end: f64) -> DrawingItem {
    DrawingItem::Arc {
        cx,
        cy,
        r,
        start,
        end,
        ccw: true,
    }
}
/// Every field of every item within 1e-9.
fn assert_close(got: &[DrawingItem], want: &[DrawingItem]) {
    assert_eq!(got.len(), want.len(), "{got:?}");
    let fields = |i: &DrawingItem| match *i {
        DrawingItem::Line { x1, y1, x2, y2 } => (0, vec![x1, y1, x2, y2]),
        DrawingItem::Circle { cx, cy, r } => (1, vec![cx, cy, r]),
        DrawingItem::Arc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
        } => (2 + usize::from(ccw), vec![cx, cy, r, start, end]),
    };
    for (k, (g, w)) in got.iter().zip(want).enumerate() {
        let ((gk, gv), (wk, wv)) = (fields(g), fields(w));
        let near = gv.iter().zip(&wv).all(|(a, b)| (a - b).abs() < 1e-9);
        assert!(gk == wk && near, "item {k}: {g:?} != {w:?}");
    }
}
fn pts(points: &[(f64, f64)]) -> Value {
    Value::Array(
        points
            .iter()
            .map(|&(x, y)| json!({"x": x, "y": y}))
            .collect(),
    )
}
fn polyline(points: &[(f64, f64)], closed: bool) -> Value {
    json!({"type": "polyline", "points": pts(points), "closed": closed})
}
fn rect(corner_radius: Value) -> Value {
    json!({"type": "rect", "x": 1, "y": 2, "width": 10, "height": 4,
        "corner_radius": corner_radius})
}

#[test]
fn lcv196_ac1_a_polyline_is_one_line_per_segment_open_or_closed() {
    let p = [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)];
    let open = vec![line(0.0, 0.0, 4.0, 0.0), line(4.0, 0.0, 4.0, 3.0)];
    assert_eq!(items(json!([polyline(&p, false)])), open);
    let mut closed = open;
    closed.push(line(4.0, 3.0, 0.0, 0.0));
    assert_eq!(items(json!([polyline(&p, true)])), closed);
}

#[test]
fn lcv196_ac1_two_and_a_thousand_points_bound_a_polyline() {
    let row = |n: usize| -> Vec<(f64, f64)> { (0..n).map(|i| (i as f64, 0.0)).collect() };
    assert_eq!(items(json!([polyline(&row(2), false)])).len(), 1);
    assert_eq!(items(json!([polyline(&row(1000), false)])).len(), 999);
    let mut ring = row(999);
    ring.push((0.0, 5.0));
    assert_eq!(items(json!([polyline(&ring, true)])).len(), 1000);
    let form = "expected a list of 2 to 1000 points {x, y} in mm";
    for n in [1, 1001] {
        assert_eq!(
            err(batch(json!([polyline(&row(n), false)]))),
            format!("create_drawing entities[0].points: has {n} items; {form}")
        );
    }
}

#[test]
fn lcv196_ac2_a_rect_without_a_radius_is_four_lines_ccw() {
    let want = vec![
        line(1.0, 2.0, 11.0, 2.0),
        line(11.0, 2.0, 11.0, 6.0),
        line(11.0, 6.0, 1.0, 6.0),
        line(1.0, 6.0, 1.0, 2.0),
    ];
    for radius in [json!(0), Value::Null] {
        assert_eq!(items(json!([rect(radius)])), want);
    }
    let mut bare = rect(Value::Null);
    bare.as_object_mut().unwrap().remove("corner_radius");
    assert_eq!(items(json!([bare])), want);
}

#[test]
fn lcv196_ac2_a_rounded_rect_is_four_sides_and_four_ccw_quarter_arcs() {
    assert_close(
        &items(json!([rect(json!(1))])),
        &[
            line(2.0, 2.0, 10.0, 2.0),
            arc(10.0, 3.0, 1.0, -FRAC_PI_2, 0.0),
            line(11.0, 3.0, 11.0, 5.0),
            arc(10.0, 5.0, 1.0, 0.0, FRAC_PI_2),
            line(10.0, 6.0, 2.0, 6.0),
            arc(2.0, 5.0, 1.0, FRAC_PI_2, PI),
            line(1.0, 5.0, 1.0, 3.0),
            arc(2.0, 3.0, 1.0, PI, 1.5 * PI),
        ],
    );
}

#[test]
fn lcv196_ac2_a_radius_of_half_the_short_side_omits_the_zero_length_sides() {
    assert_close(
        &items(json!([rect(json!(2))])),
        &[
            line(3.0, 2.0, 9.0, 2.0),
            arc(9.0, 4.0, 2.0, -FRAC_PI_2, 0.0),
            arc(9.0, 4.0, 2.0, 0.0, FRAC_PI_2),
            line(9.0, 6.0, 3.0, 6.0),
            arc(3.0, 4.0, 2.0, FRAC_PI_2, PI),
            arc(3.0, 4.0, 2.0, PI, 1.5 * PI),
        ],
    );
}

#[test]
fn lcv196_ac3_a_hexagon_starts_at_start_deg() {
    let got = items(json!([{"type": "polygon", "cx": 1, "cy": 2, "r": 10,
        "sides": 6, "start_deg": 30}]));
    let v = |k: usize| {
        let a = (30.0 + 60.0 * k as f64).to_radians();
        (1.0 + 10.0 * a.cos(), 2.0 + 10.0 * a.sin())
    };
    let want: Vec<DrawingItem> = (0..6)
        .map(|k| {
            let ((x1, y1), (x2, y2)) = (v(k), v(k + 1));
            line(x1, y1, x2, y2)
        })
        .collect();
    assert_close(&got, &want);
}

#[test]
fn lcv196_ac3_three_and_sixty_four_sides_bound_a_polygon() {
    let polygon =
        |sides: Value| json!({"type": "polygon", "cx": 0, "cy": 0, "r": 1, "sides": sides});
    assert_eq!(items(json!([polygon(json!(3))])).len(), 3);
    assert_eq!(items(json!([polygon(json!(64))])).len(), 64);
    for n in [2, 65] {
        assert_eq!(
            err(batch(json!([polygon(json!(n))]))),
            format!(
                "create_drawing entities[0].sides: {n} is out of range; expected an integer from 3 to 64"
            )
        );
    }
    assert_eq!(
        err(batch(json!([polygon(json!(6.5))]))),
        "create_drawing entities[0].sides: not an integer; expected an integer from 3 to 64"
    );
}

#[test]
fn lcv196_ac4_text_is_the_text_command_layout() {
    let got = items(json!([{"type": "text", "x": 3, "y": 4, "height": 5, "text": "AB"}]));
    let want: Vec<DrawingItem> = crate::text::layout_text(
        "AB",
        crate::geometry::Vec2::new(3.0, 4.0),
        5.0,
        crate::text::layout::DEFAULT_SPACING_FACTOR,
    )
    .into_iter()
    .map(|e| match e {
        crate::document::Entity::Line(l) => line(l.p1.x, l.p1.y, l.p2.x, l.p2.y),
        other => panic!("{other:?}"),
    })
    .collect();
    assert!(want.len() > 4, "control: {want:?}");
    assert_eq!(got, want);
}

#[test]
fn lcv196_ac8_a_repeated_point_is_refused_cyclically() {
    let point = "expected a point {x, y} in mm";
    assert_eq!(
        err(batch(json!([polyline(
            &[(0.0, 0.0), (1.0, 0.0), (1.0, 0.0)],
            false
        )]))),
        format!("create_drawing entities[0].points[2]: repeats the previous point; {point}")
    );
    assert_eq!(
        err(batch(json!([polyline(&[(0.0, 0.0), (1.0, 0.0)], true)]))),
        "create_drawing entities[0].closed: true with 2 points; \
         expected false, or true with 3 or more points"
    );
    let square = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 0.0)];
    assert_eq!(
        err(batch(json!([polyline(&square, true)]))),
        format!("create_drawing entities[0].points[3]: repeats the first point; {point}")
    );
    assert_eq!(items(json!([polyline(&square, false)])).len(), 3);
}

#[test]
fn lcv196_ac8_a_bad_point_names_its_path() {
    let mut p = polyline(&[(0.0, 0.0), (1.0, 0.0)], false);
    p["points"][1]["y"] = json!("0");
    assert_eq!(
        err(batch(json!([p]))),
        "create_drawing entities[0].points[1].y: not a number; expected a number in mm"
    );
    p["points"][1] = json!(7);
    assert_eq!(
        err(batch(json!([p]))),
        "create_drawing entities[0].points[1]: not an object; expected a point {x, y} in mm"
    );
    p["points"] = json!({});
    assert_eq!(
        err(batch(json!([p]))),
        "create_drawing entities[0].points: not a list; \
         expected a list of 2 to 1000 points {x, y} in mm"
    );
}

#[test]
fn lcv196_ac8_an_oversize_or_negative_corner_radius_is_refused() {
    for r in ["2.5", "-1"] {
        assert_eq!(
            err(batch(json!([rect(serde_json::from_str(r).unwrap())]))),
            format!(
                "create_drawing entities[0].corner_radius: {r} is out of range; \
                 expected a number in mm from 0 to half the shorter side"
            )
        );
    }
    let mut flat = rect(Value::Null);
    flat["height"] = json!(0);
    assert_eq!(
        err(batch(json!([flat]))),
        "create_drawing entities[0].height: 0 is out of range; expected a positive number in mm"
    );
}

#[test]
fn lcv196_ac8_a_bad_text_string_is_refused() {
    let text = |t: Value| json!({"type": "text", "x": 0, "y": 0, "height": 5, "text": t});
    let form = "expected a string of 1 to 256 characters without control characters";
    for (t, reason) in [
        (json!(""), "is empty".to_owned()),
        (json!("I".repeat(257)), "has 257 characters".to_owned()),
        (json!("A\nB"), "contains a control character".to_owned()),
        (json!(7), "not a string".to_owned()),
    ] {
        assert_eq!(
            err(batch(json!([text(t)]))),
            format!("create_drawing entities[0].text: {reason}; {form}")
        );
    }
    assert!(!items(json!([text(json!("I".repeat(256)))])).is_empty());
}

/// LCV-185 holds for the new types; the optional flag is per type: polygon
/// may omit `start_deg`, an arc may not.
#[test]
fn lcv196_ac8_null_foreign_keys_and_per_type_optional_keys() {
    let mut p = polyline(&[(0.0, 0.0), (1.0, 0.0)], false);
    p["r"] = Value::Null;
    p["of"] = Value::Null;
    assert_eq!(items(json!([p])).len(), 1);
    let mut r = rect(json!(0));
    r["points"] = Value::Null;
    r["text"] = Value::Null;
    assert_eq!(items(json!([r])).len(), 4);
    r["r"] = json!(1);
    assert_eq!(
        err(batch(json!([r]))),
        "create_drawing entities[0].r: not a rect key; \
         expected null or a rect key (x, y, width, height, corner_radius)"
    );
    let hex = json!({"type": "polygon", "cx": 0, "cy": 0, "r": 1, "sides": 4});
    let mut null_start = hex.clone();
    null_start["start_deg"] = Value::Null;
    assert_eq!(items(json!([hex])), items(json!([null_start])));
    assert_close(&items(json!([hex]))[..1], &[line(1.0, 0.0, 0.0, 1.0)]);
    assert_eq!(
        err(batch(
            json!([{"type": "arc", "cx": 0, "cy": 0, "r": 1, "end_deg": 9, "ccw": true}])
        )),
        "create_drawing entities[0].start_deg: missing; expected a number in degrees"
    );
}

// ── LCV-196: arrays ──────────────────────────────────────────────────────────

fn circle_at(cx: f64, cy: f64) -> DrawingItem {
    DrawingItem::Circle { cx, cy, r: 1.0 }
}
fn linear(of: Value, count: Value, dx: f64, dy: f64) -> Value {
    json!({"type": "linear_array", "of": of, "count": count, "dx": dx, "dy": dy})
}
const OF_FORM: &str = "expected the index of an earlier item, once; not an array of arrays";

#[test]
fn lcv196_ac5_a_linear_array_copies_its_items_in_batch_order() {
    let got = items(json!([
        {"type": "line", "x1": 0, "y1": 0, "x2": 1, "y2": 0},
        {"type": "circle", "cx": 5, "cy": 0, "r": 1},
        linear(json!([0, 1]), json!(3), 10.0, 1.0)
    ]));
    assert_eq!(
        got,
        vec![
            line(0.0, 0.0, 1.0, 0.0),
            circle_at(5.0, 0.0),
            line(10.0, 1.0, 11.0, 1.0),
            circle_at(15.0, 1.0),
            line(20.0, 2.0, 21.0, 2.0),
            circle_at(25.0, 2.0),
        ]
    );
}

#[test]
fn lcv196_ac6_a_polar_array_rotates_endpoints_and_arc_angles() {
    let got = items(json!([
        {"type": "line", "x1": 1, "y1": 0, "x2": 2, "y2": 0},
        {"type": "arc", "cx": 3, "cy": 0, "r": 1, "start_deg": 0, "end_deg": 90, "ccw": true},
        {"type": "polar_array", "of": [0, 1], "count": 3, "cx": 0, "cy": 0, "step_deg": 90}
    ]));
    assert_close(
        &got,
        &[
            line(1.0, 0.0, 2.0, 0.0),
            arc(3.0, 0.0, 1.0, 0.0, FRAC_PI_2),
            line(0.0, 1.0, 0.0, 2.0),
            arc(0.0, 3.0, 1.0, FRAC_PI_2, PI),
            line(-1.0, 0.0, -2.0, 0.0),
            arc(-3.0, 0.0, 1.0, PI, 1.5 * PI),
        ],
    );
}

#[test]
fn lcv196_ac7_an_array_of_an_array_is_a_grid() {
    let got = items(json!([
        {"type": "circle", "cx": 0, "cy": 0, "r": 1},
        linear(json!([0]), json!(3), 10.0, 0.0),
        linear(json!([1]), json!(2), 0.0, 10.0)
    ]));
    let want: Vec<DrawingItem> = [(0, 0), (10, 0), (20, 0), (0, 10), (10, 10), (20, 10)]
        .iter()
        .map(|&(x, y)| circle_at(f64::from(x), f64::from(y)))
        .collect();
    assert_eq!(got, want);
    // Listing a base item and the array that already lists it copies it once.
    let both = items(json!([
        {"type": "circle", "cx": 0, "cy": 0, "r": 1},
        linear(json!([0]), json!(2), 10.0, 0.0),
        linear(json!([1, 0]), json!(2), 0.0, 10.0)
    ]));
    assert_eq!(both.len(), 4);
}

#[test]
fn lcv196_ac8_of_must_name_distinct_earlier_items_two_levels_deep() {
    let c = json!({"type": "circle", "cx": 0, "cy": 0, "r": 1});
    let cases = [
        (
            json!([c, linear(json!([1]), json!(2), 1.0, 0.0)]),
            "entities[1].of[0]: names itself",
        ),
        (
            json!([c, linear(json!([0, 2]), json!(2), 1.0, 0.0), c]),
            "entities[1].of[1]: names a later item",
        ),
        (
            json!([c, linear(json!([0, 0]), json!(2), 1.0, 0.0)]),
            "entities[1].of[1]: repeats an earlier entry",
        ),
        (
            json!([c, linear(json!([0.5]), json!(2), 1.0, 0.0)]),
            "entities[1].of[0]: not an index",
        ),
        (
            json!([
                c,
                linear(json!([0]), json!(2), 1.0, 0.0),
                linear(json!([1]), json!(2), 0.0, 1.0),
                linear(json!([0, 2]), json!(2), 0.0, 1.0)
            ]),
            "entities[3].of[1]: names an array that lists an array",
        ),
    ];
    for (entities, want) in cases {
        assert_eq!(
            err(batch(entities)),
            format!("create_drawing {want}; {OF_FORM}")
        );
    }
    let list = "expected a list of distinct indices of earlier items";
    assert_eq!(
        err(batch(json!([c, linear(json!([]), json!(2), 1.0, 0.0)]))),
        format!("create_drawing entities[1].of: has 0 items; {list}")
    );
    assert_eq!(
        err(batch(json!([c, linear(json!(0), json!(2), 1.0, 0.0)]))),
        format!("create_drawing entities[1].of: not a list; {list}")
    );
}

#[test]
fn lcv196_ac8_count_is_two_to_a_thousand() {
    let c = json!({"type": "circle", "cx": 0, "cy": 0, "r": 1});
    assert_eq!(
        items(json!([c, linear(json!([0]), json!(2), 1.0, 0.0)])).len(),
        2
    );
    assert_eq!(
        items(json!([c, linear(json!([0]), json!(1000), 1.0, 0.0)])).len(),
        1000
    );
    for n in [1, 1001] {
        assert_eq!(
            err(batch(json!([c, linear(json!([0]), json!(n), 1.0, 0.0)]))),
            format!(
                "create_drawing entities[1].count: {n} is out of range; \
                 expected an integer from 2 to 1000"
            )
        );
    }
}

#[test]
fn lcv196_ac8_the_cap_counts_the_expansion_before_building_it() {
    let c = json!({"type": "circle", "cx": 0, "cy": 0, "r": 1});
    let form = "expected items that expand to 1 to 1000 entities";
    assert_eq!(
        err(batch(json!([
            c,
            linear(json!([0]), json!(1000), 1.0, 0.0),
            c
        ]))),
        format!("create_drawing entities: expands to 1001 entities; {form}")
    );
    // A million copies are counted, never built.
    assert_eq!(
        err(batch(json!([
            c,
            linear(json!([0]), json!(1000), 1.0, 0.0),
            linear(json!([1]), json!(1000), 0.0, 1.0)
        ]))),
        format!("create_drawing entities: expands to 1000000 entities; {form}")
    );
    assert_eq!(
        err(batch(
            json!([{"type": "text", "x": 0, "y": 0, "height": 5, "text": "   "}])
        )),
        format!("create_drawing entities: expands to 0 entities; {form}")
    );
}

/// LCV-196 AC 6 — a polar copy of an arc keeps its orientation, and its
/// endpoints are the source endpoints rotated about the centre.
#[test]
fn lcv196_ac6_a_polar_arc_copy_keeps_ccw_and_rotates_its_endpoints() {
    use crate::geometry::{Arc, EPSILON, Transform, Vec2};
    let as_arc = |item: &DrawingItem| match *item {
        DrawingItem::Arc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
        } => Arc::new(Vec2::new(cx, cy), r, start, end, ccw),
        ref other => panic!("{other:?}"),
    };
    for ccw in [true, false] {
        let got = items(json!([
            {"type": "arc", "cx": 7, "cy": -2, "r": 3, "start_deg": 10, "end_deg": 130, "ccw": ccw},
            {"type": "polar_array", "of": [0], "count": 3, "cx": 1, "cy": 2, "step_deg": 37}
        ]));
        assert_eq!(got.len(), 3);
        let source = as_arc(&got[0]);
        for (k, copy) in got.iter().enumerate().skip(1) {
            let copy = as_arc(copy);
            let turn = Transform::Rotate {
                base: Vec2::new(1.0, 2.0),
                angle: (37.0 * k as f64).to_radians(),
            };
            assert_eq!(copy.ccw, ccw, "copy {k}");
            assert!(
                copy.start_point()
                    .distance(turn.point(source.start_point()))
                    < EPSILON
            );
            assert!(copy.end_point().distance(turn.point(source.end_point())) < EPSILON);
        }
    }
}

// ── LCV-196 T14: mutation survivors ──────────────────────────────────────

/// Vertex k is taken mod `sides`: the first vertex of a 0° polygon is exact,
/// and the last side ends exactly where the first begins.
#[test]
fn lcv196_ac3_a_polygon_closes_on_its_exact_first_vertex() {
    let got = items(json!([{"type": "polygon", "cx": 0, "cy": 0, "r": 10, "sides": 5}]));
    let ends = |i: &DrawingItem| match *i {
        DrawingItem::Line { x1, y1, x2, y2 } => ((x1, y1), (x2, y2)),
        _ => panic!("{i:?} is not a line"),
    };
    let (first, last) = (ends(&got[0]), ends(&got[4]));
    assert_eq!(first.0, (10.0, 0.0));
    assert_eq!(last.1, first.0);
}

/// A side omitted at `EPSILON` long, not only at zero.
#[test]
fn lcv196_ac2_a_side_of_epsilon_length_is_omitted() {
    let thin = json!({"type": "rect", "x": 1, "y": 2, "width": 1e-9, "height": 4});
    let x2 = 1.0 + 1e-9;
    assert_close(
        &items(json!([thin])),
        &[line(x2, 2.0, x2, 6.0), line(1.0, 6.0, 1.0, 2.0)],
    );
}

/// `closed` and `step_deg` refusals carry their own forms.
#[test]
fn lcv196_ac8_closed_and_step_deg_refusals_name_their_forms() {
    let mut open = polyline(&[(0.0, 0.0), (1.0, 0.0)], false);
    open["closed"] = json!("yes");
    assert_eq!(
        err(batch(json!([open]))),
        "create_drawing entities[0].closed: not a boolean; expected true or false"
    );
    let c = json!({"type": "circle", "cx": 0, "cy": 0, "r": 1});
    let polar = json!({"type": "polar_array", "of": [0], "count": 2, "cx": 0, "cy": 0,
        "step_deg": "x"});
    assert_eq!(
        err(batch(json!([c, polar]))),
        "create_drawing entities[1].step_deg: not a number; expected a number in degrees"
    );
}
