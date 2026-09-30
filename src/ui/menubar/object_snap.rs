//! View > Object snap (LCV-161): one checkbox per [`SnapKind`].
//!
//! The item list is [`SNAP_KIND_LABELS`]; a toggle goes through
//! [`App::set_object_snap`], which writes `settings.object_snaps` and
//! persists it. F3 (`App::snap_enabled`) stays the master switch.

use crate::app::App;
use crate::geometry::SnapKind;

/// Every snap kind with its menu label, in menu order.
const SNAP_KIND_LABELS: [(SnapKind, &str); 8] = [
    (SnapKind::Endpoint, "Endpoint"),
    (SnapKind::Midpoint, "Midpoint"),
    (SnapKind::Center, "Center"),
    (SnapKind::Intersection, "Intersection"),
    (SnapKind::Quadrant, "Quadrant"),
    (SnapKind::Perpendicular, "Perpendicular"),
    (SnapKind::Tangent, "Tangent"),
    (SnapKind::Nearest, "Nearest"),
];

/// The `Object snap` submenu: one checkbox per [`SNAP_KIND_LABELS`] entry.
pub(super) fn object_snap_menu(ui: &mut egui::Ui, app: &mut App) {
    ui.menu_button("Object snap", |ui| {
        for (kind, label) in SNAP_KIND_LABELS {
            let mut on = app.settings.object_snaps.contains(kind);
            if ui.checkbox(&mut on, label).changed() {
                app.set_object_snap(kind, on);
            }
        }
    });
}
