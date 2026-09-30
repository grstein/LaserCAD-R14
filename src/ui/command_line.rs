//! Command-line widget: the persistent bottom dock — prompt, feedback,
//! destination preview, field (LCV-068, rewired by LCV-111, holds focus in
//! raw mode by LCV-112, split into two rows by LCV-139).
//!
//! Since LCV-139 the dock is two rows, not one: a **context row** (the active
//! tool's `status_text()` as a read-only prompt, the last submit's feedback,
//! and a read-only destination label answering "where would Enter send this,
//! right now?") above an **editable row** holding the single-line field
//! alone. The split exists so a long prompt or feedback message can never
//! squeeze, truncate or overlap the field an operator is actively typing
//! into — the two failure modes the old single-row strip had no way to avoid
//! (demand §Problem).
//!
//! **Enter** hands the field's text to [`crate::app::submit`] and clears it.
//! This widget does not parse, dispatch or resolve anything: the whole
//! contract lives in `src/app/cmdline.rs` (ADR 0003 §B5). The destination
//! label is read-only in the same sense: [`crate::ui::destination_label`] is
//! the only thing that decides its text, and it opens no panel, arms no
//! turn, sends nothing and mutates nothing (LCV-139 AC 6).
//!
//! **Raw-input mode** (ADR 0003 §D): while the active tool wants the command
//! line as a free-text field (`TextTool` mid-flow — LCV-112), this widget
//! requests focus every frame instead of only on the one-shot seed. That is
//! the whole reason raw mode needs no keyboard-gate exception: with the
//! field focused, `ctx.wants_keyboard_input()` already suppresses the bare
//! tool-activation keys.
//!
//! **Escape** clears the field and the feedback — and nothing else.
//! Cancelling the active tool belongs to the keyboard gate in
//! `src/app/input.rs`, which already ran this frame (LCV-103 / ADR 0002 §A6).
//! This widget must never route a key into the active tool, and must never
//! take a key event out of the input state to hide a collision: a second
//! reader of a key is a double dispatch. The two `key_pressed` reads below
//! exist **solely to disambiguate this widget's own `lost_focus()`**
//! (ADR 0003 §E5) — never to dispatch.
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-068.

use crate::app::App;
use crate::ui::command_destination::destination_label;

/// Prompt verb / request / option split (LCV-184).
mod prompt;

/// Bound on either the prompt or the feedback segment's width in the context
/// row (LCV-139 AC 2): beyond it, egui's own `Label::truncate` elides the
/// text to one row and attaches a full-text tooltip automatically whenever
/// the laid-out galley reports `elided == true` (egui-0.29.1
/// `widgets/label.rs::Widget::ui`). Neither `app.command_line_input` nor
/// `app.command_feedback` nor the recall ring is ever touched by this: only
/// a borrowed `&str` is laid out, so the *stored* text stays byte-exact and
/// only the *display* elides.
const CONTEXT_SEGMENT_MAX_WIDTH: f32 = 260.0;

/// The single-line editor's fixed id (LCV-139 AC 1): a test reads its own
/// painted rect back with `ctx.read_response`, which needs a stable id
/// rather than one derived from call-site position.
fn editor_id() -> egui::Id {
    egui::Id::new("command_line_editor")
}

/// Draw the two-row command-line dock inside `ui`.
///
/// Call this inside `egui::TopBottomPanel::bottom("command_line")` in
/// [`crate::app::App::update_ui`]. The panel must be declared **after**
/// `"statusbar"` so egui stacks it above the status bar.
pub fn draw_command_line(ui: &mut egui::Ui, app: &mut App) {
    ui.vertical(|ui| {
        draw_context_row(ui, app);
        draw_editor_row(ui, app);
    });
}

/// The context row: prompt, feedback, destination label — read-only, bounded,
/// side-effect-free (LCV-139 AC 1-6).
fn draw_context_row(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        // Prompt label: reflects the active tool's current instruction.
        let prompt = app.tool_manager.active_status_text();
        bounded_label(ui, prompt_job(ui, prompt));

        // Feedback: the last submit's result, in a colour the prompt never
        // uses. A display string only — nothing reads it back (LCV-111 AC 24).
        // A line starting `! ` is an error, painted `status.error` (LCV-167).
        if !app.command_feedback.is_empty() {
            let colour = if app.command_feedback.starts_with("! ") {
                ui.visuals().error_fg_color
            } else {
                ui.visuals().warn_fg_color
            };
            let feedback = egui::RichText::new(app.command_feedback.as_str()).color(colour);
            bounded_label(ui, feedback);
        }

        // LCV-139 AC 3-6: what today's Enter would do with the field's exact
        // current text. `wants_raw_input` and `crate::app::agent_available`
        // are read, never written; `destination_label` calls nothing that
        // could open a panel, arm a turn or touch the document/history.
        let destination = destination_label(
            &app.command_line_input,
            app.tool_manager.wants_raw_input(),
            crate::app::agent_available(app),
            app.agent.busy,
        );
        ui.label("Destination:");
        ui.label(destination);
    });
}

/// Paint `text` truncated to at most [`CONTEXT_SEGMENT_MAX_WIDTH`] points.
/// `Label::truncate()` attaches egui's own full-text hover tooltip
/// automatically once the laid-out galley no longer fits (LCV-139 AC 2) —
/// nothing here re-implements or requests that.
fn bounded_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) {
    ui.scope(|ui| {
        ui.set_max_width(CONTEXT_SEGMENT_MAX_WIDTH);
        ui.add(egui::Label::new(text).truncate());
    });
}

