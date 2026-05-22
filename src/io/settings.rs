//! User-preference persistence via a JSON file in the platform config directory.
//!
//! ## Two-layer API
//!
//! - **Public** [`load()`] / [`Settings::save()`]: resolve the platform path via
//!   [`directories::ProjectDirs`] and delegate to the path-injected variants.
//! - **`pub(crate)`** [`load_from()`] / [`save_to()`]: accept an explicit
//!   [`std::path::Path`] so unit tests can inject a temporary path.
//!
//! `load()` and `load_from()` are both **infallible** — they return
//! [`Settings::default()`] on any error (missing file, parse failure, no
//! platform dir).  `save` and `save_to` return a [`Result`] so callers can
//! log or surface the error.
//!
//! ## Atomic write
//!
//! [`save_to()`] writes to a `.tmp` sibling file then renames, so a crash
//! mid-write never corrupts the live settings file.
//!
//! ## Kernel purity
//!
//! This module MUST NOT import `egui`, `eframe`, or `rfd`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the settings *write* path.
///
/// The *read* path is infallible — it returns [`Settings::default()`] instead.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The platform could not supply a config directory (rare, but possible on
    /// some minimal Linux installations without `$HOME`).
    #[error("could not determine platform config directory")]
    NoPlatformPath,

    /// JSON serialisation failed.
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem I/O failed (create dir, write tmp, rename).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Settings struct
// ---------------------------------------------------------------------------

/// Persisted user preferences.
///
/// All fields carry `#[serde(default)]` so that a settings file written by an
/// older version of the app remains loadable after new fields are added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Ordered list of recently opened file paths (most recent first).
    ///
    /// Capped at [`RECENT_FILES_CAP`] entries.  Use [`push_recent_file`] to
    /// add entries; never push directly.
    pub recent_files: Vec<String>,

    /// OpenAI-compatible API base URL.
    /// Default: `"https://api.openai.com/v1"`.
    #[serde(default = "default_agent_endpoint")]
    pub agent_endpoint: String,

    /// API key sent in the `Authorization: Bearer …` header.
    /// Default: `""` (agent disabled until set).
    #[serde(default)]
    pub agent_api_key: String,
}

fn default_agent_endpoint() -> String {
    "https://api.openai.com/v1".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            agent_endpoint: default_agent_endpoint(),
            agent_api_key: String::new(),
        }
    }
}

/// Maximum number of entries kept in [`Settings::recent_files`].
const RECENT_FILES_CAP: usize = 10;

impl Settings {
    /// Insert `path` into the recent-files list.
    ///
    /// 1. Removes any existing occurrence of `path` (deduplication).
    /// 2. Inserts `path` at position 0 (most-recent first).
    /// 3. Truncates the list to [`RECENT_FILES_CAP`] entries.
    pub fn push_recent_file(&mut self, path: String) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(RECENT_FILES_CAP);
    }

    /// Load settings from the platform config directory (infallible).
    ///
    /// Returns [`Settings::default()`] if the platform directory is
    /// unavailable, the file does not exist, or parsing fails.
    pub fn load() -> Settings {
        match platform_path() {
            Some(path) => load_from(&path),
            None => Settings::default(),
        }
    }

    /// Persist settings to the platform config directory.
    ///
    /// Uses an atomic write: serialises to a `.tmp` sibling first, then
    /// renames to the final path.
    pub fn save(&self) -> Result<(), SettingsError> {
        let path = platform_path().ok_or(SettingsError::NoPlatformPath)?;
        save_to(self, &path)
    }
}

// ---------------------------------------------------------------------------
// Path-injected helpers (pub(crate) for testing)
// ---------------------------------------------------------------------------

