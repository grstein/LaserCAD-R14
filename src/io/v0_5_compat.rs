//! LCV-202 AC 4 — files written by v0.5.0 open in 1.0 without loss or error.
//!
//! `tests/fixtures/v0_5/` holds a `settings.json`, an `autosave.json` and a
//! mother `mother.svg` written by v0.5.0 itself (commit 8c6701d), from a
//! throwaway ignored test in that commit's `src/io/mod.rs` test module, run
//! in a scratch worktree with `V05_OUT` set to the fixture folder:
//!
//! ```text
//! let settings = Settings {
//!     recent_files: vec!["/home/op/jobs/plate.svg".into(), "/home/op/jobs/box & lid.svg".into()],
//!     agent_endpoint: "http://192.0.2.10:8080/v1".into(),
//!     agent_api_key: "fixture-not-a-key".into(),
//!     agent_model: "fixture/model-v05".into(),
//!     agent_step_budget: 64,
//!     default_bed_mm: [320.0, 210.0],
//!     agent_system_prompt: Some("Draw only what is asked.".into()),
//!     agent_allow_canvas_capture: true,
//!     agent_model_supports_vision: true,
//!     agent_context_tokens: 32_000,
//!     object_snaps: SnapKinds { midpoint: false, tangent: false, nearest: true, ..SnapKinds::default() },
//! };
//! super::settings_store::save_to(&settings, &out.join("settings.json")).unwrap();
//! let (cut, mark, engrave) = (LayerId(0), LayerId(1), LayerId(2));
//! let layers = vec![
//!     Layer::default_cut(),
//!     Layer { id: mark, name: "Mark & <score>".into(), color: [0, 0, 255], output: false },
//!     Layer { id: engrave, name: "Engrave".into(), color: [0, 170, 0], output: true },
//! ];
//! let v = Vec2::new;
//! let entities = vec![
//!     Entity::Line(Line::new(v(10.0, 15.0), v(120.5, 15.0))),
//!     Entity::Circle(Circle::new(v(60.0, 80.0), 12.25)),
//!     Entity::Arc(Arc::new(v(200.0, 100.0), 30.0, 0.5, 2.5, true)),
//!     Entity::Line(Line::new(v(120.5, 15.0), v(120.5, 90.0))),
//!     Entity::Arc(Arc::new(v(250.0, 50.0), 15.0, 3.0, 1.0, false)),
//! ];
//! let entity_layers = vec![cut, mark, cut, mark, cut];
//! let doc = Document::from_parts([320.0, 210.0], layers, mark, entities, entity_layers).unwrap();
//! crate::io::autosave::save_autosave_to(&doc, &out.join("autosave.json")).unwrap();
//! std::fs::write(out.join("mother.svg"), crate::io::export_svg(&doc)).unwrap();
//! ```
//!
//! Each test loads a tempdir copy, because `settings_store::load_from`
//! renames a corrupt file to `.bak` beside it. A fixture that fails here is
//! a compatibility bug to fix in code; the fixtures are never edited.

use std::path::{Path, PathBuf};

use crate::document::{Document, Entity, Layer, LayerId};
use crate::geometry::{Arc, Circle, Line, SnapKinds, Vec2};
use crate::io::settings::Settings;

/// Half a unit in the fourth decimal SVG export writes, plus float room.
const FORMAT_TOL: f64 = 5e-5 + 1e-9;

/// A copy of fixture `name` in a fresh temp folder.
fn fixture_copy(name: &str) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_5")
        .join(name);
    let dir = std::env::temp_dir().join(format!("lcv202_v05_{}", name.replace('.', "_")));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let dst = dir.join(name);
    std::fs::copy(&src, &dst).expect("fixture copied");
    dst
}

/// The document the v0.5.0 generator built.
fn v05_doc() -> Document {
    let (cut, mark, engrave) = (LayerId(0), LayerId(1), LayerId(2));
    let layers = vec![
        Layer::default_cut(),
        Layer {
            id: mark,
            name: "Mark & <score>".into(),
            color: [0, 0, 255],
            output: false,
        },
        Layer {
            id: engrave,
            name: "Engrave".into(),
            color: [0, 170, 0],
            output: true,
        },
    ];
    let v = Vec2::new;
    let entities = vec![
        Entity::Line(Line::new(v(10.0, 15.0), v(120.5, 15.0))),
        Entity::Circle(Circle::new(v(60.0, 80.0), 12.25)),
        Entity::Arc(Arc::new(v(200.0, 100.0), 30.0, 0.5, 2.5, true)),
        Entity::Line(Line::new(v(120.5, 15.0), v(120.5, 90.0))),
        Entity::Arc(Arc::new(v(250.0, 50.0), 15.0, 3.0, 1.0, false)),
    ];
    let entity_layers = vec![cut, mark, cut, mark, cut];
    Document::from_parts([320.0, 210.0], layers, mark, entities, entity_layers)
        .expect("valid v0.5 document")
}

