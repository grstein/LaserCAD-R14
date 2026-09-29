//! LaserCAD v2 library crate.
//!
//! Module boundaries and rules are documented in `AGENTS.md` at the repository
//! root. Kernel modules ([`geometry`], [`document`], [`io::svg`], [`text`],
//! [`cmdline`]) MUST NOT import `egui`, `eframe`, or `rfd`.

#![cfg_attr(
    test,
    expect(
        clippy::float_cmp,
        reason = "unit tests assert exact values; the non-test build still enforces it"
    )
)]

pub mod agent;
pub mod app;
pub mod cmdline;
pub mod document;
pub mod geometry;
pub mod io;
pub mod render;
pub mod text;
pub mod tools;
pub mod ui;
pub mod util;

/// Window title shown in the OS title bar.
///
/// Frozen by demand LCV-007 — the Phase 0 window contract — and amended by
/// LCV-105, which dropped the Phase 0 suffix now that the product ships.
/// Downstream demands (LCV-030, LCV-067) reference this constant by name
/// rather than copying the literal so the title stays single-sourced; any
/// future change follows the same route — the constant, its test and the
/// LCV-007 demand body move together.
pub const APP_TITLE: &str = "LaserCAD v2";

/// Default native window inner size in screen pixels.
///
/// Pixels (egui surface units), NOT millimeters — window size is a UI
/// concern, not a CAD-world quantity. Frozen by demand LCV-007.
pub const DEFAULT_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];

/// Application entry point. Opens the main window.
///
/// Wired by [`crate::app::App`] — see `src/app/mod.rs`. Returns the
/// [`eframe::Result`] from [`eframe::run_native`] unchanged so callers
/// (notably `src/main.rs`) can propagate it with `?`.
///
/// Arms native file dialogs (`crate::io::arm_native_dialogs`) as its first
/// statement, before `eframe::run_native`. `run()` is the single boot path,
/// called only from `src/main.rs`; nothing else in the tree may call it or
/// the arming function. See ADR 0005, LCV-118.
pub fn run() -> eframe::Result<()> {
    crate::io::arm_native_dialogs();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_inner_size(DEFAULT_WINDOW_SIZE)
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LaserCAD v2",
        native_options,
        Box::new(|_cc| Ok(Box::new(app::App::new()))),
    )
}

#[cfg(test)]
mod tests {
    use super::{APP_TITLE, DEFAULT_WINDOW_SIZE};

    /// LCV-007 — freeze the window contract, as amended by LCV-105.
    ///
    /// The title literal and default inner size are part of the demand's
    /// acceptance criteria; this test catches any silent rename. The name is
    /// deliberately unchanged so `git log -S` keeps tracking the contract
    /// across the LCV-105 title change.
    #[test]
    fn bootstrap_window_contract() {
        assert_eq!(APP_TITLE, "LaserCAD v2");
        assert_eq!(DEFAULT_WINDOW_SIZE, [1280.0_f32, 800.0_f32]);
    }

    /// AC 6 (LCV-118, ADR 0005) — `run()` must arm native dialogs as its
    /// *first statement*, before `eframe::run_native`. "First statement" is
    /// the load-bearing part of the design, so a mere `contains()` check is
    /// not enough — that would pass if the arming call sat in a doc comment,
    /// after `eframe::run_native`, or outside `run()` entirely.
    ///
    /// The haystack is bounded to `&src[..cfg_test_at]` — everything before
    /// the bare `#[cfg(test)]` at column 0 that opens this very module —
    /// exactly as `guard_is_runtime_not_cfg` does in `src/io/dialogs.rs`.
    /// Without that bound, this test's own body (which necessarily contains
    /// the literals `arm_native_dialogs()` and `eframe::run_native` as
    /// `.find()` arguments) sits later in the same `include_str!` output and
    /// gives every mutation that removes the real arming call a fake,
    /// trivially-ordered match to fall through to — a review found this
    /// exact self-match kept the test green when the real call was deleted
    /// from `run()` outright.
    ///
    /// Byte-offset ordering alone is still not enough: commenting the real
    /// call out (`// crate::io::arm_native_dialogs();`) leaves the literal
    /// substring sitting at the same relative position, so an ordering-only
    /// check keeps passing. The trailing assertion below rejects a match
    /// whose own line is a `//` comment.
    #[test]
    fn run_arms_native_dialogs_as_its_first_statement() {
        let src = include_str!("lib.rs");
        let cfg_test_marker = "\n#[cfg(test)]";
        let cfg_test_at = src
            .find(cfg_test_marker)
            .expect("lib.rs must contain a bare #[cfg(test)] mod tests marker");
        let implementation = &src[..cfg_test_at];

        let run_at = implementation
            .find("pub fn run()")
            .expect("lib.rs must declare pub fn run() before the #[cfg(test)] module");
        let arm_at = run_at
            + implementation[run_at..]
                .find("arm_native_dialogs()")
                .expect(
                    "run() must call arm_native_dialogs() before the #[cfg(test)] \
                     module — needle `arm_native_dialogs()` not found after `pub fn \
                     run()` in the implementation section",
                );
        let run_native_at = arm_at
            + implementation[arm_at..].find("eframe::run_native").expect(
                "run() must call eframe::run_native after arm_native_dialogs(), \
                     before the #[cfg(test)] module — needle `eframe::run_native` not \
                     found after the arming call in the implementation section",
            );

        assert!(
            run_at < arm_at,
            "arm_native_dialogs() must appear after pub fn run() (AC 6)"
        );
        assert!(
            arm_at < run_native_at,
            "arm_native_dialogs() must be called before eframe::run_native (AC 6)"
        );

        let arm_line_start = implementation[..arm_at]
            .rfind('\n')
            .map(|newline_at| newline_at + 1)
            .unwrap_or(0);
        let arm_line = implementation[arm_line_start..]
            .lines()
            .next()
            .unwrap_or("");
        assert!(
            !arm_line.trim_start().starts_with("//"),
            "arm_native_dialogs() must be live code, not commented out (AC 6) — \
             found line `{arm_line}`"
        );
    }
}
