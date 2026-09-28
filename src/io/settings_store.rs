//! Settings file I/O, split from `settings.rs` (ADR 0007 §D8 seam).
//!
//! ## One path-injected API (ADR 0006)
//!
//! - [`platform_path()`] resolves the real per-user file through
//!   [`directories::ProjectDirs`]. It is called from exactly one place,
//!   `App::new()`, which stores the result on the `App` it returns; nothing
//!   below boot resolves a real user location.
//! - [`load_from()`] / [`save_to()`] take an explicit [`std::path::Path`], so
//!   the only way to read or write settings is to hand over a path. A test
//!   points that path at a temporary directory; a process that was given no
//!   path (`App::default()`) writes nothing at all.
//!
//! `load_from()` is **infallible** — it returns
//! [`Settings::default()`] on any error (missing file, unreadable file, parse
//! failure). A file that exists but fails to parse is renamed to a
//! `.bak` sibling before defaults are returned (mirroring the `.tmp` staging
//! file used by `save_to`), so the unreadable bytes are preserved rather than
//! silently discarded; if that rename itself fails (e.g. an unwritable
//! directory), the failure is ignored and defaults are still returned. A
//! missing file is left alone — no `.bak`, no side effects. `save_to` returns
//! a [`Result`] so callers can log or surface the error.
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

use crate::io::settings::{Settings, SettingsError};

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
///
/// **Boot-only (ADR 0006).** The single caller is `App::new()`, which stores
/// the result on the `App` it returns and hands it to every later read and
/// write. Calling this anywhere else reopens the defect LCV-119 closed: a
/// test would reach the developer's real `~/.config/lasercad/settings.json`.
pub(crate) fn platform_path() -> Option<PathBuf> {
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
}
