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

    // ---------------------------------------------------------------------
    // LCV-117 — the whole-table vertical-metrics contract.
    //
    // These tests are keyed on character class, never on per-glyph golden
    // coordinates, so they bite on any future glyph authored against the
    // wrong vertical origin instead of freezing one repair's exact choices.
    // ---------------------------------------------------------------------

    /// The 17 punctuation marks that legitimately extend below the baseline.
    /// They are exempt from the "ends flush on the baseline" rule and nothing
    /// else is; adding a character here to silence a failure trips
    /// [`exemption_list_is_exactly_the_documented_17`].
    const BASELINE_CROSSING_PUNCTUATION: [char; 17] = [
        '#', '$', '(', ')', ',', '/', ';', '<', '>', '@', '[', '\\', ']', '_', '{', '|', '}',
    ];

    /// Lowercase letters whose body rises to the cap line.
    const ASCENDERS: [char; 7] = ['b', 'd', 'f', 'h', 'k', 'l', 't'];

    /// The only glyphs allowed below the baseline, all bottoming at `y = +4`.
    const DESCENDERS: [char; 5] = ['g', 'j', 'p', 'q', 'y'];

    /// Depth of every descender, in Hershey units below the baseline.
    const DESCENDER_DEPTH: i8 = 4;

    /// `(min y, max y)` over every stored point of `ch`; `None` when blank.
    fn y_extent(ch: char) -> Option<(i8, i8)> {
        extent(ch, |&(_, y)| y)
    }

    /// `(min x, max x)` over every stored point of `ch`; `None` when blank.
    fn x_extent(ch: char) -> Option<(i8, i8)> {
        extent(ch, |&(x, _)| x)
    }

    fn extent(ch: char, axis: fn(&(i8, i8)) -> i8) -> Option<(i8, i8)> {
        glyph_strokes(ch)
            .iter()
            .flat_map(|stroke| stroke.iter())
            .map(axis)
            .fold(None, |acc, v| match acc {
                None => Some((v, v)),
                Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
            })
    }

    /// AC 1–9: every glyph in the table obeys the documented vertical
    /// convention — baseline at `y = 0`, cap line at `y = −9`, descenders at
    /// exactly `y = +4`, and one explicit punctuation exemption list.
    #[test]
    fn vertical_metrics_contract() {
        for code in 32_u8..=126 {
            let ch = code as char;
            let Some((min_y, max_y)) = y_extent(ch) else {
                continue; // blank glyph (space) — nothing to measure
            };

            // AC 2 — universal bound.
            assert!(
                (-9..=9).contains(&min_y) && (-9..=9).contains(&max_y),
                "'{ch}' [{min_y}..{max_y}] escapes the table's vertical range [-9..9]",
            );

            // AC 3/4/5 — the top of the glyph, by character class.
            if ch.is_ascii_uppercase() || ch.is_ascii_digit() || ASCENDERS.contains(&ch) {
                assert_eq!(
                    min_y, -9,
                    "'{ch}' [{min_y}..{max_y}] is a cap/digit/ascender and must top at y = -9",
                );
            } else if ch == 'i' || ch == 'j' {
                assert_eq!(
                    min_y, -8,
                    "'{ch}' [{min_y}..{max_y}] must top at its dot, y = -8",
                );
            } else if ch.is_ascii_lowercase() {
                assert!(
                    (-6..=-5).contains(&min_y),
                    "'{ch}' [{min_y}..{max_y}] is a lowercase body and must top at y = -6 or -5",
                );
            }

            // AC 6/7/8 — the bottom of the glyph.
            if ch.is_ascii_alphanumeric() || ch == '?' {
                if DESCENDERS.contains(&ch) {
                    assert_eq!(
                        max_y, DESCENDER_DEPTH,
                        "'{ch}' [{min_y}..{max_y}] is a descender and must bottom at exactly \
                         y = +{DESCENDER_DEPTH}",
                    );
                } else {
                    assert_eq!(
                        max_y, 0,
                        "'{ch}' [{min_y}..{max_y}] must end flush on the baseline, y = 0",
                    );
                }
            } else if !BASELINE_CROSSING_PUNCTUATION.contains(&ch) {
                // AC 9 — punctuation outside the exemption list never dips
                // below the line.  It may stop short of it (`-`, `=`, `~`,
                // `^`, `"`, `'`, backtick all float above the baseline).
                assert!(
                    max_y <= 0,
                    "'{ch}' [{min_y}..{max_y}] crosses the baseline but is not exempt",
                );
            }
        }
    }

    /// AC 2: nothing in the table escapes `[-9..9]`, stated on its own so a
    /// failure reads "out of range" rather than "wrong class".
    #[test]
    fn no_glyph_exceeds_the_vertical_bounds() {
        for code in 32_u8..=126 {
            let ch = code as char;
            let Some((min_y, max_y)) = y_extent(ch) else {
                continue;
            };
            assert!(
                min_y >= -9,
                "'{ch}' reaches y = {min_y}, above the cap line y = -9",
            );
            assert!(
                max_y <= 9,
                "'{ch}' reaches y = {max_y}, below the table's floor y = 9",
            );
        }
    }

    /// AC 9: the exemption list is exactly the documented 17 marks, sorted,
    /// unique, and free of anything LCV-117 repaired.
    #[test]
    fn exemption_list_is_exactly_the_documented_17() {
        assert_eq!(
            BASELINE_CROSSING_PUNCTUATION.len(),
            17,
            "the exemption list is a closed set of 17 marks",
        );
        for pair in BASELINE_CROSSING_PUNCTUATION.windows(2) {
            assert!(
                pair[0] < pair[1],
                "exemption list must stay sorted and duplicate-free, got {:?} then {:?}",
                pair[0],
                pair[1],
            );
        }
        for ch in BASELINE_CROSSING_PUNCTUATION {
            assert!(
                ch.is_ascii_graphic() && !ch.is_ascii_alphanumeric() && ch != '?',
                "'{ch}' is not baseline-crossing punctuation and may not be exempted",
            );
        }
    }

    /// The 23 glyphs repaired by LCV-117 and the envelope each must keep:
    /// `(char, top, min x, max x, stroke count, advance)`.
    const REPAIRED_ENVELOPES: [(char, i8, i8, i8, usize, i8); 23] = [
        ('0', -9, -5, 5, 1, 11),
        ('5', -9, -5, 5, 1, 11),
        ('6', -9, -5, 5, 1, 11),
        ('8', -9, -5, 5, 2, 11),
        ('9', -9, -5, 5, 2, 11),
        ('?', -9, -4, 5, 2, 9),
        ('O', -9, -5, 5, 1, 12),
        ('Q', -9, -5, 5, 2, 12),
        ('S', -9, -5, 5, 1, 10),
        ('a', -6, -4, 5, 2, 11),
        ('b', -9, -5, 5, 2, 11),
        ('c', -6, -5, 5, 1, 10),
        ('d', -9, -5, 5, 2, 11),
        ('e', -6, -5, 5, 1, 11),
        ('g', -6, -5, 5, 2, 11),
        ('j', -8, -1, 2, 2, 6),
        ('o', -6, -5, 5, 1, 11),
        ('p', -5, -5, 5, 2, 11),
        ('q', -5, -5, 5, 2, 11),
        ('s', -6, -5, 5, 1, 9),
        ('t', -9, -3, 3, 2, 8),
        ('u', -5, -5, 5, 2, 11),
        ('y', -5, -5, 5, 2, 11),
    ];

    /// AC 11/12/13: the repair moved y coordinates only.  Every repaired
    /// glyph keeps the top, the x envelope, the stroke count and the advance
    /// it had before — a restyle would trip this.
    #[test]
    fn repaired_glyph_envelopes_are_frozen() {
        for (ch, top, min_x, max_x, strokes, advance) in REPAIRED_ENVELOPES {
            let measured_y = y_extent(ch).expect("repaired glyph must have strokes");
            let measured_x = x_extent(ch).expect("repaired glyph must have strokes");
            assert_eq!(measured_y.0, top, "'{ch}' top moved");
            assert_eq!(measured_x.0, min_x, "'{ch}' left edge moved");
            assert_eq!(measured_x.1, max_x, "'{ch}' right edge moved");
            assert_eq!(
                glyph_strokes(ch).len(),
                strokes,
                "'{ch}' gained or lost a pen-down stroke",
            );
            assert_eq!(advance_width(ch), advance, "'{ch}' advance width moved");
        }
    }

    /// Every advance width in the table, in ASCII order from 32 to 126.
    const ADVANCE_WIDTHS: [i8; 95] = [
        8, 8, 8, 12, 9, 13, 11, 6, 8, 8, 9, 13, 7, 13, 7, 9, 11, 9, 11, 11, 11, 11, 11, 11, 11, 11,
        7, 7, 13, 13, 13, 9, 14, 11, 11, 11, 11, 11, 11, 11, 13, 8, 9, 11, 11, 13, 13, 12, 11, 12,
        11, 10, 11, 12, 11, 15, 11, 11, 11, 8, 9, 8, 12, 11, 8, 11, 11, 10, 11, 11, 8, 11, 11, 6,
        6, 11, 6, 15, 11, 11, 11, 11, 8, 9, 8, 11, 11, 15, 11, 11, 11, 8, 8, 8, 13,
    ];

    /// AC 12: horizontal layout is untouched — no glyph's advance changed.
    #[test]
    fn every_advance_width_is_unchanged() {
        for (index, expected) in ADVANCE_WIDTHS.iter().enumerate() {
            let ch = (32 + index as u8) as char;
            assert_eq!(
                advance_width(ch),
                *expected,
                "advance width of '{ch}' (ASCII {code}) changed",
                code = 32 + index,
            );
        }
    }

    /// AC 14: `Q` keeps a real tail that leaves the bowl at the lower right
    /// and stops on the baseline — it did not quietly become an `O`.
    ///
    /// Every claim about the tail is scoped to the tail stroke. `Q`'s bowl is
    /// byte-identical to `O`'s, and on its own it already offers a lower-right
    /// point and a point on the baseline, so an "any stroke" predicate is
    /// satisfied by the bowl alone and sleeps through a truncated tail.
    #[test]
    fn q_keeps_a_baseline_tail_and_differs_from_o() {
        let q = glyph_strokes('Q');
        let o = glyph_strokes('O');
        assert_eq!(o.len(), 1, "'O' must stay a single closed outline");
        assert_eq!(
            q.len(),
            2,
            "'Q' must be a bowl plus exactly one tail stroke"
        );
        let (bowl, tail) = (q[0], q[1]);
        assert_eq!(
            bowl, o[0],
            "'Q' bowl must stay identical to 'O' — the tail is the only difference",
        );
        assert!(
            tail.len() >= 2,
            "'Q' tail {tail:?} is not a drawable stroke"
        );
        assert!(
            tail.iter().any(|&(x, y)| x >= 4 && y == 0),
            "'Q' tail {tail:?} must reach the baseline at the lower right \
             (a point with x >= 4 and y == 0); a tail that stops short leaves \
             'Q' looking like an 'O' with a nick in it",
        );
        assert!(
            tail.iter().any(|&(_, y)| y <= -2),
            "'Q' tail {tail:?} must leave the bowl from inside it, not sit \
             flat on the baseline",
        );
        for &(x, y) in tail {
            assert!(y <= 0, "'Q' tail point ({x},{y}) dips below the baseline");
        }
    }

    /// AC 15: the repair introduced no stroke that `layout_text` would drop.
    #[test]
    fn no_repaired_stroke_is_degenerate() {
        for (ch, ..) in REPAIRED_ENVELOPES {
            for (index, stroke) in glyph_strokes(ch).iter().enumerate() {
                assert!(
                    stroke.len() >= 2,
                    "'{ch}' stroke {index} has fewer than 2 points",
                );
                assert!(
                    stroke.iter().any(|point| *point != stroke[0]),
                    "'{ch}' stroke {index} is a single repeated point and would be dropped",
                );
            }
        }
    }

    /// Visual smoke instrument for font work: renders glyph data as ASCII art
    /// with the cap line (`.`), baseline (`-`) and descender line (`:`)
    /// marked, so a human can see what the metrics contract cannot.
    ///
    /// Ignored by default; run it with
    /// `cargo test -p lasercad --lib hershey -- --ignored --nocapture`.
    #[test]
    #[ignore = "visual instrument: prints ASCII art, asserts nothing"]
    fn glyph_ascii_art() {
        const SX: i32 = 3;
        const SY: i32 = 2;
        for line in [
            "HOQ 0689",
            "ABCDEFGHIJKLM",
            "NOPQRSTUVWXYZ",
            "abcdefghijklm",
            "nopqrstuvwxyz",
            "Quiz? Gypsy jabs.",
        ] {
            println!("=== {line}");
            let mut cells: Vec<(i32, i32)> = Vec::new();
            let mut cursor = 0_i32;
            for ch in line.chars() {
                for stroke in glyph_strokes(ch) {
                    for pair in stroke.windows(2) {
                        plot(
                            ((cursor + pair[0].0 as i32) * SX, pair[0].1 as i32 * SY),
                            ((cursor + pair[1].0 as i32) * SX, pair[1].1 as i32 * SY),
                            &mut cells,
                        );
                    }
                }
                cursor += advance_width(ch) as i32;
            }
            let x_max = cells.iter().map(|c| c.0).max().unwrap_or(0);
            for y in (-9 * SY)..=(DESCENDER_DEPTH as i32 * SY) {
                let row: String = (-SX..=x_max)
                    .map(|x| {
                        if cells.contains(&(x, y)) {
                            '#'
                        } else if y == 0 {
                            '-'
                        } else if y == -9 * SY {
                            '.'
                        } else if y == DESCENDER_DEPTH as i32 * SY {
                            ':'
                        } else {
                            ' '
                        }
                    })
                    .collect();
                println!("{row}");
            }
        }
    }

    /// Bresenham rasteriser for [`glyph_ascii_art`].
    fn plot((mut x0, mut y0): (i32, i32), (x1, y1): (i32, i32), out: &mut Vec<(i32, i32)>) {
        let (dx, dy) = ((x1 - x0).abs(), (y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx - dy;
        loop {
            out.push((x0, y0));
            if x0 == x1 && y0 == y1 {
                return;
            }
            let err2 = 2 * err;
            if err2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if err2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}
