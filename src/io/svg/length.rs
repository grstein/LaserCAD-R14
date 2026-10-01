//! SVG 2 lengths (LCV-173 AC 1, SVG 2 ch. 8): a number with an optional unit,
//! converted to user units at 96 px = 1 in, one user unit = 1 px.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

/// Pixels per millimetre: 96 px = 1 in = 25.4 mm.
pub(super) const PX_PER_MM: f64 = 96.0 / 25.4;

/// A length unit, as written (ASCII case-insensitive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unit {
    /// No unit: user units, i.e. px.
    None,
    /// `px`.
    Px,
    /// `mm`.
    Mm,
    /// `cm`.
    Cm,
    /// `Q`, a quarter millimetre.
    Q,
    /// `in`.
    In,
    /// `pt`, 1/72 in.
    Pt,
    /// `pc`, 12 pt.
    Pc,
    /// `%` of a reference length.
    Percent,
    /// `em`, 16 px (CSS initial `font-size`).
    Em,
    /// `ex`, 8 px.
    Ex,
}

/// A parsed length: its number and unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Length {
    /// The number as written.
    pub(super) value: f64,
    /// The unit as written.
    pub(super) unit: Unit,
}

impl Length {
    /// The length in millimetres when its unit is absolute (none, `px`,
    /// `mm`, `cm`, `Q`, `in`, `pt`, `pc`); `None` for `%`, `em`, `ex`.
    /// `mm` is returned as written, never through a px product.
    pub(super) fn to_mm(self) -> Option<f64> {
        let v = self.value;
        Some(match self.unit {
            Unit::Mm => v,
            Unit::Cm => v * 10.0,
            Unit::Q => v / 4.0,
            Unit::In => v * 25.4,
            Unit::Pt => v * 25.4 / 72.0,
            Unit::Pc => v * 25.4 / 6.0,
            Unit::None | Unit::Px => v / PX_PER_MM,
            Unit::Percent | Unit::Em | Unit::Ex => return None,
        })
    }
}

