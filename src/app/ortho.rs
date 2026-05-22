//! Ortho-constraint helper (LCV-053).
//!
//! [`apply_ortho`] constrains a cursor position to the nearest horizontal or
//! vertical axis from a fixed anchor point. This is the single source of truth
//! for the F8 ortho-lock pipeline in [`super::App::update`].
//!
//! No `egui`, `eframe`, or `rfd` imports — only [`crate::geometry::Vec2`].

use crate::geometry::Vec2;

/// Constrain `cursor` to the nearest cardinal axis from `anchor`.
///
/// Computes `dx = |cursor.x − anchor.x|` and `dy = |cursor.y − anchor.y|`.
/// - If `dx >= dy`: horizontal constraint → `Vec2::new(cursor.x, anchor.y)`.
/// - If `dy > dx`: vertical constraint → `Vec2::new(anchor.x, cursor.y)`.
///
/// When `cursor == anchor` (degenerate, `dx == dy == 0`) returns `anchor`
/// unchanged (horizontal branch wins; result is harmless). Introduced by
/// demand LCV-053.
pub fn apply_ortho(anchor: Vec2, cursor: Vec2) -> Vec2 {
    let dx = (cursor.x - anchor.x).abs();
    let dy = (cursor.y - anchor.y).abs();
    if dx >= dy {
        Vec2::new(cursor.x, anchor.y)
    } else {
        Vec2::new(anchor.x, cursor.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ao(ax: f64, ay: f64, cx: f64, cy: f64) -> Vec2 {
        apply_ortho(Vec2::new(ax, ay), Vec2::new(cx, cy))
    }

    #[test] // AC 6a — horizontal when |dx| > |dy|
    fn ortho_horizontal_dx_gt_dy() {
        assert_eq!(ao(0.0, 0.0, 10.0, 3.0), Vec2::new(10.0, 0.0));
    }

    #[test] // AC 6b — vertical when |dy| > |dx|
    fn ortho_vertical_dy_gt_dx() {
        assert_eq!(ao(0.0, 0.0, 3.0, 10.0), Vec2::new(0.0, 10.0));
    }

    #[test] // AC 6c — tie (dx == dy) prefers horizontal
    fn ortho_tie_prefers_horizontal() {
        assert_eq!(ao(0.0, 0.0, 5.0, 5.0), Vec2::new(5.0, 0.0));
    }

    #[test] // AC 6d — offset anchor, horizontal (dx=7, dy=4)
    fn ortho_offset_anchor_horizontal() {
        assert_eq!(ao(5.0, 3.0, 12.0, 7.0), Vec2::new(12.0, 3.0));
    }

    #[test] // AC 6e — offset anchor, vertical (dx=1, dy=6)
    fn ortho_offset_anchor_vertical() {
        assert_eq!(ao(5.0, 3.0, 6.0, 9.0), Vec2::new(5.0, 9.0));
    }

    #[test] // AC 6f — degenerate: cursor on anchor
    fn ortho_degenerate_cursor_on_anchor() {
        assert_eq!(ao(2.0, 2.0, 2.0, 2.0), Vec2::new(2.0, 2.0));
    }

    #[test] // AC 6, negative coords — horizontal
    fn ortho_negative_offset_horizontal() {
        assert_eq!(ao(0.0, 0.0, -8.0, 2.0), Vec2::new(-8.0, 0.0));
    }

    #[test] // AC 6, negative coords — vertical
    fn ortho_negative_offset_vertical() {
        assert_eq!(ao(0.0, 0.0, 1.0, -9.0), Vec2::new(0.0, -9.0));
    }
}