/// The points an SVG states for `e`, plus the arc midpoint (which pins the
/// sweep flag); `None` for a kind v0.5 could not write.
fn points(e: &Entity) -> Option<Vec<Vec2>> {
    Some(match e {
        Entity::Line(l) => vec![l.p1, l.p2],
        Entity::Circle(c) => vec![c.center, Vec2::new(c.r, 0.0)],
        Entity::Arc(a) => {
            let half = if a.ccw { 0.5 } else { -0.5 } * a.sweep_angle();
            let mid = Circle::new(a.center, a.r).point_at_angle(a.start_angle + half);
            vec![a.start_point(), a.end_point(), Vec2::new(a.r, 0.0), mid]
        }
        _ => return None,
    })
}

/// AC 4 — every v0.5 setting survives; the field added since loads its
/// default and the file is neither rewritten nor backed up.
#[test]
fn v0_5_settings_load_without_loss() {
    let path = fixture_copy("settings.json");
    let before = std::fs::read(&path).expect("readable");
    let got = super::settings_store::load_from(&path);
    let want = Settings {
        recent_files: vec![
            "/home/op/jobs/plate.svg".into(),
            "/home/op/jobs/box & lid.svg".into(),
        ],
        agent_endpoint: "http://192.0.2.10:8080/v1".into(),
        agent_api_key: "fixture-not-a-key".into(),
        agent_model: "fixture/model-v05".into(),
        agent_step_budget: 64,
        default_bed_mm: [320.0, 210.0],
        agent_system_prompt: Some("Draw only what is asked.".into()),
        agent_allow_canvas_capture: true,
        agent_model_supports_vision: true,
        agent_feedback_after_changes: false,
        agent_context_tokens: 32_000,
        object_snaps: SnapKinds {
            midpoint: false,
            tangent: false,
            nearest: true,
            ..SnapKinds::default()
        },
    };
    assert_eq!(got, want);
    assert_eq!(
        std::fs::read(&path).expect("still there"),
        before,
        "untouched"
    );
    assert!(!path.with_extension("json.bak").exists(), "no backup made");
}

/// AC 4 — the v0.5 autosave restores bed, layers, current layer,
/// membership and every entity exactly.
#[test]
fn v0_5_autosave_restores_without_loss() {
    let path = fixture_copy("autosave.json");
    let got = super::autosave::load_autosave_from(&path).expect("v0.5 autosave restores");
    let want = v05_doc();
    assert_eq!(got.bed_mm, want.bed_mm);
    assert_eq!(got.layers(), want.layers());
    assert_eq!(got.current_layer(), want.current_layer());
    assert_eq!(got.entities, want.entities);
    let layers = |d: &Document| {
        (0..d.entity_count())
            .map(|i| d.entity_layer(i))
            .collect::<Vec<_>>()
    };
    assert_eq!(layers(&got), layers(&want));
}

/// AC 4 — the v0.5 mother SVG opens with its bed, every layer (the empty
/// one included), the current layer, the membership and every entity
/// within `FORMAT_TOL` (midpoints within 1 µm), and reports nothing.
#[test]
fn v0_5_mother_svg_opens_without_loss() {
    let path = fixture_copy("mother.svg");
    let text = std::fs::read_to_string(&path).expect("readable");
    let imported = crate::io::import_svg(&text).expect("v0.5 mother imports");
    assert!(imported.report.is_empty(), "{:?}", imported.report);
    let got = imported.into_document().expect("valid layers");
    let want = v05_doc();
    assert_eq!(got.bed_mm, want.bed_mm);
    assert_eq!(got.layers(), want.layers());
    assert_eq!(got.current_layer(), want.current_layer());
    // The mother groups entities by layer, in layer order.
    let order: Vec<usize> = want
        .layers()
        .iter()
        .flat_map(|l| {
            (0..want.entity_count())
                .filter(|&i| want.entity_layer(i) == Some(l.id))
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(got.entity_count(), order.len());
    for (i, &k) in order.iter().enumerate() {
        assert_eq!(got.entity_layer(i), want.entity_layer(k), "entity {i}");
        let kind = std::mem::discriminant::<Entity>;
        assert_eq!(kind(&got.entities[i]), kind(&want.entities[k]), "entity {i}");
        let g = points(&got.entities[i]).expect("a v0.5 kind");
        let w = points(&want.entities[k]).expect("a v0.5 kind");
        for (n, (a, b)) in g.iter().zip(&w).enumerate() {
            let tol = if n == 3 { 1e-3 } else { FORMAT_TOL };
            assert!(a.approx_eq(*b, tol), "entity {i}: {a:?} vs {b:?}");
        }
    }
}
