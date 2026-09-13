//! Status bar widget rendered at the bottom of the application window.
//!
//! Exposes two public items:
//! - [`format_coords`] — pure string formatter for cursor coordinates (unit-testable).
//! - [`draw_statusbar`] — egui widget that reads live state from [`App`] and
//!   renders the bar.
//!
//! The badge formatters ([`format_ortho`], [`format_preset`]) are crate-private
//! pure functions: the bar itself cannot be scraped for text, so they are what
//! the tests assert on.
//!
//! Introduced by demand LCV-067.

use crate::app::App;
use crate::geometry::Vec2;
use crate::io::Preset;

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

/// Return the status-bar label for an export preset.
///
/// `"CUT"` / `"MARK"` / `"ENGRAVE"`, uppercase to match the tool and ORTHO
/// segments. Unconditional — unlike [`format_ortho`] there is no "off" state,
/// because every save writes into exactly one preset and an operator who
/// cannot see which one can burn through the workpiece (LCV-115 AC 7).
pub(crate) fn format_preset(preset: Preset) -> &'static str {
    match preset {
        Preset::Cut => "CUT",
        Preset::Mark => "MARK",
        Preset::Engrave => "ENGRAVE",
    }
}

/// Render the status bar into `ui`.
///
/// Displays — left to right — cursor coordinates, a separator, the active tool
/// name (uppercased), a separator, the document entity count, a separator and
/// the active export preset (LCV-115), and — when ortho mode is on — a
/// separator followed by `"ORTHO"` (LCV-053).
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
        ui.separator();
        ui.label(format_preset(app.export_preset));
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

    /// LCV-115 AC#7 — all three presets have an uppercase badge, and it is the
    /// preset's `id()` uppercased, so the label can never name a different
    /// group than the one the exporter writes.
    #[test]
    fn format_preset_covers_all_three_variants() {
        assert_eq!(format_preset(Preset::Cut), "CUT");
        assert_eq!(format_preset(Preset::Mark), "MARK");
        assert_eq!(format_preset(Preset::Engrave), "ENGRAVE");
        for preset in Preset::ALL {
            assert_eq!(
                format_preset(preset),
                preset.id().to_uppercase(),
                "badge must be the group id, uppercased"
            );
        }
    }

    /// LCV-115 AC#7 — the preset segment is unconditional and sits between the
    /// entity count and the conditional ORTHO badge. Bounded to `draw_statusbar`
    /// so this test's own body cannot satisfy the scan.
    #[test]
    fn status_bar_shows_the_preset_after_the_entity_count() {
        let src = include_str!("statusbar.rs");
        let start = src
            .find("pub fn draw_statusbar(")
            .expect("draw_statusbar must exist");
        let end = src[start..]
            .find("\n#[cfg(test)]")
            .expect("tests must follow the implementation")
            + start;
        let body = &src[start..end];
        let count = body
            .find("Entities: {count}")
            .expect("entity count present");
        let preset = body
            .find("format_preset(app.export_preset)")
            .expect("preset badge present");
        let ortho = body
            .find("format_ortho(app.ortho_enabled)")
            .expect("ortho badge present");
        assert!(count < preset && preset < ortho, "count, preset, ortho");
        assert!(
            !body[..preset].contains("if "),
            "the preset badge must not sit behind a conditional"
        );
    }
}
