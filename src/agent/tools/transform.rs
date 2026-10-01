//! LCV-186 — the `indices` form of the six edit tools: one call, one
//! [`AgentAction::Set`], one step. LCV-191 adds `set_layer`, without `index`.
//! LCV-188 adds `ids`/`id` (strings `e<N>`), building [`AgentAction::ById`].
//!
//! Only the shape of the list is checked here — 1..=1000 entries, each a
//! non-negative integer (or an `e<N>` id), no duplicate, at most one of
//! `index`, `indices`, `id`, `ids`. The range
//! check against the live drawing is the apply site's (ADR 0007 §D2a). The
//! operation's own arguments go through the single-index parser unchanged, so
//! a set call and a single call refuse exactly the same bad arguments.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Value, json};

use super::{ToolCallError, expected_form, parse_tool_call};
use crate::agent::bridge::{AgentAction, SetOp};
use crate::agent::drawing;

/// The tools that accept `indices`.
const SET_TOOLS: [&str; 6] = [
    "delete_entity",
    "move_entity",
    "copy_entity",
    "rotate_entity",
    "mirror_entity",
    "scale_entity",
];

/// The ways a call names its entities; at most one per call (LCV-188).
const HANDLES: [&str; 4] = ["index", "indices", "id", "ids"];

/// The most entities one set call may list, as `create_drawing`'s batch.
const MAX_SET: usize = 1000;

/// The entities a set call names: positions, or stable ids (ADR 0014 §7).
enum Target {
    Indices(Vec<usize>),
    Ids(Vec<u64>),
}

impl Target {
    fn with(self, op: SetOp) -> AgentAction {
        match self {
            Self::Indices(indices) => AgentAction::Set { indices, op },
            Self::Ids(ids) => AgentAction::ById { ids, op },
        }
    }
}

/// `Some` when `name` is a set tool called with a non-null `indices`, `id`
/// or `ids`, or with two handles; `None` sends the call down the
/// single-index path unchanged (AC8).
pub(super) fn parse_set(name: &str, args: &Value) -> Option<Result<AgentAction, ToolCallError>> {
    let tool = SET_TOOLS.into_iter().find(|t| *t == name)?;
    match handle(tool, args) {
        Err(e) => Some(Err(e)),
        Ok(None | Some(("index", _))) => None,
        Ok(Some((key, raw))) => Some(build(tool, key, raw, args)),
    }
}

/// LCV-191 — `set_layer {indices | ids | id, layer}`: both required, no
/// `index` form. The list is checked as for the edit tools; the name by
/// `layer_arg`.
pub(super) fn parse_set_layer(args: &Value) -> Result<AgentAction, ToolCallError> {
    const TOOL: &str = "set_layer";
    if present(args, "index").is_some() {
        return Err(refuse(TOOL, "index", "not accepted", "indices instead"));
    }
    let (key, raw) =
        handle(TOOL, args)?.ok_or_else(|| ToolCallError::arg(TOOL, "indices", "missing"))?;
    let target = target(TOOL, key, raw)?;
    let layer = drawing::layer_arg(args)
        .and_then(|name| name.ok_or_else(|| "missing".to_owned()))
        .map_err(|reason| ToolCallError::arg(TOOL, "layer", reason))?;
    Ok(target.with(SetOp::Layer { layer }))
}

/// `args[key]` unless absent or JSON `null`.
fn present<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key).filter(|v| !v.is_null())
}

/// The one handle given, if any; two are refused, the first named.
fn handle<'a>(
    tool: &str,
    args: &'a Value,
) -> Result<Option<(&'static str, &'a Value)>, ToolCallError> {
    let mut given = HANDLES
        .into_iter()
        .filter_map(|key| present(args, key).map(|raw| (key, raw)));
    let first = given.next();
    if let (Some((a, _)), Some((b, _))) = (first, given.next()) {
        let (reason, expected) = (
            format!("given together with {b}"),
            format!("either {a} or {b}, not both"),
        );
        return Err(refuse(tool, a, &reason, &expected));
    }
    Ok(first)
}

