//! LCV-194 — `measure`'s argument parse. Only the shape is checked here:
//! the query, finite `{x, y}` points, at most one of `indices` / `ids`, and
//! the operand count each query takes. Whether an index or id is live, and
//! whether an entity's kind fits the query, is the apply site's question
//! (ADR 0007 §D2a).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::Value;

use super::transform::{handle, id_entry, index_entry, list, present, refuse};
use super::{ToolCallError, expected_form, get_f64};
use crate::agent::bridge::{MeasureQuery, MeasureRequest, MeasureTargets};
use crate::geometry::Vec2;

const TOOL: &str = "measure";

/// `measure {query, points, indices | ids}`, operand counts per query.
pub(super) fn parse(args: &Value) -> Result<MeasureRequest, ToolCallError> {
    let query = query(args)?;
    let raw_points = present(args, "points");
    if raw_points.is_some() && query != MeasureQuery::Distance {
        let reason = format!(r#"not accepted by query "{}""#, query.name());
        return Err(refuse(
            TOOL,
            "points",
            &reason,
            "entities in indices or ids",
        ));
    }
    let given = handle(TOOL, args)?;
    let points = raw_points.map_or(Ok(Vec::new()), points)?;
    let (key, targets) = match given {
        None => ("indices", MeasureTargets::Indices(Vec::new())),
        Some(("indices", raw)) => (
            "indices",
            MeasureTargets::Indices(list(TOOL, "indices", raw, index_entry)?),
        ),
        Some(("ids", raw)) => (
            "ids",
            MeasureTargets::Ids(list(TOOL, "ids", raw, id_entry)?),
        ),
        Some((other, _)) => {
            let instead = if other == "id" {
                "ids instead"
            } else {
                "indices instead"
            };
            return Err(refuse(TOOL, other, "not accepted", instead));
        }
    };
    let n = match &targets {
        MeasureTargets::Indices(v) => v.len(),
        MeasureTargets::Ids(v) => v.len(),
    };
    count(query, key, points.len(), n, given.is_some())?;
    Ok(MeasureRequest {
        query,
        points,
        targets,
    })
}

/// The query, by its wire name.
fn query(args: &Value) -> Result<MeasureQuery, ToolCallError> {
    let raw = present(args, "query").ok_or_else(|| ToolCallError::arg(TOOL, "query", "missing"))?;
    let name = raw
        .as_str()
        .ok_or_else(|| ToolCallError::arg(TOOL, "query", "not a string"))?;
    MeasureQuery::ALL
        .into_iter()
        .find(|q| q.name() == name)
        .ok_or_else(|| ToolCallError::arg(TOOL, "query", "unknown query"))
}

/// A list of `{x, y}` points, each coordinate a number in mm; a NaN or an
/// infinity reaches here as `null`, so it reads as `missing`.
fn points(raw: &Value) -> Result<Vec<Vec2>, ToolCallError> {
    let list = raw
        .as_array()
        .ok_or_else(|| refuse(TOOL, "points", "not a list", expected_form("points")))?;
    let mut out = Vec::with_capacity(list.len());
    for (k, entry) in list.iter().enumerate() {
        if !entry.is_object() {
            let path = format!("points[{k}]");
            return Err(refuse(TOOL, &path, "not an object", expected_form("point")));
        }
        let coord = |c: &str| {
            get_f64(entry, TOOL, c).map_err(|e| match e {
                ToolCallError::Arg {
                    reason, expected, ..
                } => refuse(TOOL, &format!("points[{k}].{c}"), &reason, &expected),
                other => other,
            })
        };
        out.push(Vec2::new(coord("x")?, coord("y")?));
    }
    Ok(out)
}

/// The operand count of `query`: two operands of either kind for
/// `distance`, one entity for `length`, two for `intersections` and
/// `angle`, any for `bbox` (an empty list was already refused).
fn count(
    query: MeasureQuery,
    key: &str,
    points: usize,
    n: usize,
    given: bool,
) -> Result<(), ToolCallError> {
    let want = match query {
        MeasureQuery::Distance => {
            let total = points + n;
            if total == 2 {
                return Ok(());
            }
            let path = if points > 0 { "points" } else { key };
            let reason = format!("{total} operand{} given", if total == 1 { "" } else { "s" });
            let expected = r#"2 operands for query "distance": points, entities or one of each"#;
            return Err(refuse(TOOL, path, &reason, expected));
        }
        MeasureQuery::Bbox => return Ok(()),
        MeasureQuery::Length => (1, "exactly 1 entity"),
        MeasureQuery::Intersections | MeasureQuery::Angle => (2, "2 entities"),
    };
    if n == want.0 {
        return Ok(());
    }
    let reason = match (given, n) {
        (false, _) => "missing".to_owned(),
        (true, 1) => "has 1 entry".to_owned(),
        (true, n) => format!("has {n} entries"),
    };
    let expected = format!(r#"{} for query "{}""#, want.1, query.name());
    Err(refuse(TOOL, key, &reason, &expected))
}
