//! Bed geometry constants and the world ↔ SVG Y mirror.
//!
//! LaserCAD's world is **Y-up** (mathematical / AutoCAD convention); SVG is
//! **Y-down**. Everything that crosses that boundary — today `io::svg::export`
//! and `io::svg::import` — goes through [`flip_y`], so the mirror lives in
//! exactly one place.
//!
//! The bed itself is **not** a constant: it is
//! [`Document::bed_mm`](crate::document::Document::bed_mm), owned by the
//! document, exported into the SVG header and read back on import (LCV-114).
//! The constants here are the *seed* for a blank document
//! ([`DEFAULT_BED_WIDTH_MM`] × [`DEFAULT_BED_HEIGHT_MM`]) and the range a
//! configured bed is held inside ([`BED_MIN_MM`] ..= [`BED_MAX_MM`], via
//! [`clamp_bed_mm`]). Nothing in an export, import or mirror path may read
//! them as the mirror axis — that axis is always the bed height of the
//! document being written or read.
//!
//! Millimetres are canonical here, as everywhere in the kernel.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.
//!
//! Introduced by demand LCV-100; parameterised by LCV-114.

/// Default bed width along world X, in millimetres.
///
/// The blank-document seed only (LCV-114 product decision 3): the live width
/// is `Document::bed_mm[0]`. Matches common GRBL hobbyist machines
/// (Ortur LM2/LM3, Atomstack A5/A10).
pub const DEFAULT_BED_WIDTH_MM: f64 = 400.0;

/// Default bed height along world Y, in millimetres.
///
/// The blank-document seed only: the axis the SVG pipeline mirrors around is
/// `Document::bed_mm[1]` of the document being exported or imported — see
/// [`flip_y`]. This constant is also the fallback bed for an SVG whose root
/// carries neither `width`/`height` nor `viewBox` (LCV-114 AC 8c).
pub const DEFAULT_BED_HEIGHT_MM: f64 = 400.0;

/// Smallest configurable bed dimension, in millimetres (v1 parity).
pub const BED_MIN_MM: f64 = 1.0;

/// Largest configurable bed dimension, in millimetres (v1 parity).
pub const BED_MAX_MM: f64 = 2000.0;

/// Hold one bed dimension inside [`BED_MIN_MM`] ..= [`BED_MAX_MM`].
///
/// `NaN` — which [`f64::clamp`] would propagate rather than reject — returns
/// the default bed dimension instead. Both defaults are 400.0 mm, so one
/// function serves both axes; if they ever diverge this takes an axis.
///
/// This is the *input* guard used by the Bed size… dialog and by the settings
/// seed. It is deliberately **not** used on import: a file declaring a
/// 5000 mm canvas is rejected outright (`SvgImportError::MalformedBedDimension`),
/// because silently clamping it to 2000 mm would place every coordinate
/// wrongly while looking plausible.
///
/// ```
/// use lasercad::util::{clamp_bed_mm, BED_MAX_MM, BED_MIN_MM};
/// assert_eq!(clamp_bed_mm(0.0), BED_MIN_MM);
/// assert_eq!(clamp_bed_mm(5000.0), BED_MAX_MM);
/// assert_eq!(clamp_bed_mm(300.0), 300.0);
/// ```
pub fn clamp_bed_mm(v: f64) -> f64 {
    if v.is_nan() {
        return DEFAULT_BED_WIDTH_MM;
    }
    v.clamp(BED_MIN_MM, BED_MAX_MM)
}