/// The prompt as one layout job, each [`prompt::PromptPart`] in its colour
/// (LCV-184 AC 6): verb `accent`, request `text.primary`, options
/// `text.muted`.
fn prompt_job(ui: &egui::Ui, text: &str) -> egui::text::LayoutJob {
    use crate::ui::theme::{ACCENT, TEXT_MUTED, TEXT_PRIMARY};
    use prompt::PromptPart;
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let valign = ui.text_valign();
    let mut job = egui::text::LayoutJob::default();
    for (part, span) in prompt::prompt_spans(text) {
        let color = match part {
            PromptPart::Verb => ACCENT,
            PromptPart::Request => TEXT_PRIMARY,
            PromptPart::Option => TEXT_MUTED,
        };
        let format = egui::TextFormat {
            font_id: font_id.clone(),
            color,
            valign,
            ..Default::default()
        };
        job.append(span, 0.0, format);
    }
    job
}

/// The editable row: the single-line field alone (LCV-139 AC 1, AC 7) — a
/// long context row can never squeeze or overlap it, because it never shares
/// a row with one.
///
/// LCV-184 AC 7: the row sits in a 1 pt frame — `border`, or `accent` while
/// the editor holds keyboard focus. The frame is begun before the row and
/// coloured after it, so it reads this frame's focus, not last frame's.
fn draw_editor_row(ui: &mut egui::Ui, app: &mut App) {
    use crate::ui::theme::{ACCENT, WIDGET_ROUNDING, border_stroke};
    let mut frame = egui::Frame::none()
        .rounding(WIDGET_ROUNDING)
        .inner_margin(EDITOR_FRAME_MARGIN)
        .begin(ui);
    draw_editor_contents(&mut frame.content_ui, app);
    frame.frame.stroke = match app.command_line_focused {
        true => egui::Stroke::new(border_stroke().width, ACCENT),
        false => border_stroke(),
    };
    frame.end(ui);
}

/// Inner margin of the editor row's frame, in points.
const EDITOR_FRAME_MARGIN: egui::Margin = egui::Margin {
    left: 4.0,
    right: 4.0,
    top: 2.0,
    bottom: 2.0,
};

/// The editor row's contents: the `▶` marker and the frameless field.
fn draw_editor_contents(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        ui.label("\u{25b6}");

        // Single-line text input bound to app.command_line_input; the row's
        // frame above is its border (LCV-184 AC 7).
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.command_line_input)
                .id(editor_id())
                .frame(false)
                .desired_width(f32::INFINITY),
        );

        // Mirror the focus state for the keyboard gate's recall check
        // (LCV-111 AC 20). Writing app state from a widget is allowed;
        // reading a key from one to dispatch is not.
        app.command_line_focused = response.has_focus();

        // The one-shot focus request set by the gate when the operator typed
        // a character while the field was unfocused (LCV-111 AC 21).
        let seeded = std::mem::take(&mut app.focus_command_line);

        // Escape: clear this widget's own buffer and the feedback, nothing
        // else — a stale error must always be dismissible.
        //
        // `lost_focus()`, not `has_focus()`: egui drops keyboard focus at the
        // *start* of the frame carrying an Escape press (a `TextEdit`'s event
        // filter has `escape: false`), so by the time the widget is drawn it
        // no longer has focus — it only just lost it.
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.command_line_input.clear();
            app.command_feedback.clear();
        } else if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            // Enter: submit the text, then clear. A single-line TextEdit
            // loses focus when the user presses Enter. `crate::app::submit`
            // and nothing else (LCV-111 AC 8).
            let text = std::mem::take(&mut app.command_line_input);
            crate::app::submit(app, &text);
            // command_line_input is already "" after mem::take above.
        }

        // Raw-input mode holds the field (ADR 0003 §D, LCV-112 AC 3): while
        // the active tool wants the command line as a free-text field, the
        // widget must keep requesting focus every frame, or the operator
        // would have to re-click the field for every character. Read
        // *after* the Enter branch above, so a submit that returns the tool
        // to `Idle` (e.g. the final accepted height) releases focus in the
        // same frame it completes, exactly like any other submit.
        if seeded || app.tool_manager.wants_raw_input() {
            response.request_focus();
        }
    });
}

#[cfg(test)]
mod tests {
    // The widget itself is exercised through the real frame body; the
    // structural / delegation tests live in their respective modules.
    // See: src/app/mod.rs — app_default_command_line_input_is_empty  (AC#3)
    //      src/tools/tool.rs  — object-safety + no-op tests         (AC#4)
    //      src/tools/manager.rs — active_status_text + delegation   (AC#5, AC#6)
    //      src/app/cmdline.rs — the submit contract                 (LCV-111)
    //      src/ui/command_destination.rs — the destination table    (LCV-139)
    //      tests/it/cmdline/drives_tools.rs — focus, recall, Enter and Escape driven
    //                           through the real frame body
    //      tests/it/cmdline/text_command.rs — raw-input focus-holding and the TEXT flow
    //      tests/it/cmdline/context_row.rs — the two-row layout,
    //                           its bounds and the destination label's paint
}
