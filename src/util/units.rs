//! Bed geometry constants and the world ↔ SVG Y mirror.
//!
//! LaserCAD's world is **Y-up** (mathematical / AutoCAD convention); SVG is
//! **Y-down**. Everything that crosses that boundary — today `io::svg::export`
//! and `io::svg::import` — goes through [`flip_y`], so the mirror lives in
//! exactly one place. [`BED_WIDTH_MM`] / [`BED_HEIGHT_MM`] are the same
//! numbers the viewport bed overlay (`render::bed::Bed::default`) draws, so
//! what the operator sees on screen is what LaserGRBL shows.
//!
//! Millimetres are canonical here, as everywhere in the kernel.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Introduced by demand LCV-100. A configurable bed (LCV-114) turns these
//! constants into a parameter threaded through [`flip_y`].

/// Bed width along world X, in millimetres.
///
/// Matches common GRBL hobbyist machines (Ortur LM2/LM3, Atomstack A5/A10).
pub const BED_WIDTH_MM: f64 = 400.0;

/// Bed height along world Y, in millimetres. This is the axis the SVG
/// pipeline mirrors around — see [`flip_y`].
pub const BED_HEIGHT_MM: f64 = 400.0;

/// Mirror a Y coordinate between world space (Y-up) and SVG space (Y-down):
/// `y_svg = BED_HEIGHT_MM - y_world`.
///
/// The map is an **involution** — `flip_y(flip_y(y)) == y` — so the same
/// function serves both directions: world → SVG on export and SVG → world on
/// import. It is defined for every finite input, including coordinates
/// outside the bed (which yield negative or over-bed values, never clamped).
///
/// ```
/// use lasercad::util::{flip_y, BED_HEIGHT_MM};
/// assert_eq!(flip_y(0.0), BED_HEIGHT_MM);
/// assert_eq!(flip_y(flip_y(37.5)), 37.5);
/// ```
pub fn flip_y(y_mm: f64) -> f64 {
    BED_HEIGHT_MM - y_mm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;

    /// AC 1 — the three items resolve through `crate::util`.
    #[test]
    fn units_module_exports_constants_and_flip() {
        use crate::util::{flip_y as reexported_flip, BED_HEIGHT_MM as H, BED_WIDTH_MM as W};
        assert_eq!(W, 400.0);
        assert_eq!(H, 400.0);
        assert_eq!(reexported_flip(0.0), 400.0);
    }

    /// AC 2 — `flip_y` matches the fixed bed and is its own inverse.
    #[test]
    fn flip_y_is_an_involution() {
        assert_eq!(flip_y(0.0), 400.0);
        assert_eq!(flip_y(400.0), 0.0);
        assert_eq!(flip_y(10.0), 390.0);
        for y in [0.0, 10.0, 250.0, 400.0, -5.0, 512.5] {
            assert!((flip_y(flip_y(y)) - y).abs() < EPSILON, "y={y}");
        }
    }

    /// Out-of-bed coordinates are mirrored, never clamped (AC 12's kernel half).
    #[test]
    fn flip_y_does_not_clamp_out_of_bed_values() {
        assert_eq!(flip_y(500.0), -100.0);
        assert_eq!(flip_y(-100.0), 500.0);
    }
}
