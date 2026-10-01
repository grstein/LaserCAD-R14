//! One menu row (LCV-166): `icon slot | label | shortcut`.
//!
//! The label is a `LayoutJob` whose leading space is the icon slot, so every
//! row's label — plain, checked or a submenu title — starts at one x. The
//! shortcut is egui's own `Button::shortcut_text`, painted right-aligned in
//! the justified menu, so the keys form one column per menu. The icon is
//! painted into the slot afterwards, in the row's text colour.

use crate::ui::icons::{ICON_STROKE, IconFn, menu};

/// Side of the icon painted in a menu row's slot, in points.
pub(crate) const MENU_ICON: f32 = 16.0;

/// Space between the icon slot and the label, in points.
const SLOT_GAP: f32 = 6.0;

/// `label` laid out after an empty icon slot, in the button font and the
/// `PLACEHOLDER` colour (the painter substitutes the row's text colour).
pub(crate) fn slot_text(ui: &egui::Ui, label: &str) -> egui::text::LayoutJob {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let format = egui::TextFormat::simple(font, egui::Color32::PLACEHOLDER);
    let mut job = egui::text::LayoutJob::default();
    job.append(label, MENU_ICON + SLOT_GAP, format);
    job
}

/// One menu row: `icon` (or an empty slot), `label`, and `shortcut` in the
/// shortcut column (`""` for none). Disable it by wrapping it in
/// `ui.add_enabled_ui`; the icon then fades with the text.
pub(crate) fn menu_row(
    ui: &mut egui::Ui,
    icon: Option<IconFn>,
    label: &str,
    shortcut: &str,
) -> egui::Response {
    let response = row(ui, label, shortcut);
    if let Some(icon) = icon {
        paint_icon(ui, &response, icon);
    }
    response
}

/// A toggle row: a check mark in the slot while `on`. A click flips `on`
/// and marks the response changed; the menu stays open.
pub(crate) fn check_row(
    ui: &mut egui::Ui,
    on: &mut bool,
    label: &str,
    shortcut: &str,
) -> egui::Response {
    let mut response = row(ui, label, shortcut);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if *on {
        paint_icon(ui, &response, menu::check);
    }
    response
}

/// The row's button: slot-indented label, shortcut column.
fn row(ui: &mut egui::Ui, label: &str, shortcut: &str) -> egui::Response {
    ui.add(egui::Button::new(slot_text(ui, label)).shortcut_text(shortcut))
}

/// Paint `icon` in `response`'s slot, in the row's text colour.
fn paint_icon(ui: &egui::Ui, response: &egui::Response, icon: IconFn) {
    let rect = response.rect;
    let left = rect.min.x + ui.spacing().button_padding.x;
    let slot = egui::Rect::from_min_size(
        egui::pos2(left, rect.center().y - MENU_ICON / 2.0),
        egui::Vec2::splat(MENU_ICON),
    );
    let color = ui.style().interact(response).text_color();
    icon(ui.painter(), slot, egui::Stroke::new(ICON_STROKE, color));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paint one row through `add` in a justified top-down layout and
    /// return the label's first-glyph x and every path's visual bounds.
    fn paint(add: impl Fn(&mut egui::Ui)) -> (f32, Vec<egui::Rect>) {
        let ctx = egui::Context::default();
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let layout = egui::Layout::top_down_justified(egui::Align::LEFT);
                ui.with_layout(layout, &add);
            });
        });
        let mut label_x = f32::NAN;
        let mut paths = Vec::new();
        let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
        while let Some(shape) = stack.pop() {
            match shape {
                egui::Shape::Vec(inner) => stack.extend(inner),
                egui::Shape::Text(t) if t.galley.text() == "Save" => {
                    let glyph = &t.galley.rows[0].glyphs[0];
                    label_x = t.pos.x + glyph.pos.x;
                }
                egui::Shape::Path(_) => paths.push(shape.visual_bounding_rect()),
                _ => {}
            }
        }
        (label_x, paths)
    }

    /// LCV-166 AC 2 — the icon lies in the slot, left of the label.
    #[test]
    fn menu_row_icon_lies_left_of_the_label() {
        let (label_x, paths) = paint(|ui| {
            menu_row(ui, Some(menu::save), "Save", "Ctrl+S");
        });
        assert!(label_x.is_finite(), "the label must be painted");
        assert!(!paths.is_empty(), "the icon must be painted");
        for p in paths {
            assert!(
                p.max.x < label_x,
                "icon {p:?} reaches the label at {label_x}"
            );
        }
    }

    /// LCV-166 AC 3 — a check row paints its mark only while on, in the slot.
    #[test]
    fn check_row_marks_only_while_on() {
        let (_, off) = paint(|ui| {
            check_row(ui, &mut false, "Save", "F7");
        });
        assert!(off.is_empty(), "no mark while off: {off:?}");
        let (label_x, on) = paint(|ui| {
            check_row(ui, &mut true, "Save", "F7");
        });
        assert_eq!(on.len(), 1, "one tick while on");
        assert!(on[0].max.x < label_x);
    }
}
