//! egui panel for editing agent connection settings.
//!
//! One of the **two** files in `src/agent/` that may import `egui` — the other
//! is `panel.rs` (ADR 0007 §D8). Every other `agent/` submodule is kernel-pure,
//! and no file here may import `eframe` or `rfd`.
//!
//! LCV-125 took the form from two fields to four. The two it added are not new
//! behaviour: `agent_model` and `agent_step_budget` have been honoured since
//! LCV-121, they were simply only reachable by hand-editing `settings.json`.
//! LCV-143 added the fifth, the system prompt.

use egui;

use crate::agent::memory::{CONTEXT_TOKENS_MAX, CONTEXT_TOKENS_MIN};
use crate::agent::{AGENT_STEP_BUDGET_MAX, AGENT_STEP_BUDGET_MIN, prompt};
use crate::io::settings::Settings;

mod copy;
use copy::{
    CANVAS_DISCLOSURE, LIVE_EDIT_NOTE, MODEL_HINT, PLAINTEXT_KEY_WARNING, STEP_BUDGET_HELP,
};

/// Minimum width of every text field, in logical pixels.
const FIELD_MIN_WIDTH: f32 = 320.0;

/// The system prompt editor's `egui::Id` source (LCV-169 AC 1): Enter stays
/// a newline while this editor has focus.
pub const SYSTEM_PROMPT_ID: &str = "agent_system_prompt";

/// Minimum width of a grid column, so the label column of the field grid and
/// of the budget grid line up despite being two separate grids.
const LABEL_COL_WIDTH: f32 = 110.0;

/// Widest the form may get. The two sentences below are long; without a bound
/// they would each stretch the dialog to a single line hundreds of pixels wide.
const FORM_MAX_WIDTH: f32 = 470.0;

/// Rows the system-prompt editor asks for before its own scroll area.
const PROMPT_ROWS: usize = 6;

/// Tallest the system-prompt editor may get: a long prompt scrolls inside it
/// rather than pushing Close down the 426pt dialog body (ADR 0009).
const PROMPT_MAX_HEIGHT: f32 = 110.0;

/// What one frame of the form reported.
#[derive(Debug)]
pub struct AgentSettingsFrame {
    /// `true` if **any** of the eight fields changed this frame — the flag
    /// [`draw_agent_settings`] always reported, now carried on a named field
    /// instead of being the whole return value.
    pub changed: bool,
    /// `true` if the Close button (LCV-141 AC 6) was clicked this frame. The
    /// caller — `src/app/panels.rs::agent_settings_dialog` — closes the
    /// window through the exact same path it already runs for the × button:
    /// same `persist_settings()` call, same `was_open &&
    /// !app.agent_settings_open` guard, never a second, parallel persistence
    /// path.
    pub close_clicked: bool,
}

