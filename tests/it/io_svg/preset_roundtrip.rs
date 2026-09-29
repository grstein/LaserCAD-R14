//! LCV-115 AC 9 — a preset survives the file.
//!
//! This is the demand's destructive-failure guard, at kernel level: if the
//! preset does not come back out of the file, every existing mark or engrave
//! drawing silently becomes a cut job on its first re-save, at cutting power,
//! through the workpiece. The file still opens, still looks right, and still
//! draws the same geometry — the only visible difference is a colour.
//!
//! `action_open` / `action_open_path` / `action_save` are deliberately **not**
//! called here: this file's job is the kernel pipeline, with no `App` and no
//! filesystem. Their `app`-level wiring is covered in
//! `src/io/file_actions.rs` — `both_open_paths_adopt_the_file_preset` is a
//! behavioural test against a temporary directory since LCV-119 injected the
//! persistence paths (ADR 0006), while `open_via_the_dialog_adopts_the_file_preset`
//! and `both_save_paths_export_in_the_session_preset` remain bounded source
//! scans because `action_open` and `action_save_as` open a native dialog that
//! ADR 0005 keeps unreachable from any test.
//!
//! Kernel-only: no `App`, no egui, no filesystem.

use lasercad::document::{Document, Entity};
use lasercad::geometry::{Line, Vec2};
use lasercad::io::svg::{export_svg, import_svg, Preset};

/// Millimetres.
const TOL_MM: f64 = 1e-9;

fn doc_with(bed_mm: [f64; 2], entities: Vec<Entity>) -> Document {
    let mut doc = Document::with_bed(bed_mm);
    entities.into_iter().for_each(|e| doc.push_current(e));
    doc
}

fn only_line(entities: &[Entity]) -> Line {
    assert_eq!(entities.len(), 1, "expected exactly one entity");
    match entities[0] {
        Entity::Line(l) => l,
        ref other => panic!("expected a line, got {other:?}"),
    }
}

/// AC 9 — export with `Mark`, import, re-export with the imported preset: the
/// geometry stays in the `mark` group and the world coordinates stay put.
///
/// The bed is deliberately non-default (300 × 180): a preset bug and a mirror
/// bug look identical at 400 × 400, where `flip_y` is its own inverse around
/// the number the exporter would hard-code.
#[test]
fn mark_export_reimports_as_mark_and_survives_a_resave() {
    let drawn = Line::new(Vec2::new(10.0, 50.0), Vec2::new(250.0, 120.0));
    let doc = doc_with([300.0, 180.0], vec![Entity::Line(drawn)]);

    let first = export_svg(&doc, Preset::Mark);
    assert!(first.contains(r#"<g id="mark""#), "{first}");

    let imported = import_svg(&first).expect("the exporter's own output imports");
    assert_eq!(imported.preset, Preset::Mark, "the file carries its preset");
    assert_eq!(imported.bed_mm, [300.0, 180.0]);

    // Re-save exactly as `action_save` would: the session preset is the one
    // the open adopted, not a fresh `Cut`.
    let reopened = doc_with(imported.bed_mm, imported.entities);
    let second = export_svg(&reopened, imported.preset);

    assert_eq!(
        second, first,
        "a re-save is byte-identical, not re-coloured"
    );

    let back = only_line(&import_svg(&second).unwrap().entities);
    for (got, want) in [
        (back.p1.x, drawn.p1.x),
        (back.p1.y, drawn.p1.y),
        (back.p2.x, drawn.p2.x),
        (back.p2.y, drawn.p2.y),
    ] {
        assert!((got - want).abs() < TOL_MM, "{got} != {want}");
    }
}

/// AC 9 — the failure this demand exists to prevent, stated as a test: a
/// marking file re-saved with the *default* preset moves its geometry into the
/// `cut` group. If `import_svg` ever stops reporting the preset, `imported
/// .preset` becomes `Cut` and the first assertion below starts matching the
/// second — which is why the two are asserted against each other.
#[test]
fn resaving_with_the_default_preset_would_recolour_the_file() {
    let doc = doc_with(
        [300.0, 180.0],
        vec![Entity::Line(Line::new(
            Vec2::new(10.0, 50.0),
            Vec2::new(250.0, 120.0),
        ))],
    );
    let marked = export_svg(&doc, Preset::Mark);
    let imported = import_svg(&marked).unwrap();

    let faithful = export_svg(&doc, imported.preset);
    let recoloured = export_svg(&doc, Preset::default());

    assert_ne!(
        faithful, recoloured,
        "adopting the file's preset must differ from defaulting to Cut"
    );
    assert!(faithful.contains(r##"<g id="mark" stroke="#0000ff""##));
    assert!(recoloured.contains(r##"<g id="cut" stroke="#ff0000""##));
}

/// AC 3 / AC 9 — all three presets round-trip, and each puts the geometry in
/// exactly one group while the other two stay empty.
#[test]
fn every_preset_round_trips_into_its_own_group_alone() {
    let doc = doc_with(
        [300.0, 180.0],
        vec![Entity::Line(Line::new(
            Vec2::new(10.0, 50.0),
            Vec2::new(250.0, 120.0),
        ))],
    );

    for preset in Preset::ALL {
        let svg = export_svg(&doc, preset);
        assert_eq!(
            svg.matches("<line ").count(),
            1,
            "the line is written once, {preset:?}"
        );
        for other in Preset::ALL {
            let open = format!(r#"<g id="{}" stroke="{}""#, other.id(), other.color());
            assert!(svg.contains(&open), "group {} present, {svg}", other.id());
        }
        let imported = import_svg(&svg).unwrap();
        assert_eq!(imported.preset, preset);
        assert_eq!(imported.entities.len(), 1);
    }
}
