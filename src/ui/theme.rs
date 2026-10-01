//! Dark CAD theme for LaserCAD.
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
/// `status.error` — errors, read by panels as egui's `error_fg_color`
/// (`#ff6b6b`, LCV-167).
pub(crate) const STATUS_ERROR: Color32 = Color32::from_rgb(0xff, 0x6b, 0x6b);
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

/// Width of the one chrome border weight, in points (DESIGN.md §5).
const BORDER_WIDTH: f32 = 1.0;

/// Corner radius of every widget, in points (DESIGN.md §5).
pub(crate) const WIDGET_ROUNDING: u8 = 3;

/// Corner radius of windows and menus, in points (DESIGN.md §5).
const WINDOW_ROUNDING: u8 = 4;

/// The 1 pt `border` stroke: windows, menus, separators, off pills.
pub(crate) fn border_stroke() -> egui::Stroke {
    egui::Stroke::new(BORDER_WIDTH, BORDER)
}

/// Apply the LaserCAD dark theme to the given egui context.
///
/// Flat (DESIGN.md §1.8): no shadow, one 1 pt `border`, 3 pt widget and
/// 4 pt window/menu corners, and every fill and text colour set explicitly
/// from the tokens above instead of left to egui's defaults. `accent` is only
/// ever a foreground — the selection's stroke, which egui uses for focus
/// rings and selected text — never a fill (LCV-184 AC 3).
///
/// Calling this once per frame (top of `App::update`) is idempotent and
/// cheap — egui only re-tessellates when visuals actually change.
pub fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT_PRIMARY);
    v.panel_fill = BG_PANEL;
    v.window_fill = BG_PANEL;
    v.extreme_bg_color = CANVAS_BG;
    v.warn_fg_color = STATUS_WARNING;
    v.error_fg_color = STATUS_ERROR;
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_stroke = border_stroke();
    v.window_corner_radius = egui::CornerRadius::same(WINDOW_ROUNDING);
    v.menu_corner_radius = egui::CornerRadius::same(WINDOW_ROUNDING);
    v.selection.bg_fill = FILL_SELECTED;
    v.selection.stroke = egui::Stroke::new(BORDER_WIDTH, ACCENT);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = BG_PANEL;
    w.noninteractive.weak_bg_fill = BG_PANEL;
    w.noninteractive.bg_stroke = border_stroke();
    for (state, fill) in [
        (&mut w.inactive, FILL_WIDGET),
        (&mut w.hovered, FILL_HOVER),
        (&mut w.active, FILL_ACTIVE),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
    }
    w.hovered.bg_stroke = border_stroke();
    w.active.bg_stroke = border_stroke();
    w.open.bg_fill = BG_PANEL;
    w.open.weak_bg_fill = FILL_ACTIVE;
    w.open.bg_stroke = border_stroke();
    for state in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.corner_radius = egui::CornerRadius::same(WIDGET_ROUNDING);
        state.fg_stroke.color = TEXT_PRIMARY;
    }
    ctx.set_visuals(v);
    // egui 0.36 raised Body and Button to 13 pt and Monospace to 13 pt; keep
    // the 12.5 / 12.5 / 12 pt every layout here was measured at (LCV-180).
    ctx.all_styles_mut(|style| {
        for (text_style, size) in [
            (egui::TextStyle::Body, 12.5),
            (egui::TextStyle::Button, 12.5),
            (egui::TextStyle::Monospace, 12.0),
        ] {
            if let Some(font) = style.text_styles.get_mut(&text_style) {
                font.size = size;
            }
        }
    });
    // egui 0.36 would otherwise send `ViewportCommand::SetTheme` from
    // `end_pass`, a viewport command that costs idle frames; the native
    // window keeps following the system theme, as under egui 0.29 (LCV-180).
    ctx.options_mut(|o| o.sync_window_theme = false);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every chrome token, by its DESIGN.md §3 name.
    const TOKENS: [(&str, Color32); 12] = [
        ("bg.canvas", BG_CANVAS),
        ("bg.panel", BG_PANEL),
        ("text.primary", TEXT_PRIMARY),
        ("text.muted", TEXT_MUTED),
        ("fill.selected", FILL_SELECTED),
        ("status.warning", STATUS_WARNING),
        ("status.error", STATUS_ERROR),
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

    /// WCAG 2 contrast ratio of two opaque colours.
    fn contrast(a: Color32, b: Color32) -> f64 {
        let luminance = |c: Color32| {
            let ch = |v: u8| {
                let v = f64::from(v) / 255.0;
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// LCV-167 AC 7 — after `apply_theme` egui's own error colour is
    /// `status.error` (#ff6b6b), readable at ≥4.5:1 on `bg.panel`; the old
    /// `Color32::RED` is not (control).
    #[test]
    fn status_error_is_egui_error_colour_and_readable_on_the_panel() {
        let v = themed();
        assert_eq!(hex(v.error_fg_color), "#ff6b6b");
        let ratio = contrast(v.error_fg_color, BG_PANEL);
        assert!(ratio >= 4.5, "status.error on bg.panel: {ratio:.2}:1");
        assert!(contrast(Color32::RED, BG_PANEL) < 4.5, "control");
    }

    /// LCV-071 AC 1 — `CANVAS_BG` is exactly `#1a1a1a`.
    #[test]
    fn canvas_bg_colour_components() {
        assert_eq!(CANVAS_BG, Color32::from_rgb(26, 26, 26));
    }

    fn themed() -> egui::Visuals {
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        ctx.global_style().visuals.clone()
    }

    /// LCV-184 AC 2 — no shadows, one 1 pt border, 3 pt widgets, 4 pt
    /// windows and menus, token fills per widget state.
    #[test]
    fn apply_theme_is_flat_with_one_border_and_token_fills() {
        let v = themed();
        assert_eq!(v.window_shadow, egui::epaint::Shadow::NONE);
        assert_eq!(v.popup_shadow, egui::epaint::Shadow::NONE);
        assert_eq!(v.window_stroke, egui::Stroke::new(1.0_f32, BORDER));
        assert_eq!(v.window_corner_radius, egui::CornerRadius::same(4));
        assert_eq!(v.menu_corner_radius, egui::CornerRadius::same(4));
        let w = &v.widgets;
        for state in [
            &w.noninteractive,
            &w.inactive,
            &w.hovered,
            &w.active,
            &w.open,
        ] {
            assert_eq!(state.corner_radius, egui::CornerRadius::same(3));
            assert_eq!(state.fg_stroke.color, TEXT_PRIMARY);
        }
        assert_eq!(
            w.noninteractive.bg_stroke,
            egui::Stroke::new(1.0_f32, BORDER)
        );
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
        assert_eq!(v.selection.stroke, egui::Stroke::new(1.0_f32, ACCENT));
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
