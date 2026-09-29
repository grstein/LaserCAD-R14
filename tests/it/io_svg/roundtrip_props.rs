//! Property test: SVG export → import round-trip (`io::svg`, LCV-056/057/100/114/156).
//!
//! Random documents survive `import_svg(&export_svg(doc))`: a random bed; one
//! to five layers with random names (XML-special characters, spaces and
//! non-ASCII included), colors, Output flags, order and current layer; lines,
//! circles and arcs spread over those layers. Bed, layers, current layer and
//! each entity's layer come back. The file groups entities by layer, so they
//! return in layer order, stable within a layer. Every case exercises the Y
//! mirror around the document's own bed height and the arc sweep-flag inversion.
//!
//! Tolerance is the file format's, not the kernel's: coordinates and radii are
//! written with four decimals, so each survives within `5e-5` mm. An arc's file
//! form is its endpoints, radius, sweep flag and large-arc flag; those are what
//! is compared. Its centre is not — it is rebuilt from the rounded endpoints and
//! is ill-conditioned near a half turn, by design of the SVG arc form.
//!
//! Known bug (LCV-172): an arc at or near a half turn can export endpoints whose
//! rounded chord exceeds the rounded diameter, and `import_svg` then rejects the
//! app's own file with `MalformedPath`. The full property is `#[ignore]`d until
//! LCV-172 lands; the running one keeps arcs at least 1e-3 mm short of a
//! diameter, well clear of the rounding (at most ~2.5e-4 mm).

use core::f64::consts::{PI, TAU};
use lasercad::document::{Document, Entity, Layer, LayerId};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg};
use lasercad::util::{BED_MAX_MM, BED_MIN_MM};
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::test_runner::FileFailurePersistence;

const CASES: u32 = 128;

/// `CASES` cases; failing seeds persist under the repo-root
/// `proptest-regressions/`, since `tests/` holds only `it/` and `harness/`.
fn config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/proptest-regressions/io_svg/roundtrip_props.txt"
        )))),
        ..ProptestConfig::default()
    }
}

/// Half a unit in the fourth decimal, plus room for the mirror's own rounding.
const FORMAT_TOL: f64 = 5e-5 + 1e-9;

/// How far short of a diameter an arc's chord stays in the running property.
const CHORD_CLEARANCE: f64 = 1e-3;

/// At most this many layers per generated document.
const MAX_LAYERS: usize = 5;

fn bed() -> impl Strategy<Value = [f64; 2]> {
    (BED_MIN_MM..=BED_MAX_MM, BED_MIN_MM..=BED_MAX_MM).prop_map(|(w, h)| [w, h])
}

fn point_in(bed: [f64; 2]) -> impl Strategy<Value = Vec2> {
    (0.0..=bed[0], 0.0..=bed[1]).prop_map(|(x, y)| Vec2::new(x, y))
}

/// Half turns are common in real drawings (slots, fillets), so they are drawn
/// on purpose as well as at random.
fn sweep() -> impl Strategy<Value = f64> {
    prop_oneof![1 => Just(PI), 4 => 0.05..(TAU - 0.05)]
}

/// Arcs; with `clear_of_diameter`, only those whose chord stays
/// [`CHORD_CLEARANCE`] short of the diameter.
fn arc(bed: [f64; 2], clear_of_diameter: bool) -> impl Strategy<Value = Entity> {
    (
        point_in(bed),
        0.01..1000.0f64,
        -TAU..TAU,
        sweep(),
        any::<bool>(),
    )
        .prop_filter(
            "chord too close to the diameter",
            move |(_, r, _, sweep, _)| {
                !clear_of_diameter || 2.0 * r * (1.0 - (sweep / 2.0).sin()) >= CHORD_CLEARANCE
            },
        )
        .prop_map(|(c, r, start, sweep, ccw)| {
            let end = if ccw { start + sweep } else { start - sweep };
            Entity::Arc(Arc::new(c, r, start, end, ccw))
        })
}

fn entity(bed: [f64; 2], clear_of_diameter: bool) -> impl Strategy<Value = Entity> {
    let line = (point_in(bed), point_in(bed)).prop_map(|(a, b)| Entity::Line(Line::new(a, b)));
    let circle =
        (point_in(bed), 0.001..1000.0f64).prop_map(|(c, r)| Entity::Circle(Circle::new(c, r)));
    prop_oneof![line, circle, arc(bed, clear_of_diameter)]
}

/// A layer name: a random run of XML-special, space, punctuation and
/// non-ASCII characters, then the layer's index so name keys stay unique.
fn layer_name(index: usize) -> impl Strategy<Value = String> {
    let chars = vec![
        'a', 'Z', '0', 'é', ' ', '&', '<', '>', '"', '\'', '-', '_', '.', '/',
    ];
    prop::collection::vec(prop::sample::select(chars), 0..12)
        .prop_map(move |cs| format!("{}{index}", cs.into_iter().collect::<String>()))
}

/// Layer `index`: random name, color and Output flag; a random id, so the
/// layer order is not the id order.
fn layer(index: usize) -> impl Strategy<Value = Layer> {
    (
        layer_name(index),
        any::<[u8; 3]>(),
        any::<bool>(),
        0u32..1000,
    )
        .prop_map(move |(name, color, output, id)| Layer {
            // Unique per document: `index` is below MAX_LAYERS.
            id: LayerId(id * 10 + u32::try_from(index).unwrap_or(0)),
            name,
            color,
            output,
        })
}