/// Parse `raw` as a length: optional surrounding whitespace, an SVG number
/// (sign, digits, `.`, exponent), an optional unit. `None` for anything else,
/// a non-finite number or an unknown unit.
pub(super) fn parse_length(raw: &str) -> Option<Length> {
    let t = raw.trim();
    let cut = t
        .rfind(|c: char| c.is_ascii_digit() || c == '.')
        .map_or(0, |i| i + 1);
    let (num, unit) = t.split_at(cut);
    let unit = match unit.trim().to_ascii_lowercase().as_str() {
        "" => Unit::None,
        "px" => Unit::Px,
        "mm" => Unit::Mm,
        "cm" => Unit::Cm,
        "q" => Unit::Q,
        "in" => Unit::In,
        "pt" => Unit::Pt,
        "pc" => Unit::Pc,
        "%" => Unit::Percent,
        "em" => Unit::Em,
        "ex" => Unit::Ex,
        _ => return None,
    };
    let svg_number = num
        .bytes()
        .all(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.' | b'e' | b'E'));
    let value = num.parse::<f64>().ok().filter(|v| v.is_finite())?;
    svg_number.then_some(Length { value, unit })
}

/// `len` in user units (px); `%` is a percentage of `ref_len` user units.
pub(super) fn to_user(len: Length, ref_len: f64) -> f64 {
    match len.unit {
        Unit::Percent => len.value / 100.0 * ref_len,
        Unit::Em => len.value * 16.0,
        Unit::Ex => len.value * 8.0,
        Unit::None | Unit::Px => len.value,
        _ => len.to_mm().map_or(len.value, |mm| mm * PX_PER_MM),
    }
}

/// A CSS `font-size` value in user units given the parent's size `parent`
/// (LCV-179): a length (`em` and `%` of `parent`, `ex` half an `em`), an
/// absolute keyword `xx-small`…`xx-large` (9, 10, 13, 16, 18, 24, 32), or
/// `larger`/`smaller` (× or ÷ 1.2). `None` when invalid or negative.
pub(super) fn font_size(raw: &str, parent: f64) -> Option<f64> {
    let keyword = match raw.trim().to_ascii_lowercase().as_str() {
        "xx-small" => 9.0,
        "x-small" => 10.0,
        "small" => 13.0,
        "medium" => 16.0,
        "large" => 18.0,
        "x-large" => 24.0,
        "xx-large" => 32.0,
        "larger" => parent * 1.2,
        "smaller" => parent / 1.2,
        _ => {
            let len = parse_length(raw)?;
            let size = match len.unit {
                Unit::Em => len.value * parent,
                Unit::Ex => len.value * parent / 2.0,
                _ => to_user(len, parent),
            };
            return (size >= 0.0).then_some(size);
        }
    };
    Some(keyword)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(raw: &str) -> f64 {
        to_user(parse_length(raw).unwrap(), 200.0)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * b.abs().max(1.0)
    }

    /// AC 1 — every absolute unit at 96 px = 1 in; unitless is px.
    #[test]
    fn absolute_units_convert_at_96_px_per_inch() {
        for (raw, px) in [
            ("10", 10.0),
            ("10px", 10.0),
            ("1in", 96.0),
            ("25.4mm", 96.0),
            ("2.54cm", 96.0),
            ("101.6Q", 96.0),
            ("72pt", 96.0),
            ("6pc", 96.0),
        ] {
            assert!(close(user(raw), px), "{raw}: {}", user(raw));
        }
    }

    /// AC 1 — `to_mm` keeps `mm` exact and refuses relative units.
    #[test]
    fn to_mm_is_exact_for_mm_and_none_for_relative_units() {
        let mm = |raw: &str| parse_length(raw).unwrap().to_mm();
        assert_eq!(mm("210mm"), Some(210.0));
        assert_eq!(mm("300.5mm").map(f64::to_bits), Some(300.5_f64.to_bits()));
        assert!(close(mm("96").unwrap(), 25.4));
        assert!(close(mm("72pt").unwrap(), 25.4));
        assert!(close(mm("1in").unwrap(), 25.4));
        assert!(close(mm("4Q").unwrap(), 1.0));
        for raw in ["50%", "2em", "3ex"] {
            assert_eq!(mm(raw), None, "{raw}");
        }
    }

    /// AC 1 — `%` of the reference, `em` 16 px, `ex` 8 px.
    #[test]
    fn relative_units() {
        assert!(close(user("50%"), 100.0));
        assert!(close(user("2em"), 32.0));
        assert!(close(user("3ex"), 24.0));
    }

    /// AC 1 — units are ASCII case-insensitive; whitespace around is allowed.
    #[test]
    fn units_ignore_case_and_surrounding_whitespace() {
        assert!(close(user(" 1IN "), 96.0));
        assert!(close(user("25.4MM"), 96.0));
        assert!(close(user("10 Px"), 10.0));
        assert!(close(user("1e1pX"), 10.0));
        assert!(close(user("-.5E1"), -5.0));
    }

    /// LCV-179 — font sizes: lengths, `em`/`ex`/`%` of the parent,
    /// keywords; negative and junk refused.
    #[test]
    fn font_sizes_resolve_against_the_parent() {
        let size = |raw| font_size(raw, 20.0);
        for (raw, want) in [
            ("12", 12.0),
            ("12px", 12.0),
            ("1in", 96.0),
            ("2em", 40.0),
            ("1ex", 10.0),
            ("50%", 10.0),
            ("xx-small", 9.0),
            ("x-small", 10.0),
            ("small", 13.0),
            ("medium", 16.0),
            ("large", 18.0),
            ("X-Large", 24.0),
            ("xx-large", 32.0),
            ("larger", 24.0),
            ("smaller", 20.0 / 1.2),
        ] {
            assert!(close(size(raw).unwrap(), want), "{raw}");
        }
        for raw in ["-1", "big", ""] {
            assert_eq!(size(raw), None, "{raw}");
        }
    }

    /// AC 1 — junk, unknown units and non-finite numbers are refused.
    #[test]
    fn junk_is_refused() {
        for raw in [
            "", " ", "mm", "abc", "10km", "10 m", "1e", "inf", "NaN", "infinity", "1..2", "--1",
            "1e999", "0x10", "5%%",
        ] {
            assert_eq!(parse_length(raw), None, "{raw:?}");
        }
    }
}
