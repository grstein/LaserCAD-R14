//! The status bar's on/off mode pill (LCV-184 AC 4).
//!
//! On: `fill.selected` fill, `accent` text. Off: no fill, a 1 pt `border`
//! outline and `text.muted` text, `fill.hover` while hovered. The label is
//! the state's second cue, so colour is never the only one (DESIGN.md §1.5).

use crate::ui::theme::{
    ACCENT, FILL_HOVER, FILL_SELECTED, TEXT_MUTED, WIDGET_ROUNDING, border_stroke,
};

/// Space between a pill's label and its edge, in points (x, y). The knob
/// for the status bar's width at 800 pt (LCV-184 AC 9).
pub(crate) const PILL_PADDING: egui::Vec2 = egui::vec2(6.0, 1.0);

/// Paint one clickable mode pill reading `label`, in its `on` or off state.
pub(crate) fn mode_pill(ui: &mut egui::Ui, on: bool, label: &str) -> egui::Response {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let colour = if on { ACCENT } else { TEXT_MUTED };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, colour);
    let size = galley.size() + 2.0 * PILL_PADDING;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, on, label)
    });
    if ui.is_rect_visible(rect) {
        let (fill, stroke) = if on {
            (FILL_SELECTED, egui::Stroke::NONE)
        } else if response.hovered() {
            (FILL_HOVER, border_stroke())
        } else {
            (egui::Color32::TRANSPARENT, border_stroke())
        };
        let painter = ui.painter();
        painter.rect(rect, WIDGET_ROUNDING, fill, stroke);
        painter.galley(rect.min + PILL_PADDING, galley, colour);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paint one pill alone and return its rect shape and text shape.
    fn paint(on: bool) -> (egui::epaint::RectShape, egui::epaint::TextShape) {
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| mode_pill(ui, on, "SNAP"));
        });
        let mut rect = None;
        let mut text = None;
        let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
        while let Some(shape) = stack.pop() {
            match shape {
                egui::Shape::Vec(inner) => stack.extend(inner),
                egui::Shape::Rect(r) if r.rect.width() < 200.0 => rect = Some(r),
                egui::Shape::Text(t) => text = Some(t),
                _ => {}
            }
        }
        (rect.expect("pill rect"), text.expect("pill text"))
    }

    #[test]
    fn an_on_pill_is_filled_selected_with_accent_text() {
        let (rect, text) = paint(true);
        assert_eq!(rect.fill, FILL_SELECTED);
        assert_eq!(rect.rounding, egui::Rounding::same(WIDGET_ROUNDING));
        assert_eq!(text.galley.job.sections[0].format.color, ACCENT);
    }

    #[test]
    fn an_off_pill_is_outlined_with_muted_text() {
        let (rect, text) = paint(false);
        assert_eq!(rect.fill, egui::Color32::TRANSPARENT);
        assert_eq!(rect.stroke, border_stroke());
        assert_eq!(text.galley.job.sections[0].format.color, TEXT_MUTED);
    }

    #[test]
    fn the_pill_pads_its_label() {
        let (rect, text) = paint(false);
        let expected = text.galley.size() + 2.0 * PILL_PADDING;
        assert_eq!(rect.rect.size(), expected);
        assert_eq!(text.pos, rect.rect.min + PILL_PADDING);
    }
}
