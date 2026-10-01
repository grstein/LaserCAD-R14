//! UI chrome around the viewport: the fixed panels and the modal dialogs.
//!
//! Three entry points, called in order from
//! [`App::update_ui`](super::App::update_ui):
//!
//! - [`draw_chrome`] — menubar, statusbar, command line, toolbar. Rendered
//!   before the `CentralPanel` so egui shrinks the canvas to what is left.
//! - [`draw_agent_side_panel`] — the AI assistant panel (LCV-080), only when
//!   `agent.panel_open`.
//! - [`draw_dialogs`] — About, Keyboard Shortcuts (LCV-116), AI Settings,
//!   the error modal, the discard-confirmation dialog (LCV-113) and the Bed
//!   Size… dialog (LCV-114), rendered after the `CentralPanel` so they float
//!   above the canvas.
//!
//! No key is read here: `src/app/input.rs` is the single keyboard gate
//! (LCV-103 / ADR 0002 §A6).
//!
//! MUST NOT import `eframe` or `rfd`.

use super::{App, Dialog, draw_bed_dialog, draw_discard_dialog, topmost};
use crate::ui::DialogKey;

/// Render the four fixed panels that frame the viewport.
pub fn draw_chrome(ui: &mut egui::Ui, app: &mut App) {
    egui::Panel::top("menubar").show(ui, |ui| {
        crate::ui::draw_menubar(ui, app);
    });

    egui::Panel::bottom("statusbar").show(ui, |ui| {
        crate::ui::draw_statusbar(ui, app);
    });

    egui::Panel::bottom("command_line").show(ui, |ui| {
        crate::ui::draw_command_line(ui, app);
    });

    let rail_frame =
        egui::Frame::side_top_panel(&ui.ctx().global_style()).inner_margin(RAIL_MARGIN);
    egui::Panel::left("toolbar")
        .resizable(false)
        .exact_size(RAIL_WIDTH)
        .frame(rail_frame)
        .show(ui, |ui| {
            crate::ui::draw_toolbar(ui, app);
        });
}

/// The tool rail's frame inner margin, in points, on every side (LCV-183).
const RAIL_MARGIN: f32 = 4.0;

/// The tool rail's fixed outer width, in points (LCV-183 AC 8: at most 80):
/// two 32 pt button columns, the 4 pt gap between them
/// (`crate::ui::toolbar::RAIL_GAP`) and [`RAIL_MARGIN`] on both sides.
/// "Including margins" as in `Panel::exact_size`'s own doc comment.
const RAIL_WIDTH: f32 = 2.0 * 32.0 + crate::ui::toolbar::RAIL_GAP + 2.0 * RAIL_MARGIN;

/// Width the agent panel opens at before the ceiling narrows it (LCV-080's
/// original default).
const AGENT_PANEL_DEFAULT_WIDTH: f32 = 300.0;

/// The panel may never claim more than this fraction of the whole
/// application window's width (LCV-141 AC 1-3) — not `CentralPanel`'s
/// remaining share after the toolbar and this panel already claimed theirs.
const AGENT_PANEL_WIDTH_FRACTION: f32 = 1.0 / 3.0;

/// The hard width ceiling for this frame, in logical points.
///
/// Recomputed from `ctx.content_rect()` on every call — never cached — so it
/// holds on the very first frame, after a width dragged wide in a bigger
/// window is carried into a smaller one, during an active resize drag, and
/// after the window itself shrinks (AC 2): `Panel` re-clamps its
/// persisted width against `width_range` on every `show`, so a ceiling that
/// is fresh every frame is all a caller has to provide.
fn agent_panel_width_ceiling(ctx: &egui::Context) -> f32 {
    ctx.content_rect().width() * AGENT_PANEL_WIDTH_FRACTION
}

/// Render the agent side panel (LCV-080) when it is open; a no-op otherwise.
pub fn draw_agent_side_panel(ui: &mut egui::Ui, app: &mut App) {
    if !app.agent.panel_open {
        return;
    }
    let ceiling = agent_panel_width_ceiling(ui.ctx());
    let default_width = AGENT_PANEL_DEFAULT_WIDTH.min(ceiling);
    egui::Panel::right("agent_panel")
        .resizable(true)
        // `.max_size` must follow `.default_size`: `Panel::default_size`
        // widens `outer_size_range.max` via `.at_least(default_size)` when the
        // default exceeds the existing max, and only `.max_size` narrows the
        // range unconditionally (egui-0.36.2
        // `containers/panel.rs::Panel::default_size`/`max_size`). Called
        // in the other order, the ceiling would silently widen back out.
        .default_size(default_width)
        .max_size(ceiling)
        .show(ui, |ui| {
            crate::agent::draw_agent_panel(ui, app);
        });
    // After the panel, so a press opens the picker in the same frame
    // (LCV-199); the panel itself never reaches `rfd` (ADR 0005).
    super::agent_attach::poll_attach_request(app);
}

