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
//! `tests/it/repo/bridge_scans.rs` asserts no file under `src/agent/` names a
//! document, and this file's own `the_panel_renders_and_reports` asserts it
//! here by name. Submitting a prompt is one call into [`crate::app::start_turn`],
//! which owns the fence, the thread and the settings reads.
//!
//! **LCV-125 renders what LCV-123 records.** The seam is one-way: LCV-123 owns
//! the role vocabulary, every sentence and their order; this file owns only how
//! each role looks. It appends no row of its own and invents no seventh role.
//!
//! **LCV-129 adds the one control that ends a turn.** `Cancel` sits in the
//! thinking row, exists only while `agent.busy`, and calls
//! [`crate::app::cancel_turn`]. The row it leaves behind is `note` — written
//! by `agent_poll`, in the six-role vocabulary, like every other row here.
//!
//! **LCV-150 adds `New Conversation`** to the header, above the transcript's
//! `ScrollArea` so it never scrolls away. Enabled from `agent.busy` in the
//! frame it is drawn; its body is one call into
//! [`crate::app::AgentState::clear_conversation`], which empties the
//! transcript and the memory and starts nothing.

use crate::app::App;

/// Colour of an action / outcome row (AC 2).
///
/// A cool accent, deliberately neither the `#d0d0d0` the theme gives assistant
/// prose (`src/ui/theme.rs::apply_theme`) nor the red an `error` row uses, so
/// the §D5 renumbering sentence is findable in a column of chat at a glance.
/// `the_row_colours_are_distinct_under_the_real_theme` pins all three apart.
const TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 190, 255);

/// Vertical space (points) the separator and the composer row below the
/// transcript always need, whether or not a turn is running (LCV-080's
/// original reservation). Generous, not exact — see [`BUSY_ROW_RESERVE`].
const COMPOSER_RESERVE: f32 = 60.0;

/// Extra vertical space the busy "Thinking… / Cancel" row and its own
/// surrounding gap need, on top of [`COMPOSER_RESERVE`], only while a turn is
/// running (LCV-141 AC 4). Generous by design: the transcript gives up a
/// little more room than that row strictly needs rather than risk the row —
/// and the composer beneath it — being pushed past the panel's clipped
/// bottom edge, which is the defect this constant exists to close.
const BUSY_ROW_RESERVE: f32 = 40.0;

/// The footer's total reservation for one frame (LCV-141 AC 4): a plain
/// function of `busy`, not an inline `if app.agent.busy` next to the block
/// below — so a source scan bounding *that* block by its own guard (AC 8's
/// `busy_block`) cannot mistake this arithmetic for it.
fn footer_reserve(busy: bool) -> f32 {
    if busy {
        COMPOSER_RESERVE + BUSY_ROW_RESERVE
    } else {
        COMPOSER_RESERVE
    }
}

// ── Public render entry ───────────────────────────────────────────────────────

/// Render the AI assistant chat panel into `ui`.
///
/// Call site: inside `egui::SidePanel::right("agent_panel")` in `App::update`,
/// gated by `app.agent.panel_open`. Wired in by LCV-080.
pub fn draw_agent_panel(ui: &mut egui::Ui, app: &mut App) {
    // ── Header row ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.heading("AI Assistant");
        // Push the close button to the right edge.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("×").clicked() {
                app.agent.panel_open = false;
            }
            let new_conversation = egui::Button::new("New Conversation").small();
            if ui.add_enabled(!app.agent.busy, new_conversation).clicked() {
                app.agent.clear_conversation();
            }
        });
    });

    ui.separator();

    // ── Chat history scroll area ──────────────────────────────────────────────
    // Reserve room for the composer row below, and — read *before* the scroll
    // area claims what is left, not after — for the busy "Thinking… / Cancel"
    // row exactly on the frames it will actually render (LCV-141 AC 4). The
    // old flat constant reserved the same amount whether or not that row
    // existed, so a long transcript could push the row itself, and the
    // composer beneath it, past the panel's clipped bottom edge.
    let scroll_height = (ui.available_height() - footer_reserve(app.agent.busy)).max(0.0);

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_height(scroll_height)
        .show(ui, |ui| {
            for (role, content) in &app.agent.chat {
                draw_chat_row(ui, role, content);
            }
        });

    // ── Thinking indicator, and the way out of it ─────────────────────────────
    // The button lives inside the `agent.busy` block and nowhere else: it can
    // only offer to end a turn that is running, and when none is the row does
    // not exist at all (LCV-129 AC 8). Its body is one call into the app, the
    // same shape as the Send button's — this file renders and reports, and
    // ending a turn is `agent_poll`'s alone (ADR 0007 §D8, §D11).
    if app.agent.busy {
        ui.horizontal(|ui| {
            ui.spinner();
            // Progress is counted UI-side, per `Act` received (ADR 0007 §D13).
            let turn = &app.agent.turn;
            ui.label(format!(
                "Thinking… {} of {} steps",
                turn.tally.steps, turn.limit
            ));
            if ui.button("Cancel").clicked() {
                crate::app::cancel_turn(app);
            }
        });
    }

    ui.separator();

    // ── Input row ─────────────────────────────────────────────────────────────
    let can_send = !app.agent.busy && !app.agent.input_draft.trim().is_empty();
    let mut do_submit = false;

    ui.horizontal(|ui| {
        // Right-to-left, Send first: `desired_width(f32::INFINITY)` only caps
        // at `ui.available_width()` measured *before* the button is placed,
        // so a plain left-to-right row leaves the button no room and the row
        // grows past the panel's own width by however wide "Send" is (LCV-141
        // AC 1 / AC 5) — measured at this egui pin, an unbounded panel this
        // narrow reproduces it exactly. Placing Send first inside a
        // right-to-left layout pins it to the trailing edge and hands the
        // text field whatever `ui.available_width()` is left over.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(can_send, egui::Button::new("Send"))
                .clicked()
            {
                do_submit = true;
            }
            let text_resp = ui.add(
                egui::TextEdit::singleline(&mut app.agent.input_draft)
                    .hint_text("Ask the AI…")
                    .desired_width(ui.available_width()),
            );
            // Submit when Enter is pressed with focus in the text field.
            if text_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_submit = true;
            }
        });
    });

    if do_submit {
        let prompt = app.agent.input_draft.trim().to_owned();
        if !prompt.is_empty() && !app.agent.busy {
            // Cleared before the turn is armed, so the field is empty the
            // instant the operator's row appears in the chat above.
            app.agent.input_draft = String::new();
            crate::app::start_turn(app, &prompt);
        }
    }
}

