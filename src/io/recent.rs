//! Recent-files data layer for LaserCAD.
//!
//! Provides a stable public API for reading and promoting entries in the
//! [`Settings::recent_files`] list so that downstream callers (the File menu,
//! keyboard shortcuts) import from one place rather than reaching into the
//! struct field directly.
//!
//! ## Responsibilities
//!
//! - [`recent_files`]: thin slice accessor over `settings.recent_files`.
//! - [`open_recent`]: bounds-check, promote the selected entry to front via
//!   [`Settings::push_recent_file`], and return the path as a [`PathBuf`].
//!
//! ## Out of scope
//!
//! - File-existence checking (stale-path tombstoning).
//! - Persisting the settings — the caller owns persistence timing, through
//!   `App::persist_settings` (ADR 0006).
//! - SVG parsing / document replacement (LCV-057, LCV-062).

use std::path::PathBuf;

use crate::io::settings::Settings;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`open_recent`].
#[derive(Debug, thiserror::Error)]
pub enum OpenRecentError {
    /// The requested index exceeds the length of the recent-files list.
    #[error("index {index} is out of range (list has {len} entries)")]
    IndexOutOfRange {
        /// The index that was asked for.
        index: usize,
        /// How many entries the list holds.
        len: usize,
    },
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns the ordered recent-files list (most-recent first) as a slice.
///
/// This is a thin accessor that provides a single stable symbol for all
/// downstream callers. It is equivalent to `settings.recent_files.as_slice()`
/// but avoids coupling callers to the struct field name.
pub fn recent_files(settings: &Settings) -> &[String] {
    settings.recent_files.as_slice()
}

/// Promote the entry at `index` to the front of the recent-files list and
/// return it as a [`PathBuf`].
///
/// # Contract
///
/// 1. If `index >= settings.recent_files.len()`, returns
///    [`OpenRecentError::IndexOutOfRange`].
/// 2. Clones the path string at `settings.recent_files[index]`.
/// 3. Calls [`Settings::push_recent_file`] which deduplicates, moves the entry
///    to `settings.recent_files[0]`, and enforces the 10-entry cap.
/// 4. Returns `Ok(PathBuf::from(path))`.
///
/// # Caller responsibilities
///
/// - Call `App::persist_settings` to persist the updated list.
/// - Pass the returned [`PathBuf`] to the SVG importer (LCV-062).
pub fn open_recent(index: usize, settings: &mut Settings) -> Result<PathBuf, OpenRecentError> {
    let len = settings.recent_files.len();
    if index >= len {
        return Err(OpenRecentError::IndexOutOfRange { index, len });
    }

    let path = settings.recent_files[index].clone();
    settings.push_recent_file(path.clone());

    Ok(PathBuf::from(path))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::settings::{load_from, save_to};

    // ------------------------------------------------------------------
    // recent_files
    // ------------------------------------------------------------------

    /// AC 1 (empty list): recent_files returns an empty slice.
    #[test]
    fn recent_files_empty() {
        let s = Settings::default();
        assert_eq!(recent_files(&s), &[] as &[String]);
    }

    /// AC 1 (non-empty list): recent_files returns the same slice as the field.
    #[test]
    fn recent_files_with_entries() {
        let mut s = Settings::default();
        s.push_recent_file("a.svg".into());
        s.push_recent_file("b.svg".into());
        s.push_recent_file("c.svg".into());
        assert_eq!(recent_files(&s), s.recent_files.as_slice());
    }

    // ------------------------------------------------------------------
    // open_recent — error paths
    // ------------------------------------------------------------------

    /// AC 2: empty settings, index 0 → IndexOutOfRange { index: 0, len: 0 }.
    #[test]
    fn open_recent_index_out_of_range_empty() {
        let mut s = Settings::default();
        let err = open_recent(0, &mut s).unwrap_err();
        assert!(
            matches!(err, OpenRecentError::IndexOutOfRange { index: 0, len: 0 }),
            "unexpected error variant: {err:?}"
        );
    }

    /// AC 3: two entries, index 5 → IndexOutOfRange { index: 5, len: 2 }.
    #[test]
    fn open_recent_index_out_of_range_nonzero_list() {
        let mut s = Settings::default();
        s.push_recent_file("a.svg".into());
        s.push_recent_file("b.svg".into());
        let err = open_recent(5, &mut s).unwrap_err();
        assert!(
            matches!(err, OpenRecentError::IndexOutOfRange { index: 5, len: 2 }),
            "unexpected error variant: {err:?}"
        );
    }

    // ------------------------------------------------------------------
    // open_recent — success paths
    // ------------------------------------------------------------------

    /// AC 4: single entry ["a.svg"], open_recent(0) → Ok(PathBuf::from("a.svg")).
    #[test]
    fn open_recent_single_entry_returns_path() {
        let mut s = Settings::default();
        s.push_recent_file("a.svg".into());
        let result = open_recent(0, &mut s).unwrap();
        assert_eq!(result, PathBuf::from("a.svg"));
    }

    /// AC 5: three entries ["c.svg", "b.svg", "a.svg"], open_recent(2) →
    /// Ok(PathBuf::from("a.svg")).
    #[test]
    fn open_recent_middle_entry_returns_correct_path() {
        let mut s = Settings::default();
        s.push_recent_file("a.svg".into());
        s.push_recent_file("b.svg".into());
        s.push_recent_file("c.svg".into());
        // list is now ["c.svg", "b.svg", "a.svg"]
        let result = open_recent(2, &mut s).unwrap();
        assert_eq!(result, PathBuf::from("a.svg"));
    }

    /// AC 6: after open_recent(2, …) on ["c.svg", "b.svg", "a.svg"], the
    /// opened entry ("a.svg") is at index 0.
    #[test]
    fn open_recent_promotes_to_front() {
        let mut s = Settings::default();
        s.push_recent_file("a.svg".into());
        s.push_recent_file("b.svg".into());
        s.push_recent_file("c.svg".into());
        open_recent(2, &mut s).unwrap();
        assert_eq!(s.recent_files[0], "a.svg");
    }

    /// AC 7: ["b.svg", "a.svg"], open_recent(1) → list is ["a.svg", "b.svg"]
    /// with no duplicates.
    #[test]
    fn open_recent_deduplication_preserved() {
        let mut s = Settings::default();
        s.push_recent_file("a.svg".into());
        s.push_recent_file("b.svg".into());
        // list is ["b.svg", "a.svg"]
        open_recent(1, &mut s).unwrap();
        assert_eq!(s.recent_files, vec!["a.svg", "b.svg"]);
    }

    /// AC 8: open_recent does not persist to disk — the settings file is
    /// unchanged until the caller explicitly calls save_to / save().
    #[test]
    fn open_recent_does_not_save_settings() {
        let tmp = std::env::temp_dir().join("lcv060_no_save_test.json");

        let mut s = Settings::default();
        s.push_recent_file("old.svg".into());

        // Write initial state to disk.
        save_to(&s, &tmp).unwrap();
        let on_disk_before = load_from(&tmp);

        // Perform open_recent — this promotes "old.svg" but must NOT save.
        open_recent(0, &mut s).unwrap();

        // File must still reflect the state before open_recent.
        let on_disk_after = load_from(&tmp);
        assert_eq!(on_disk_before, on_disk_after);

        let _ = std::fs::remove_file(&tmp);
    }

    /// AC 10: compile-time assertion that the re-exports exist on crate::io.
    #[test]
    fn io_reexports_recent_symbols() {
        use crate::io::OpenRecentError as _OREErr;
        let _: fn(&crate::io::settings::Settings) -> &[String] = crate::io::recent_files;
        let _: fn(
            usize,
            &mut crate::io::settings::Settings,
        ) -> Result<std::path::PathBuf, _OREErr> = crate::io::open_recent;
    }
}
