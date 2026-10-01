//! LCV-201 — Windows and macOS packaging. Packaging is shell scripts, CI YAML
//! and docs, so most criteria are text scans of those files, read fresh from
//! disk. `release.sh --list-assets` is the one behavioural check (Unix only).

use std::fs;
use std::path::{Path, PathBuf};

/// Absolute path of a repository-relative file.
fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Contents of a repository file; panics with the path when unreadable.
fn read(rel: &str) -> String {
    let path = repo(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {e}", path.display()))
}

/// LCV-201 AC 2 — release builds on Windows use the GUI subsystem (no console
/// window); debug builds keep the console.
#[test]
fn main_rs_hides_console_in_windows_release_builds() {
    let main = read("src/main.rs");
    assert!(
        main.contains(
            r#"#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]"#
        ),
        "src/main.rs must gate windows_subsystem on all(windows, not(debug_assertions))"
    );
}
