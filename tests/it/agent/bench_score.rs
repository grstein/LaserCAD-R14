//! LCV-200 — the bench's score of one drawing against its task's reference:
//! the raster IoU of the two inked masks and the pass/fail of each geometric
//! assertion (AC 3).
//!
//! Both drawings go through `render/raster.rs::rasterize` on the reference's
//! bed at [`PX_PER_MM`], and each inked mask is dilated by one pixel so a
//! stroke half a pixel off still overlaps. Pure and deterministic: the same
//! two documents always score the same.

use lasercad::document::{Document, Entity, Finding, check_drawing};
use lasercad::geometry::{Circle, Line, Vec2};
use lasercad::render::raster::{INK, rasterize};
use serde::Deserialize;

/// Raster resolution of the score, pixels per mm.
pub const PX_PER_MM: f64 = 2.0;

/// One geometric assertion of a task's `assertions.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Assertion {
    /// Exactly `count` circles whose radius is within `tol` of `r`, mm.
    Circles { r: f64, count: usize, tol: f64 },
    /// The drawing's bounding box is `w` × `h` mm, each within `tol`.
    Bbox { w: f64, h: f64, tol: f64 },
    /// The drawing is not empty and its check finds no open end and no gap
    /// (LCV-190).
    Closed,
    /// The drawing holds at least `n` entities.
    MinEntities { n: usize },
}

impl Assertion {
    /// Whether `doc` satisfies this assertion.
    pub fn passes(&self, doc: &Document) -> bool {
        match *self {
            Self::Circles { r, count, tol } => {
                let near = |e: &&Entity| matches!(e, Entity::Circle(c) if (c.r - r).abs() <= tol);
                doc.entities.iter().filter(near).count() == count
            }
            Self::Bbox { w, h, tol } => doc.bounds().is_some_and(|(lo, hi)| {
                ((hi.x - lo.x) - w).abs() <= tol && ((hi.y - lo.y) - h).abs() <= tol
            }),
            Self::Closed => {
                !doc.entities.is_empty()
                    && !check_drawing(doc)
                        .findings
                        .iter()
                        .any(|f| matches!(f, Finding::OpenEnd { .. } | Finding::Gap { .. }))
            }
            Self::MinEntities { n } => doc.entities.len() >= n,
        }
    }
}

/// The inked pixels of `entities` on `bed`, dilated by one pixel.
fn ink_mask(entities: &[Entity], bed: [f64; 2]) -> Vec<bool> {
    let px = |mm: f64| (mm * PX_PER_MM).round() as u32 + 1;
    let (w, h) = (px(bed[0]), px(bed[1]));
    let pixels = rasterize(entities, bed, [0.0, 0.0, bed[0], bed[1]], w, h);
    let (w, h) = (w as usize, h as usize);
    let inked = |x: usize, y: usize| pixels[y * w + x] == INK;
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let xs = x.saturating_sub(1)..=(x + 1).min(w - 1);
            xs.clone()
                .any(|nx| (y.saturating_sub(1)..=(y + 1).min(h - 1)).any(|ny| inked(nx, ny)))
        })
        .collect()
}

/// Intersection over union of the two inked masks, on `reference`'s bed.
/// Two empty drawings score 1.0.
pub fn iou(drawing: &Document, reference: &Document) -> f64 {
    let bed = reference.bed_mm;
    let a = ink_mask(&drawing.entities, bed);
    let b = ink_mask(&reference.entities, bed);
    let both = a.iter().zip(&b).filter(|(x, y)| **x && **y).count();
    let either = a.iter().zip(&b).filter(|(x, y)| **x || **y).count();
    if either == 0 {
        return 1.0;
    }
    both as f64 / either as f64
}

/// A default document holding `entities`.
fn doc(entities: &[Entity]) -> Document {
    let mut doc = Document::default();
    for e in entities {
        doc.push_current(*e);
    }
    doc
}

fn square(x: f64, y: f64, side: f64) -> Vec<Entity> {
    let p = [(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    (0..4)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % 4]);
            Entity::Line(Line::new(Vec2::new(a.0, a.1), Vec2::new(b.0, b.1)))
        })
        .collect()
}

fn hole(x: f64, y: f64, r: f64) -> Entity {
    Entity::Circle(Circle::new(Vec2::new(x, y), r))
}

/// AC 3 — a drawing scored against itself is 1.0, against a disjoint one
/// 0.0, and against a near one strictly between.
#[test]
fn iou_is_one_for_identical_zero_for_disjoint_and_between_otherwise() {
    let plate = doc(&[square(20.0, 20.0, 50.0), vec![hole(45.0, 45.0, 5.0)]].concat());
    assert_eq!(iou(&plate, &plate), 1.0);
    let far = doc(&square(200.0, 200.0, 50.0));
    assert_eq!(iou(&plate, &far), 0.0);
    let near = doc(&square(20.0, 20.0, 50.0));
    let score = iou(&near, &plate);
    assert!(
        score > 0.5 && score < 1.0,
        "a missing hole costs IoU: {score}"
    );
    assert_eq!(iou(&doc(&[]), &doc(&[])), 1.0, "two empty drawings agree");
}

/// AC 3 — each assertion kind passes on the drawing it describes and fails
/// on one it does not.
#[test]
fn each_assertion_kind_passes_and_fails_where_it_should() {
    let closed = doc(&[square(10.0, 10.0, 40.0), vec![hole(30.0, 30.0, 3.0)]].concat());
    let mut open = square(10.0, 10.0, 40.0);
    open.pop();
    let open = doc(&open);
    let parse = |json: &str| serde_json::from_str::<Assertion>(json).expect("an assertion");
    let cases = [
        (
            r#"{"kind": "circles", "r": 3, "count": 1, "tol": 0.05}"#,
            true,
            false,
        ),
        (
            r#"{"kind": "circles", "r": 3.2, "count": 1, "tol": 0.05}"#,
            false,
            false,
        ),
        (
            r#"{"kind": "circles", "r": 3, "count": 0, "tol": 0.05}"#,
            false,
            true,
        ),
        (
            r#"{"kind": "bbox", "w": 40, "h": 40, "tol": 0.1}"#,
            true,
            true,
        ),
        (
            r#"{"kind": "bbox", "w": 40, "h": 30, "tol": 0.1}"#,
            false,
            false,
        ),
        (r#"{"kind": "closed"}"#, true, false),
        (r#"{"kind": "min_entities", "n": 0}"#, true, true),
        (r#"{"kind": "min_entities", "n": 5}"#, true, false),
        (r#"{"kind": "min_entities", "n": 3}"#, true, true),
    ];
    for (json, on_closed, on_open) in cases {
        let a = parse(json);
        assert_eq!(a.passes(&closed), on_closed, "{json} on the closed drawing");
        assert_eq!(a.passes(&open), on_open, "{json} on the open drawing");
    }
    assert!(serde_json::from_str::<Assertion>(r#"{"kind": "area"}"#).is_err());
    let empty = doc(&[]);
    assert!(
        !parse(r#"{"kind": "closed"}"#).passes(&empty),
        "nothing drawn is not closed"
    );
}
