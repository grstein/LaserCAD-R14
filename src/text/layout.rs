//! Hershey text layout engine: converts a UTF-8 string into a flat list of
//! [`Entity::Line`] segments in millimeter space.
//!
//! ## Usage
//!
//! ```rust
//! use lasercad::text::layout_text;
//! use lasercad::geometry::Vec2;
//!
//! let lines = layout_text("Hi", Vec2::new(0.0, 0.0), 10.0, 1.0);
//! assert!(!lines.is_empty());
//! ```
//!
//! ## Coordinate mapping
//!
//! 1. **Scale**: `scale = height_mm / CAP_HEIGHT_HERSHEY`.
//! 2. **X**: `px = cursor_x + hx * scale`, advancing cursor by
//!    `advance * scale * spacing_factor` per glyph.
//! 3. **Y**: Hershey Y is negated before scaling so that the stored Y-down
//!    convention becomes Y-up: `py = origin.y + (−hy) * scale`.
//! 4. **Origin**: `(origin.x, origin.y)` is the baseline anchor (bottom-left
//!    of the first capital letter).
//! 5. **Degenerate lines** (`p1 == p2` in mm-space after rounding) are
//!    silently dropped.

use crate::document::entity::Entity;
use crate::geometry::{Line, Vec2};
use crate::text::hershey::{CAP_HEIGHT_HERSHEY, advance_width, glyph_strokes};

