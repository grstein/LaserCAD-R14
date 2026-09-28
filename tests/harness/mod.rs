//! tests/harness/mod.rs — shared headless-frame plumbing for LCV-1xx tests.
//!
//! Three modules, divided by kind:
//!
//! - **this file** — input: the screen rect, a key tap, a frame, the
//!   command-line gestures, and the five hard rules below;
//! - [`paint`] — what actually reached the screen, and the eight traps that
//!   decide whether a painted-text assertion means anything;
//! - [`scan`] — the source-scan walker and matcher, and the four rules that
//!   keep a scan able to fail.
//!
//! Since LCV-152 this directory is compiled once, into the single integration
//! binary `tests/it/main.rs`, so it carries no `allow(dead_code)`: a helper
//! that loses its last caller fails `-D warnings` like any other dead code.
//!
//! Five rules for every test built on this harness (ADR 0002 §A4):
//!
//! 1. **Never send `Ctrl+O`, `Ctrl+S` or `Ctrl+Shift+S`.** They reach
//!    `src/io/file_actions.rs`, which opens a blocking native `rfd` dialog and
//!    hangs the run. `Ctrl+N` and `Ctrl+Z` / `Ctrl+Y` are safe.
//! 2. **Autosave is path-injected (LCV-119, ADR 0006).** An `App` built by
//!    `App::default()` carries `autosave_path: None`, so a fired autosave
//!    writes nothing at all — letting the debounce elapse is now harmless.
//!    A test that wants a real flush sets `autosave_path` to a file inside a
//!    temporary directory it owns and asserts on those bytes; never point it
//!    at the user's real data directory.
//! 3. **Pointer tests run a warm-up frame** carrying `PointerMoved` alone
//!    before the frame carrying `PointerButton`: with both in one frame the
//!    widget rect is not yet registered for hit-testing, `response.hovered()`
//!    is `false`, and the viewport handler never runs. Keyboard-only tests are
//!    single-frame.
//! 4. **Enter only submits if the command line had focus in the *previous*
//!    frame** (ADR 0003 §F3 trap 5). The widget submits on
//!    `lost_focus() && key_pressed(Enter)`, so a single-frame Enter tap
//!    against an unfocused field does nothing — a silent false negative that
//!    looks exactly like a broken parser. Every command-line test is
//!    therefore at least two frames; [`submit_command`] handles this.
//! 5. **`egui::Event::PointerGone` is the only headless route to `dragged() &&
//!    !hovered()`.** It clears `latest_pos` while deliberately leaving
//!    `pointer.down` set — a slider drag is meant to survive the pointer
//!    leaving the viewport — which drops `potential_drag_id` and flips
//!    `hovered()` to `false` while `dragged()` survives. A `PointerMoved` to a
//!    position outside the widget does **not** do this: `latest_pos` stays
//!    populated, so `hovered()` stays `true` and a mutant term goes uncaught.

pub mod paint;
pub mod scan;

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

/// One frame of raw input at an arbitrary screen size. `modifiers` is taken
/// from the first key event so `ctx.input(|i| i.modifiers)` agrees with it.
///
/// The screen is a parameter because a layout claim is only true at a size:
/// `tests/lcv133_shortcuts_dialog_fits.rs` makes the same claim at three of
/// them, and a fixed `SCREEN` forced it to carry a second copy of this.
pub fn raw_input_at(screen: [f32; 2], events: Vec<egui::Event>) -> egui::RawInput {
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
            egui::vec2(screen[0], screen[1]),
        )),
        modifiers,
        events,
        ..Default::default()
    }
}

/// [`raw_input_at`] at the default [`SCREEN`].
pub fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
    raw_input_at(SCREEN, events)
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

/// One `Event::Text` carrying `s` — what a real keyboard emits **alongside**
/// the `Event::Key` for a printable keystroke.
pub fn text_events(s: &str) -> Vec<egui::Event> {
    vec![egui::Event::Text(s.to_owned())]
}

/// Type `s` into the command line the honest way: one frame per character.
///
/// The first character reaches the keyboard gate, which seeds the field and
/// requests focus (LCV-111 AC 21); focus is granted at the end of that frame,
/// so every later character is consumed by the focused `TextEdit` itself.
/// Commands are short, so the frame count is trivial.
pub fn type_command(ctx: &egui::Context, app: &mut App, s: &str) {
    for ch in s.chars() {
        frame(ctx, app, text_events(&ch.to_string()));
    }
}

/// [`type_command`] followed by one Enter tap — the full operator gesture.
///
/// The Enter frame is separate on purpose (trap 4 above): the field must have
/// held focus during the previous frame for `lost_focus()` to fire.
pub fn submit_command(ctx: &egui::Context, app: &mut App, s: &str) {
    type_command(ctx, app, s);
    tap(ctx, app, egui::Key::Enter, egui::Modifiers::NONE);
}
