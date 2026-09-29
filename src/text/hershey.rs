//! Hershey Simplex Roman glyph access helpers.
//!
//! Wraps [`super::hershey_data::GLYPHS`] with safe, ergonomic accessors.
//! Unsupported characters (outside ASCII 32–126) return the space glyph's
//! stroke slice (empty) and advance width (8 Hershey units).
//!
//! ## Cap height constant
//! [`CAP_HEIGHT_HERSHEY`] is the height of the capital letter `'H'` in
//! Hershey units.  In the stored data, capital tops sit at `y = −9` and the
//! baseline at `y = 0`, so the cap height is **9 Hershey units**.
//! [`super::layout`] uses this to compute the mm-per-Hershey-unit scale
//! factor.

use crate::text::hershey_data::GLYPHS;

/// Cap height of `'H'` in Hershey coordinate units.
///
/// Capital letters span from `y = −9` (top) to `y = 0` (baseline), giving a
/// cap height of **9 units**.  Dividing `height_mm` by this constant yields
/// the mm-per-Hershey-unit scale factor used in text layout.
pub const CAP_HEIGHT_HERSHEY: f64 = 9.0;

/// Returns the stroke list for `ch`, or an empty slice for unsupported chars.
///
/// The return type is a static reference to a list of strokes; each stroke is
/// itself a static slice of `(x, y)` pairs in Hershey coordinate space
/// (Y-down, baseline at `y = 0`, cap-top at `y = −9`).
///
/// Supports ASCII 32 (`' '`) through 126 (`'~'`).  Anything outside that
/// range returns `&[]` (no strokes) so callers can advance the cursor without
/// panic.
pub fn glyph_strokes(ch: char) -> &'static [&'static [(i8, i8)]] {
    let code = ch as usize;
    if (32..=126).contains(&code) {
        GLYPHS[code - 32].1
    } else {
        &[]
    }
}

/// Returns the advance width for `ch` in Hershey units.
///
/// Falls back to `8` (the space width) for unsupported characters outside
/// ASCII 32–126.
pub fn advance_width(ch: char) -> i8 {
    let code = ch as usize;
    if (32..=126).contains(&code) {
        GLYPHS[code - 32].0
    } else {
        8
    }
}

#[cfg(test)]
mod tests;
