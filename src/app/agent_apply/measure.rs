//! The `measure` arm of `agent_apply.rs::plan` (LCV-194): resolve the
//! operands against the live drawing, check their kinds, and answer one
//! sentence computed by `src/geometry/`. Commits nothing, so the answer moves
//! no revision and trips no fence (AC 7).
//!
//! Every number is `{:.3}` mm or degrees, never `-0.000`.
//!
//! Imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere.

use super::Planned;
use super::edit::out_of_range;
use super::set::resolve;
use crate::agent::tools::refusal;
use crate::agent::{AgentOutcome, MeasureQuery, MeasureRequest, MeasureTargets};
use crate::document::{Document, Entity};
use crate::geometry::distance::intersections;
use crate::geometry::{Prim, Vec2, closest, overlaps};

const TOOL: &str = "measure";

/// Answer `req` against `doc`, or refuse naming the first operand that is
/// unknown or of the wrong kind (LCV-192 shape).
pub(super) fn answer(req: &MeasureRequest, doc: &Document) -> Planned {
    let outcome = match operands(req, doc) {
        Ok((key, indices)) => measure(req, &key, &indices, doc),
        Err(refused) => Err(refused),
    };
    Planned::Answer(outcome.map_or_else(|refused| refused, AgentOutcome::Ok))
}

/// The handle the targets were given by, and their indices in order.
fn operands(req: &MeasureRequest, doc: &Document) -> Result<(String, Vec<usize>), AgentOutcome> {
    match &req.targets {
        MeasureTargets::Ids(ids) => Ok(("ids".to_owned(), resolve(TOOL, ids, doc)?)),
        MeasureTargets::Indices(indices) => {
            let count = doc.entity_count();
            if let Some((k, &i)) = indices.iter().enumerate().find(|(_, i)| **i >= count) {
                return Err(out_of_range(TOOL, &format!("indices[{k}]"), i, count));
            }
            Ok(("indices".to_owned(), indices.clone()))
        }
    }
}

/// The sentence for one query. Indexing is safe: the operand counts fit the
/// query (`tools/measure.rs::parse` builds every `MeasureRequest`) and
/// `indices` passed the range check in [`operands`].
fn measure(
    req: &MeasureRequest,
    key: &str,
    indices: &[usize],
    doc: &Document,
) -> Result<String, AgentOutcome> {
    let entity = |k: usize| doc.entities[indices[k]];
    let prim = |k: usize| prim(key, k, entity(k));
    Ok(match req.query {
        MeasureQuery::Distance => {
            let mut all: Vec<Prim> = req.points.iter().map(|&p| Prim::Point(p)).collect();
            for k in 0..indices.len() {
                all.push(prim(k)?);
            }
            let (p, q) = closest(all[0], all[1]);
            let d = q - p;
            format!(
                "distance: {} mm, dx {}, dy {}, from {} to {}",
                num(d.length()),
                num(d.x),
                num(d.y),
                point(p),
                point(q)
            )
        }
        MeasureQuery::Length => format!("length: {} mm", num(length(entity(0)))),
        MeasureQuery::Bbox => bbox(indices, doc),
        MeasureQuery::Intersections => {
            let (a, b) = (prim(0)?, prim(1)?);
            let pts = intersections(a, b);
            let text = if overlaps(a, b) {
                "overlap".to_owned()
            } else if pts.is_empty() {
                "none".to_owned()
            } else {
                pts.into_iter().map(point).collect::<Vec<_>>().join("; ")
            };
            format!("intersections: {text}")
        }
        MeasureQuery::Angle => {
            let (a, b) = (direction(key, 0, entity(0))?, direction(key, 1, entity(1))?);
            let directed = a.cross(b).atan2(a.dot(b)).to_degrees().rem_euclid(360.0);
            let between = directed % 180.0;
            let mut ccw = num(directed);
            if ccw == "360.000" {
                ccw = num(0.0);
            }
            format!(
                "angle: {ccw}° counter-clockwise from the first line to the second; {}° between the lines",
                num(between.min(180.0 - between))
            )
        }
    })
}