// ── One transcript row ────────────────────────────────────────────────────────

/// Render one `agent.chat` row the way its role deserves (LCV-125 AC 1).
///
/// The vocabulary is **closed at six**, and every value in it is written
/// elsewhere — this function only decides how each one looks:
///
/// | role | written by | what it says |
/// |---|---|---|
/// | `user` | LCV-123 AC 3 (`app::arm_turn`), and LCV-124 when the prompt arrives from the command line | the prompt, verbatim |
/// | `tool` | LCV-123 AC 23 (`app::agent_apply::transcribe`) | one action that happened |
/// | `refused` | LCV-123 AC 23 (same site, plus `app::agent_poll` for a fence refusal) | one action that did not |
/// | `assistant` | LCV-123 AC 7 (`app::agent_poll::turn_end::end_turn`) | the model's closing prose |
/// | `error` | LCV-123 AC 7 (same site) | the turn failed, and why |
/// | `note` | LCV-123 AC 11 (`app::agent_poll::turn_end::finish_turn`), LCV-193 (`turn_end::end_turn`), LCV-129, LCV-187 | the turn's undo shape, its metrics line, a cancel, an image's fate |
///
/// Why `tool` and `refused` are loud: ADR 0007 §D5 makes the outcome sentence
/// the *disclosure mechanism* for positional indices shifting under the model
/// (`Deleted entity 1 (…). Indices 2..3 are now 1..2.`). A disclosure nobody
/// can pick out of a grey column is not a disclosure.
///
/// The `_` arm is the safety net: a role this build does not know still gets
/// rendered, plainly, rather than silently dropped.
fn draw_chat_row(ui: &mut egui::Ui, role: &str, content: &str) {
    match role {
        "user" => {
            // Right-aligned. `.wrap()` is load-bearing here and only here: a
            // right-to-left layout defaults to `TextWrapMode::Extend`, so a
            // long prompt would run off the panel instead of folding.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add(egui::Label::new(content).wrap());
            });
        }
        "assistant" => {
            ui.add(egui::Label::new(content).wrap());
        }
        "error" => {
            ui.add(egui::Label::new(egui::RichText::new(content).color(egui::Color32::RED)).wrap());
        }
        "tool" => {
            // One `ui.add` per row in a vertical layout, so two consecutive
            // actions are two lines and never run on (AC 2).
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("▸ {content}"))
                        .text_style(egui::TextStyle::Monospace)
                        .color(TOOL_COLOR),
                )
                .wrap(),
            );
        }
        "refused" => {
            // The refusal text is LCV-123's and is rendered verbatim; only the
            // marker and the colour are added here.
            let warn = ui.visuals().warn_fg_color;
            ui.add(
                egui::Label::new(egui::RichText::new(format!("⚠ {content}")).color(warn)).wrap(),
            );
        }
        "note" => {
            // The separator bounds the end of a turn, so the undo shape is
            // findable without scrolling back through it (AC 4).
            ui.separator();
            ui.add(
                egui::Label::new(egui::RichText::new(content).text_style(egui::TextStyle::Small))
                    .wrap(),
            );
        }
        _ => {
            ui.label(content);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
