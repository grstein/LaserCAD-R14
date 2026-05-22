//! Crash-recovery autosave: debounced periodic save to the platform data dir,
//! restored on the next boot if the file is present.
//!
//! ## Two-layer API
//!
//! - **Public** [`save_autosave`] / [`load_autosave`] / [`clear_autosave`]: resolve
//!   the platform path via [`directories::ProjectDirs`] and delegate to the
//!   path-injected variants.
//! - **`pub(crate)`** [`save_autosave_to`] / [`load_autosave_from`]: accept an
//!   explicit [`std::path::Path`] so unit tests can inject a temporary path.
//!
//! ## On-disk format
//!
//! A JSON object `{ "schema_version": 1, "entities": [...] }`.  Only entities
//! are persisted — selection and history are ephemeral and are not included.
//! The schema version must match [`crate::document::entity::SCHEMA_VERSION`];
//! any mismatch causes [`load_autosave`] to return `None` (safe silent
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

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the autosave *write* path.
///
/// The *read* path is infallible — it returns `None` instead.
#[derive(Debug, thiserror::Error)]
pub enum AutosaveError {
    /// The platform data directory could not be resolved.
    #[error("could not determine platform data directory")]
    NoPlatformPath,

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
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Persist `doc` to the platform autosave file.
///
/// Uses an atomic rename: writes to a `.tmp` sibling first, then renames to
/// the final path to avoid leaving a corrupt file on a mid-write crash.
/// Returns an error if the platform data directory cannot be resolved or if
/// I/O fails; the caller should log but not surface this to the user.
pub fn save_autosave(doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
    let path = platform_path().ok_or(AutosaveError::NoPlatformPath)?;
    save_autosave_to(doc, &path).map_err(Into::into)
}

/// Restore a document from the platform autosave file.
///
/// Returns `None` on any error: missing file, parse failure, version mismatch,
/// or no platform directory.  This is intentionally infallible so boot always
/// succeeds.
pub fn load_autosave() -> Option<Document> {
    let path = platform_path()?;
    load_autosave_from(&path)
}

/// Delete the platform autosave file if it exists.
///
/// Silently ignores errors (file already absent, permission denied, etc.).
pub fn clear_autosave() {
    if let Some(path) = platform_path() {
        let _ = std::fs::remove_file(path);
    }
}

// ---------------------------------------------------------------------------
// Path-injected helpers (pub(crate) for testing)
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
        ..Document::default()
    })
}

// ---------------------------------------------------------------------------
// Platform path resolution
// ---------------------------------------------------------------------------

/// Returns `{data_dir}/lasercad/autosave.json`, or `None` if the platform
/// cannot supply a data directory.
fn platform_path() -> Option<PathBuf> {
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
}
