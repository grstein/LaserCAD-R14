//! tests/harness/mod.rs — shared headless-frame plumbing for LCV-1xx tests.
//!
//! Each integration-test binary compiles its own copy, hence `allow(dead_code)`.
//!
//! Three rules for every test built on this harness (ADR 0002 §A4):
//!
//! 1. **Never send `Ctrl+O`, `Ctrl+S` or `Ctrl+Shift+S`.** They reach
//!    `src/io/file_actions.rs`, which opens a blocking native `rfd` dialog and
//!    hangs the run. `Ctrl+N` and `Ctrl+Z` / `Ctrl+Y` are safe.
//! 2. **Never let the autosave debounce elapse.** A fired autosave writes to
//!    the user's real data directory.
//! 3. **Pointer tests run a warm-up frame** carrying `PointerMoved` alone
//!    before the frame carrying `PointerButton`: with both in one frame the
//!    widget rect is not yet registered for hit-testing, `response.hovered()`
//!    is `false`, and the viewport handler never runs. Keyboard-only tests are
//!    single-frame.
#![allow(dead_code)]

use lasercad::app::App;

/// Screen size handed to egui, in points. Matches `DEFAULT_WINDOW_SIZE`.
pub const SCREEN: [f32; 2] = [1280.0, 800.0];

/// A full key *tap*: press followed by release, both `repeat: false`.
///
/// The release is mandatory. egui auto-detects key repeats: pressing the same
/// key in two consecutive frames without a release makes the second event
/// `repeat: true`, which `src/ui/shortcuts.rs` filters out — the test would
/// then read a phantom no-op. There is deliberately no single-event helper.
pub fn key_events(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    vec![
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        },
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        },
    ]
}

/// One frame of raw input with a realistic screen rect. `modifiers` is taken
/// from the first key event so `ctx.input(|i| i.modifiers)` agrees with it.
pub fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
    let modifiers = events
        .iter()
        .find_map(|e| match e {
            egui::Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(egui::Modifiers::NONE);
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(SCREEN[0], SCREEN[1]),
        )),
        modifiers,
        events,
        ..Default::default()
    }
}

/// Drive exactly one `App::update_ui` tick with `events`.
pub fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
    // `FullOutput` is `#[must_use]`; headless tests assert on `App` state, not
    // on the paint output.
    let _ = ctx.run(raw_input(events), |ctx| app.update_ui(ctx));
}

/// Drive one frame containing a single complete key tap.
pub fn tap(ctx: &egui::Context, app: &mut App, key: egui::Key, modifiers: egui::Modifiers) {
    frame(ctx, app, key_events(key, modifiers));
}