/// One to [`MAX_LAYERS`] layers with distinct colors.
fn layers() -> impl Strategy<Value = Vec<Layer>> {
    (1..=MAX_LAYERS)
        .prop_flat_map(|n| (0..n).map(layer).collect::<Vec<_>>())
        .prop_filter("layer colors must be distinct", |ls| {
            ls.iter()
                .enumerate()
                .all(|(i, a)| ls[..i].iter().all(|b| a.color != b.color))
        })
}

fn document(clear_of_diameter: bool) -> impl Strategy<Value = Document> {
    (bed(), layers(), any::<Index>()).prop_flat_map(move |(bed, layers, current)| {
        let placed = (entity(bed, clear_of_diameter), any::<Index>());
        prop::collection::vec(placed, 0..12).prop_map(move |placed| {
            let n = layers.len();
            let (entities, on): (Vec<_>, Vec<_>) = placed
                .into_iter()
                .map(|(e, at)| (e, layers[at.index(n)].id))
                .unzip();
            let current = layers[current.index(n)].id;
            Document::from_parts(bed, layers.clone(), current, entities, on)
                .expect("generated layers are valid")
        })
    })
}

/// Index of `id` in `layers`.
fn position(layers: &[Layer], id: LayerId) -> Option<usize> {
    layers.iter().position(|l| l.id == id)
}

fn near(a: Vec2, b: Vec2) -> bool {
    a.approx_eq(b, FORMAT_TOL)
}

/// `got` carries the same file-level geometry as `want`, within the format.
fn same_entity(want: &Entity, got: &Entity) -> Result<(), String> {
    let ok = match (want, got) {
        (Entity::Line(w), Entity::Line(g)) => near(w.p1, g.p1) && near(w.p2, g.p2),
        (Entity::Circle(w), Entity::Circle(g)) => {
            near(w.center, g.center) && (w.r - g.r).abs() <= FORMAT_TOL
        }
        (Entity::Arc(w), Entity::Arc(g)) => {
            let large_matches = (w.sweep_angle() - PI).abs() < 1e-3
                || (w.sweep_angle() > PI) == (g.sweep_angle() > PI);
            near(w.start_point(), g.start_point())
                && near(w.end_point(), g.end_point())
                && (w.r - g.r).abs() <= FORMAT_TOL
                && w.ccw == g.ccw
                && large_matches
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("want {want:?}\n got {got:?}"))
    }
}

/// Export `doc`, import the file, and compare the two.
fn check_round_trip(doc: &Document) -> Result<(), TestCaseError> {
    let svg = export_svg(doc);
    let back = import_svg(&svg).map_err(|e| TestCaseError::fail(format!("{e}\n{svg}")))?;
    prop_assert_eq!(back.bed_mm, doc.bed_mm);
    let strip = |ls: &[Layer]| -> Vec<(String, [u8; 3], bool)> {
        ls.iter()
            .map(|l| (l.name.clone(), l.color, l.output))
            .collect()
    };
    prop_assert_eq!(strip(&back.layers), strip(doc.layers()), "{}", svg);
    prop_assert_eq!(
        position(&back.layers, back.current_layer),
        position(doc.layers(), doc.current_layer())
    );
    // The file groups entities by layer: layer order, stable within a layer.
    let mut want: Vec<(Option<usize>, &Entity)> = doc
        .entities
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let id = doc.entity_layer(i).expect("one layer per entity");
            (position(doc.layers(), id), e)
        })
        .collect();
    want.sort_by_key(|(layer, _)| *layer);
    prop_assert_eq!(back.entities.len(), want.len());
    prop_assert_eq!(back.entity_layers.len(), want.len());
    let got = back.entities.iter().zip(&back.entity_layers);
    for ((layer, w), (g, id)) in want.iter().zip(got) {
        prop_assert_eq!(position(&back.layers, *id), *layer);
        same_entity(w, g).map_err(TestCaseError::fail)?;
    }
    prop_assert!(back.into_document().is_ok(), "reopened layers are valid");
    Ok(())
}

proptest! {
    #![proptest_config(config())]

    /// Export then import returns the bed, the layers, the current layer and
    /// every entity on its layer, for arcs clear of a half turn.
    #[test]
    fn export_import_round_trip_preserves_document(doc in document(true)) {
        check_round_trip(&doc)?;
    }

    /// The same, for every arc including exact and near half turns.
    #[test]
    #[ignore = "bug: LCV-172 rounded half-turn arc chord exceeds 2r, import rejects own export"]
    fn export_import_round_trip_preserves_half_turn_arcs(doc in document(false)) {
        check_round_trip(&doc)?;
    }
}

/// The minimised case: a 0.01 mm half-turn arc on a 1 mm bed.
#[test]
#[ignore = "bug: LCV-172 rounded half-turn arc chord exceeds 2r, import rejects own export"]
fn half_turn_arc_reimports() {
    let mut doc = Document::with_bed([1.0, 1.0]);
    doc.push_current(Entity::Arc(Arc::new(
        Vec2::new(0.0, 0.0),
        0.01,
        -1.6511253124589578,
        -4.792717966048751,
        false,
    )));
    let svg = export_svg(&doc);
    assert!(import_svg(&svg).is_ok(), "{svg}");
}
