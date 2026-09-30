//! Dark CAD theme for LaserCAD v2.
//!
//! Provides [`CANVAS_BG`] (the dark viewport background colour) and
//! [`apply_theme`], which configures egui's [`Visuals`] for a
//! dark-on-dark CAD aesthetic suitable for laser-cutting workflows.
//!
//! LCV-071: Theme + visual polish. LCV-184: every chrome colour is a named
//! token below, one per DESIGN.md §3 row; nothing else in `src/ui/` builds a
//! `Color32` from a literal.

use egui::Color32;

/// `bg.canvas` — the canvas outside the bed (`#1a1a1a`).
pub(crate) const BG_CANVAS: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
/// `bg.panel` — panels and windows (`#252525`).
pub(crate) const BG_PANEL: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
/// `text.primary` — all chrome text (`#d0d0d0`).
pub(crate) const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xd0, 0xd0, 0xd0);
/// `text.muted` — secondary text, only on `bg.panel` (`#8c8c8c`).
pub(crate) const TEXT_MUTED: Color32 = Color32::from_rgb(0x8c, 0x8c, 0x8c);
/// `fill.selected` — selected widget fill (`#005c80`).
pub(crate) const FILL_SELECTED: Color32 = Color32::from_rgb(0x00, 0x5c, 0x80);
/// `status.warning` — warnings and command feedback (`#ff8f00`).
pub(crate) const STATUS_WARNING: Color32 = Color32::from_rgb(0xff, 0x8f, 0x00);
/// `accent` — foreground-only highlight, never a fill (`#4fa3e0`).
pub(crate) const ACCENT: Color32 = Color32::from_rgb(0x4f, 0xa3, 0xe0);
/// `border` — the one 1 pt chrome border, gray 64 (`#404040`).
pub(crate) const BORDER: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);
/// `fill.widget` — an idle button or field (`#3c3c3c`).
pub(crate) const FILL_WIDGET: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x3c);
/// `fill.hover` — a hovered widget (`#464646`).
pub(crate) const FILL_HOVER: Color32 = Color32::from_rgb(0x46, 0x46, 0x46);
/// `fill.active` — a pressed or open widget (`#373737`).
pub(crate) const FILL_ACTIVE: Color32 = Color32::from_rgb(0x37, 0x37, 0x37);

/// Background colour for the CAD viewport canvas: the `bg.canvas` token.
///
/// Used both as `extreme_bg_color` in the egui visuals and directly by
/// [`crate::app::App::update`] to fill the central-panel rect.
pub const CANVAS_BG: Color32 = BG_CANVAS;

