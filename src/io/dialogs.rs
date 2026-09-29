//! Thin blocking wrappers around [`rfd::FileDialog`] for the three file-system
//! operations LaserCAD needs: open a file, save a file, and pick a directory.
//!
//! These functions are the sole consumers of `rfd` in the codebase.  All file
//! I/O (reading / writing SVG) is handled by [`crate::io::svg`]; these
//! wrappers only present the OS dialog and return the path the user chose.
//!
//! **Purity**: this module imports only `rfd` and `std`.  It must not import
//! any UI framework or kernel module (`geometry`, `document`, `io::svg`,
//! `agent`, `text`).  See `AGENTS.md` §Purity rule.
//!
//! **Disarmed by default (ADR 0005, LCV-118)**: each wrapper below calls
//! [`require_armed`] before it touches `rfd::FileDialog`. A test that reaches
//! a dialog panics with a named message instead of blocking the process
//! forever; only `crate::run()` arms them, as its first statement, before
//! `eframe::run_native`. No test in this crate may call `arm_native_dialogs`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether native dialogs are armed. Starts `false`; only
/// [`arm_native_dialogs`] ever sets it `true`, and nothing ever clears it
/// back — there is no disarm.
///
/// `Ordering::Relaxed` is correct for both the load and the store: this flag
/// publishes no other data (no dialog state depends on a happens-before
/// relationship established here), and a stale read can only produce an
/// extra panic — never a hang — so Relaxed's weaker guarantee costs nothing.
/// See ADR 0005, LCV-118.
static NATIVE_DIALOGS_ARMED: AtomicBool = AtomicBool::new(false);

/// Arms native dialogs so [`open_file_dialog`], [`save_file_dialog`] and
/// [`pick_folder_dialog`] may reach `rfd`.
///
/// Called exactly once, by `crate::run()`, as its first statement, before
/// `eframe::run_native` — see `src/lib.rs`. No test may call it: a headless
/// test that legitimately needs a chosen path calls `action_open_path` /
/// `action_save` directly instead, exactly as it does today. There is no
/// `disarm_native_dialogs` — the flag is write-once by design. See ADR 0005.
pub fn arm_native_dialogs() {
    NATIVE_DIALOGS_ARMED.store(true, Ordering::Relaxed);
}

/// Panics with a single, CI-actionable message naming `fn_name` when a native
/// dialog wrapper is called while dialogs are disarmed.
///
/// This is a deliberate `panic!`, not a silent `unwrap`/`expect`, and not a
/// violation of `AGENTS.md`'s ban on those: returning `None` here would be
/// indistinguishable from "the user cancelled", which is a valid, documented
/// return from all three wrappers — a disarmed call returning `None` would
/// let a regression that reaches this point run down a wrong code path and
/// go green. Panicking is an ordinary libtest failure: it names the test,
/// prints this message, and the rest of the run continues. See ADR 0005
/// §Decision, LCV-118.
fn require_armed(fn_name: &str) {
    if !NATIVE_DIALOGS_ARMED.load(Ordering::Relaxed) {
        panic!(
            "native file dialog `{fn_name}` was called while dialogs are disarmed — this process is not the LaserCAD app binary. Only `crate::run()` arms them (`arm_native_dialogs`, src/io/dialogs.rs); tests never do. A code path under test reached a real OS dialog, which would block the process forever with no output; the usual cause is an inverted or missing guard predicate upstream of a file action. Fix that caller — do not arm dialogs from a test. See docs/adr/0005-native-dialogs-disarmed-by-default.md"
        );
    }
}

/// Opens a native file-open dialog filtered to SVG files and all files.
///
/// Returns `Some(path)` when the user confirms a selection, `None` when the
/// user cancels or closes the dialog without choosing a file.
pub fn open_file_dialog() -> Option<PathBuf> {
    require_armed("open_file_dialog");
    rfd::FileDialog::new()
        .add_filter("SVG files", &["svg"])
        .add_filter("All files", &["*"])
        .pick_file()
}

/// Opens a native file-save dialog with the filename input pre-filled to
/// `default_name`.
///
/// The same two filters as [`open_file_dialog`] are present.  Returns
/// `Some(path)` on confirm, `None` on cancel.
pub fn save_file_dialog(default_name: &str) -> Option<PathBuf> {
    require_armed("save_file_dialog");
    rfd::FileDialog::new()
        .add_filter("SVG files", &["svg"])
        .add_filter("All files", &["*"])
        .set_file_name(default_name)
        .save_file()
}

