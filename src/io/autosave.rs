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
//! A JSON object `{ "schema_version": 1, "entities": [...], "bed_mm": [W, H] }`.
//! Only entities and the bed size are persisted — selection and history are
//! ephemeral and are not included. `bed_mm` is additive and carries its own
//! serde default (LCV-114 AC 13), so an envelope written before that demand
//! still recovers, at the default bed; [`SCHEMA_VERSION`] is therefore **not**
//! bumped — the policy bumps it only on a breaking change.
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

use crate::document::{entity::SCHEMA_VERSION, Document, Entity};
use crate::util::{DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

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

/// JSON envelope written to disk.  Only entities are serialized; selection
/// and history are not persisted.
#[derive(Serialize, Deserialize)]
struct DocumentEnvelope {
    /// Schema version stamp — must equal [`SCHEMA_VERSION`] to be loadable.
    schema_version: u32,
    /// The placed entities, in insertion order.
    entities: Vec<Entity>,
    /// The document's bed size in millimetres `[width, height]` (LCV-114).
    /// Absent in envelopes written before LCV-114; those recover at the
    /// default bed rather than failing to load at all.
    #[serde(default = "default_envelope_bed_mm")]
    bed_mm: [f64; 2],
}

fn default_envelope_bed_mm() -> [f64; 2] {
    [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
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
        entities: doc.entities.clone(),
        bed_mm: doc.bed_mm,
    };
    let json = serde_json::to_string_pretty(&envelope)?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Restore a document from `path`.
///
/// Returns `None` if the file is absent, unparseable, or the schema version
/// does not match [`SCHEMA_VERSION`].
pub(crate) fn load_autosave_from(path: &Path) -> Option<Document> {
    let content = std::fs::read_to_string(path).ok()?;
    let envelope: DocumentEnvelope = serde_json::from_str(&content).ok()?;

    if envelope.schema_version != SCHEMA_VERSION {
        return None;
    }

    Some(Document {
        entities: envelope.entities,
        bed_mm: envelope.bed_mm,
        ..Document::default()
    })
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
        doc.entities.push(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 5.0),
        )));
        doc.entities
            .push(Entity::Circle(Circle::new(Vec2::new(20.0, 20.0), 3.0)));
        doc.entities.push(Entity::Arc(Arc::new(
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

    /// AC#7 — load_autosave_from on a valid empty-entities envelope returns
    /// a Document with no entities.
    #[test]
    fn load_empty_entities_returns_empty_document() {
        let path = tmp_path("lcv059_empty_entities.json");
        let json = format!(r#"{{"schema_version":{SCHEMA_VERSION},"entities":[]}}"#);
        std::fs::write(&path, json).unwrap();

        let doc = load_autosave_from(&path).unwrap();
        assert_eq!(doc.entity_count(), 0);
        let _ = std::fs::remove_file(&path);
    }

    /// LCV-114 AC 13 — an envelope written before this demand has no
    /// `bed_mm`; it must still load, at the default bed. Without the field
    /// default, a single stale autosave.json would make the app boot empty.
    #[test]
    fn envelope_without_bed_mm_loads_at_the_default() {
        let path = tmp_path("lcv114_no_bed.json");
        let json = format!(r#"{{"schema_version":{SCHEMA_VERSION},"entities":[]}}"#);
        std::fs::write(&path, json).unwrap();

        let doc = load_autosave_from(&path).expect("a pre-LCV-114 envelope must still load");
        assert_eq!(doc.bed_mm, [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]);
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

    /// LCV-114 AC 13 — `bed_mm` is additive and back-compatible, so the
    /// schema stamp does **not** move. The policy bumps it only on a breaking
    /// change; the test above is what makes that claim true.
    #[test]
    fn schema_version_is_unchanged_by_the_bed_field() {
        assert_eq!(SCHEMA_VERSION, 1);
    }
}