/// The entity as a bounded primitive. An ellipse or a Bézier has no exact
/// closest-point or intersection routine, so it is refused naming
/// `<key>[k]` rather than answered approximately.
fn prim(key: &str, k: usize, e: Entity) -> Result<Prim, AgentOutcome> {
    match e {
        Entity::Line(l) => Ok(Prim::Line(l)),
        Entity::Circle(c) => Ok(Prim::Circle(c)),
        Entity::Arc(a) => Ok(Prim::Arc(a)),
        Entity::Ellipse(_) | Entity::Bezier(_) => Err(AgentOutcome::Refused(refusal(
            TOOL,
            &format!("{key}[{k}]"),
            &format!(
                "is {}, which distance and intersections do not measure",
                curve(e)
            ),
            "a line, circle or arc (bbox and length take any entity)",
        ))),
    }
}

/// Chord tolerance of the polyline that measures a curve's length, in mm.
const LENGTH_TOL_MM: f64 = 1e-6;

/// A line's length, an arc's arc length, a circle's circumference; an
/// ellipse's or Bézier's as the sum of a fine polyline's chords.
fn length(e: Entity) -> f64 {
    let chords = |pts: Vec<Vec2>| pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    match e {
        Entity::Line(l) => l.length(),
        Entity::Circle(c) => c.circumference(),
        Entity::Arc(a) => a.arc_length(),
        Entity::Ellipse(el) => chords(el.polyline(LENGTH_TOL_MM)),
        Entity::Bezier(b) => chords(b.polyline(LENGTH_TOL_MM)),
    }
}

/// `an ellipse`, `an elliptical arc` or `a Bézier curve`; empty for the
/// kinds `prim` and `direction` name themselves.
fn curve(e: Entity) -> &'static str {
    match e {
        Entity::Ellipse(el) if el.span.is_none() => "an ellipse",
        Entity::Ellipse(_) => "an elliptical arc",
        Entity::Bezier(_) => "a Bézier curve",
        Entity::Line(_) | Entity::Circle(_) | Entity::Arc(_) => "",
    }
}

/// The extents of the listed entities, or of the whole drawing (all layers).
fn bbox(indices: &[usize], doc: &Document) -> String {
    let boxes = indices.iter().map(|&i| doc.entities[i].bbox());
    let union = boxes.reduce(|(a, b), (c, d)| {
        let min = Vec2::new(a.x.min(c.x), a.y.min(c.y));
        (min, Vec2::new(b.x.max(d.x), b.y.max(d.y)))
    });
    let Some((min, max)) = union.or_else(|| doc.bounds()) else {
        return "bbox: the drawing is empty".to_owned();
    };
    let size = max - min;
    format!(
        "bbox: min {}, max {}, width {} mm, height {} mm",
        point(min),
        point(max),
        num(size.x),
        num(size.y)
    )
}

/// The unit direction of a line operand, start to end; anything else, or a
/// zero-length line, is refused naming `<key>[k]`.
fn direction(key: &str, k: usize, e: Entity) -> Result<Vec2, AgentOutcome> {
    let refuse = |reason: &str, expected: &str| {
        AgentOutcome::Refused(refusal(TOOL, &format!("{key}[{k}]"), reason, expected))
    };
    match e {
        Entity::Line(l) => l.direction().ok_or_else(|| {
            refuse(
                "is a zero-length line",
                "a line with two distinct endpoints",
            )
        }),
        Entity::Circle(_) => Err(refuse("is a circle", "a line")),
        Entity::Arc(_) => Err(refuse("is an arc", "a line")),
        Entity::Ellipse(_) | Entity::Bezier(_) => {
            Err(refuse(&format!("is {}", curve(e)), "a line"))
        }
    }
}

/// `(x, y)` to 3 decimals.
fn point(p: Vec2) -> String {
    format!("({}, {})", num(p.x), num(p.y))
}

/// `{:.3}`, with a value that rounds to zero printed without a minus sign.
fn num(v: f64) -> String {
    let text = format!("{v:.3}");
    if text == "-0.000" {
        "0.000".to_owned()
    } else {
        text
    }
}