/// Render the modal dialogs (LCV-069, LCV-076, LCV-062, LCV-113, LCV-114,
/// LCV-116, LCV-156).
///
/// Called after the `CentralPanel` so the windows float above the canvas.
/// `key`, taken by `input.rs::take_dialog_key`, goes to the topmost dialog
/// only (LCV-169 AC 1).
pub fn draw_dialogs(ctx: &egui::Context, app: &mut App, key: Option<DialogKey>) {
    let top = topmost(app);
    let key_for = |d: Dialog| key.filter(|_| top == Some(d));
    crate::ui::about_dialog(ctx, &mut app.about_open, key_for(Dialog::About));
    let shortcuts_key = key_for(Dialog::Shortcuts);
    crate::ui::shortcuts_dialog(ctx, &mut app.shortcuts_open, shortcuts_key);
    agent_settings_dialog(ctx, app, key_for(Dialog::AiSettings));
    error_modal(ctx, app, key_for(Dialog::Error));
    draw_discard_dialog(ctx, app, key_for(Dialog::Discard));
    draw_bed_dialog(ctx, app, key_for(Dialog::Bed));
    crate::ui::draw_layers_dialog(ctx, app, key_for(Dialog::Layers));
    crate::ui::check_dialog(ctx, &mut app.check_report);
}

/// The AI Settings window (LCV-076). Persists the settings when the window
/// closes, whether by the × button, the Close button (LCV-141 AC 6), or
/// programmatically.
fn agent_settings_dialog(ctx: &egui::Context, app: &mut App, key: Option<DialogKey>) {
    let was_open = app.agent_settings_open;
    // Set from inside the content closure below when Close is clicked. Kept
    // separate from `agent_settings_open` itself: `Window::open` already
    // borrows that field for the whole `.show()` call, so a second mutable
    // borrow of the same field from the content closure would not compile —
    // this is the one new piece of state the Close button needs, read only
    // after every borrow above has ended (LCV-141 AC 6).
    let mut close_clicked = false;
    {
        // The window borrows `agent_settings_open` and `settings` mutably for
        // its whole lifetime; `App::persist_settings` needs `&App`, so the
        // borrows are scoped and the write happens after they end (LCV-119).
        let open = &mut app.agent_settings_open;
        let settings = &mut app.settings;
        egui::Window::new("AI Settings")
            .open(open)
            .default_height(crate::ui::DIALOG_HEIGHT)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // LCV-141 AC 7: bounded scrolling, so a form that grows in a
                // later demand scrolls instead of pushing Close off the bottom
                // of the window (ADR 0009).
                egui::ScrollArea::vertical().show(ui, |ui| {
                    close_clicked = crate::agent::draw_agent_settings(ui, settings).close_clicked;
                });
            });
    }
    // Close runs the exact same close as the × button: it only ever sets the
    // same flag the window's own `Window::open` would have set, and the one
    // guard below fires either way — never a second, parallel persistence
    // path (AC 6).
    if close_clicked || key.is_some() {
        app.agent_settings_open = false;
    }
    // Save on dialog close (× button, Close, or programmatic close).
    if was_open && !app.agent_settings_open {
        app.persist_settings();
    }
}

