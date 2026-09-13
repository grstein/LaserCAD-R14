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
//! platform dir). A file that exists but fails to parse is renamed to a
//! `.bak` sibling before defaults are returned (mirroring the `.tmp` staging
//! file used by `save_to`), so the unreadable bytes are preserved rather than
//! silently discarded; if that rename itself fails (e.g. an unwritable
//! directory), the failure is ignored and defaults are still returned. A
//! missing file is left alone — no `.bak`, no side effects. `save` and
//! `save_to` return a [`Result`] so callers can log or surface the error.
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

use crate::util::{clamp_bed_mm, DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

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
    /// Default: `"https://openrouter.ai/api/v1"`.
    #[serde(default = "default_agent_endpoint")]
    pub agent_endpoint: String,

    /// API key sent in the `Authorization: Bearer …` header.
    /// Default: `""` (agent disabled until set).
    #[serde(default)]
    pub agent_api_key: String,

    /// The bed size, in millimetres `[width, height]`, that a **new** document
    /// starts at (LCV-114 AC 11).
    ///
    /// A seed, never the truth: the live bed is
    /// [`Document::bed_mm`](crate::document::Document::bed_mm). Written only
    /// by the Bed size… dialog — opening a 300 × 200 file must not re-home the
    /// operator's machine. Read it through [`Settings::clamped_default_bed_mm`]
    /// rather than directly: a hand-edited settings file can hold anything.
    ///
    /// Needs its own serde default because the field's `Default`
    /// (`[0.0, 0.0]`) would be a zero-sized bed.
    #[serde(default = "default_bed_mm")]
    pub default_bed_mm: [f64; 2],
}

fn default_agent_endpoint() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

