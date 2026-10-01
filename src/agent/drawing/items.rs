//! LCV-196 — one `create_drawing` item parsed into a [`Shape`]: the key check
//! against the [`keys`] table, then each type's own values. Every refusal
//! names its path, `entities[i].key` or `entities[i].points[k].x` (LCV-192).
//!
//! Names no drawing-state type; MUST NOT import `egui`, `eframe`, or `rfd`.

use serde_json::{Map, Value};

use super::keys::{self, EntityType};
use super::{DrawingItem, MAX_DRAWING_ENTITIES, TOOL, arg, cut};
use crate::agent::tools::{ToolCallError, expected_form, validate_r};
use crate::geometry::{EPSILON, Vec2};
use crate::text::layout::{MAX_HEIGHT_MM, MIN_HEIGHT_MM};

mod array;
pub(super) use array::Step;

/// Longest `text` an item may carry, in characters.
const MAX_TEXT_CHARS: usize = 256;

/// One parsed item, before expansion into [`DrawingItem`]s.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Shape {
    /// A line, circle or arc, as is.
    Item(DrawingItem),
    /// Points with no repeated neighbour; `closed` adds the last-to-first line.
    Polyline { points: Vec<Vec2>, closed: bool },
    /// Lower-left corner, size and corner radius (0 for square corners), mm.
    Rect {
        corner: Vec2,
        width: f64,
        height: f64,
        radius: f64,
    },
    /// A regular polygon inscribed in `r`; `start` in radians.
    Polygon {
        center: Vec2,
        r: f64,
        sides: usize,
        start: f64,
    },
    /// The `TEXT` command's layout of `text` at `origin`, `height` mm.
    Text {
        origin: Vec2,
        height: f64,
        text: String,
    },
    /// `count − 1` copies of the output of the earlier items `of`.
    Array {
        of: Vec<usize>,
        count: usize,
        step: Step,
    },
}

/// One entity: an object, a known `type`, that type's keys (another type's
/// key only as `null`), then its values; `earlier` are the shapes before it.
pub(super) fn item(index: usize, value: &Value, earlier: &[Shape]) -> Result<Shape, ToolCallError> {
    let obj = value.as_object().ok_or_else(|| {
        let form = "an object with a type and that type's keys";
        arg(format!("entities[{index}]"), "not an object", form)
    })?;
    let f = Fields { index, obj };
    let kind = obj.get("type").ok_or_else(|| f.fail("type", "missing"))?;
    let Some(ty) = kind.as_str().and_then(keys::entity_type) else {
        return Err(f.fail("type", "unknown type"));
    };
    check_keys(&f, ty)?;
    Ok(match ty.name {
        "line" => Shape::Item(DrawingItem::Line {
            x1: f.num("x1")?,
            y1: f.num("y1")?,
            x2: f.num("x2")?,
            y2: f.num("y2")?,
        }),
        "circle" => Shape::Item(DrawingItem::Circle {
            cx: f.num("cx")?,
            cy: f.num("cy")?,
            r: f.radius()?,
        }),
        "arc" => Shape::Item(DrawingItem::Arc {
            cx: f.num("cx")?,
            cy: f.num("cy")?,
            r: f.radius()?,
            // The one unit boundary of this file: degrees in, radians out.
            start: f.num("start_deg")?.to_radians(),
            end: f.num("end_deg")?.to_radians(),
            ccw: f.bool("ccw")?,
        }),
        "polyline" => polyline(&f)?,
        "rect" => rect(&f)?,
        "polygon" => Shape::Polygon {
            center: Vec2::new(f.num("cx")?, f.num("cy")?),
            r: f.radius()?,
            sides: f.int("sides", 3, 64)?,
            start: f.opt_num("start_deg")?.to_radians(),
        },
        "text" => Shape::Text {
            origin: Vec2::new(f.num("x")?, f.num("y")?),
            height: text_height(&f)?,
            text: text(&f)?,
        },
        name => array::array(&f, name == "polar_array", earlier)?,
    })
}

/// Another type's key only as `null` (ADR 0010 §2); every required own key
/// present.
fn check_keys(f: &Fields, ty: &EntityType) -> Result<(), ToolCallError> {
    let (a, name, own_keys) = (ty.article(), ty.name, ty.own_keys());
    let own = |k: &str| k == "type" || ty.takes(k);
    for (key, value) in f.obj.iter().filter(|(k, _)| !own(k)) {
        if !keys::published(key) {
            return Err(arg(f.at(&cut(key)), "unknown key", &own_keys));
        } else if !value.is_null() {
            let reason = format!("not {a} {name} key");
            return Err(arg(f.at(key), &reason, &format!("null or {own_keys}")));
        }
    }
    match ty
        .keys
        .iter()
        .find(|k| !k.optional && !f.obj.contains_key(k.name))
    {
        Some(key) => Err(f.fail(key.name, "missing")),
        None => Ok(()),
    }
}

