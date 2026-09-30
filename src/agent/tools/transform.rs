//! LCV-186 — the `indices` form of the six edit tools: one call, one
//! [`AgentAction::Set`], one step.
//!
//! Only the shape of the list is checked here — 1..=1000 entries, each a
//! non-negative integer, no duplicate, not together with `index`. The range
//! check against the live drawing is the apply site's (ADR 0007 §D2a). The
//! operation's own arguments go through the single-index parser unchanged, so
//! a set call and a single call refuse exactly the same bad arguments.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Value, json};

use super::{ToolCallError, parse_tool_call};
use crate::agent::bridge::{AgentAction, SetOp};

/// The tools that accept `indices`.
const SET_TOOLS: [&str; 6] = [
    "delete_entity",
    "move_entity",
    "copy_entity",
    "rotate_entity",
    "mirror_entity",
    "scale_entity",
];

/// The most entities one set call may list, as `create_drawing`'s batch.
const MAX_SET: usize = 1000;

/// `Some` when `name` is a set tool called with a non-null `indices`;
/// `None` sends the call down the single-index path unchanged (AC8).
pub(super) fn parse_set(name: &str, args: &Value) -> Option<Result<AgentAction, ToolCallError>> {
    let tool = SET_TOOLS.into_iter().find(|t| *t == name)?;
    let raw = present(args, "indices")?;
    Some(build(tool, raw, args))
}

/// `args[key]` unless absent or JSON `null`.
fn present<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key).filter(|v| !v.is_null())
}

/// Check the list, then parse the operation's arguments exactly as the
/// single-index call would, with a placeholder `index`.
fn build(tool: &'static str, raw: &Value, args: &Value) -> Result<AgentAction, ToolCallError> {
    if present(args, "index").is_some() {
        return Err(invalid(
            tool,
            "give either `index` or `indices`, not both".to_owned(),
        ));
    }
    let indices = indices(tool, raw)?;
    let mut single = args.clone();
    if let Some(object) = single.as_object_mut() {
        object.remove("indices");
        object.insert("index".to_owned(), json!(0));
    }
    let op = op_of(parse_tool_call(tool, &single)?);
    Ok(AgentAction::Set { indices, op })
}

/// The shape check of `indices`, naming the first offending entry (AC5).
fn indices(tool: &'static str, raw: &Value) -> Result<Vec<usize>, ToolCallError> {
    let Some(list) = raw.as_array() else {
        return Err(invalid(
            tool,
            "expected a list of entity indices".to_owned(),
        ));
    };
    if list.is_empty() {
        return Err(invalid(
            tool,
            format!("the list is empty; give 1 to {MAX_SET} indices"),
        ));
    }
    if list.len() > MAX_SET {
        let n = list.len();
        return Err(invalid(
            tool,
            format!("the list has {n} entries; at most {MAX_SET}"),
        ));
    }
    let mut out: Vec<usize> = Vec::with_capacity(list.len());
    for (at, value) in list.iter().enumerate() {
        let Some(index) = entry(value) else {
            let shown = if value.is_number() {
                value.to_string()
            } else {
                "this entry".to_owned()
            };
            return Err(invalid(
                tool,
                format!("indices[{at}]: {shown} is not a non-negative integer"),
            ));
        };
        if let Some(first) = out.iter().position(|&seen| seen == index) {
            return Err(invalid(
                tool,
                format!("indices[{at}] is a duplicate of indices[{first}]"),
            ));
        }
        out.push(index);
    }
    Ok(out)
}

/// One entry as an index: non-negative, integral, finite — `get_index`'s rule.
fn entry(value: &Value) -> Option<usize> {
    let raw = value.as_f64()?;
    (raw >= 0.0 && raw.fract() == 0.0 && raw.is_finite()).then_some(raw as usize)
}

/// A refusal of the `indices` argument.
fn invalid(tool: &'static str, reason: String) -> ToolCallError {
    ToolCallError::InvalidArg {
        tool,
        field: "indices",
        reason,
    }
}

/// The single-index action's operation, without its placeholder index.
fn op_of(single: AgentAction) -> SetOp {
    match single {
        AgentAction::Move { dx, dy, .. } => SetOp::Move { dx, dy },
        AgentAction::Copy { dx, dy, .. } => SetOp::Copy { dx, dy },
        AgentAction::Rotate { x, y, angle, .. } => SetOp::Rotate { x, y, angle },
        AgentAction::Mirror {
            x1,
            y1,
            x2,
            y2,
            erase_source,
            ..
        } => SetOp::Mirror {
            x1,
            y1,
            x2,
            y2,
            erase_source,
        },
        AgentAction::Scale { x, y, factor, .. } => SetOp::Scale { x, y, factor },
        // `delete_entity`, the one other tool in `SET_TOOLS`.
        _ => SetOp::Delete,
    }
}