fn default_bed_mm() -> [f64; 2] {
    [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            agent_endpoint: default_agent_endpoint(),
            agent_api_key: String::new(),
            default_bed_mm: default_bed_mm(),
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

    /// [`Settings::default_bed_mm`] with each axis held inside
    /// `BED_MIN_MM ..= BED_MAX_MM` (LCV-114 AC 11).
    ///
    /// The stored value comes from a JSON file the operator can edit, so every
    /// reader — `File > New` and the cold-boot seed — goes through this rather
    /// than trusting the field.
    pub fn clamped_default_bed_mm(&self) -> [f64; 2] {
        [
            clamp_bed_mm(self.default_bed_mm[0]),
            clamp_bed_mm(self.default_bed_mm[1]),
        ]
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
/// - A missing file returns [`Settings::default()`] with no side effects: no
///   file is created.
/// - A file that exists but does not parse as [`Settings`] JSON is
///   **preserved**, not overwritten: it is renamed to a `.bak` sibling (an
///   existing `.bak` is replaced) before [`Settings::default()`] is
///   returned. If the rename fails (e.g. the directory is not writable),
///   that failure is ignored — `load_from` still returns defaults and never
///   panics.
/// - A file that parses successfully is left untouched: `load_from` never
///   truncates, creates, or rewrites the file it reads.
pub(crate) fn load_from(path: &Path) -> Settings {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Settings::default(),
    };
    match serde_json::from_str(&content) {
        Ok(settings) => settings,
        Err(_) => {
            // Best-effort: an unwritable directory must not turn a corrupt
            // settings file into a panic, so the rename's Result is dropped.
            let _ = std::fs::rename(path, sibling_with_suffix(path, "bak"));
            Settings::default()
        }
    }
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

    let tmp_path = sibling_with_suffix(path, "tmp");

    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Returns `path` with `.<suffix>` appended to its file name, e.g.
/// `settings.json` + `"tmp"` → `settings.json.tmp`. Shared by `save_to`'s
/// atomic-write staging file and `load_from`'s corrupt-file backup, so the
/// two derivations can never collide with each other.
fn sibling_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.to_path_buf();
    let stem = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("settings");
    p.set_file_name(format!("{stem}.{suffix}"));
    p
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

    /// AC#5 (LCV-058) / AC 9 (LCV-101) — load_from with a nonexistent path
    /// returns Settings::default(), and leaves the filesystem untouched: no
    /// file and no `.bak` sibling are created.
    #[test]
    fn load_from_nonexistent_path_returns_default_and_creates_nothing() {
        let path = Path::new("/tmp/__lcv058_no_such_file_xyz__.json");
        let bak = Path::new("/tmp/__lcv058_no_such_file_xyz__.json.bak");

        let result = load_from(path);

        assert_eq!(result, Settings::default());
        assert!(!path.exists());
        assert!(!bak.exists());
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

    /// AC 10 — a corrupt file is renamed to a `.bak` sibling (byte-identical
    /// to the original) rather than overwritten, and `load_from` returns
    /// defaults without panicking. Covers three cases: garbled JSON, an
    /// empty file, and a pre-existing `.bak` being replaced.
    #[test]
    fn load_from_corrupt_file_backs_it_up() {
        let dir = std::env::temp_dir().join("lcv101_corrupt_backup");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let bak = dir.join("settings.json.bak");

        // Case 1: garbled JSON is backed up and defaults are returned.
        let garbage: &[u8] = b"{ this is not json }";
        fs::write(&path, garbage).unwrap();
        let result = load_from(&path);
        assert_eq!(result, Settings::default());
        assert!(
            !path.exists(),
            "corrupt file must be moved away, not left in place"
        );
        assert_eq!(
            fs::read(&bak).unwrap(),
            garbage,
            ".bak bytes must be identical to the corrupt input"
        );

        // Case 2: a second corrupt write replaces the existing .bak.
        let garbage2: &[u8] = b"{ still not json }";
        fs::write(&path, garbage2).unwrap();
        let result2 = load_from(&path);
        assert_eq!(result2, Settings::default());
        assert_eq!(
            fs::read(&bak).unwrap(),
            garbage2,
            "a pre-existing .bak must be replaced by the newer corrupt file"
        );

        // Case 3: an empty file is corrupt too (not valid JSON).
        fs::write(&path, b"").unwrap();
        let result3 = load_from(&path);
        assert_eq!(result3, Settings::default());
        assert!(!path.exists());
        assert_eq!(fs::read(&bak).unwrap(), b"");

        let _ = fs::remove_dir_all(&dir);
    }

    /// AC 11 — if the `.bak` rename fails (here: the containing directory is
    /// not writable), `load_from` still returns defaults and does not panic.
    /// Unix-only: relies on directory-permission enforcement that Windows
    /// ACLs do not model the same way, and this repo's primary platform is
    /// Linux (AGENTS.md).
    #[cfg(unix)]
    #[test]
    fn load_from_corrupt_file_survives_failed_backup() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join("lcv101_readonly_dir");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, b"{ not json }").unwrap();

        // Strip write permission from the directory: rename() needs write
        // access on the containing directory, not on the file itself.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();

        let result = load_from(&path);

        // Restore permissions before any cleanup, or remove_dir_all fails.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();

        assert_eq!(result, Settings::default());
        let _ = fs::remove_dir_all(&dir);
    }

    /// AC 12 — a file that parses successfully is left byte-for-byte
    /// unchanged: `load_from` never truncates, creates, or rewrites it.
    #[test]
    fn load_from_does_not_modify_a_valid_file() {
        let tmp = std::env::temp_dir().join("lcv101_valid_unmodified.json");
        let json = r#"{"recent_files":["foo.lcad"],"agent_endpoint":"https://openrouter.ai/api/v1","agent_api_key":""}"#;
        fs::write(&tmp, json).unwrap();

        let before = fs::read(&tmp).unwrap();
        let _ = load_from(&tmp);
        let after = fs::read(&tmp).unwrap();

        assert_eq!(before, after);
        let _ = fs::remove_file(&tmp);
    }

    // ------------------------------------------------------------------
    // LCV-076 — agent settings fields
    // ------------------------------------------------------------------

    /// §T1 — AC 1 + AC 2 (LCV-076) / AC 5 + AC 6 (LCV-101) — agent field
    /// defaults: OpenRouter endpoint, empty key, empty recent list.
    #[test]
    fn agent_field_defaults() {
        let s = Settings::default();
        assert_eq!(s.agent_endpoint, "https://openrouter.ai/api/v1");
        assert_eq!(s.agent_api_key, "");
        assert!(s.recent_files.is_empty());
    }

    /// §T2 — AC 4 (LCV-076) / AC 14 (LCV-101) — agent-field round-trip
    /// preserves a non-default endpoint, proving persistence rather than
    /// tautologically matching the (now identical) default.
    #[test]
    fn agent_fields_round_trip() {
        let tmp = std::env::temp_dir().join("lcv076_agent_roundtrip.json");

        let original = Settings {
            agent_endpoint: "https://api.openai.com/v1".into(),
            agent_api_key: "sk-test".into(),
            ..Settings::default()
        };

        save_to(&original, &tmp).unwrap();
        let loaded = load_from(&tmp);

        assert_eq!(original, loaded);
        assert_ne!(loaded.agent_endpoint, Settings::default().agent_endpoint);
        let _ = fs::remove_file(&tmp);
    }

    /// AC 7 — a settings file that stores an explicit (non-default)
    /// endpoint keeps it: the fallback default must not clobber a value the
    /// user actually set.
    #[test]
    fn explicit_endpoint_in_file_is_preserved() {
        let tmp = std::env::temp_dir().join("lcv101_explicit_endpoint.json");
        fs::write(&tmp, r#"{"agent_endpoint":"https://api.openai.com/v1"}"#).unwrap();

        let result = load_from(&tmp);
        assert_eq!(result.agent_endpoint, "https://api.openai.com/v1");

        let _ = fs::remove_file(&tmp);
    }

    /// §T2b — AC 3 (LCV-076) / AC 8 (LCV-101) — legacy JSON (no agent
    /// fields) deserialises to the OpenRouter default.
    #[test]
    fn legacy_json_without_agent_fields_uses_defaults() {
        let tmp = std::env::temp_dir().join("lcv076_legacy_compat.json");
        fs::write(&tmp, r#"{"recent_files":[]}"#).unwrap();

        let result = load_from(&tmp);
        assert_eq!(result.agent_endpoint, "https://openrouter.ai/api/v1");
        assert_eq!(result.agent_api_key, "");

        let _ = fs::remove_file(&tmp);
    }

    /// LCV-114 AC 11 — the seed defaults to the 400 mm pair, not to the
    /// field's own `[0.0, 0.0]`, which would be a zero-sized bed.
    #[test]
    fn default_bed_mm_is_the_default_bed() {
        assert_eq!(
            Settings::default().default_bed_mm,
            [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
        );
    }

    /// LCV-114 AC 11 — a settings file written before this demand still loads
    /// with a usable bed seed rather than a zero-sized one.
    #[test]
    fn legacy_json_without_default_bed_mm_uses_the_default_bed() {
        let tmp = std::env::temp_dir().join("lcv114_legacy_bed.json");
        fs::write(&tmp, r#"{"recent_files":[]}"#).unwrap();

        let result = load_from(&tmp);
        assert_eq!(
            result.default_bed_mm,
            [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
        );

        let _ = fs::remove_file(&tmp);
    }

    /// LCV-114 AC 11 — a chosen seed survives a save/load cycle.
    #[test]
    fn default_bed_mm_round_trips_through_the_file() {
        let tmp = std::env::temp_dir().join("lcv114_bed_seed.json");
        let s = Settings {
            default_bed_mm: [300.0, 180.0],
            ..Settings::default()
        };
        save_to(&s, &tmp).unwrap();

        assert_eq!(load_from(&tmp).default_bed_mm, [300.0, 180.0]);

        let _ = fs::remove_file(&tmp);
    }

    /// LCV-114 AC 11 — the file is operator-editable, so every reader goes
    /// through the clamp rather than trusting the stored pair.
    #[test]
    fn clamped_default_bed_mm_holds_each_axis_in_range() {
        let mut s = Settings {
            default_bed_mm: [0.0, 5000.0],
            ..Settings::default()
        };
        assert_eq!(s.clamped_default_bed_mm(), [1.0, 2000.0]);
        s.default_bed_mm = [300.0, 180.0];
        assert_eq!(s.clamped_default_bed_mm(), [300.0, 180.0]);
        s.default_bed_mm = [f64::NAN, 180.0];
        assert_eq!(s.clamped_default_bed_mm(), [DEFAULT_BED_WIDTH_MM, 180.0]);
    }
}
