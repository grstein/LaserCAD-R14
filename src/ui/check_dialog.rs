//! The Check window (LCV-190 AC 1): a read-only snapshot of the last `check`
//! report, every line in a scroll area under the ADR 0009 body cap.
//!
//! Non-modal and keyless: it reads no keyboard event (ADR 0002 §A6) and is
//! not one of the `Dialog`s that take Enter and Escape (LCV-169), so the
//! command line keeps working while it is open. Close (or the title-bar ×)
//! clears the report. MUST NOT import `eframe` or `rfd`.

/// ADR 0009: the whole body (report plus Close row) fits in this many points.
const BODY_CAP: f32 = 426.0;

/// Paint the Check window while `report` is `Some`; Close sets it to `None`.
pub fn check_dialog(ctx: &egui::Context, report: &mut Option<Vec<String>>) {
    let Some(lines) = report.as_ref() else {
        return;
    };
    let mut open = true;
    let mut close = false;
    egui::Window::new("Check")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            let spacing = ui.spacing();
            let close_row = spacing.interact_size.y + 2.0 * spacing.item_spacing.y;
            egui::ScrollArea::vertical()
                .max_height(BODY_CAP - close_row)
                .show(ui, |ui| {
                    for line in lines {
                        ui.add(egui::Label::new(line.as_str()).extend());
                    }
                });
            close = ui.button("Close").clicked();
        });
    if close || !open {
        *report = None;
    }
}