/// The error modal (LCV-062) — rendered last so it floats above everything.
fn error_modal(ctx: &egui::Context, app: &mut App, key: Option<DialogKey>) {
    if let Some(msg) = app.error_message.clone()
        && crate::ui::error_dialog(ctx, "Error", &msg, key)
    {
        app.error_message = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LCV-105 — the agent side panel is skipped entirely while the panel is
    /// closed, which is what keeps the canvas full-width by default.
    #[test]
    fn agent_side_panel_is_skipped_while_closed() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        assert!(!app.agent.panel_open);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            draw_agent_side_panel(ui, &mut app);
        });
        assert!(!app.agent.panel_open);
    }

    /// The implementation section — everything before the bare `#[cfg(test)]`
    /// at column 0 — with comment lines dropped, so neither scan below can
    /// match its own literal or the prose next to it.
    fn implementation_code() -> String {
        let src = include_str!("panels.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("panels.rs must have a bare #[cfg(test)] marker");
        let code: Vec<&str> = src[..at]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect();
        assert!(
            code.len() > 40,
            "positive control: the haystack must be the whole implementation, got {} lines",
            code.len()
        );
        code.join("\n")
    }

    /// LCV-125 AC 13 — **source scan**: the settings dialog reaches
    /// `draw_agent_settings` through `src/agent/mod.rs`'s re-export, not
    /// through a deep path into the module's private file layout (AGENTS.md
    /// §Module tree). The compile is half the test — the deep path stops
    /// resolving only once `pub use settings_ui::draw_agent_settings;` exists —
    /// and this is the half that notices it coming back.
    ///
    /// Scoped to this one call by decision: the tree carries twelve more
    /// cross-module deep paths, several of them legitimate, and a scan shipped
    /// with a twelve-entry grandfather list is the weakest shape this
    /// repository has. That is `architect`'s call, in its own demand.
    #[test]
    fn ac13_the_settings_dialog_uses_the_module_re_export_source_scan() {
        let implementation = implementation_code();
        let deep = concat!("crate::agent::settings", "_ui::");
        let witness = "crate::agent::settings_ui::draw_agent_settings(ui, settings);";
        assert!(
            witness.contains(deep),
            "control: `{deep}` must be a needle that can match something"
        );
        assert!(
            !implementation.contains(deep),
            "the dialog must not reach into `agent`'s file layout"
        );
        assert!(
            implementation.contains(concat!(
                "crate::agent::draw_agent",
                "_settings(ui, settings)"
            )),
            "positive control: the dialog must still draw the form"
        );
    }

    /// The body of `if close_clicked { .. }` inside `agent_settings_dialog`,
    /// brace-matched — the same slicing idiom `src/agent/panel.rs::busy_block`
    /// uses for LCV-129's Cancel button, so a line moved out of the guarded
    /// block is no longer in *this* string even though it is still in the file.
    fn close_clicked_block(implementation: &str) -> String {
        let head = concat!("if close_", "clicked || key.is_some() {");
        let start = implementation
            .find(head)
            .unwrap_or_else(|| panic!("agent_settings_dialog must guard a `{head}` block"))
            + head.len();
        let mut depth = 1usize;
        for (offset, ch) in implementation[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return implementation[start..start + offset].to_owned();
                    }
                }
                _ => {}
            }
        }
        panic!("the close_clicked block is never closed — panels.rs does not parse");
    }

    /// LCV-141 AC 6 — **source scan**: Close's whole effect is setting the same
    /// flag `Window::open` sets for ×, not a second call into
    /// `App::persist_settings`. The one persist call below — reached through
    /// `was_open && !app.agent_settings_open`, which is true after *either*
    /// path — is what actually saves, and this pins that Close does not also
    /// reach it directly.
    #[test]
    fn ac6_close_only_sets_the_shared_close_flag_source_scan() {
        let implementation = implementation_code();
        let block = close_clicked_block(&implementation);

        assert!(
            block.contains(concat!("agent_settings", "_open = false")),
            "AC 6: the close_clicked block must set the same flag × sets: {block}"
        );

        let witness =
            "if close_clicked { app.agent_settings_open = false; app.persist_settings(); }";
        let forbidden = concat!("persist_", "settings");
        assert!(
            witness.contains(forbidden),
            "control: `{forbidden}` must be a needle that can match something"
        );
        assert!(
            !block.contains(forbidden),
            "AC 6: Close must not call `{forbidden}` itself — that is the shared \
             guard's job, reached the same way × reaches it: {block}"
        );
    }

    /// LCV-141 AC 7 — **source scan**: the dialog's body is wrapped in a
    /// bounded `ScrollArea`, matching `src/ui/shortcuts_dialog.rs`'s own
    /// pattern (ADR 0009). `tests/it/agent/panel_width_and_settings.rs`
    /// proves the *mechanism* — a `Window` + `ScrollArea` absorbs an
    /// overflowing body instead of the window growing — against a
    /// hand-built harness, because nothing outside this private function can
    /// inject a growth probe into the real dialog; this scan is what ties
    /// that proof back to `agent_settings_dialog` itself.
    #[test]
    fn ac7_the_dialog_wraps_its_body_in_a_scroll_area_source_scan() {
        let implementation = implementation_code();
        assert!(
            implementation.contains(concat!("ScrollArea::", "vertical()")),
            "AC 7: agent_settings_dialog must wrap its content in a ScrollArea"
        );
    }

    /// LCV-105 — the dialog phase on a default `App` renders nothing modal and
    /// leaves every flag untouched.
    #[test]
    fn dialogs_on_default_app_do_not_panic() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            draw_dialogs(ui.ctx(), &mut app, None);
        });
        assert!(!app.about_open);
        assert!(!app.agent_settings_open);
        assert!(app.error_message.is_none());
    }
}