/// Apply the LaserCAD dark theme to the given egui context.
///
/// Calling this once per frame (top of `App::update`) is idempotent and
/// cheap — egui only re-tessellates when visuals actually change.
pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_PANEL;
    visuals.extreme_bg_color = CANVAS_BG;
    ctx.set_visuals(visuals);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every chrome token, by its DESIGN.md §3 name.
    const TOKENS: [(&str, Color32); 11] = [
        ("bg.canvas", BG_CANVAS),
        ("bg.panel", BG_PANEL),
        ("text.primary", TEXT_PRIMARY),
        ("text.muted", TEXT_MUTED),
        ("fill.selected", FILL_SELECTED),
        ("status.warning", STATUS_WARNING),
        ("accent", ACCENT),
        ("border", BORDER),
        ("fill.widget", FILL_WIDGET),
        ("fill.hover", FILL_HOVER),
        ("fill.active", FILL_ACTIVE),
    ];

    /// The rows of DESIGN.md §3's chrome table, as `(token, value, home)`.
    fn chrome_rows() -> Vec<(String, String, String)> {
        let design = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/DESIGN.md"));
        let start = design.find("**Chrome**").expect("§3 chrome table");
        let end = design[start..].find("**Canvas**").expect("§3 canvas table") + start;
        design[start..end]
            .lines()
            .filter(|l| l.starts_with("| `"))
            .map(|l| {
                let cells: Vec<&str> = l.split('|').map(str::trim).collect();
                let token = cells[1].trim_matches('`').to_owned();
                (token, cells[2].to_owned(), cells[5].to_owned())
            })
            .collect()
    }

    fn hex(c: Color32) -> String {
        format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
    }

    /// LCV-184 AC 1 — each token constant matches its DESIGN.md §3 row, and
    /// every chrome row whose Home is `ui/theme.rs` is a token here.
    #[test]
    fn tokens_match_design_md_chrome_rows() {
        let rows = chrome_rows();
        assert!(rows.len() >= 9, "positive control: the table parsed");
        for (name, colour) in TOKENS {
            let row = rows.iter().find(|r| r.0 == name);
            let (_, value, home) = row.unwrap_or_else(|| panic!("no §3 row for `{name}`"));
            assert_eq!(value, &hex(colour), "`{name}` value");
            assert!(home.contains("ui/theme.rs"), "`{name}` Home: {home}");
        }
        for (name, _, home) in &rows {
            if home.contains("ui/theme.rs") {
                assert!(
                    TOKENS.iter().any(|t| t.0 == name),
                    "`{name}` has no constant"
                );
            }
        }
    }

    /// LCV-071 AC 1 — `CANVAS_BG` is exactly `#1a1a1a`.
    #[test]
    fn canvas_bg_colour_components() {
        assert_eq!(CANVAS_BG, Color32::from_rgb(26, 26, 26));
    }

    fn themed() -> egui::Visuals {
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        ctx.style().visuals.clone()
    }

    /// LCV-184 AC 2 — no shadows, one 1 pt border, 3 pt widgets, 4 pt
    /// windows and menus, token fills per widget state.
    #[test]
    fn apply_theme_is_flat_with_one_border_and_token_fills() {
        let v = themed();
        assert_eq!(v.window_shadow, egui::epaint::Shadow::NONE);
        assert_eq!(v.popup_shadow, egui::epaint::Shadow::NONE);
        assert_eq!(v.window_stroke, egui::Stroke::new(1.0, BORDER));
        assert_eq!(v.window_rounding, egui::Rounding::same(4.0));
        assert_eq!(v.menu_rounding, egui::Rounding::same(4.0));
        let w = &v.widgets;
        for state in [
            &w.noninteractive,
            &w.inactive,
            &w.hovered,
            &w.active,
            &w.open,
        ] {
            assert_eq!(state.rounding, egui::Rounding::same(3.0));
            assert_eq!(state.fg_stroke.color, TEXT_PRIMARY);
        }
        assert_eq!(w.noninteractive.bg_stroke, egui::Stroke::new(1.0, BORDER));
        for (state, fill) in [
            (&w.inactive, FILL_WIDGET),
            (&w.hovered, FILL_HOVER),
            (&w.active, FILL_ACTIVE),
        ] {
            assert_eq!((state.bg_fill, state.weak_bg_fill), (fill, fill));
        }
        assert_eq!(v.panel_fill, BG_PANEL);
        assert_eq!(v.window_fill, BG_PANEL);
        assert_eq!(v.extreme_bg_color, BG_CANVAS);
        assert_eq!(v.override_text_color, Some(TEXT_PRIMARY));
        assert_eq!(v.warn_fg_color, STATUS_WARNING);
    }

    /// LCV-184 AC 3 — the selection is `fill.selected` with an `accent`
    /// foreground, and no fill anywhere in the visuals is `accent`.
    #[test]
    fn accent_is_foreground_only() {
        let v = themed();
        assert_eq!(v.selection.bg_fill, FILL_SELECTED);
        assert_eq!(v.selection.stroke, egui::Stroke::new(1.0, ACCENT));
        let w = &v.widgets;
        let mut fills = vec![
            v.selection.bg_fill,
            v.panel_fill,
            v.window_fill,
            v.extreme_bg_color,
            v.faint_bg_color,
            v.code_bg_color,
        ];
        for state in [
            &w.noninteractive,
            &w.inactive,
            &w.hovered,
            &w.active,
            &w.open,
        ] {
            fills.extend([state.bg_fill, state.weak_bg_fill]);
        }
        assert!(
            !fills.contains(&ACCENT),
            "accent is never a fill: {fills:?}"
        );
    }
}