/// Lay out `text` as a sequence of [`Entity::Line`] strokes in mm-space.
///
/// # Parameters
/// - `text`: the string to render (only ASCII 32–126 are drawn; other
///   codepoints advance the cursor without emitting strokes and without
///   panicking).
/// - `origin`: baseline anchor in millimeters, i.e. the bottom-left point
///   where the first character sits.
/// - `height_mm`: desired cap height (height of `'H'`) in millimeters.
///   Must be positive; non-positive values produce an empty `Vec`.
/// - `spacing_factor`: multiplied by each glyph's advance width before
///   advancing the cursor.  `1.0` is normal spacing; values below `1.0`
///   tighten, above `1.0` loosen.
///
/// # Returns
/// A `Vec<Entity>` where every element is an `Entity::Line`.  The vector is
/// empty when `text` is empty, `height_mm ≤ 0`, or after all degenerate
/// segments are filtered.
pub fn layout_text(text: &str, origin: Vec2, height_mm: f64, spacing_factor: f64) -> Vec<Entity> {
    if height_mm <= 0.0 {
        return Vec::new();
    }

    let scale = height_mm / CAP_HEIGHT_HERSHEY;
    let mut entities = Vec::new();
    let mut cursor_x = 0.0_f64;

    for ch in text.chars() {
        let strokes = glyph_strokes(ch);

        for stroke in strokes {
            // A stroke needs at least 2 points to form any line segment.
            if stroke.len() < 2 {
                continue;
            }
            for window in stroke.windows(2) {
                let (hx1, hy1) = (window[0].0 as f64, window[0].1 as f64);
                let (hx2, hy2) = (window[1].0 as f64, window[1].1 as f64);

                let p1 = Vec2::new(origin.x + cursor_x + hx1 * scale, origin.y + (-hy1) * scale);
                let p2 = Vec2::new(origin.x + cursor_x + hx2 * scale, origin.y + (-hy2) * scale);

                // Skip degenerate lines (p1 == p2 in mm-space).
                if p1 == p2 {
                    continue;
                }

                entities.push(Entity::Line(Line::new(p1, p2)));
            }
        }

        // Advance cursor regardless of whether the character produced strokes.
        cursor_x += advance_width(ch) as f64 * scale * spacing_factor;
    }

    entities
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;

    /// Non-positive height produces an empty result.
    #[test]
    fn zero_and_negative_height_return_empty() {
        assert!(layout_text("A", Vec2::default(), 0.0, 1.0).is_empty());
        assert!(layout_text("A", Vec2::default(), -5.0, 1.0).is_empty());
    }

    /// Empty string produces no entities.
    #[test]
    fn empty_string_returns_empty() {
        assert!(layout_text("", Vec2::default(), 10.0, 1.0).is_empty());
    }

    /// A space character produces no strokes.
    #[test]
    fn space_produces_no_lines() {
        assert!(layout_text(" ", Vec2::default(), 10.0, 1.0).is_empty());
    }

    /// 'H' produces lines and the topmost point is exactly `height_mm`
    /// above the origin (Y-up after negation).
    #[test]
    fn h_cap_height_matches_height_mm() {
        let height_mm = 10.0;
        let origin = Vec2::new(0.0, 0.0);
        let lines = layout_text("H", origin, height_mm, 1.0);

        assert!(!lines.is_empty(), "'H' must emit lines");

        let max_y = lines
            .iter()
            .flat_map(|e| {
                if let Entity::Line(l) = e {
                    [l.p1.y, l.p2.y]
                } else {
                    [0.0, 0.0]
                }
            })
            .fold(f64::NEG_INFINITY, f64::max);

        assert!(
            (max_y - height_mm).abs() < EPSILON,
            "top of 'H' should be at height_mm={height_mm}, got {max_y}",
        );
    }

    /// The baseline sits at `origin.y`.
    #[test]
    fn baseline_at_origin_y() {
        let origin = Vec2::new(5.0, 3.0);
        let lines = layout_text("H", origin, 10.0, 1.0);

        let min_y = lines
            .iter()
            .flat_map(|e| {
                if let Entity::Line(l) = e {
                    [l.p1.y, l.p2.y]
                } else {
                    [0.0, 0.0]
                }
            })
            .fold(f64::INFINITY, f64::min);

        assert!(
            (min_y - origin.y).abs() < EPSILON,
            "bottom of 'H' should sit on baseline origin.y={o}, got min_y={min_y}",
            o = origin.y,
        );
    }

    /// Unsupported characters do not panic and still advance the cursor.
    #[test]
    fn unsupported_char_does_not_panic_advances_cursor() {
        // 'é' is outside ASCII 32–126; the result should be the same as spaces
        // with the same cursor advance as the fallback (8 units).
        let with_unsupported = layout_text("A\u{00e9}B", Vec2::default(), 10.0, 1.0);
        let lines_a = layout_text("A", Vec2::default(), 10.0, 1.0);
        // The unsupported character must not panic, and 'B' must still be laid out.
        assert!(!with_unsupported.is_empty());
        // The output should have more lines than 'A' alone (because 'B' also contributes).
        assert!(with_unsupported.len() > lines_a.len());
    }

    /// Y-axis negation: all Y values for 'H' should be ≥ origin.y.
    #[test]
    fn y_negation_places_strokes_above_baseline() {
        let origin = Vec2::new(0.0, 0.0);
        let lines = layout_text("H", origin, 10.0, 1.0);
        for e in &lines {
            if let Entity::Line(l) = e {
                let p1y = l.p1.y;
                let p2y = l.p2.y;
                let oy = origin.y;
                assert!(p1y >= oy - EPSILON, "p1.y={p1y} should be >= baseline {oy}",);
                assert!(p2y >= oy - EPSILON, "p2.y={p2y} should be >= baseline {oy}",);
            }
        }
    }

    /// Two identical characters placed side by side should produce more lines
    /// than one, and the second character's lines should start to the right of
    /// the first one's advance.
    #[test]
    fn two_chars_cursor_advances() {
        let lines_one = layout_text("I", Vec2::default(), 10.0, 1.0);
        let lines_two = layout_text("II", Vec2::default(), 10.0, 1.0);
        assert!(
            lines_two.len() > lines_one.len(),
            "two 'I's should produce more lines than one"
        );
    }

    /// Spacing factor of 2 doubles the cursor advance, meaning the second
    /// character's strokes start further right.
    #[test]
    fn spacing_factor_scales_advance() {
        let origin = Vec2::new(0.0, 0.0);
        let factor_1 = layout_text("AB", origin, 10.0, 1.0);
        let factor_2 = layout_text("AB", origin, 10.0, 2.0);

        // Right-most x should be further right with wider spacing.
        let max_x = |entities: &[Entity]| {
            entities
                .iter()
                .flat_map(|e| {
                    if let Entity::Line(l) = e {
                        [l.p1.x, l.p2.x]
                    } else {
                        [0.0, 0.0]
                    }
                })
                .fold(f64::NEG_INFINITY, f64::max)
        };

        assert!(
            max_x(&factor_2) > max_x(&factor_1),
            "spacing_factor=2 should push 'B' further right"
        );
    }

    /// All emitted entities are `Entity::Line` variants.
    #[test]
    fn all_entities_are_lines() {
        for e in layout_text("Hello, World!", Vec2::default(), 10.0, 1.0) {
            assert!(
                matches!(e, Entity::Line(_)),
                "expected Entity::Line, got {e:?}",
            );
        }
    }

    /// No degenerate (p1==p2) lines are emitted.
    #[test]
    fn no_degenerate_lines_emitted() {
        // Render a range of characters most likely to contain dots.
        let lines = layout_text("!.,:;ij", Vec2::default(), 10.0, 1.0);
        for e in &lines {
            if let Entity::Line(l) = e {
                let p1 = l.p1;
                let p2 = l.p2;
                assert!(p1 != p2, "degenerate line emitted: p1={p1:?} p2={p2:?}",);
            }
        }
    }

    // ---------------------------------------------------------------------
    // LCV-117 — millimetre-space regression for the repaired glyph table.
    // ---------------------------------------------------------------------

    /// The five glyphs allowed below the baseline.
    const DESCENDERS: [char; 5] = ['g', 'j', 'p', 'q', 'y'];

    /// Descender depth as a fraction of the requested cap height: 4 of the 9
    /// Hershey units that make up the cap height.
    const DESCENDER_FRACTION: f64 = 4.0 / 9.0;

    /// Every endpoint y of `text` laid out alone at `height_mm` from the
    /// world origin.
    fn endpoint_ys(text: &str, height_mm: f64) -> Vec<f64> {
        layout_text(text, Vec2::new(0.0, 0.0), height_mm, 1.0)
            .iter()
            .filter_map(|e| match e {
                Entity::Line(l) => Some([l.p1.y, l.p2.y]),
                _ => None,
            })
            .flatten()
            .collect()
    }

    fn min_max(values: &[f64]) -> (f64, f64) {
        (
            values.iter().copied().fold(f64::INFINITY, f64::min),
            values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Every printable alphanumeric, in ASCII order.
    fn alphanumerics() -> impl Iterator<Item = char> {
        (32_u8..=126)
            .map(char::from)
            .filter(char::is_ascii_alphanumeric)
    }

    /// AC 18: no alphanumeric other than `g j p q y` — and not `?` either —
    /// puts any geometry below the baseline or above the cap height.
    #[test]
    fn every_non_descender_sits_on_or_above_the_baseline() {
        let height_mm = 10.0;
        for ch in alphanumerics().chain(['?']) {
            if DESCENDERS.contains(&ch) {
                continue;
            }
            let ys = endpoint_ys(&ch.to_string(), height_mm);
            assert!(!ys.is_empty(), "'{ch}' must emit lines");
            let (min_y, max_y) = min_max(&ys);
            assert!(
                min_y >= -EPSILON,
                "'{ch}' dips {d} mm below the baseline",
                d = -min_y,
            );
            assert!(
                max_y <= height_mm + EPSILON,
                "'{ch}' rises to {max_y} mm, above the cap height {height_mm} mm",
            );
        }
    }

    /// AC 19: `g j p q y` all bottom out at `height_mm * 4 / 9` below the
    /// baseline — the same depth for all five, scaling with the requested
    /// height and with nothing else.
    #[test]
    fn descenders_reach_exactly_four_ninths_below_the_baseline() {
        for height_mm in [10.0, 3.5] {
            let expected = -height_mm * DESCENDER_FRACTION;
            for ch in DESCENDERS {
                let ys = endpoint_ys(&ch.to_string(), height_mm);
                assert!(!ys.is_empty(), "'{ch}' must emit lines");
                let (min_y, max_y) = min_max(&ys);
                assert!(
                    (min_y - expected).abs() < EPSILON,
                    "'{ch}' at {height_mm} mm descends to {min_y} mm, expected {expected} mm",
                );
                assert!(
                    max_y <= height_mm + EPSILON,
                    "'{ch}' at {height_mm} mm rises to {max_y} mm, above the cap height",
                );
            }
        }
    }

    /// AC 20: every capital and digit is exactly cap-height tall and flush on
    /// the baseline — the "no letter is taller, shorter or lower than its
    /// neighbours" guarantee, in millimetres.
    #[test]
    fn caps_and_digits_are_exactly_cap_height_and_flush() {
        let height_mm = 10.0;
        for ch in alphanumerics().filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
            let ys = endpoint_ys(&ch.to_string(), height_mm);
            assert!(!ys.is_empty(), "'{ch}' must emit lines");
            let (min_y, max_y) = min_max(&ys);
            assert!(
                (max_y - height_mm).abs() < EPSILON,
                "top of '{ch}' is {max_y} mm, expected {height_mm} mm",
            );
            assert!(
                min_y.abs() < EPSILON,
                "bottom of '{ch}' is {min_y} mm, expected 0 mm",
            );
        }
    }

    /// AC 21: the reported symptom — an `O` next to an `H` used to be twice
    /// as tall and hang half its body under the line.
    #[test]
    fn h_and_o_share_both_extremes() {
        let height_mm = 10.0;
        let ys = endpoint_ys("HO", height_mm);
        let (min_y, max_y) = min_max(&ys);
        assert!(
            (max_y - height_mm).abs() < EPSILON,
            "top of \"HO\" is {max_y} mm, expected {height_mm} mm",
        );
        assert!(
            min_y.abs() < EPSILON,
            "bottom of \"HO\" is {min_y} mm, expected 0 mm (the 'O' used to sag)",
        );
    }
}
