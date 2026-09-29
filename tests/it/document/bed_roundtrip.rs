//! LCV-114 AC 6 / AC 7 / AC 10 — the bed survives a round trip, and the world
//! coordinates survive with it.
//!
//! This is the file that guards the demand's central hazard: if any leg of the
//! SVG pipeline keeps a constant 400 mm mirror axis while the document is on
//! another bed, the exported file still looks plausible in a text editor and
//! still imports without error — it simply cuts `400 − actual_height` mm away
//! from where the operator drew it. The default-bed twin below would not
//! notice that at all, which is exactly why the non-default case is here.
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::document::{Document, Entity};
use lasercad::geometry::{Arc, Circle, Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg, Preset};
use lasercad::util::{DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

/// Millimetres. The demand asks for 1e-9; the exporter quantises coordinates
/// to four decimals, so a value that survives the text format exactly is
/// compared exactly and one that does not is stated as such.
const TOL_MM: f64 = 1e-9;

fn doc_with(bed_mm: [f64; 2], entities: Vec<Entity>) -> Document {
    Document {
        entities,
        bed_mm,
        ..Document::default()
    }
}

fn line(y: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(10.0, y), Vec2::new(250.0, y)))
}

/// AC 6 / AC 7 — a document on a 300 × 180 bed exports and re-imports onto
/// the same bed, with every Y back where it started.
#[test]
fn roundtrip_at_a_non_default_bed_preserves_world_coordinates() {
    let doc = doc_with([300.0, 180.0], vec![line(50.0)]);
    let svg = export_svg(&doc, Preset::Cut);

    // The header states the document's bed, in both places LaserGRBL reads.
    assert!(svg.contains(r#"width="300mm""#), "{svg}");
    assert!(svg.contains(r#"height="180mm""#), "{svg}");
    assert!(svg.contains(r#"viewBox="0 0 300 180""#), "{svg}");
    // 180 − 50 = 130. Mirroring around the default bed would write 350.
    assert!(svg.contains(r#"y1="130.0000""#), "{svg}");
    assert!(!svg.contains("350.0000"), "mirrored around 400: {svg}");

    let imported = import_svg(&svg).expect("the exporter's own output must import");
    assert_eq!(
        imported.bed_mm,
        [300.0, 180.0],
        "the opened document must adopt the file's bed"
    );
    let Entity::Line(l) = imported.entities[0] else {
        panic!("expected a line back")
    };
    assert!((l.p1.y - 50.0).abs() < TOL_MM, "p1.y = {}", l.p1.y);
    assert!((l.p2.y - 50.0).abs() < TOL_MM, "p2.y = {}", l.p2.y);
    assert!((l.p1.x - 10.0).abs() < TOL_MM, "p1.x = {}", l.p1.x);
    assert!((l.p2.x - 250.0).abs() < TOL_MM, "p2.x = {}", l.p2.x);
}

/// AC 6 — the regression twin: the default bed behaves exactly as it did
/// before this demand.
#[test]
fn roundtrip_at_the_default_bed() {
    let doc = doc_with(
        [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM],
        vec![line(50.0)],
    );
    let svg = export_svg(&doc, Preset::Cut);
    assert!(svg.contains(r#"width="400mm""#), "{svg}");
    assert!(svg.contains(r#"height="400mm""#), "{svg}");
    assert!(svg.contains(r#"y1="350.0000""#), "{svg}");

    let imported = import_svg(&svg).unwrap();
    assert_eq!(imported.bed_mm, [400.0, 400.0]);
    let Entity::Line(l) = imported.entities[0] else {
        panic!("expected a line back")
    };
    assert!((l.p1.y - 50.0).abs() < TOL_MM);
    assert!((l.p2.y - 50.0).abs() < TOL_MM);
}

/// AC 6 / AC 7 — circles and arcs mirror around the same axis as lines: a
/// centre 20 mm above the bed floor must come back 20 mm above it.
#[test]
fn roundtrip_at_a_non_default_bed_preserves_circles_and_arcs() {
    let doc = doc_with(
        [128.0, 128.0],
        vec![
            Entity::Circle(Circle::new(Vec2::new(30.0, 20.0), 5.0)),
            Entity::Arc(Arc::new(
                Vec2::new(60.0, 20.0),
                10.0,
                0.0,
                std::f64::consts::FRAC_PI_2,
                true,
            )),
        ],
    );
    let imported = import_svg(&export_svg(&doc, Preset::Cut)).unwrap();
    assert_eq!(imported.bed_mm, [128.0, 128.0]);

    let Entity::Circle(c) = imported.entities[0] else {
        panic!("expected a circle back")
    };
    assert!(
        (c.center.y - 20.0).abs() < TOL_MM,
        "centre.y = {}",
        c.center.y
    );
    assert!((c.center.x - 30.0).abs() < TOL_MM);
    assert!((c.r - 5.0).abs() < TOL_MM);

    let Entity::Arc(a) = imported.entities[1] else {
        panic!("expected an arc back")
    };
    assert!(
        (a.center.y - 20.0).abs() < TOL_MM,
        "centre.y = {}",
        a.center.y
    );
    assert!((a.center.x - 60.0).abs() < TOL_MM);
    assert!(a.ccw, "handedness must survive the mirror");
}

/// AC 10's file-level half — a file authored at 300 × 180, opened and
/// re-exported unchanged, comes back out at 300 × 180 with the same bytes.
#[test]
fn reexporting_an_imported_file_is_byte_stable() {
    let doc = doc_with([300.0, 180.0], vec![line(50.0), line(90.0)]);
    let first = export_svg(&doc, Preset::Cut);
    let imported = import_svg(&first).unwrap();
    let reopened = doc_with(imported.bed_mm, imported.entities);
    assert_eq!(export_svg(&reopened, Preset::Cut), first);
}

/// AC 16 — the AGENTS.md SVG-export checklist must describe what the code now
/// does: the header is the document's bed and the mirror axis is a parameter.
///
/// The haystack is bounded to that one section — this file's own text and
/// every other section are outside it — and the absence claims sit next to
/// positive ones over the same slice, so the scan cannot pass vacuously: it
/// fails on the `flip_y(y, bed_height)` control long before it reaches them.
#[test]
fn agents_md_svg_section_describes_the_document_bed() {
    const AGENTS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/AGENTS.md"));
    let start = AGENTS
        .find("### SVG export (LaserGRBL compatibility)")
        .expect("AGENTS.md must keep its SVG export checklist");
    let end = AGENTS[start..]
        .find("\n## ")
        .expect("the checklist must be followed by another section")
        + start;
    let section = &AGENTS[start..end];

    assert!(
        section.contains("the document's bed size in mm"),
        "the header line must say the bed is the document's"
    );
    assert!(
        section.contains("flip_y(y_world, bed_height)"),
        "the mirror line must show the height as a parameter"
    );
    assert!(
        section.contains(r#"stroke-width="0.1""#),
        "positive control: the rest of the checklist is still in the haystack"
    );
    for stale in ["`BED_WIDTH_MM`", "`BED_HEIGHT_MM`", "<BED_WIDTH_MM>"] {
        assert!(
            !section.contains(stale),
            "the checklist still names the removed constant {stale}"
        );
    }
}