/// Mirror a Y coordinate between world space (Y-up) and SVG space (Y-down):
/// `y_svg = bed_height_mm - y_world`.
///
/// `bed_height_mm` is the **bed height of the document being written or
/// read**, never a constant (LCV-114): mirroring around 400 mm a drawing
/// authored on a 180 mm-tall machine offsets the whole job by 220 mm, and the
/// file still looks plausible in a text editor.
///
/// The map is an **involution** for every fixed height —
/// `flip_y(flip_y(y, h), h) == y` — so the same function serves both
/// directions: world → SVG on export and SVG → world on import. It is defined
/// for every finite input, including coordinates outside the bed (which yield
/// negative or over-bed values, never clamped).
///
/// ```
/// use lasercad::util::{flip_y, DEFAULT_BED_HEIGHT_MM};
/// assert_eq!(flip_y(0.0, DEFAULT_BED_HEIGHT_MM), DEFAULT_BED_HEIGHT_MM);
/// assert_eq!(flip_y(flip_y(37.5, 180.0), 180.0), 37.5);
/// assert_eq!(flip_y(50.0, 200.0), 150.0);
/// ```
pub fn flip_y(y_mm: f64, bed_height_mm: f64) -> f64 {
    bed_height_mm - y_mm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;

    /// LCV-100 AC 1 / LCV-114 AC 1 — every item resolves through
    /// `crate::util`, under the LCV-114 names.
    #[test]
    fn units_module_exports_constants_and_helpers() {
        use crate::util::{
            clamp_bed_mm as reexported_clamp, flip_y as reexported_flip, BED_MAX_MM as MAX,
            BED_MIN_MM as MIN, DEFAULT_BED_HEIGHT_MM as H, DEFAULT_BED_WIDTH_MM as W,
        };
        assert_eq!(W, 400.0);
        assert_eq!(H, 400.0);
        assert_eq!(MIN, 1.0);
        assert_eq!(MAX, 2000.0);
        assert_eq!(reexported_flip(0.0, H), 400.0);
        assert_eq!(reexported_clamp(300.0), 300.0);
    }

    /// LCV-114 AC 1 — the four documented cases of `clamp_bed_mm`.
    #[test]
    fn clamp_bed_mm_clamps_and_rejects_nan() {
        assert_eq!(clamp_bed_mm(0.0), 1.0);
        assert_eq!(clamp_bed_mm(5000.0), 2000.0);
        assert_eq!(clamp_bed_mm(f64::NAN), DEFAULT_BED_WIDTH_MM);
        assert_eq!(clamp_bed_mm(300.0), 300.0);
    }

    /// LCV-114 AC 1 — the bounds themselves are inclusive, and the infinities
    /// land on them rather than propagating.
    #[test]
    fn clamp_bed_mm_is_inclusive_at_both_bounds() {
        assert_eq!(clamp_bed_mm(BED_MIN_MM), BED_MIN_MM);
        assert_eq!(clamp_bed_mm(BED_MAX_MM), BED_MAX_MM);
        assert_eq!(clamp_bed_mm(f64::INFINITY), BED_MAX_MM);
        assert_eq!(clamp_bed_mm(f64::NEG_INFINITY), BED_MIN_MM);
    }

    /// LCV-100 AC 2 — `flip_y` matches the default bed and is its own inverse.
    #[test]
    fn flip_y_is_an_involution() {
        assert_eq!(flip_y(0.0, 400.0), 400.0);
        assert_eq!(flip_y(400.0, 400.0), 0.0);
        assert_eq!(flip_y(10.0, 400.0), 390.0);
        for y in [0.0, 10.0, 250.0, 400.0, -5.0, 512.5] {
            assert!(
                (flip_y(flip_y(y, 400.0), 400.0) - y).abs() < EPSILON,
                "y={y}"
            );
        }
    }

    /// LCV-114 AC 2 — the involution holds at every bed height, not just the
    /// default one, and the mirror axis really is the parameter.
    #[test]
    fn flip_y_is_an_involution_at_any_bed_height() {
        for h in [400.0, 128.0, 1.0, 2000.0] {
            for y in [0.0, 10.0, 0.5, -5.0, 512.5] {
                assert!((flip_y(flip_y(y, h), h) - y).abs() < EPSILON, "h={h} y={y}");
            }
            assert!((flip_y(0.0, h) - h).abs() < EPSILON, "h={h}");
            assert!(flip_y(h, h).abs() < EPSILON, "h={h}");
        }
        // The mirror axis is the argument: the same Y lands elsewhere on a
        // different bed. This is the whole point of LCV-114.
        assert_eq!(flip_y(50.0, 200.0), 150.0);
        assert_eq!(flip_y(50.0, 400.0), 350.0);
    }

    /// Out-of-bed coordinates are mirrored, never clamped (LCV-100 AC 12's
    /// kernel half).
    #[test]
    fn flip_y_does_not_clamp_out_of_bed_values() {
        assert_eq!(flip_y(500.0, 400.0), -100.0);
        assert_eq!(flip_y(-100.0, 400.0), 500.0);
    }
}
