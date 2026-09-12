//! Command-line widget: a persistent bottom strip with a prompt label and a
//! single-line text field for entering precise numeric values (LCV-068).
//!
//! The strip mirrors the AutoCAD R14 command bar: the active tool's
//! `status_text()` is displayed as a read-only prompt on the left, followed
//! by an editable field the user types into.
//!
//! **Enter** submits the current text to the active tool via
//! `ToolManager::on_command_input`, then clears the field.
//!
//! **Escape** clears the field — and nothing else. Cancelling the active tool
//! belongs to the keyboard gate in `src/app/input.rs`, which already ran this
//! frame (LCV-103 / ADR 0002 §A6). This widget must never route a key into
//! the active tool, and must never take a key event out of the input state to
//! hide a collision: a second reader of a key is a double dispatch.
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
        ui.label("▶");

        // Single-line text input bound to app.command_line_input.
        let response = ui.add(
            egui::TextEdit::singleline(&mut app.command_line_input).desired_width(f32::INFINITY),
        );

        // Escape: clear this widget's own buffer, nothing else.
        //
        // `lost_focus()`, not `has_focus()`: egui drops keyboard focus at the
        // *start* of the frame carrying an Escape press (a `TextEdit`'s event
        // filter has `escape: false`), so by the time the widget is drawn it
        // no longer has focus — it only just lost it.
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.command_line_input.clear();
            return;
        }

        // Enter: submit text, then clear.
        // A single-line TextEdit loses focus when the user presses Enter.
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let text = std::mem::take(&mut app.command_line_input);
            let mut tm = std::mem::take(&mut app.tool_manager);
            tm.on_command_input(&text, &mut app.document, &mut app.history);
            app.tool_manager = tm;
            // command_line_input is already "" after mem::take above.
        }
    });
}

#[cfg(test)]
mod tests {
    // The widget itself is exercised manually (headless environment).
    // The structural / delegation tests live in their respective modules.
    // See: src/app.rs   — app_default_command_line_input_is_empty  (AC#3)
    //      src/tools/tool.rs  — object-safety + no-op tests         (AC#4)
    //      src/tools/manager.rs — active_status_text + delegation   (AC#5, AC#6)
}