/// Check the list, then parse the operation's arguments exactly as the
/// single-index call would, with a placeholder `index`.
fn build(
    tool: &'static str,
    key: &str,
    raw: &Value,
    args: &Value,
) -> Result<AgentAction, ToolCallError> {
    let target = target(tool, key, raw)?;
    let mut single = args.clone();
    if let Some(object) = single.as_object_mut() {
        object.remove(key);
        object.insert("index".to_owned(), json!(0));
    }
    let op = op_of(parse_tool_call(tool, &single)?);
    Ok(target.with(op))
}

/// The entities `key` names: a list of indices, a list of ids, or one id.
fn target(tool: &str, key: &str, raw: &Value) -> Result<Target, ToolCallError> {
    match key {
        "indices" => list(tool, key, raw, index_entry).map(Target::Indices),
        "ids" => list(tool, key, raw, id_entry).map(Target::Ids),
        _ => id_entry(raw)
            .map(|id| Target::Ids(vec![id]))
            .map_err(|(reason, form)| refuse(tool, key, &reason, form)),
    }
}

/// The shape check of a list, naming the first offending entry (AC5) in
/// the LCV-192 shape.
fn list<T: PartialEq>(
    tool: &str,
    key: &str,
    raw: &Value,
    entry: fn(&Value) -> Result<T, (String, &'static str)>,
) -> Result<Vec<T>, ToolCallError> {
    let list_form = expected_form(key);
    let Some(list) = raw.as_array() else {
        return Err(refuse(tool, key, "not a list", list_form));
    };
    if list.is_empty() {
        return Err(refuse(tool, key, "empty list", list_form));
    }
    if list.len() > MAX_SET {
        let reason = format!("has {} entries", list.len());
        return Err(refuse(tool, key, &reason, list_form));
    }
    let mut out: Vec<T> = Vec::with_capacity(list.len());
    for (at, value) in list.iter().enumerate() {
        let path = format!("{key}[{at}]");
        let item = entry(value).map_err(|(reason, form)| refuse(tool, &path, &reason, form))?;
        if let Some(first) = out.iter().position(|seen| *seen == item) {
            let reason = format!("duplicate of {key}[{first}]");
            return Err(refuse(tool, &path, &reason, &format!("distinct {key}")));
        }
        out.push(item);
    }
    Ok(out)
}

/// One `indices` entry: non-negative, integral, finite — `get_index`'s rule.
fn index_entry(value: &Value) -> Result<usize, (String, &'static str)> {
    let form = expected_form("index");
    let raw = value.as_f64().ok_or(("not a number".to_owned(), form))?;
    let ok = raw >= 0.0 && raw.fract() == 0.0 && raw.is_finite();
    ok.then_some(raw as usize)
        .ok_or((format!("{raw} is not an index"), form))
}

/// One id: the string `e<N>`, N a decimal `u64` ≥ 1 (ADR 0014 §7). The
/// string itself is never echoed.
fn id_entry(value: &Value) -> Result<u64, (String, &'static str)> {
    let form = expected_form("id");
    let text = value.as_str().ok_or(("not a string".to_owned(), form))?;
    let digits = text
        .strip_prefix('e')
        .filter(|d| d.bytes().all(|b| b.is_ascii_digit()));
    let id = digits
        .and_then(|d| d.parse::<u64>().ok())
        .filter(|&n| n >= 1);
    id.ok_or(("not an id".to_owned(), form))
}

/// A refusal of a handle argument or one of its entries.
fn refuse(tool: &str, path: &str, reason: &str, expected: &str) -> ToolCallError {
    ToolCallError::Arg {
        tool: tool.to_owned(),
        path: path.to_owned(),
        reason: reason.to_owned(),
        expected: expected.to_owned(),
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

    fn reason(name: &str, args: Value) -> (String, String) {
        match parse_tool_call(name, &args) {
            Err(ToolCallError::Arg { path, reason, .. }) => (path, reason),
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
        let (field, text) = reason("move_entity", json!({"indices":[0],"dx":1}));
        assert_eq!((field.as_str(), text.as_str()), ("dy", "missing"));
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
        let (path, text) = reason("delete_entity", json!({"indices":[5, 0, 5, 0]}));
        assert_eq!(
            (path.as_str(), text.as_str()),
            ("indices[2]", "duplicate of indices[0]")
        );
        let (path, text) = reason("delete_entity", json!({"indices":[true]}));
        assert_eq!(
            (path.as_str(), text.as_str()),
            ("indices[0]", "not a number")
        );
    }
}
