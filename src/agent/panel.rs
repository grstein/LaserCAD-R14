//! LCV-080 — Agent panel: chat history widget and AI input row.
//!
//! Exports [`draw_agent_panel`], which renders into an existing [`egui::Ui`]
//! that lives inside a right-side panel. The channel payload it sends moved to
//! [`crate::agent::AgentEvent`] with LCV-122 — a protocol shared with a
//! kernel-pure worker thread does not belong in an egui file.
//!
//! **Purity rule**: this file may import `egui` but MUST NOT import `eframe`
//! or `rfd`. The panel function receives `ui: &mut egui::Ui` directly.
//!
//! **This file renders and reports** (ADR 0007 §D8). It spawns no thread and
//! constructs no `Document` and no `History` — until LCV-123 it did all three,
//! against a throwaway document whose geometry went nowhere, and it was the one
//! documented exception to ADR 0007 §D1. That exception is now empty:
//! `tests/lcv122_source_scans.rs` asserts no file under `src/agent/` names a
//! document, and this file's own `the_panel_renders_and_reports` asserts it
//! here by name. Submitting a prompt is one call into [`crate::app::start_turn`],
//! which owns the fence, the thread and the settings reads.

use crate::app::App;

// ── Public render entry ───────────────────────────────────────────────────────

/// Render the AI assistant chat panel into `ui`.
///
/// Call site: inside `egui::SidePanel::right("agent_panel")` in `App::update`,
/// gated by `app.agent_panel_open`. Wired in by LCV-080.
pub fn draw_agent_panel(ui: &mut egui::Ui, app: &mut App) {
    // ── Header row ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.heading("AI Assistant");
        // Push the close button to the right edge.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("×").clicked() {
                app.agent_panel_open = false;
            }
        });
    });

    ui.separator();

    // ── Chat history scroll area ──────────────────────────────────────────────
    // Reserve enough vertical space for the thinking indicator, separator, and
    // input row below (~60 px total).
    let scroll_height = (ui.available_height() - 60.0).max(0.0);

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_height(scroll_height)
        .show(ui, |ui| {
            for (role, content) in &app.agent_chat {
                match role.as_str() {
                    "user" => {
                        // Right-align user messages.
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.label(content);
                        });
                    }
                    "assistant" => {
                        ui.label(content);
                    }
                    "error" => {
                        ui.colored_label(egui::Color32::RED, content);
                    }
                    _ => {
                        ui.label(content);
                    }
                }
            }
        });

    // ── Thinking indicator ────────────────────────────────────────────────────
    if app.agent_busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Thinking…");
        });
    }

    ui.separator();

    // ── Input row ─────────────────────────────────────────────────────────────
    let can_send = !app.agent_busy && !app.agent_input_draft.trim().is_empty();
    let mut do_submit = false;

    ui.horizontal(|ui| {
        let text_resp = ui.add(
            egui::TextEdit::singleline(&mut app.agent_input_draft)
                .hint_text("Ask the AI…")
                .desired_width(f32::INFINITY),
        );
        // Submit when Enter is pressed with focus in the text field.
        if text_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            do_submit = true;
        }
        if ui
            .add_enabled(can_send, egui::Button::new("Send"))
            .clicked()
        {
            do_submit = true;
        }
    });

    if do_submit {
        let prompt = app.agent_input_draft.trim().to_owned();
        if !prompt.is_empty() && !app.agent_busy {
            // Cleared before the turn is armed, so the field is empty the
            // instant the operator's row appears in the chat above.
            app.agent_input_draft = String::new();
            crate::app::start_turn(app, &prompt);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::agent::AgentEvent;

    /// LCV-080 AC#3, retargeted by LCV-122 — the terminal events the spawned
    /// thread sends are distinct, and they are now the bridge's, not the
    /// panel's. `panel.rs` declaring its own payload type again is what this
    /// test's sibling scan in `tests/lcv122_source_scans.rs` catches.
    #[test]
    fn the_two_terminal_events_are_distinct() {
        let done = AgentEvent::Done("x".into());
        let failed = AgentEvent::Failed("e".into());
        assert_ne!(
            std::mem::discriminant(&done),
            std::mem::discriminant(&failed),
            "Done and Failed must be distinct variants",
        );
    }

    /// LCV-123 AC 2 / ADR 0007 §D8 — the panel renders and reports. It builds
    /// no document state and starts no thread; the whole of submitting is one
    /// call into `crate::app::start_turn`.
    ///
    /// A bounded source scan, because the property is *what this file is
    /// allowed to contain* and no runtime test can observe absence. The
    /// haystack stops at the bare `#[cfg(test)]` at column 0 and every needle
    /// is built with `concat!`, so this test cannot match its own source — and
    /// the loop below proves it: each needle is first found in a witness string
    /// that spells out what the pre-LCV-123 `submit` did, so a needle that had
    /// been silently misspelt fails here instead of passing vacuously.
    #[test]
    fn the_panel_renders_and_reports() {
        let src = include_str!("panel.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("panel.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("crate::app::start", "_turn(app, &prompt)")),
            "positive control: submitting must be one call into start_turn"
        );

        // What `submit` used to be, verbatim enough for every needle to hit.
        let witness = "let mut doc = Document::default(); \
                       let mut history = History::default(); \
                       let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>(); \
                       std::thread::spawn(move || {}); fn submit(app: &mut App)";
        for forbidden in [
            concat!("Doc", "ument"),
            concat!("His", "tory"),
            concat!("thread", "::spawn"),
            concat!("mpsc", "::channel"),
            concat!("fn sub", "mit"),
        ] {
            assert!(
                witness.contains(forbidden),
                "control: `{forbidden}` must be a needle that can match something"
            );
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "panel.rs must not name `{forbidden}`: {hit:?}"
            );
        }
    }
}
