//! The snap kind's name beside its glyph (LCV-164 AC 3): `endpoint`,
//! `midpoint`, … in the `snap` colour, below and right of the point.
//!
//! The name lives here, not on the kernel's `SnapKind`: it is display text.
//!
//! MUST NOT import `eframe` or `rfd`.

use crate::geometry::SnapKind;

/// Offset, in points, from the snap point to the label's left-top corner:
/// past the glyph's right edge by 4 pt and below its bottom edge by 2 pt, so
/// the text covers neither the point nor the crosshair lines through it.
pub(crate) const LABEL_OFFSET_PT: egui::Vec2 = egui::Vec2::new(
    super::MARKER_SIZE_PX / 2.0 + 4.0,
    super::MARKER_SIZE_PX / 2.0 + 2.0,
);

/// The lower-case name painted beside `kind`'s glyph.
pub(crate) fn snap_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "endpoint",
        SnapKind::Midpoint => "midpoint",
        SnapKind::Center => "center",
        SnapKind::Intersection => "intersection",
        SnapKind::Quadrant => "quadrant",
        SnapKind::Perpendicular => "perpendicular",
        SnapKind::Tangent => "tangent",
        SnapKind::Nearest => "nearest",
    }
}

/// Paint `kind`'s name at `pos + LABEL_OFFSET_PT` in the body font and
/// `color`, with no backing.
pub(crate) fn draw_label(
    painter: &egui::Painter,
    pos: egui::Pos2,
    kind: SnapKind,
    color: egui::Color32,
) {
    let font = egui::TextStyle::Body.resolve(&painter.ctx().global_style());
    let at = pos + LABEL_OFFSET_PT;
    painter.text(at, egui::Align2::LEFT_TOP, snap_label(kind), font, color);
}
