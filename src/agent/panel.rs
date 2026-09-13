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
//! **The single documented exception to ADR 0007 §D1** (LCV-122 AC 3): this is
//! the only file under `src/agent/` that names `Document` or `History`. It
//! constructs the throwaway pair that [`submit`] hands to the turn, which is a
//! defect ADR 0007 exists to close and which LCV-123 closes in one commit, by
//! replacing the spawn below with the fenced rendezvous. Nothing new may be
//! added to this exception, and `tests/lcv122_source_scans.rs` asserts the
//! exception list is exactly one file long.

use crate::agent::AgentEvent;
use crate::app::App;
use crate::document::{Document, History};

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
        submit(app);
    }
}

// ── Submit logic ──────────────────────────────────────────────────────────────

/// Pull the current input draft, push a user message onto the history, spawn
/// the background agent thread, and arm the receiver channel.
///
/// Returns early when `agent_input_draft` is empty or `agent_busy` is already
/// `true` (guard against double-submit within a single frame).
fn submit(app: &mut App) {
    let text = app.agent_input_draft.trim().to_string();
    if text.is_empty() || app.agent_busy {
        return;
    }

    // 1. Record the user turn.
    app.agent_chat.push(("user".into(), text.clone()));
    // 2. Clear the draft.
    app.agent_input_draft = String::new();

    // 3. Clone settings so the spawned thread owns its own copy. The budget
    //    is clamped here, at the read site: the stored value comes from a
    //    hand-editable JSON file (ADR 0007 §D7). This is the temporary home of
    //    both reads — LCV-123 moves them into `src/app/agent_turn.rs`.
    let endpoint = app.settings.agent_endpoint.clone();
    let api_key = app.settings.agent_api_key.clone();
    let model = app.settings.agent_model.clone();
    let step_budget = crate::agent::loop_::clamp_step_budget(app.settings.agent_step_budget);

    // 4. Arm the channel.
    let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
    app.agent_rx = Some(rx);
    app.agent_busy = true;

    // 5. Spawn OS thread (run_agent_turn is synchronous / blocking).
    std::thread::spawn(move || {
        let mut doc = Document::default();
        let mut history = History::default();
        let result = crate::app::run_agent_turn(
            &text,
            &endpoint,
            &api_key,
            &model,
            step_budget,
            &mut doc,
            &mut history,
        );
        let msg = match result {
            Ok(reply) => AgentEvent::Done(reply),
            Err(e) => AgentEvent::Failed(e.to_string()),
        };
        tx.send(msg).ok();
    });
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    /// LCV-121 AC 14 — `submit` is the one place that reads the two new agent
    /// settings until LCV-123 moves the spawn into `src/app/agent_turn.rs`, and
    /// it must clamp the budget rather than trust the stored value.
    ///
    /// A bounded source scan: spawning a real turn needs a socket and a live
    /// `egui::Ui`, and the property at stake is *what submit reads*. The
    /// haystack stops at the bare `#[cfg(test)]` at column 0 and the needles
    /// are built with `concat!`, so this test cannot match its own source.
    #[test]
    fn submit_reads_the_model_and_clamps_the_budget() {
        let src = include_str!("panel.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("panel.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        for needle in [
            concat!("settings.agent_", "model"),
            concat!("clamp_step", "_budget(app.settings.agent_step_budget)"),
        ] {
            assert!(
                implementation.contains(needle),
                "submit must pass `{needle}` into run_agent_turn"
            );
        }
        assert!(
            !implementation.contains(concat!("app.settings.agent_step_budget,")),
            "the stored budget must reach run_agent_turn only through the clamp"
        );
    }
}