/// Draw the agent-settings form into `ui`.
///
/// Labelled rows, in the order the operator meets them:
/// - **Endpoint URL** — plain single-line field editing `settings.agent_endpoint`.
/// - **Model** — plain single-line field editing `settings.agent_model`. Free
///   text, hinted with the default: the endpoint is the authority on which ids
///   exist, and it already reports a bad one through LCV-121's status mapping.
/// - **API Key** — password-masked field editing `settings.agent_api_key`,
///   followed by [`PLAINTEXT_KEY_WARNING`]. The mask stops shoulder-surfing;
///   the warning states where the bytes actually live. Both, deliberately.
/// - **Steps per turn** — a slider over
///   `AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX` (the range lives with the
///   loop that enforces it, ADR 0007 §D7) editing `settings.agent_step_budget`.
/// - **Context tokens** — an integer over
///   `CONTEXT_TOKENS_MIN..=CONTEXT_TOKENS_MAX` editing
///   `settings.agent_context_tokens` (LCV-153).
/// - **Allow canvas capture** / **Model supports images** — the two LCV-145
///   opt-ins, followed by [`CANVAS_DISCLOSURE`].
/// - **System prompt** — a multiline editor over the *effective* prompt
///   (`prompt::resolve`), with a **Restore Default** button (LCV-143). The
///   text is copied into a per-frame buffer, so only a real edit writes
///   `Some(text)` — opening the dialog creates no override — and the button,
///   drawn before the editor, sets `None` so the built-in text shows on the
///   same frame.
///
/// Every text field has a minimum width of [`FIELD_MIN_WIDTH`] logical pixels.
///
/// [`AgentSettingsFrame::changed`] is `true` if **any** of the eight fields
/// changed this frame, `false` otherwise — `agent_settings_dialog` persists on
/// close, so the flag is what tells the operator's edit apart from an idle
/// frame. [`AgentSettingsFrame::close_clicked`] is `true` the one frame the new
/// Close button (AC 6) is clicked.
///
/// The slider clamps as it draws (`SliderClamping::Always`), so a settings file
/// hand-edited to `5000` is written back as `4096` on the first frame the dialog
/// is open — and reports `changed` that frame, which is what gets the clamped
/// value persisted. The read-site clamp in `src/app/agent_turn.rs` stays
/// regardless: a file the dialog was never opened on is still a hand-edited file.
///
/// The body is not wrapped in its own `ScrollArea`: `agent_settings_dialog`
/// wraps the whole call in one (AC 7, ADR 0009), so a caller measuring this
/// function in isolation — every inline test below — sees exactly the rows
/// and sentences this function draws, with no scrolling machinery of its own
/// to account for.
pub fn draw_agent_settings(ui: &mut egui::Ui, settings: &mut Settings) -> AgentSettingsFrame {
    let mut changed = false;
    ui.set_max_width(FORM_MAX_WIDTH);

    grid("agent_settings_grid").show(ui, |ui| {
        ui.label("Endpoint URL");
        changed |= ui.add(field(&mut settings.agent_endpoint)).changed();
        ui.end_row();

        ui.label("Model");
        changed |= ui
            .add(field(&mut settings.agent_model).hint_text(MODEL_HINT))
            .changed();
        ui.end_row();

        ui.label("API Key");
        changed |= ui
            .add(field(&mut settings.agent_api_key).password(true))
            .changed();
        ui.end_row();
    });

    warn_label(ui, PLAINTEXT_KEY_WARNING);

    grid("agent_budget_grid").show(ui, |ui| {
        ui.label("Steps per turn");
        let before = settings.agent_step_budget;
        // Logarithmic: 1..=4096 (LCV-142) is too wide for a linear track to
        // land on small values; the number box still takes any typed value.
        let budget = ui.add(
            egui::Slider::new(
                &mut settings.agent_step_budget,
                AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX,
            )
            .logarithmic(true),
        );
        // `Response::changed()` covers an interaction; the comparison covers the
        // clamp. egui writes an out-of-range stored value back into range as it
        // draws but reports nothing, so without the second term a file
        // hand-edited to `5000` would be silently corrected and never persisted.
        changed |= budget.changed() || settings.agent_step_budget != before;
        ui.end_row();
    });
    ui.add(egui::Label::new(egui::RichText::new(STEP_BUDGET_HELP).small()).wrap());

    // LCV-153: memory is capped at half of this. Clamped as drawn, like the
    // budget, so the comparison catches a silent write-back.
    grid("agent_context_grid").show(ui, |ui| {
        ui.label("Context tokens");
        let before = settings.agent_context_tokens;
        let range = CONTEXT_TOKENS_MIN..=CONTEXT_TOKENS_MAX;
        let tokens = ui.add(egui::DragValue::new(&mut settings.agent_context_tokens).range(range));
        changed |= tokens.changed() || settings.agent_context_tokens != before;
        ui.end_row();
    });

    // LCV-145: both off by default; read live at every capture.
    changed |= ui
        .checkbox(
            &mut settings.agent_allow_canvas_capture,
            "Allow canvas capture",
        )
        .changed();
    changed |= ui
        .checkbox(
            &mut settings.agent_model_supports_vision,
            "Model supports images",
        )
        .changed();
    ui.add(egui::Label::new(egui::RichText::new(CANVAS_DISCLOSURE).small()).wrap());

    changed |= prompt_editor(ui, settings);

    ui.add(egui::Label::new(egui::RichText::new(LIVE_EDIT_NOTE).small()).wrap());
    let close_clicked = ui.button("Close").clicked();

    AgentSettingsFrame {
        changed,
        close_clicked,
    }
}

/// The System prompt row: label, Restore Default, then the editor. Returns
/// whether either changed `settings.agent_system_prompt`.
///
/// The editor carries the fixed id [`SYSTEM_PROMPT_ID`]: while it has focus,
/// `src/app/input.rs::take_dialog_key` leaves Enter to it, so Enter inserts
/// a newline instead of closing the dialog (LCV-169 AC 1).
fn prompt_editor(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("System prompt");
        if ui.button("Restore Default").clicked() {
            settings.agent_system_prompt = None;
            changed = true;
        }
    });
    let mut text = prompt::resolve(settings.agent_system_prompt.as_deref()).to_owned();
    let edited = egui::ScrollArea::vertical()
        .id_salt("agent_system_prompt_scroll")
        .max_height(PROMPT_MAX_HEIGHT)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .id(egui::Id::new(SYSTEM_PROMPT_ID))
                    .desired_width(f32::INFINITY)
                    .desired_rows(PROMPT_ROWS),
            )
            .changed()
        })
        .inner;
    if edited {
        settings.agent_system_prompt = Some(text);
        changed = true;
    }
    changed
}

/// One two-column form grid. Two of them, so the long sentences between and
/// below them get the full form width to wrap into rather than a grid cell.
fn grid(id: &str) -> egui::Grid {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([8.0, 6.0])
        .min_col_width(LABEL_COL_WIDTH)
}

/// One single-line text field of at least [`FIELD_MIN_WIDTH`] points.
fn field(text: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text).min_size(egui::vec2(FIELD_MIN_WIDTH, 0.0))
}

/// One always-visible warning line, wrapped inside the form width.
fn warn_label(ui: &mut egui::Ui, text: &str) {
    let warn = ui.visuals().warn_fg_color;
    ui.add(egui::Label::new(egui::RichText::new(text).color(warn)).wrap());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
