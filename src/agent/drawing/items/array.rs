//! LCV-196 — `linear_array` and `polar_array` items: `of` names earlier items
//! by batch position, each once; a listed array may itself list no array, so
//! a grid is two levels at most. Split out of `items.rs` for the LOC cap.
//!
//! Names no drawing-state type; MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{Fields, Shape};
use crate::agent::drawing::{MAX_DRAWING_ENTITIES, arg};
use crate::agent::tools::ToolCallError;
use crate::geometry::Vec2;

/// The accepted form of one `of` entry.
const OF_FORM: &str = "the index of an earlier item, once; not an array of arrays";

/// Where copy k of an array goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::agent::drawing) enum Step {
    /// Copy k moves by k · this offset, mm.
    Linear(Vec2),
    /// Copy k turns by k · `angle` radians, CCW, about `center`.
    Polar {
        /// The fixed point, mm.
        center: Vec2,
        /// The turn per copy, radians.
        angle: f64,
    },
}

/// An array item; `earlier` are the shapes before it in the batch.
pub(super) fn array(f: &Fields, polar: bool, earlier: &[Shape]) -> Result<Shape, ToolCallError> {
    let of = of(f, earlier)?;
    let count = f.int("count", 2, MAX_DRAWING_ENTITIES)?;
    let step = if polar {
        Step::Polar {
            center: Vec2::new(f.num("cx")?, f.num("cy")?),
            angle: f.num("step_deg")?.to_radians(),
        }
    } else {
        Step::Linear(Vec2::new(f.num("dx")?, f.num("dy")?))
    };
    Ok(Shape::Array { of, count, step })
}

/// A non-empty list of distinct indices of earlier items, none of them an
/// array that lists an array.
fn of(f: &Fields, earlier: &[Shape]) -> Result<Vec<usize>, ToolCallError> {
    let list = f
        .get("of")
        .as_array()
        .ok_or_else(|| f.fail("of", "not a list"))?;
    if list.is_empty() {
        return Err(f.fail("of", "has 0 items"));
    }
    let own = earlier.len();
    let mut of = Vec::with_capacity(list.len());
    for (k, value) in list.iter().enumerate() {
        let refuse = |reason: &str| arg(format!("{}[{k}]", f.at("of")), reason, OF_FORM);
        let j = value
            .as_f64()
            .filter(|n| *n >= 0.0 && n.fract() == 0.0 && n.is_finite())
            .ok_or_else(|| refuse("not an index"))? as usize;
        if j == own {
            return Err(refuse("names itself"));
        } else if j > own {
            return Err(refuse("names a later item"));
        } else if of.contains(&j) {
            return Err(refuse("repeats an earlier entry"));
        } else if lists_an_array(&earlier[j], earlier) {
            return Err(refuse("names an array that lists an array"));
        }
        of.push(j);
    }
    Ok(of)
}

/// `true` when `shape` is an array whose `of` names an array.
fn lists_an_array(shape: &Shape, earlier: &[Shape]) -> bool {
    match shape {
        Shape::Array { of, .. } => of
            .iter()
            .any(|&j| matches!(earlier[j], Shape::Array { .. })),
        _ => false,
    }
}