/// 2..=1000 `{x, y}` points, no point repeating its predecessor; a closed
/// polyline needs 3+ points and a last point other than the first.
fn polyline(f: &Fields) -> Result<Shape, ToolCallError> {
    let list = f
        .get("points")
        .as_array()
        .ok_or_else(|| f.fail("points", "not a list"))?;
    let n = list.len();
    if !(2..=MAX_DRAWING_ENTITIES).contains(&n) {
        return Err(f.fail("points", &format!("has {n} items")));
    }
    let point_at = |k: usize| format!("{}[{k}]", f.at("points"));
    let mut points: Vec<Vec2> = Vec::with_capacity(n);
    for (k, value) in list.iter().enumerate() {
        let p = point(&point_at(k), value)?;
        if points.last().is_some_and(|q| q.distance(p) <= EPSILON) {
            let reason = "repeats the previous point";
            return Err(arg(point_at(k), reason, expected_form("point")));
        }
        points.push(p);
    }
    let closed = f.bool("closed")?;
    if closed && n == 2 {
        let form = "false, or true with 3 or more points";
        return Err(arg(f.at("closed"), "true with 2 points", form));
    }
    if closed && points[0].distance(points[n - 1]) <= EPSILON {
        let reason = "repeats the first point";
        return Err(arg(point_at(n - 1), reason, expected_form("point")));
    }
    Ok(Shape::Polyline { points, closed })
}

/// One `{x, y}` object of finite numbers at `path`.
fn point(path: &str, value: &Value) -> Result<Vec2, ToolCallError> {
    let form = expected_form("point");
    let obj = value
        .as_object()
        .ok_or_else(|| arg(path.to_owned(), "not an object", form))?;
    if let Some(key) = obj.keys().find(|k| !["x", "y"].contains(&k.as_str())) {
        return Err(arg(format!("{path}.{}", cut(key)), "unknown key", form));
    }
    let coord = |key: &str| {
        let reason = if obj.contains_key(key) {
            "not a number"
        } else {
            "missing"
        };
        obj.get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .ok_or_else(|| arg(format!("{path}.{key}"), reason, expected_form(key)))
    };
    Ok(Vec2::new(coord("x")?, coord("y")?))
}

/// Positive size; `corner_radius` from 0 to half the shorter side.
fn rect(f: &Fields) -> Result<Shape, ToolCallError> {
    let corner = Vec2::new(f.num("x")?, f.num("y")?);
    let (width, height) = (f.positive("width")?, f.positive("height")?);
    let radius = f.opt_num("corner_radius")?;
    if radius < 0.0 || radius > width.min(height) / 2.0 {
        return Err(f.fail("corner_radius", &format!("{radius} is out of range")));
    }
    Ok(Shape::Rect {
        corner,
        width,
        height,
        radius,
    })
}

/// The `TEXT` command's height range, `MIN_HEIGHT_MM..=MAX_HEIGHT_MM`.
fn text_height(f: &Fields) -> Result<f64, ToolCallError> {
    let h = f.num("height")?;
    if !(MIN_HEIGHT_MM..=MAX_HEIGHT_MM).contains(&h) {
        let form = format!("a number in mm from {MIN_HEIGHT_MM} to {MAX_HEIGHT_MM}");
        return Err(arg(f.at("height"), &format!("{h} is out of range"), &form));
    }
    Ok(h)
}

/// 1..=256 characters, none of them a control character.
fn text(f: &Fields) -> Result<String, ToolCallError> {
    let text = f
        .get("text")
        .as_str()
        .ok_or_else(|| f.fail("text", "not a string"))?;
    let chars = text.chars().count();
    if chars == 0 {
        return Err(f.fail("text", "is empty"));
    } else if chars > MAX_TEXT_CHARS {
        return Err(f.fail("text", &format!("has {chars} characters")));
    } else if text.chars().any(char::is_control) {
        return Err(f.fail("text", "contains a control character"));
    }
    Ok(text.to_owned())
}

/// Typed access to one item's keys, refusing at `entities[index].key`.
struct Fields<'a> {
    index: usize,
    obj: &'a Map<String, Value>,
}

impl Fields<'_> {
    fn at(&self, key: &str) -> String {
        format!("entities[{}].{key}", self.index)
    }

    fn fail(&self, key: &str, reason: &str) -> ToolCallError {
        arg(self.at(key), reason, &keys::form(key))
    }

    /// The key's value; an absent key reads as `null`.
    fn get(&self, key: &str) -> &Value {
        self.obj.get(key).unwrap_or(&Value::Null)
    }

    fn num(&self, key: &str) -> Result<f64, ToolCallError> {
        self.get(key)
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| self.fail(key, "not a number"))
    }

    /// An optional number: absent or `null` is 0.
    fn opt_num(&self, key: &str) -> Result<f64, ToolCallError> {
        match self.get(key) {
            Value::Null => Ok(0.0),
            _ => self.num(key),
        }
    }

    /// A positive, finite number: the scalar radius rule (ADR 0010 §3).
    fn positive(&self, key: &str) -> Result<f64, ToolCallError> {
        let v = self.num(key)?;
        validate_r(TOOL, v).map_err(|e| match e {
            ToolCallError::Arg { reason, .. } => self.fail(key, &reason),
            other => other,
        })?;
        Ok(v)
    }

    fn radius(&self) -> Result<f64, ToolCallError> {
        self.positive("r")
    }

    fn bool(&self, key: &str) -> Result<bool, ToolCallError> {
        self.get(key)
            .as_bool()
            .ok_or_else(|| self.fail(key, "not a boolean"))
    }

    /// A whole number in `lo..=hi`.
    fn int(&self, key: &str, lo: usize, hi: usize) -> Result<usize, ToolCallError> {
        let v = self
            .get(key)
            .as_f64()
            .filter(|v| v.is_finite() && v.fract() == 0.0)
            .ok_or_else(|| self.fail(key, "not an integer"))?;
        if v < lo as f64 || v > hi as f64 {
            return Err(self.fail(key, &format!("{v} is out of range")));
        }
        Ok(v as usize)
    }
}