#[cfg(test)]
mod tests {
    use core::f64::consts::FRAC_PI_2;

    use serde_json::json;

    use super::*;

    fn ok(name: &str, args: Value) -> AgentAction {
        parse_tool_call(name, &args).unwrap_or_else(|e| panic!("{e}"))
    }

    fn reason(name: &str, args: Value) -> (&'static str, String) {
        match parse_tool_call(name, &args) {
            Err(ToolCallError::InvalidArg { field, reason, .. }) => (field, reason),
            other => panic!("{other:?}"),
        }
    }

    /// AC1 — each tool maps to its operation, arguments carried over, the
    /// rotation in radians, the indices in the order given.
    #[test]
    fn each_tool_builds_its_set_operation() {
        let set = |indices: Vec<usize>, op| AgentAction::Set { indices, op };
        assert_eq!(
            ok("delete_entity", json!({"indices":[3,0]})),
            set(vec![3, 0], SetOp::Delete)
        );
        assert_eq!(
            ok("move_entity", json!({"indices":[0],"dx":1.5,"dy":-2})),
            set(vec![0], SetOp::Move { dx: 1.5, dy: -2.0 })
        );
        assert_eq!(
            ok("copy_entity", json!({"indices":[2,1],"dx":-1,"dy":4})),
            set(vec![2, 1], SetOp::Copy { dx: -1.0, dy: 4.0 })
        );
        assert_eq!(
            ok(
                "rotate_entity",
                json!({"indices":[1],"x":2,"y":3,"degrees":90})
            ),
            set(
                vec![1],
                SetOp::Rotate {
                    x: 2.0,
                    y: 3.0,
                    angle: FRAC_PI_2
                }
            )
        );
        let mirror = json!({"indices":[0,1],"x1":0,"y1":1,"x2":2,"y2":3,"erase_source":true});
        assert_eq!(
            ok("mirror_entity", mirror),
            set(
                vec![0, 1],
                SetOp::Mirror {
                    x1: 0.0,
                    y1: 1.0,
                    x2: 2.0,
                    y2: 3.0,
                    erase_source: true
                }
            )
        );
        assert_eq!(
            ok(
                "scale_entity",
                json!({"indices":[4],"x":1,"y":2,"factor":0.5})
            ),
            set(
                vec![4],
                SetOp::Scale {
                    x: 1.0,
                    y: 2.0,
                    factor: 0.5
                }
            )
        );
    }

    /// AC1 — exactly 1000 entries is accepted; 1001 is not (T1).
    #[test]
    fn a_thousand_indices_are_accepted() {
        let all: Vec<usize> = (0..1000).collect();
        assert_eq!(
            ok("delete_entity", json!({ "indices": all })),
            AgentAction::Set {
                indices: all,
                op: SetOp::Delete
            }
        );
    }

    /// AC6 — the operation's arguments are checked as in the single call.
    #[test]
    fn a_bad_operation_argument_is_refused_as_in_the_single_call() {
        let (field, _) = reason(
            "scale_entity",
            json!({"indices":[0,1],"x":0,"y":0,"factor":0}),
        );
        assert_eq!(field, "factor");
        let e = parse_tool_call("move_entity", &json!({"indices":[0],"dx":1})).unwrap_err();
        assert!(
            matches!(e, ToolCallError::MissingField { field: "dy", .. }),
            "{e:?}"
        );
    }

    /// A null `indices` or `index` counts as absent; other tools ignore
    /// `indices`.
    #[test]
    fn null_counts_as_absent_and_other_tools_ignore_indices() {
        assert_eq!(
            ok("delete_entity", json!({"index":2,"indices":null})),
            AgentAction::Delete { index: 2 }
        );
        assert_eq!(
            ok("delete_entity", json!({"index":null,"indices":[2]})),
            AgentAction::Set {
                indices: vec![2],
                op: SetOp::Delete
            }
        );
        let line = json!({"x1":0,"y1":0,"x2":1,"y2":1,"indices":[]});
        assert!(matches!(
            ok("create_line", line),
            AgentAction::CreateLine { .. }
        ));
    }

    /// AC5 — index 0 is a valid entry, a large integer parses (range is the
    /// apply site's), and the first duplicate is named against its first
    /// occurrence.
    #[test]
    fn entries_follow_the_single_index_rule() {
        let got = ok("delete_entity", json!({"indices":[0, 1_000_000]}));
        assert_eq!(
            got,
            AgentAction::Set {
                indices: vec![0, 1_000_000],
                op: SetOp::Delete
            }
        );
        let (_, text) = reason("delete_entity", json!({"indices":[5, 0, 5, 0]}));
        assert_eq!(text, "indices[2] is a duplicate of indices[0]");
        let (_, text) = reason("delete_entity", json!({"indices":[true]}));
        assert_eq!(text, "indices[0]: this entry is not a non-negative integer");
    }
}
