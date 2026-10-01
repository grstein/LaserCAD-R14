//! The `create_drawing` item key table (LCV-196): each entity `type` with the
//! keys it takes, their JSON kind and whether `null`/absent is allowed. The
//! parser and [`schema`](super::schema) both read it, so the published schema
//! and the LCV-185 null-tolerance rule cannot drift apart.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::agent::tools::expected_form;

/// The JSON kind of one item key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// A finite number.
    Num,
    /// A whole number.
    Int,
    /// `true` or `false`.
    Bool,
    /// A string.
    Str,
    /// A list of `{x, y}` points.
    Points,
    /// A list of earlier item indices.
    IndexList,
}

/// One key of one entity type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// The JSON key.
    pub name: &'static str,
    /// Its JSON kind.
    pub kind: KeyKind,
    /// `true` when this type accepts the key absent or `null` (read as 0).
    pub optional: bool,
}

/// An entity type and the keys it takes besides `type`, in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityType {
    /// The `type` value.
    pub name: &'static str,
    /// Its keys.
    pub keys: &'static [Key],
}

impl EntityType {
    /// `true` when this type takes `key`.
    pub fn takes(&self, key: &str) -> bool {
        self.keys.iter().any(|k| k.name == key)
    }

    /// `"a line key (x1, y1, x2, y2)"`: the type's keys as an expected form.
    pub(super) fn own_keys(&self) -> String {
        let names: Vec<&str> = self.keys.iter().map(|k| k.name).collect();
        format!(
            "{} {} key ({})",
            self.article(),
            self.name,
            names.join(", ")
        )
    }

    /// `"an"` before a vowel, else `"a"`.
    pub(super) fn article(&self) -> &'static str {
        if self.name.starts_with(['a', 'e', 'i', 'o', 'u']) {
            "an"
        } else {
            "a"
        }
    }
}

const fn req(name: &'static str, kind: KeyKind) -> Key {
    Key {
        name,
        kind,
        optional: false,
    }
}

const fn opt(name: &'static str, kind: KeyKind) -> Key {
    Key {
        name,
        kind,
        optional: true,
    }
}

use KeyKind::{Bool, IndexList, Int, Num, Points, Str};

/// Every entity type of a `create_drawing` item; the one source of the item
/// schema and of the item key check.
pub static ENTITY_TYPES: [EntityType; 9] = [
    EntityType {
        name: "line",
        keys: &[
            req("x1", Num),
            req("y1", Num),
            req("x2", Num),
            req("y2", Num),
        ],
    },
    EntityType {
        name: "circle",
        keys: &[req("cx", Num), req("cy", Num), req("r", Num)],
    },
    EntityType {
        name: "arc",
        keys: &[
            req("cx", Num),
            req("cy", Num),
            req("r", Num),
            req("start_deg", Num),
            req("end_deg", Num),
            req("ccw", Bool),
        ],
    },
    EntityType {
        name: "polyline",
        keys: &[req("points", Points), req("closed", Bool)],
    },
    EntityType {
        name: "rect",
        keys: &[
            req("x", Num),
            req("y", Num),
            req("width", Num),
            req("height", Num),
            opt("corner_radius", Num),
        ],
    },
    EntityType {
        name: "polygon",
        keys: &[
            req("cx", Num),
            req("cy", Num),
            req("r", Num),
            req("sides", Int),
            opt("start_deg", Num),
        ],
    },
    EntityType {
        name: "text",
        keys: &[
            req("x", Num),
            req("y", Num),
            req("height", Num),
            req("text", Str),
        ],
    },
    EntityType {
        name: "linear_array",
        keys: &[
            req("of", IndexList),
            req("count", Int),
            req("dx", Num),
            req("dy", Num),
        ],
    },
    EntityType {
        name: "polar_array",
        keys: &[
            req("of", IndexList),
            req("count", Int),
            req("cx", Num),
            req("cy", Num),
            req("step_deg", Num),
        ],
    },
];

/// The type named `name`, if any.
pub(super) fn entity_type(name: &str) -> Option<&'static EntityType> {
    ENTITY_TYPES.iter().find(|t| t.name == name)
}

/// `true` when some type publishes `key`.
pub(super) fn published(key: &str) -> bool {
    ENTITY_TYPES.iter().any(|t| t.takes(key))
}

/// The accepted form of an item key, for its refusal; `type` lists every
/// type of [`ENTITY_TYPES`], other keys fall back to the scalar tools' form.
pub(super) fn form(key: &str) -> String {
    match key {
        "type" => {
            let mut quoted = ENTITY_TYPES.iter().map(|t| format!("\"{}\"", t.name));
            let last = quoted.next_back().unwrap_or_default();
            format!("{} or {last}", quoted.collect::<Vec<_>>().join(", "))
        }
        "width" | "height" => "a positive number in mm".to_owned(),
        "corner_radius" => "a number in mm from 0 to half the shorter side".to_owned(),
        "sides" => "an integer from 3 to 64".to_owned(),
        "count" => "an integer from 2 to 1000".to_owned(),
        "step_deg" => "a number in degrees".to_owned(),
        "closed" => "true or false".to_owned(),
        "text" => "a string of 1 to 256 characters without control characters".to_owned(),
        "points" => "a list of 2 to 1000 points {x, y} in mm".to_owned(),
        "of" => "a list of distinct indices of earlier items".to_owned(),
        _ => expected_form(key).to_owned(),
    }
}
