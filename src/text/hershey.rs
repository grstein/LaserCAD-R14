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
mod tests {
    use super::*;

    /// The space character has no strokes.
    #[test]
    fn space_has_no_strokes() {
        assert!(glyph_strokes(' ').is_empty());
    }

    /// The advance for space is 8 Hershey units.
    #[test]
    fn space_advance_is_8() {
        assert_eq!(advance_width(' '), 8);
    }

    /// 'H' cap height: the stored data must have every stroke point's y value
    /// in the range [−CAP_HEIGHT, 0] (no descenders on H).
    #[test]
    fn h_cap_height_calibration() {
        let strokes = glyph_strokes('H');
        // Must have at least one stroke.
        assert!(!strokes.is_empty(), "'H' must have strokes");
        let min_y = strokes
            .iter()
            .flat_map(|s| s.iter())
            .map(|&(_, y)| y)
            .min()
            .unwrap();
        // The topmost point must be exactly at −cap_height.
        assert_eq!(
            min_y,
            -(CAP_HEIGHT_HERSHEY as i8),
            "'H' must reach y = −{CAP_HEIGHT_HERSHEY} (cap top)",
        );
    }

    /// Unsupported characters (non-ASCII or control codes) return no strokes.
    #[test]
    fn unsupported_char_returns_empty_strokes() {
        assert!(glyph_strokes('\x00').is_empty());
        assert!(glyph_strokes('é').is_empty());
    }

    /// Unsupported characters return advance width 8.
    #[test]
    fn unsupported_char_advance_is_8() {
        assert_eq!(advance_width('\x01'), 8);
        assert_eq!(advance_width('ñ'), 8);
    }

    /// Every glyph in the table has a positive advance width.
    #[test]
    fn all_glyphs_have_positive_advance() {
        for code in 32_u8..=126 {
            let ch = code as char;
            assert!(
                advance_width(ch) > 0,
                "advance for '{ch}' (ASCII {code}) must be positive",
            );
        }
    }

    /// 'A' has a non-empty stroke list.
    #[test]
    fn a_has_strokes() {
        assert!(!glyph_strokes('A').is_empty());
    }

    /// Boundary: ASCII 32 and 126 are both in range.
    #[test]
    fn boundary_ascii_32_and_126_are_in_range() {
        let _ = glyph_strokes(' ');
        let _ = glyph_strokes('~');
        let _ = advance_width(' ');
        let _ = advance_width('~');
    }

    /// ASCII 31 (below range) returns no strokes and advance 8.
    #[test]
    fn ascii_31_below_range() {
        assert!(glyph_strokes('\x1f').is_empty());
        assert_eq!(advance_width('\x1f'), 8);
    }
}
