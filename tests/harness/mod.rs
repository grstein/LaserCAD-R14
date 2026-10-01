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

/// One frame of raw input at an arbitrary screen size. The modifiers are taken
/// from the first key event so `ctx.input(|i| i.modifiers)` agrees with it.
/// egui 0.36 has no `RawInput::modifiers` and keeps the last
/// `Event::ModifiersChanged` across frames, so every frame that carries events
/// starts with one, `NONE` included, as each 0.29 frame carried its own. A
/// frame with no events stays empty: any event asks egui for a repaint, and
/// the idle-repaint tests need a frame that carries nothing (LCV-180).
///
/// A frame with no events is the pointer resting, so it also advances egui's
/// clock by [`REST_DT`] instead of 1/60 s: egui 0.36 stamps
/// `last_move_time` on every `PointerMoved` and holds a tooltip back for
/// `tooltip_delay` (0.5 s) after it, where 0.29 only counted a measured
/// velocity. Frames with events keep the 1/60 s step, so a press and its
/// release in the next frame stay a click (LCV-180).
///
/// The screen is a parameter because a layout claim is only true at a size:
/// `tests/it/ui/shortcuts_dialog_fits.rs` makes the same claim at three of
/// them, and a fixed `SCREEN` forced it to carry a second copy of this.
pub fn raw_input_at(screen: [f32; 2], events: Vec<egui::Event>) -> egui::RawInput {
    let modifiers = events
        .iter()
        .find_map(|e| match e {
            egui::Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(egui::Modifiers::NONE);
    let mut all = Vec::with_capacity(events.len() + 1);
    if !events.is_empty() {
        all.push(egui::Event::ModifiersChanged(modifiers));
    }
    all.extend(events);
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(screen[0], screen[1]),
        )),
        predicted_dt: if all.is_empty() { REST_DT } else { 1.0 / 60.0 },
        events: all,
        ..Default::default()
    }
}

/// Seconds an event-less frame advances egui's clock: past egui's 0.5 s
/// `tooltip_delay`, short of its 0.8 s `max_click_duration`.
pub const REST_DT: f32 = 0.6;

/// [`raw_input_at`] at the default [`SCREEN`].
pub fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
    raw_input_at(SCREEN, events)
}

/// Drive exactly one `App::update_ui` tick with `events`.
pub fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
    // `FullOutput` is `#[must_use]`; headless tests assert on `App` state, not
    // on the paint output.
    let _ = ctx.run_ui(raw_input(events), |ui| app.update_ui(ui));
}

/// Run event-less frames until the canvas rect stops moving, and return it.
/// egui 0.36 sizes a bottom panel from its previous frame's content, so the
/// command-line dock reaches its final height only on the third frame; egui
/// 0.29 laid it out on the first. Reads the rect inside each frame.
pub fn settle(ctx: &egui::Context, app: &mut App) -> egui::Rect {
    let mut last = egui::Rect::NOTHING;
    for _ in 0..6 {
        let mut canvas = egui::Rect::NOTHING;
        let _ = ctx.run_ui(raw_input(vec![]), |ui| {
            let c = &ui.ctx().clone();
            app.update_ui(ui);
            canvas = canvas_rect(c);
        });
        if canvas == last {
            break;
        }
        last = canvas;
    }
    last
}

/// The area id of the `egui::Window` titled `title`. egui 0.36 derives it
/// from the title's `Atoms::text()`, an `Option`, so it is
/// `Id::new(Some(title))`, no longer `Id::new(title)` as in 0.29.
pub fn window_id(title: &str) -> egui::Id {
    egui::Id::new(Some(title))
}

/// The canvas rect of the last frame `ctx` ran: the viewport widget's own
/// response, read back through its stable id (`lasercad::app::VIEWPORT_ID`).
/// `Rect::NOTHING` before the first frame.
pub fn canvas_rect(ctx: &egui::Context) -> egui::Rect {
    ctx.read_response(egui::Id::new(lasercad::app::VIEWPORT_ID))
        .map_or(egui::Rect::NOTHING, |r| r.rect)
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