/// Opens a native folder-picker dialog (no file-extension filter).
///
/// Reserved for future batch-export workflows.  Returns `Some(path)` on
/// confirm, `None` on cancel.
pub fn pick_folder_dialog() -> Option<PathBuf> {
    require_armed("pick_folder_dialog");
    rfd::FileDialog::new().pick_folder()
}

#[cfg(test)]
mod tests {
    use super::{open_file_dialog, pick_folder_dialog, save_file_dialog};

    /// LCV-061 AC#1–3 — compile-time assertion that the three wrappers carry
    /// exactly the declared signatures.  No dialog is opened; the test passes
    /// as long as the function pointer types match.
    #[test]
    fn dialog_fn_signatures() {
        let _: fn() -> Option<std::path::PathBuf> = open_file_dialog;
        let _: fn(&str) -> Option<std::path::PathBuf> = save_file_dialog;
        let _: fn() -> Option<std::path::PathBuf> = pick_folder_dialog;
    }

    // -- LCV-118 (ADR 0005) — native dialogs disarmed until `crate::run()` --

    use super::NATIVE_DIALOGS_ARMED;
    use std::sync::atomic::Ordering;

    /// AC 1 — dialogs are disarmed by default. No test in this crate may
    /// arm, so this must hold no matter which tests ran first in this
    /// binary.
    #[test]
    fn dialogs_are_disarmed_by_default() {
        assert!(!NATIVE_DIALOGS_ARMED.load(Ordering::Relaxed));
    }

    /// AC 4 — the panic names the wrapper that was called.
    #[test]
    #[should_panic(expected = "open_file_dialog")]
    fn open_file_dialog_panics_when_disarmed() {
        open_file_dialog();
    }

    /// AC 4 — same witness for the save vector (half of ADR 0002 §A4 rule 1).
    #[test]
    #[should_panic(expected = "save_file_dialog")]
    fn save_file_dialog_panics_when_disarmed() {
        save_file_dialog("");
    }

    /// AC 4 — `pick_folder_dialog` has no caller today; the guard belongs to
    /// the boundary, not to the current call graph, so it is tested anyway.
    #[test]
    #[should_panic(expected = "pick_folder_dialog")]
    fn pick_folder_dialog_panics_when_disarmed() {
        pick_folder_dialog();
    }

    /// AC 4 — a second `should_panic` on the same wrapper, asserting a
    /// second substring: `expected` only matches one substring at a time.
    #[test]
    #[should_panic(expected = "docs/adr/0005-native-dialogs-disarmed-by-default.md")]
    fn panic_message_names_adr_0005() {
        open_file_dialog();
    }

    /// AC 2 — there is no disarm function. The flag is write-once; a
    /// `disarm_native_dialogs` would be an escape hatch whose only caller
    /// would be a test that should not exist. The needle is built with
    /// `concat!` so this assertion's own source does not contain the
    /// contiguous literal it searches for — the same self-matching trap
    /// AC 8's `tests/` scan has to dodge.
    #[test]
    fn no_disarm_function_exists() {
        let src = include_str!("dialogs.rs");
        let needle = concat!("fn ", "disarm");
        assert!(
            !src.contains(needle),
            "dialogs.rs must not define any disarm function"
        );
    }

    /// AC 3 — the guard must be a runtime `AtomicBool` read, not a
    /// compile-time `cfg`. `cfg(test)` is set only for the crate currently
    /// being compiled in test mode: when an integration-test binary links
    /// `lasercad`, the library is an ordinary dependency with `cfg(test)`
    /// off, so a `#[cfg(test)]` guard here would be absent from precisely
    /// the binary that hung under LCV-113 (ADR 0005 §Context). This test
    /// stops that "simplification" from being reintroduced later: the
    /// implementation section — everything before the bare `#[cfg(test)]`
    /// at column 0 — must contain no `cfg(` at all.
    #[test]
    fn guard_is_runtime_not_cfg() {
        let src = include_str!("dialogs.rs");
        let marker = "\n#[cfg(test)]";
        let cfg_test_at = src
            .find(marker)
            .expect("dialogs.rs must contain a bare #[cfg(test)] mod tests marker");
        let implementation = &src[..cfg_test_at];
        assert!(
            !implementation.contains("cfg("),
            "the dialog guard must be a runtime AtomicBool read, not a cfg — found \
             `cfg(` before the #[cfg(test)] test module boundary in dialogs.rs"
        );
    }
}
