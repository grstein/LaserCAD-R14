//! Canvas tokens (DESIGN.md §3): the single home of `preview`, `danger` and
//! `hover` (LCV-163 AC 8) and of the grid, origin and snap-edge tokens (LCV-164).
//!
//! State on the canvas is shown by form first (dash, width) and hue second,
//! because entity colour belongs to the layer (LCV-156).
//!
//! MUST NOT import `eframe` or `rfd`.

/// The `preview` token: translucent amber, rgba(255,220,100,160), for
/// rubber-band geometry and selection boxes (LCV-037).
///
/// Stored premultiplied, as egui 0.29's `from_rgba_unmultiplied` computed it
/// (in linear space): egui 0.36 premultiplies in gamma space, which would
/// darken the painted colour (LCV-180).
pub fn preview() -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(208, 179, 80, 160)
}

/// The `danger` token: opaque red-pink #ff4d6a for what TRIM or ERASE will
/// remove. ≈4.6:1 on the bed (gray 40); hue ≥30° from `preview`, `snap` and
/// `status.warning`, so it never reads as another amber.
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(255, 77, 106);

/// The `hover` token: stroke width, in points, of the entity under the
/// pickbox. Its colour is the entity's layer colour; the width (thicker than
/// the 1 pt entity stroke) is what marks it.
pub const HOVER_WIDTH_PT: f32 = 2.5;

/// The `grid.minor` token: gray 62, ≈1.38:1 on the bed fill (gray 40),
/// painted 1 pt wide (LCV-164 AC 1).
pub const GRID_MINOR: egui::Color32 = egui::Color32::from_gray(62);

/// The `grid.major` token: gray 96, ≈2.34:1 on the bed fill, every tenth
/// line, 1 pt wide (LCV-164 AC 1).
pub const GRID_MAJOR: egui::Color32 = egui::Color32::from_gray(96);

/// The `snap.edge` token: the canvas background, painted under every snap
/// glyph shape 2 pt wider, so the glyph reads over bright geometry
/// (LCV-164 AC 2). The one colour `render/` takes from `ui/`.
pub const SNAP_EDGE: egui::Color32 = crate::ui::CANVAS_BG;

/// The `origin` token: gray 220, the machine origin (0,0) where LaserGRBL
/// starts (LCV-164 AC 4). Its arms lie on the bed's lower-left border, so
/// width and brightness, not hue, set them apart.
pub const ORIGIN: egui::Color32 = egui::Color32::from_gray(220);

/// Length, in points, of each origin arm, along +X and +Y.
pub const ORIGIN_ARM_PT: f32 = 12.0;

/// Stroke width, in points, of the origin arms.
pub const ORIGIN_WIDTH_PT: f32 = 2.0;

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2 relative luminance of an opaque colour.
    fn luminance(c: egui::Color32) -> f64 {
        let ch = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
    }

    /// HSV hue in degrees of an sRGB triple.
    fn hue([r, g, b]: [u8; 3]) -> f64 {
        let (r, g, b) = (f64::from(r), f64::from(g), f64::from(b));
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let d = max - min;
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / d)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        h.rem_euclid(360.0)
    }

    fn hue_distance(a: f64, b: f64) -> f64 {
        let d = (a - b).abs();
        d.min(360.0 - d)
    }

    /// WCAG 2 contrast ratio of two opaque colours.
    fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// LCV-164 AC 1 — minor grid ≥1.35:1 and major grid ≥2.2:1 on the bed
    /// fill (gray 40), both opaque.
    #[test]
    fn grid_tokens_contrast_with_the_bed_fill() {
        let bed = egui::Color32::from_gray(40);
        assert!(GRID_MINOR.a() == 255 && GRID_MAJOR.a() == 255);
        let (minor, major) = (contrast(GRID_MINOR, bed), contrast(GRID_MAJOR, bed));
        assert!(minor >= 1.35, "minor grid contrast {minor:.2}:1");
        assert!(major >= 2.2, "major grid contrast {major:.2}:1");
    }

    /// LCV-163 AC 8 — `danger` is opaque with ≥3:1 WCAG contrast on the bed
    /// fill (gray 40).
    #[test]
    fn danger_contrasts_with_the_bed_fill() {
        assert_eq!(DANGER.a(), 255);
        let (a, b) = (luminance(DANGER), luminance(egui::Color32::from_gray(40)));
        let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
        assert!(ratio >= 3.0, "contrast {ratio:.2}:1");
    }

    /// LCV-163 AC 8 — `danger`'s hue is ≥30° from `preview`, `snap`
    /// (#ffa000) and `status.warning` (#ff8f00).
    #[test]
    fn danger_hue_is_distinct_from_preview_snap_and_warning() {
        let danger = hue([DANGER.r(), DANGER.g(), DANGER.b()]);
        let snap = crate::render::snaps::marker_color();
        for (name, other) in [
            ("preview", [255, 220, 100]),
            ("snap", [snap.r(), snap.g(), snap.b()]),
            ("status.warning", [0xff, 0x8f, 0x00]),
        ] {
            let d = hue_distance(danger, hue(other));
            assert!(d >= 30.0, "danger is {d:.1}° from {name}");
        }
    }

    /// LCV-163 AC 8 — `preview` keeps its translucent amber.
    #[test]
    fn preview_is_translucent_amber() {
        assert!(preview().a() < 255);
    }

    /// LCV-163 AC 3, AC 8 — the hover stroke is thicker than the 1 pt entity
    /// stroke.
    #[test]
    fn hover_width_is_thicker_than_one_point() {
        const { assert!(HOVER_WIDTH_PT > 1.0) };
    }
}