/// Load settings from `path` (infallible).
///
/// Returns [`Settings::default()`] if the file does not exist or is not valid
/// JSON that matches the [`Settings`] schema.
pub(crate) fn load_from(path: &Path) -> Settings {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Settings::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Persist `settings` to `path` using an atomic rename.
///
/// 1. Creates intermediate directories if they do not exist.
/// 2. Serialises to `path` with a `.tmp` extension.
/// 3. Renames the `.tmp` file to `path`.
pub(crate) fn save_to(settings: &Settings, path: &Path) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp_path: PathBuf = {
        let mut p = path.to_path_buf();
        let stem = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("settings");
        p.set_file_name(format!("{stem}.tmp"));
        p
    };

    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Platform path resolution
// ---------------------------------------------------------------------------

/// Returns the canonical settings file path for this platform, or `None` if
/// `directories` cannot resolve a config dir (e.g. no `$HOME`).
fn platform_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "lasercad")
        .map(|dirs| dirs.config_dir().join("settings.json"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // ------------------------------------------------------------------
    // Settings struct
    // ------------------------------------------------------------------

    /// AC#1 — default settings have an empty recent-files list.
    #[test]
    fn default_settings_have_empty_recent_files() {
        let s = Settings::default();
        assert!(s.recent_files.is_empty());
    }

    /// AC#3 — push_recent_file inserts at position 0 (most-recent first).
    #[test]
    fn push_recent_file_inserts_at_front() {
        let mut s = Settings::default();
        s.push_recent_file("a.lcad".into());
        s.push_recent_file("b.lcad".into());
        s.push_recent_file("c.lcad".into());
        assert_eq!(s.recent_files, vec!["c.lcad", "b.lcad", "a.lcad"]);
    }

    /// AC#2 — push_recent_file deduplicates: existing occurrence is removed
    /// before re-inserting at the front.
    #[test]
    fn push_recent_file_deduplicates() {
        let mut s = Settings::default();
        s.push_recent_file("a.lcad".into());
        s.push_recent_file("b.lcad".into());
        // Push "a" again — it should move to the front, not appear twice.
        s.push_recent_file("a.lcad".into());
        assert_eq!(s.recent_files, vec!["a.lcad", "b.lcad"]);
    }

    /// AC#4 — push_recent_file caps the list at exactly 10 entries.
    #[test]
    fn push_recent_file_caps_at_ten() {
        let mut s = Settings::default();
        for i in 0..12u32 {
            s.push_recent_file(format!("file{i}.lcad"));
        }
        assert_eq!(s.recent_files.len(), 10);
        // Most-recently pushed is at the front.
        assert_eq!(s.recent_files[0], "file11.lcad");
    }

    // ------------------------------------------------------------------
    // load_from / save_to
    // ------------------------------------------------------------------

    /// AC#5 — load_from with a nonexistent path returns Settings::default().
    #[test]
    fn load_from_nonexistent_path_returns_default() {
        let result = load_from(Path::new("/tmp/__lcv058_no_such_file_xyz__.json"));
        assert_eq!(result, Settings::default());
    }

    /// AC#6 — load_from with a valid JSON file returns the deserialised settings.
    #[test]
    fn load_from_valid_json_parses_correctly() {
        let tmp = std::env::temp_dir().join("lcv058_load_test.json");
        let json = r#"{"recent_files":["foo.lcad","bar.lcad"]}"#;
        fs::write(&tmp, json).unwrap();

        let s = load_from(&tmp);
        assert_eq!(s.recent_files, vec!["foo.lcad", "bar.lcad"]);

        let _ = fs::remove_file(&tmp);
    }

    /// AC#7 — save_to + load_from round-trip preserves all fields.
    #[test]
    fn save_to_load_from_round_trip() {
        let tmp = std::env::temp_dir().join("lcv058_roundtrip.json");

        let mut original = Settings::default();
        original.push_recent_file("alpha.lcad".into());
        original.push_recent_file("beta.lcad".into());

        save_to(&original, &tmp).unwrap();
        let loaded = load_from(&tmp);

        assert_eq!(original, loaded);
        let _ = fs::remove_file(&tmp);
    }

    /// AC#8 — save_to creates intermediate directories if they don't exist.
    #[test]
    fn save_to_creates_parent_dirs() {
        let base = std::env::temp_dir().join("lcv058_nested_test_dir");
        let path = base.join("sub").join("settings.json");

        // Ensure the directory doesn't exist.
        let _ = fs::remove_dir_all(&base);

        save_to(&Settings::default(), &path).unwrap();
        assert!(path.exists());

        let _ = fs::remove_dir_all(&base);
    }

    /// AC#9 — save_to uses atomic rename (no stale .tmp after success).
    #[test]
    fn save_to_no_tmp_file_after_success() {
        let tmp = std::env::temp_dir().join("lcv058_atomic.json");
        let tmp_file = std::env::temp_dir().join("lcv058_atomic.json.tmp");

        save_to(&Settings::default(), &tmp).unwrap();

        assert!(tmp.exists(), "final file must exist");
        assert!(!tmp_file.exists(), ".tmp file must be gone after rename");

        let _ = fs::remove_file(&tmp);
    }

    /// Malformed JSON returns Settings::default() without panicking.
    #[test]
    fn load_from_malformed_json_returns_default() {
        let tmp = std::env::temp_dir().join("lcv058_bad_json.json");
        fs::write(&tmp, b"{ this is not json }").unwrap();
        let s = load_from(&tmp);
        assert_eq!(s, Settings::default());
        let _ = fs::remove_file(&tmp);
    }

    // ------------------------------------------------------------------
    // LCV-076 — agent settings fields
    // ------------------------------------------------------------------

    /// §T1 — AC 1 + AC 2 — agent field defaults.
    #[test]
    fn agent_field_defaults() {
        let s = Settings::default();
        assert_eq!(s.agent_endpoint, "https://api.openai.com/v1");
        assert_eq!(s.agent_api_key, "");
    }

    /// §T2 — AC 4 — agent-field round-trip preserves non-default values.
    #[test]
    fn agent_fields_round_trip() {
        let tmp = std::env::temp_dir().join("lcv076_agent_roundtrip.json");

        let original = Settings {
            agent_endpoint: "https://openrouter.ai/api/v1".into(),
            agent_api_key: "sk-test".into(),
            ..Settings::default()
        };

        save_to(&original, &tmp).unwrap();
        let loaded = load_from(&tmp);

        assert_eq!(original, loaded);
        let _ = fs::remove_file(&tmp);
    }

    /// §T2b — AC 3 — legacy JSON (no agent fields) deserialises to defaults.
    #[test]
    fn legacy_json_without_agent_fields_uses_defaults() {
        let tmp = std::env::temp_dir().join("lcv076_legacy_compat.json");
        fs::write(&tmp, r#"{"recent_files":[]}"#).unwrap();

        let result = load_from(&tmp);
        assert_eq!(result.agent_endpoint, "https://api.openai.com/v1");
        assert_eq!(result.agent_api_key, "");

        let _ = fs::remove_file(&tmp);
    }
}
