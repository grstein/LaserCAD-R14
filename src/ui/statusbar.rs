//! Status bar widget rendered at the bottom of the application window.
//!
//! Exposes two public items:
//! - [`format_coords`] — pure string formatter for cursor coordinates (unit-testable).
//! - [`draw_statusbar`] — egui widget that reads live state from [`App`] and
//!   renders the bar.
//!
//! Introduced by demand LCV-067.

use crate::app::App;
use crate::geometry::Vec2;

/// Format cursor world-space coordinates for display in the status bar.
///
/// Returns `"X: 123.45  Y:  67.89"` (values right-aligned in 6 chars, 2 dp)
/// when `pos` is `Some`, or `"X: —  Y: —"` (em-dash) when `pos` is `None`.
///
/// # Examples
/// ```
/// use lasercad::ui::format_coords;
/// use lasercad::geometry::Vec2;
///
/// assert_eq!(format_coords(None), "X: —  Y: —");
/// let s = format_coords(Some(Vec2::new(123.45, 67.89)));
/// assert_eq!(s, "X: 123.45  Y:  67.89");
/// ```
pub fn format_coords(pos: Option<Vec2>) -> String {
    match pos {
        None => "X: \u{2014}  Y: \u{2014}".to_owned(),
        Some(p) => format!("X: {:>6.2}  Y: {:>6.2}", p.x, p.y),
    }
}

/// Return the ORTHO mode badge string when ortho is active.
///
/// Returns `Some("ORTHO")` when `ortho == true`, `None` otherwise.
/// Used by [`draw_statusbar`] to conditionally show the badge (LCV-053).
pub(crate) fn format_ortho(ortho: bool) -> Option<&'static str> {
    if ortho {
        Some("ORTHO")
    } else {
        None
    }
}

/// Render the status bar into `ui`.
///
/// Displays — left to right — cursor coordinates, a separator, the active tool
/// name (uppercased), a separator, the document entity count, and — when ortho
/// mode is on — a separator followed by `"ORTHO"` (LCV-053).
///
/// **Call site**: add a `TopBottomPanel::bottom("statusbar")` *before* the
/// `CentralPanel` in `App::update`.
pub fn draw_statusbar(ui: &mut egui::Ui, app: &App) {
    let coord_str = format_coords(app.last_cursor_world);
    let tool_str = app.tool_manager.active_tool_name().to_uppercase();
    let count = app.document.entity_count();

    ui.horizontal(|ui| {
        ui.label(&coord_str);
        ui.separator();
        ui.label(&tool_str);
        ui.separator();
        ui.label(format!("Entities: {count}"));
        if let Some(badge) = format_ortho(app.ortho_enabled) {
            ui.separator();
            ui.label(badge);
        }
    });
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// LCV-067 AC — `format_coords(None)` produces the em-dash placeholder.
    #[test]
    fn format_coords_none_returns_dash_placeholder() {
        assert_eq!(format_coords(None), "X: \u{2014}  Y: \u{2014}");
    }

    /// LCV-067 AC — positive coordinates are formatted to two decimal places,
    /// right-aligned in a 6-char field.
    #[test]
    fn format_coords_some_positive_values() {
        let s = format_coords(Some(Vec2::new(123.45, 67.89)));
        assert_eq!(s, "X: 123.45  Y:  67.89");
    }

    /// LCV-067 AC — negative x value is formatted correctly (sign included in
    /// the 6-char field, extending it naturally).
    #[test]
    fn format_coords_some_negative_x() {
        let s = format_coords(Some(Vec2::new(-5.0, 0.0)));
        assert_eq!(s, "X:  -5.00  Y:   0.00");
    }

    /// LCV-067 AC — zero coordinates produce all-zero output, not "—".
    #[test]
    fn format_coords_some_zero() {
        let s = format_coords(Some(Vec2::new(0.0, 0.0)));
        assert_eq!(s, "X:   0.00  Y:   0.00");
    }

    /// LCV-067 AC — large values are not truncated (no width cap).
    #[test]
    fn format_coords_large_values_not_truncated() {
        let s = format_coords(Some(Vec2::new(1234.56, 9876.54)));
        // 1234.56 is 7 chars — the field is at least that wide.
        assert!(s.contains("1234.56"), "x value present");
        assert!(s.contains("9876.54"), "y value present");
    }

    /// LCV-053 AC#13 — `format_ortho(true)` returns `Some("ORTHO")`.
    #[test]
    fn format_ortho_true_returns_some_badge() {
        assert_eq!(format_ortho(true), Some("ORTHO"));
    }

    /// LCV-053 AC#13 — `format_ortho(false)` returns `None`.
    #[test]
    fn format_ortho_false_returns_none() {
        assert_eq!(format_ortho(false), None);
    }
}
