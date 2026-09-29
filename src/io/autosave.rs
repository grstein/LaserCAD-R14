//! Crash-recovery autosave: debounced periodic save to the platform data dir,
//! restored on the next boot if the file is present.
//!
//! ## One path-injected API (ADR 0006)
//!
//! - [`platform_path()`] resolves the real per-user file through
//!   [`directories::ProjectDirs`]. It is called from exactly one place,
//!   `App::new()`, which stores the result on the `App` it returns; nothing
//!   below boot resolves a real user location.
//! - [`save_autosave_to`] / [`load_autosave_from`] / [`clear_autosave_at`]
//!   take an explicit [`std::path::Path`], so the only way to touch an
//!   autosave file is to hand over a path. A test points that path at a
//!   temporary directory; a process that was given no path
//!   (`App::default()`) neither writes nor deletes anything. Before LCV-119
//!   this module's pathless wrappers made `cargo test` delete the
//!   developer's own crash-recovery file on every run.
//!
//! ## On-disk format
//!
//! A JSON object `{ "schema_version": 2, "bed_mm": [W, H], "layers": [...],
//! "current_layer": id, "entities": [...], "entity_layers": [...] }`
//! (LCV-156, ADR 0012 §8), loaded through `Document::from_parts`. Selection
//! and history are ephemeral and are not included. Schema 1 (no layers) is
//! discarded once on upgrade.
//! The schema version must match [`crate::document::entity::SCHEMA_VERSION`];
//! any mismatch causes [`load_autosave_from`] to return `None` (safe silent
//! discard).
//!
//! ## Kernel purity
//!
//! This module MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Introduced by demand LCV-059.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::{Document, Entity, Layer, LayerId, entity::SCHEMA_VERSION};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the autosave *write* path.
///
/// The *read* path is infallible — it returns `None` instead.
#[derive(Debug, thiserror::Error)]
pub enum AutosaveError {
    /// JSON serialisation failed.
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem I/O failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// On-disk envelope
// ---------------------------------------------------------------------------

/// JSON envelope written to disk (schema 2, LCV-156). Selection and history
/// are not persisted.
#[derive(Serialize, Deserialize)]
struct DocumentEnvelope {
    /// Schema version stamp — must equal [`SCHEMA_VERSION`] to be loadable.
    schema_version: u32,
    /// The document's bed size in millimetres `[width, height]` (LCV-114).
    bed_mm: [f64; 2],
    /// The layers, in list order.
    layers: Vec<Layer>,
    /// The layer new entities land on.
    current_layer: LayerId,
    /// The placed entities, in insertion order.
    entities: Vec<Entity>,
    /// Each entity's layer, parallel to `entities`.
    entity_layers: Vec<LayerId>,
}

/// Just the stamp, read first so an old envelope is refused by version and
/// not by whichever field it happens to lack.
#[derive(Deserialize)]
struct VersionProbe {
    schema_version: u32,
}

// ---------------------------------------------------------------------------
// Path-injected API (ADR 0006 — the caller always supplies the path)
// ---------------------------------------------------------------------------

/// Persist `doc` to `path` using an atomic rename.
///
/// 1. Creates intermediate directories if they do not exist.
/// 2. Serialises to `path` with a `.tmp` extension.
/// 3. Renames the `.tmp` file to `path`.
pub(crate) fn save_autosave_to(doc: &Document, path: &Path) -> Result<(), AutosaveError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp_path: PathBuf = {
        let stem = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("autosave");
        let mut p = path.to_path_buf();
        p.set_file_name(format!("{stem}.tmp"));
        p
    };

