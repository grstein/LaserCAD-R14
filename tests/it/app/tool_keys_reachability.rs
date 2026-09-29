//! tests/it/app/tool_keys_reachability.rs — LCV-110: reachability of every `TOOL_KEYS` letter is
//! preserved through the single-map (`ToolKind` + `tools::make`) refactor.
//!
//! Unit tests calling `dispatch_shortcuts` directly (see
//! `src/ui/shortcuts.rs`) prove the table is correct; they prove nothing
//! about whether the table is *reachable* from a real frame (ADR 0002
//! §Context). This is the test that would catch a `TOOL_KEYS` conversion
//! that compiled but was never wired.

use crate::harness;

use egui::{Key, Modifiers};
use harness::tap;
use lasercad::app::App;

/// AC 14 — each of the ten bound letters still activates the same tool it
/// activated before this demand, driven through the real
/// `App::update_ui(ctx)`.
#[test]
fn every_tool_key_still_activates_its_tool() {
    let cases: [(Key, &str); 10] = [
        (Key::L, "LINE"),
        (Key::P, "PLINE"),
        (Key::R, "RECT"),
        (Key::C, "CIRCLE"),
        (Key::A, "ARC"),
        (Key::M, "MOVE"),
        (Key::E, "ERASE"),
        (Key::T, "TRIM"),
        (Key::X, "EXTEND"),
        (Key::D, "TEXT"),
    ];

    for (key, expected_name) in cases {
        let ctx = egui::Context::default();
        let mut app = App::default();
        assert_ne!(
            app.tool_manager.active_tool_name(),
            expected_name,
            "fresh App must not already be on {expected_name} for key {key:?}"
        );

        tap(&ctx, &mut app, key, Modifiers::NONE);

        assert_eq!(
            app.tool_manager.active_tool_name(),
            expected_name,
            "a bare {key:?} tap must activate {expected_name}"
        );
    }
}
