//! Command-line widget: the persistent bottom strip — prompt, feedback,
//! field (LCV-068, rewired by LCV-111).
//!
//! The strip mirrors the AutoCAD R14 command bar and renders three things
//! left to right: the active tool's `status_text()` as a read-only **prompt**
//! (pulled fresh every frame, ADR 0003 §C), the last submit's one-line
//! **feedback** in a warning colour (nothing at all when it is empty), and the
//! editable **field** the operator types into.
//!
//! **Enter** hands the field's text to [`crate::app::submit`] and clears it.
//! This widget does not parse, dispatch or resolve anything: the whole
//! contract lives in `src/app/cmdline.rs` (ADR 0003 §B5).
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

/// Draw the command-line strip inside `ui`.
///
/// Call this inside `egui::TopBottomPanel::bottom("command_line")` in
/// [`crate::app::App::update_ui`]. The panel must be declared **after**
/// `"statusbar"` so egui stacks it above the status bar.
pub fn draw_command_line(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        // Prompt label: reflects the active tool's current instruction.
        let prompt = app.tool_manager.active_status_text();
        ui.label(prompt);

        // Feedback: the last submit's result, in a colour the prompt never
        // uses. A display string only — nothing reads it back (LCV-111 AC 24).
        if !app.command_feedback.is_empty() {
            let colour = ui.visuals().warn_fg_color;
            ui.colored_label(colour, app.command_feedback.as_str());
        }

        ui.label("▶");

        // Single-line text input bound to app.command_line_input.
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.command_line_input).desired_width(f32::INFINITY),
        );

        // Mirror the focus state for the keyboard gate's recall check
        // (LCV-111 AC 20). Writing app state from a widget is allowed;
        // reading a key from one to dispatch is not.
        app.command_line_focused = response.has_focus();

        // The one-shot focus request set by the gate when the operator typed
        // a character while the field was unfocused (LCV-111 AC 21).
        if std::mem::take(&mut app.focus_command_line) {
            response.request_focus();
        }

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
            return;
        }

        // Enter: submit the text, then clear. A single-line TextEdit loses
        // focus when the user presses Enter. `crate::app::submit` and nothing
        // else (LCV-111 AC 8).
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let text = std::mem::take(&mut app.command_line_input);
            crate::app::submit(app, &text);
            // command_line_input is already "" after mem::take above.
        }
    });
}

#[cfg(test)]
mod tests {
    // The widget itself is exercised manually (headless environment).
    // The structural / delegation tests live in their respective modules.
    // See: src/app/mod.rs — app_default_command_line_input_is_empty  (AC#3)
    //      src/tools/tool.rs  — object-safety + no-op tests         (AC#4)
    //      src/tools/manager.rs — active_status_text + delegation   (AC#5, AC#6)
    //      src/app/cmdline.rs — the submit contract                 (LCV-111)
    //      tests/lcv111.rs    — focus, recall, Enter and Escape driven
    //                           through the real frame body
}