    let envelope = DocumentEnvelope {
        schema_version: SCHEMA_VERSION,
        bed_mm: doc.bed_mm,
        layers: doc.layers().to_vec(),
        current_layer: doc.current_layer(),
        entities: doc.entities.clone(),
        entity_layers: (0..doc.entity_count())
            .filter_map(|i| doc.entity_layer(i))
            .collect(),
    };
    let json = serde_json::to_string_pretty(&envelope)?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Restore a document from `path`.
///
/// Returns `None` if the file is absent, unparseable, the schema version
/// does not match [`SCHEMA_VERSION`] (a v1 envelope included), or its layers
/// fail `Document::from_parts` validation.
pub(crate) fn load_autosave_from(path: &Path) -> Option<Document> {
    let content = std::fs::read_to_string(path).ok()?;
    let version: VersionProbe = serde_json::from_str(&content).ok()?;
    if version.schema_version != SCHEMA_VERSION {
        return None;
    }
    let e: DocumentEnvelope = serde_json::from_str(&content).ok()?;
    Document::from_parts(
        e.bed_mm,
        e.layers,
        e.current_layer,
        e.entities,
        e.entity_layers,
    )
    .ok()
}

/// Delete the autosave file at `path` if it exists.
///
/// Silently ignores every error (file already absent, permission denied,
/// read-only medium): crash recovery is best-effort and a failure to tidy up
/// must never abort the file action that asked for it.
pub(crate) fn clear_autosave_at(path: &Path) {
    let _ = std::fs::remove_file(path);
}

// ---------------------------------------------------------------------------
// Platform path resolution
// ---------------------------------------------------------------------------

/// Returns `{data_dir}/lasercad/autosave.json`, or `None` if the platform
/// cannot supply a data directory.
///
/// **Boot-only (ADR 0006).** The single caller is `App::new()`, which stores
/// the result on the `App` it returns and hands it to every later read,
/// write and delete. Calling this anywhere else reopens the defect LCV-119
/// closed: `cargo test` used to delete the developer's real
/// `~/.local/share/lasercad/autosave.json` through exactly this resolver.
pub(crate) fn platform_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "lasercad")
        .map(|dirs| dirs.data_local_dir().join("autosave.json"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line, Vec2};
    use std::f64::consts::FRAC_PI_2;

    fn tmp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn sample_doc() -> Document {
        let mut doc = Document::default();
        doc.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 5.0),
        )));
        doc.push_current(Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 3.0)));
        doc.push_current(Entity::Arc(Arc::new(
            Vec2::new(5.0, 5.0),
            2.0,
            0.0,
            FRAC_PI_2,
            true,
        )));
        doc
    }

    /// AC#1 — save_autosave_to + load_autosave_from round-trip preserves
    /// all entities in insertion order.
    #[test]
    fn round_trip_preserves_entities() {
        let path = tmp_path("lcv059_roundtrip.json");
        let original = sample_doc();

        save_autosave_to(&original, &path).unwrap();
        let loaded = load_autosave_from(&path).unwrap();

        assert_eq!(original.entities, loaded.entities);
        let _ = std::fs::remove_file(&path);
    }

    /// AC#2 — load_autosave_from on a missing file returns None.
    #[test]
    fn load_from_missing_file_returns_none() {
        let path = tmp_path("lcv059_no_such_file_xyz.json");
        let _ = std::fs::remove_file(&path);
        assert!(load_autosave_from(&path).is_none());
    }

    /// AC#3 — load_autosave_from on a malformed JSON file returns None.
    #[test]
    fn load_from_malformed_json_returns_none() {
        let path = tmp_path("lcv059_bad_json.json");
        std::fs::write(&path, b"{ not valid json }").unwrap();
        assert!(load_autosave_from(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }

    /// AC#4 — load_autosave_from rejects a mismatched schema_version.
    #[test]
    fn load_from_wrong_schema_version_returns_none() {
        let path = tmp_path("lcv059_wrong_version.json");
        let json = r#"{"schema_version":999,"entities":[]}"#;
        std::fs::write(&path, json).unwrap();
        assert!(load_autosave_from(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }

    /// AC#5 — save_autosave_to creates parent directories if missing.
    #[test]
    fn save_creates_parent_dirs() {
        let base = std::env::temp_dir().join("lcv059_nested_dir");
        let path = base.join("sub").join("autosave.json");
        let _ = std::fs::remove_dir_all(&base);

        save_autosave_to(&Document::default(), &path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// AC#6 — no stale .tmp file after a successful save.
    #[test]
    fn save_no_tmp_after_success() {
        let path = tmp_path("lcv059_atomic.json");
        let tmp = tmp_path("lcv059_atomic.json.tmp");

        save_autosave_to(&Document::default(), &path).unwrap();

        assert!(path.exists(), "final file must exist");
        assert!(!tmp.exists(), ".tmp must be gone after rename");
        let _ = std::fs::remove_file(&path);
    }

    /// AC#7 — load_autosave_from on a valid envelope with no entities
    /// returns a Document with no entities.
    #[test]
    fn load_empty_entities_returns_empty_document() {
        let path = tmp_path("lcv059_empty_entities.json");
        save_autosave_to(&Document::default(), &path).unwrap();

        let doc = load_autosave_from(&path).unwrap();
        assert_eq!(doc.entity_count(), 0);
        assert_eq!(doc.layers(), &[Layer::default_cut()]);
        let _ = std::fs::remove_file(&path);
    }

    /// LCV-114 AC 13 — a non-default bed survives the write/read cycle, so a
    /// crash recovery comes back on the operator's machine size.
    #[test]
    fn envelope_round_trips_bed_mm() {
        let path = tmp_path("lcv114_bed_roundtrip.json");
        let mut doc = sample_doc();
        doc.bed_mm = [300.0, 180.0];
        save_autosave_to(&doc, &path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("bed_mm"), "the envelope must carry it: {raw}");

        let back = load_autosave_from(&path).unwrap();
        assert_eq!(back.bed_mm, [300.0, 180.0]);
        assert_eq!(back.entity_count(), doc.entity_count());
        let _ = std::fs::remove_file(&path);
    }

    /// Two layers (the second current), entities on both.
    fn layered_doc() -> Document {
        let mark = Layer {
            id: LayerId(7),
            name: "Mark".into(),
            color: [0, 0, 255],
            output: false,
        };
        let entities = sample_doc().entities.clone();
        let members = vec![LayerId(7), LayerId(0), LayerId(7)];
        Document::from_parts(
            [300.0, 180.0],
            vec![Layer::default_cut(), mark],
            LayerId(7),
            entities,
            members,
        )
        .unwrap()
    }

    /// LCV-156 AC 13 — layers, the current layer and every entity's layer
    /// survive autosave and restore.
    #[test]
    fn autosave_round_trips_layers_current_and_membership() {
        let path = tmp_path("lcv156_layers_roundtrip.json");
        let doc = layered_doc();
        save_autosave_to(&doc, &path).unwrap();

        let back = load_autosave_from(&path).expect("a v2 envelope loads");
        assert_eq!(back.layers(), doc.layers());
        assert_eq!(back.current_layer(), LayerId(7));
        assert_eq!(back.entities, doc.entities);
        let members: Vec<_> = (0..3).map(|i| back.entity_layer(i)).collect();
        assert_eq!(
            members,
            vec![Some(LayerId(7)), Some(LayerId(0)), Some(LayerId(7))]
        );
        let _ = std::fs::remove_file(&path);
    }

    /// LCV-156 (ADR 0012 §8) — a v1 envelope, written before layers, is
    /// discarded rather than restored without them.
    #[test]
    fn v1_envelope_is_discarded() {
        let path = tmp_path("lcv156_v1_envelope.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"entities":[],"bed_mm":[400,400]}"#,
        )
        .unwrap();
        assert!(load_autosave_from(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }

    /// LCV-156 (ADR 0012 §8) — an envelope whose layers fail validation
    /// (membership out of lockstep, unknown current layer) is discarded.
    #[test]
    fn invalid_layer_envelope_is_discarded() {
        let path = tmp_path("lcv156_invalid_envelope.json");
        save_autosave_to(&layered_doc(), &path).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        for (key, bad) in [
            ("entity_layers", serde_json::json!([7, 0])),
            ("current_layer", serde_json::json!(99)),
            ("layers", serde_json::json!([])),
        ] {
            let mut broken = json.clone();
            broken[key] = bad;
            std::fs::write(&path, broken.to_string()).unwrap();
            assert!(load_autosave_from(&path).is_none(), "{key}");
        }
        json["schema_version"] = serde_json::json!(SCHEMA_VERSION);
        std::fs::write(&path, json.to_string()).unwrap();
        assert!(load_autosave_from(&path).is_some(), "positive control");
        let _ = std::fs::remove_file(&path);
    }

    /// LCV-156 — the envelope shape changed (layers), so the stamp moved.
    #[test]
    fn schema_version_is_two() {
        assert_eq!(SCHEMA_VERSION, 2);
    }
}
