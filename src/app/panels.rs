//! UI chrome around the viewport: the fixed panels and the modal dialogs.
//!
//! Three entry points, called in order from
//! [`App::update_ui`](super::App::update_ui):
//!
//! - [`draw_chrome`] — menubar, statusbar, command line, toolbar. Rendered
//!   before the `CentralPanel` so egui shrinks the canvas to what is left.
//! - [`draw_agent_side_panel`] — the AI assistant panel (LCV-080), only when
//!   `agent_panel_open`.
//! - [`draw_dialogs`] — About, Keyboard shortcuts (LCV-116), Agent Settings,
//!   the error modal, the discard-confirmation dialog (LCV-113) and the Bed
//!   size… dialog (LCV-114), rendered after the `CentralPanel` so they float
//!   above the canvas.
//!
//! No key is read here: `src/app/input.rs` is the single keyboard gate
//! (LCV-103 / ADR 0002 §A6).
//!
//! MUST NOT import `eframe` or `rfd`.

use super::{draw_bed_dialog, draw_discard_dialog, App};

/// Render the four fixed panels that frame the viewport.
pub fn draw_chrome(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
        crate::ui::draw_menubar(ui, app);
    });

    egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
        crate::ui::draw_statusbar(ui, app);
    });

    egui::TopBottomPanel::bottom("command_line").show(ctx, |ui| {
        crate::ui::draw_command_line(ui, app);
    });

    egui::SidePanel::left("toolbar").show(ctx, |ui| {
        crate::ui::draw_toolbar(ui, app);
    });
}

/// Render the agent side panel (LCV-080) when it is open; a no-op otherwise.
pub fn draw_agent_side_panel(ctx: &egui::Context, app: &mut App) {
    if !app.agent_panel_open {
        return;
    }
    egui::SidePanel::right("agent_panel")
        .resizable(true)
        .default_width(300.0)
        .show(ctx, |ui| {
            crate::agent::draw_agent_panel(ui, app);
        });
}

/// Render the modal dialogs (LCV-069, LCV-076, LCV-062, LCV-113, LCV-114,
/// LCV-116).
///
/// Called after the `CentralPanel` so the windows float above the canvas.
pub fn draw_dialogs(ctx: &egui::Context, app: &mut App) {
    crate::ui::about_dialog(ctx, &mut app.about_open);
    crate::ui::shortcuts_dialog(ctx, &mut app.shortcuts_open);
    agent_settings_dialog(ctx, app);
    error_modal(ctx, app);
    draw_discard_dialog(ctx, app);
    draw_bed_dialog(ctx, app);
}

/// The Agent Settings window (LCV-076). Persists the settings when the window
/// closes, whether by the × button or programmatically.
fn agent_settings_dialog(ctx: &egui::Context, app: &mut App) {
    let was_open = app.agent_settings_open;
    {
        // The window borrows `agent_settings_open` and `settings` mutably for
        // its whole lifetime; `App::persist_settings` needs `&App`, so the
        // borrows are scoped and the write happens after they end (LCV-119).
        let open = &mut app.agent_settings_open;
        let settings = &mut app.settings;
        egui::Window::new("Agent Settings")
            .open(open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                crate::agent::draw_agent_settings(ui, settings);
            });
    }
    // Save on dialog close (× button or programmatic close).
    if was_open && !app.agent_settings_open {
        app.persist_settings();
    }
}

/// The error modal (LCV-062) — rendered last so it floats above everything.
fn error_modal(ctx: &egui::Context, app: &mut App) {
    if let Some(msg) = app.error_message.clone() {
        if crate::ui::error_dialog(ctx, "Error", &msg) {
            app.error_message = None;
        }
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
        assert!(!app.agent_panel_open);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw_agent_side_panel(ctx, &mut app);
        });
        assert!(!app.agent_panel_open);
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

    /// LCV-105 — the dialog phase on a default `App` renders nothing modal and
    /// leaves every flag untouched.
    #[test]
    fn dialogs_on_default_app_do_not_panic() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw_dialogs(ctx, &mut app);
        });
        assert!(!app.about_open);
        assert!(!app.agent_settings_open);
        assert!(app.error_message.is_none());
    }
}
