//! tests/lcv118.rs — native dialogs stay disarmed in an integration binary
//! (LCV-118, ADR 0005).
//!
//! This file exists because `cfg(test)` is only set for the crate currently
//! being compiled in test mode: when this binary links `lasercad`, the
//! library is compiled as an ordinary dependency with `cfg(test)` **off**.
//! A `#[cfg(test)]` guard inside `src/io/dialogs.rs` would therefore be
//! absent from precisely this compilation mode — the mode in which the
//! LCV-113 review's inverted-guard mutation actually reached
//! `rfd::FileDialog::pick_file()` and hung the test binary. The tests below
//! prove the runtime `AtomicBool` guard holds here too.
//!
//! No `mod harness;` — these tests need no `egui::Context` and no frame.
//!
//! **If any test in this file ever *hangs* instead of failing, the guard has
//! been removed or bypassed.** That is the diagnosis, not a flake: rerun it,
//! but go straight to `src/io/dialogs.rs` and confirm every wrapper still
//! calls `require_armed` as its first statement, and that nothing in this
//! tree calls the arming function outside `crate::run()`.
//!
//! ADR 0002 §A4 rule 1 and ADR 0003 §F3 trap 1 are unchanged by this demand:
//! no test may send `Ctrl+O`, `Ctrl+S` or `Ctrl+Shift+S` either. This file is
//! the backstop underneath that rule, not a replacement for it.

/// The decisive test: a native dialog reached from an integration binary
/// (`cfg(test)` off in `lasercad`) panics instead of blocking the process.
#[test]
#[should_panic(expected = "disarmed")]
fn open_file_dialog_panics_in_an_integration_binary() {
    lasercad::io::open_file_dialog();
}

/// Same witness for the save vector — `Ctrl+S` / `Ctrl+Shift+S` are the
/// other half of ADR 0002 §A4 rule 1.
#[test]
#[should_panic(expected = "disarmed")]
fn save_file_dialog_panics_in_an_integration_binary() {
    lasercad::io::save_file_dialog("untitled.svg");
}

/// AC 8 — no file under `tests/` (including `tests/harness/`) ever arms
/// native dialogs. Built with `concat!("arm_native_", "dialogs")` so this
/// file's own literal use of the name in prose above does not make the
/// needle match this file's own source.
#[test]
fn no_integration_test_arms_native_dialogs() {
    let needle = concat!("arm_native_", "dialogs");
    let tests_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");
    let mut offenders = Vec::new();
    visit_rs_files(std::path::Path::new(tests_dir), &mut |path, contents| {
        if contents.contains(needle) {
            offenders.push(path.display().to_string());
        }
    });
    assert!(
        offenders.is_empty(),
        "no file under tests/ may call {needle}, but found it in: {offenders:?}"
    );
}

/// Recursively visits every `*.rs` file under `dir`, calling `f` with its
/// path and contents.
fn visit_rs_files(dir: &std::path::Path, f: &mut dyn FnMut(&std::path::Path, &str)) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read tests dir {}: {e}", dir.display()));
    for entry in entries {
        let entry = entry.expect("failed to read a directory entry under tests/");
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, f);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let contents = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            f(&path, &contents);
        }
    }
}
