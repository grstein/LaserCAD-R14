//! The grammar: `parse`, `parse_number`, and the tool/toggle/zoom alias
//! tables. See the LCV-110 demand body for the full grammar table (every row
//! of it is asserted by a test in this file) and ADR 0003 §A2 / §A2a for the
//! alias set — letters closed, words open, `tool_alias` is the single copy.
//!
//! **Every number in this crate's command grammar goes through
//! [`parse_number`].** A second bare `str::parse::<f64>` anywhere in
//! `src/cmdline/` is a review blocker (LCV-110 demand, AC 5).

use crate::cmdline::{CommandInput, ToggleKind, ToolKind, ZoomKind};
use crate::geometry::Vec2;

/// Parse a single number in the product's one number grammar: millimetres,
/// `.` as the only decimal separator, optional leading `+`/`-`, ASCII
/// whitespace trimmed, non-finite results rejected.
///
/// Returns `None` for anything that does not parse as an `f64`, and for any
/// value that is not [`f64::is_finite`] — so `"nan"`, `"inf"`, `"-inf"`,
/// `"infinity"` and an overflowing literal like `"1e400"` (which parses to
/// `f64::INFINITY` rather than erroring) are all rejected.
///
/// This is the **only** place in `src/cmdline/` that calls
/// `str::parse::<f64>`; every coordinate, offset and distance in the grammar
/// goes through this function (LCV-110 demand, AC 5).
pub fn parse_number(raw: &str) -> Option<f64> {
    let value: f64 = raw.trim().parse().ok()?;
    if value.is_finite() { Some(value) } else { None }
}

/// Parse one line of command-line input. Total: never panics, never
/// returns an error. See the LCV-110 demand body's grammar table — every
/// row there is a test in this module.
///
/// Case-insensitive and whitespace-tolerant (including around the comma and
/// between `zoom` and its argument). The `Unknown` payload is always the
/// **trimmed original text**, in its **original case** — never the
/// lowercased matching key used internally for keyword comparison.
pub fn parse(raw: &str) -> CommandInput {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return CommandInput::Empty;
    }
    let lower = trimmed.to_lowercase();

    if let Some(kind) = tool_alias(&lower) {
        return CommandInput::Tool(kind);
    }
    if let Some(kind) = toggle_alias(&lower) {
        return CommandInput::Toggle(kind);
    }
    if let Some(kind) = zoom_form(&lower) {
        return CommandInput::Zoom(kind);
    }
    // R14's word and alias (ADR 0012 §7); not a letter-axis entry.
    if lower == "layer" || lower == "la" {
        return CommandInput::Layers;
    }
    // LCV-190: a command word, not a tool and not a letter (ADR 0003 §A2a).
    if lower == "check" {
        return CommandInput::Check;
    }
    // Polar entry (LCV-159, ADR 0003 amendment (5)): `@d<a` is an offset from
    // the anchor, `d<a` a point from the origin. No new variant.
    if trimmed.contains('<') {
        let (relative, body) = match trimmed.strip_prefix('@') {
            Some(rest) => (true, rest),
            None => (false, trimmed),
        };
        return match parse_polar(body) {
            Some(v) if relative => CommandInput::Relative(v),
            Some(v) => CommandInput::Point(v),
            None => CommandInput::Unknown(trimmed.to_owned()),
        };
    }
    if let Some(rest) = trimmed.strip_prefix('@') {
        return match parse_pair(rest.trim()) {
            Some(v) => CommandInput::Relative(v),
            None => CommandInput::Unknown(trimmed.to_owned()),
        };
    }
    if trimmed.contains(',') {
        return match parse_pair(trimmed) {
            Some(v) => CommandInput::Point(v),
            None => CommandInput::Unknown(trimmed.to_owned()),
        };
    }
    match parse_number(trimmed) {
        Some(value) => CommandInput::Distance(value),
        None => CommandInput::Unknown(trimmed.to_owned()),
    }
}

/// The command-line tool-alias table (ADR 0003 §A2a): two axes onto
/// [`ToolKind`], one closed and one open. `lower` is already lowercased and
/// trimmed.
///
/// **The letter axis is closed.** `l p r c a s t e m` is the whole set and it
/// does not grow — a letter is a scarce, memorised, one-keystroke resource
/// shared with the bare tool-activation keys
/// (`src/ui/shortcuts.rs::TOOL_KEYS`). `"e"` maps to [`ToolKind::Delete`],
/// **not** `Extend` — product decision 2, pinned by
/// `src/ui/shortcuts.rs::alias_table_agrees_with_tool_keys`. `Extend` has no
/// letter; it is reachable only by word.
///
/// **The word axis is open** (LCV-131). A word is a row exactly when it spells
/// a command this product ships and maps to an existing `ToolKind` variant.
/// The approved set, the primary AutoCAD R14 spelling of every aliased tool:
///
/// | word | [`ToolKind`] | word | [`ToolKind`] |
/// |---|---|---|---|
/// | `line` | `Line` | `select` | `Select` |
/// | `polyline` | `Polyline` | `trim` | `Trim` |
/// | `pline` | `Polyline` | `extend` | `Extend` |
/// | `rect` | `Rect` | `move` | `Move` |
/// | `rectangle` | `Rect` | `text` | `Text` |
/// | `circle` | `Circle` | `delete` | `Delete` |
/// | `arc` | `Arc` | `del` | `Delete` |
/// | `copy` | `Copy` | `erase` | `Delete` |
/// | `co` | `Copy` | `cp` | `Copy` |
/// | `dist` | `Dist` | `di` | `Dist` |
/// | `rotate` | `Rotate` | `ro` | `Rotate` |
/// | `mirror` | `Mirror` | `mi` | `Mirror` |
/// | `scale` | `Scale` | `sc` | `Scale` |
///
/// `delete` / `del` / `erase` all reach the same tool: R14 says `ERASE`, the
/// tool and its menu entry say Delete, so both vocabularies are accepted
/// rather than picking a side. **`offset` is deliberately absent**: there is
/// no `ToolKind::Offset` (the tool was rejected as LCV-054), so the word is
/// unspellable, not merely unwanted — a word may never be the way a new tool
/// enters the product. No F1-dialog row documents these; see the rustdoc's
/// own citation above and `CHANGELOG.md`.
fn tool_alias(lower: &str) -> Option<ToolKind> {
    match lower {
        // Letters — closed axis (ADR 0003 §A2a).
        "l" => Some(ToolKind::Line),
        "p" => Some(ToolKind::Polyline),
        "r" => Some(ToolKind::Rect),
        "c" => Some(ToolKind::Circle),
        "a" => Some(ToolKind::Arc),
        "s" => Some(ToolKind::Select),
        "t" => Some(ToolKind::Trim),
        "e" => Some(ToolKind::Delete),
        "m" => Some(ToolKind::Move),
        // Words — open axis (LCV-131).
        "line" => Some(ToolKind::Line),
        "polyline" | "pline" => Some(ToolKind::Polyline),
        "rect" | "rectangle" => Some(ToolKind::Rect),
        "circle" => Some(ToolKind::Circle),
        "arc" => Some(ToolKind::Arc),
        "select" => Some(ToolKind::Select),
        "trim" => Some(ToolKind::Trim),
        "extend" => Some(ToolKind::Extend),
        "move" => Some(ToolKind::Move),
        "text" => Some(ToolKind::Text),
        "delete" | "del" | "erase" => Some(ToolKind::Delete),
        "copy" | "co" | "cp" => Some(ToolKind::Copy),
        "dist" | "di" => Some(ToolKind::Dist),
        "rotate" | "ro" => Some(ToolKind::Rotate),
        "mirror" | "mi" => Some(ToolKind::Mirror),
        "scale" | "sc" => Some(ToolKind::Scale),
        _ => None,
    }
}

/// The three toggle aliases. `lower` is already lowercased and trimmed.
fn toggle_alias(lower: &str) -> Option<ToggleKind> {
    match lower {
        "snap" => Some(ToggleKind::Snap),
        "grid" => Some(ToggleKind::Grid),
        "ortho" => Some(ToggleKind::Ortho),
        _ => None,
    }
}

/// The zoom forms: `"ze"` (extents alias) and `"zoom <in|out|extents>"`,
/// tolerant of extra ASCII whitespace between the keyword and its argument.
/// `lower` is already lowercased and trimmed.
fn zoom_form(lower: &str) -> Option<ZoomKind> {
    if lower == "ze" {
        return Some(ZoomKind::Extents);
    }
    let mut tokens = lower.split_whitespace();
    if tokens.next()? != "zoom" {
        return None;
    }
    let arg = tokens.next()?;
    if tokens.next().is_some() {
        return None; // more than one argument, e.g. "zoom in extra"
    }
    match arg {
        "in" => Some(ZoomKind::In),
        "out" => Some(ZoomKind::Out),
        "extents" => Some(ZoomKind::Extents),
        _ => None,
    }
}

/// Split `s` on exactly one `,` into two numbers via [`parse_number`].
///
/// `None` if there is not exactly one comma, or either side fails to parse —
/// so `"1,2,3"` (three fields) and `"10,"` / `",10"` (an empty side) are
/// rejected rather than silently truncated.
fn parse_pair(s: &str) -> Option<Vec2> {
    let mut parts = s.split(',');
    let x_str = parts.next()?;
    let y_str = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let x = parse_number(x_str.trim())?;
    let y = parse_number(y_str.trim())?;
    Some(Vec2::new(x, y))
}

/// Split `s` on one `<` into a distance and an angle in degrees, CCW from +X,
/// both via [`parse_number`], and return the offset `d·(cos a, sin a)` in mm.
///
/// Degrees exist only here, at the text boundary. A multiple of 90° uses an
/// exact unit vector, so `10<90` is (0, 10), not (6e-16, 10). `None` when
/// either side is missing or does not parse, so `<30`, `10<` and `10<<5` are
/// rejected.
fn parse_polar(s: &str) -> Option<Vec2> {
    let (d_str, a_str) = s.split_once('<')?;
    let distance = parse_number(d_str)?;
    let degrees = parse_number(a_str)?.rem_euclid(360.0);
    let unit = if degrees % 90.0 == 0.0 {
        match (degrees / 90.0) as u8 % 4 {
            0 => Vec2::new(1.0, 0.0),
            1 => Vec2::new(0.0, 1.0),
            2 => Vec2::new(-1.0, 0.0),
            _ => Vec2::new(0.0, -1.0),
        }
    } else {
        let radians = degrees.to_radians();
        Vec2::new(radians.cos(), radians.sin())
    };
    Some(unit * distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    // parse_number (AC 5)
    // -----------------------------------------------------------------

    #[test]
    fn parse_number_accepts_plain_decimal_and_signed() {
        assert_eq!(parse_number("37.5"), Some(37.5));
        assert_eq!(parse_number("-37.5"), Some(-37.5));
        assert_eq!(parse_number("0"), Some(0.0));
        assert_eq!(parse_number("+5"), Some(5.0));
        assert_eq!(parse_number("5."), Some(5.0));
    }

    #[test]
    fn parse_number_rejects_non_finite() {
        for raw in ["nan", "NaN", "inf", "-inf", "infinity", "1e400"] {
            assert_eq!(parse_number(raw), None, "expected None for {raw:?}");
        }
    }

    #[test]
    fn parse_number_trims_whitespace() {
        assert_eq!(parse_number(" -3.5 "), Some(-3.5));
        assert_eq!(parse_number("\t5\t"), Some(5.0));
    }

    #[test]
    fn parse_number_rejects_empty_and_garbage() {
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("   "), None);
        assert_eq!(parse_number("abc"), None);
        assert_eq!(parse_number("12abc"), None);
    }

    // -----------------------------------------------------------------
    // The grammar (AC 6, 7, 8) — every row of the demand's table.
    // -----------------------------------------------------------------

    #[test]
    fn parses_absolute_points() {
        assert_eq!(parse("50,25"), CommandInput::Point(Vec2::new(50.0, 25.0)));
        assert_eq!(
            parse("  50 , 25  "),
            CommandInput::Point(Vec2::new(50.0, 25.0))
        );
        assert_eq!(parse("-3.5,0"), CommandInput::Point(Vec2::new(-3.5, 0.0)));
        assert_eq!(parse("0,0"), CommandInput::Point(Vec2::new(0.0, 0.0)));
        assert_eq!(
            parse("10,5"),
            CommandInput::Point(Vec2::new(10.0, 5.0)),
            "10,5 is the point (10, 5), never the number 10.5"
        );
    }

    #[test]
    fn parses_relative_offsets() {
        assert_eq!(
            parse("@10,-5"),
            CommandInput::Relative(Vec2::new(10.0, -5.0))
        );
        assert_eq!(
            parse("@ 10 , -5"),
            CommandInput::Relative(Vec2::new(10.0, -5.0))
        );
    }

    /// LCV-159 AC1–AC3, AC5 — `@d<a` is a relative offset, `d<a` an absolute
    /// point, degrees CCW from +X; signs, decimals and spaces around `<` are
    /// accepted, quarter turns are exact, malformed polar text is `Unknown`.
    #[test]
    fn parses_polar_input() {
        let near = |got: CommandInput, want: CommandInput| {
            let (g, w) = match (got, want) {
                (CommandInput::Relative(g), CommandInput::Relative(w)) => (g, w),
                (CommandInput::Point(g), CommandInput::Point(w)) => (g, w),
                (g, w) => panic!("got {g:?}, want {w:?}"),
            };
            assert!((g - w).length() <= 1e-12, "got {g:?}, want {w:?}");
        };
        let d30 = 30f64.to_radians();
        let d45 = 45f64.to_radians();
        near(
            parse("@50<30"),
            CommandInput::Relative(Vec2::new(50.0 * d30.cos(), 50.0 * d30.sin())),
        );
        near(
            parse("50<30"),
            CommandInput::Point(Vec2::new(50.0 * d30.cos(), 50.0 * d30.sin())),
        );
        near(
            parse("@10<-45"),
            CommandInput::Relative(Vec2::new(10.0 * d45.cos(), -10.0 * d45.sin())),
        );
        near(
            parse("@-10<45"),
            CommandInput::Relative(Vec2::new(-10.0 * d45.cos(), -10.0 * d45.sin())),
        );
        near(
            parse("@ 10 < 45"),
            CommandInput::Relative(Vec2::new(10.0 * d45.cos(), 10.0 * d45.sin())),
        );
        near(
            parse("@2.5<12.5"),
            CommandInput::Relative(Vec2::new(
                2.5 * 12.5f64.to_radians().cos(),
                2.5 * 12.5f64.to_radians().sin(),
            )),
        );

        // Quarter turns are exact, not merely close.
        assert_eq!(parse("@10<0"), CommandInput::Relative(Vec2::new(10.0, 0.0)));
        assert_eq!(
            parse("@10<90"),
            CommandInput::Relative(Vec2::new(0.0, 10.0))
        );
        assert_eq!(
            parse("@10<180"),
            CommandInput::Relative(Vec2::new(-10.0, 0.0))
        );
        assert_eq!(
            parse("@10<270"),
            CommandInput::Relative(Vec2::new(0.0, -10.0))
        );
        assert_eq!(
            parse("@10<-90"),
            CommandInput::Relative(Vec2::new(0.0, -10.0))
        );
        assert_eq!(parse("100<0"), CommandInput::Point(Vec2::new(100.0, 0.0)));

        for raw in [
            "@<30", "@10<", "10<<5", "<30", "@10<abc", "1,2<3", "@10<nan",
        ] {
            assert_eq!(parse(raw), CommandInput::Unknown(raw.to_owned()), "{raw:?}");
        }
    }

    #[test]
    fn parses_bare_distances() {
        assert_eq!(parse("37.5"), CommandInput::Distance(37.5));
        assert_eq!(parse("-37.5"), CommandInput::Distance(-37.5));
        assert_eq!(parse("0"), CommandInput::Distance(0.0));
    }

    #[test]
    fn parses_tool_aliases() {
        assert_eq!(parse("l"), CommandInput::Tool(ToolKind::Line));
        assert_eq!(parse("L"), CommandInput::Tool(ToolKind::Line));
        assert_eq!(parse("  L  "), CommandInput::Tool(ToolKind::Line));
        assert_eq!(parse("p"), CommandInput::Tool(ToolKind::Polyline));
        assert_eq!(parse("r"), CommandInput::Tool(ToolKind::Rect));
        assert_eq!(parse("c"), CommandInput::Tool(ToolKind::Circle));
        assert_eq!(parse("a"), CommandInput::Tool(ToolKind::Arc));
        assert_eq!(parse("s"), CommandInput::Tool(ToolKind::Select));
        assert_eq!(parse("t"), CommandInput::Tool(ToolKind::Trim));
        assert_eq!(
            parse("e"),
            CommandInput::Tool(ToolKind::Delete),
            "product decision 2: e is ERASE, not EXTEND"
        );
        assert_eq!(parse("E"), CommandInput::Tool(ToolKind::Delete));
        assert_eq!(parse("m"), CommandInput::Tool(ToolKind::Move));
        assert_eq!(parse("text"), CommandInput::Tool(ToolKind::Text));
        assert_eq!(parse("TEXT"), CommandInput::Tool(ToolKind::Text));
        assert_eq!(parse(" Text "), CommandInput::Tool(ToolKind::Text));

        // LCV-131 AC 1 — the word axis (ADR 0003 §A2a): all approved
        // rows, each naming the exact `ToolKind` a swapped mapping would
        // otherwise survive. `text` repeats the assertion above; it is in the
        // set (already shipped) and belongs in the one table that names it.
        let words: &[(&str, ToolKind)] = &[
            ("line", ToolKind::Line),
            ("polyline", ToolKind::Polyline),
            ("pline", ToolKind::Polyline),
            ("rect", ToolKind::Rect),
            ("rectangle", ToolKind::Rect),
            ("circle", ToolKind::Circle),
            ("arc", ToolKind::Arc),
            ("select", ToolKind::Select),
            ("trim", ToolKind::Trim),
            ("extend", ToolKind::Extend),
            ("move", ToolKind::Move),
            ("text", ToolKind::Text),
            ("delete", ToolKind::Delete),
            ("del", ToolKind::Delete),
            ("erase", ToolKind::Delete),
            ("copy", ToolKind::Copy),
            ("co", ToolKind::Copy),
            ("cp", ToolKind::Copy),
            ("dist", ToolKind::Dist),
            ("di", ToolKind::Dist),
            ("rotate", ToolKind::Rotate),
            ("ro", ToolKind::Rotate),
            ("mirror", ToolKind::Mirror),
            ("mi", ToolKind::Mirror),
            ("scale", ToolKind::Scale),
            ("sc", ToolKind::Scale),
        ];
        assert_eq!(
            words.len(),
            26,
            "the approved word set is exactly twenty-six"
        );
        for &(word, kind) in words {
            assert_eq!(
                parse(word),
                CommandInput::Tool(kind),
                "word {word:?} must resolve to {kind:?}"
            );
        }
    }

    /// LCV-157 AC1 — `copy`, `co` and `cp` (any case) are COPY; `c` stays CIRCLE.
    #[test]
    fn copy_words_are_copy() {
        for word in ["copy", "COPY", "co", "Co", "cp", " CP "] {
            assert_eq!(parse(word), CommandInput::Tool(ToolKind::Copy), "{word:?}");
        }
        assert_eq!(parse("c"), CommandInput::Tool(ToolKind::Circle));
    }

    /// LCV-158 AC1 — `rotate` and `ro` (any case) are ROTATE; `r` stays RECT.
    #[test]
    fn rotate_words_are_rotate() {
        for word in ["rotate", "ROTATE", "ro", " Ro "] {
            assert_eq!(
                parse(word),
                CommandInput::Tool(ToolKind::Rotate),
                "{word:?}"
            );
        }
        assert_eq!(parse("r"), CommandInput::Tool(ToolKind::Rect));
    }

    /// LCV-181 AC1 — `mirror` and `mi` (any case) are MIRROR; `m` stays MOVE.
    #[test]
    fn mirror_words_are_mirror() {
        for word in ["mirror", "MIRROR", "mi", " Mi "] {
            assert_eq!(
                parse(word),
                CommandInput::Tool(ToolKind::Mirror),
                "{word:?}"
            );
        }
        assert_eq!(parse("m"), CommandInput::Tool(ToolKind::Move));
    }

    /// LCV-182 AC1 — `scale` and `sc` (any case) are SCALE; `s` stays SELECT.
    #[test]
    fn scale_words_are_scale() {
        for word in ["scale", "SCALE", "sc", " Sc "] {
            assert_eq!(parse(word), CommandInput::Tool(ToolKind::Scale), "{word:?}");
        }
        assert_eq!(parse("s"), CommandInput::Tool(ToolKind::Select));
    }

    /// LCV-190 AC 1 — `check` (any case) is the Check command; no shorter
    /// form or letter is a word.
    #[test]
    fn parses_check_word() {
        for raw in ["check", "CHECK", " Check "] {
            assert_eq!(parse(raw), CommandInput::Check, "{raw:?}");
        }
        for raw in ["ch", "chk", "checks", "check all"] {
            assert_eq!(parse(raw), CommandInput::Unknown(raw.into()), "{raw:?}");
        }
    }

    /// LCV-156 AC 4 — `layer` and `la` (any case) are the Layers command;
    /// `lay` and `layers` are not words.
    #[test]
    fn parses_layer_words() {
        for raw in ["layer", "LAYER", " La "] {
            assert_eq!(parse(raw), CommandInput::Layers, "{raw:?}");
        }
        for raw in ["lay", "layers"] {
            assert_eq!(parse(raw), CommandInput::Unknown(raw.into()), "{raw:?}");
        }
    }

    #[test]
    fn parses_toggles() {
        assert_eq!(parse("snap"), CommandInput::Toggle(ToggleKind::Snap));
        assert_eq!(parse("SNAP"), CommandInput::Toggle(ToggleKind::Snap));
        assert_eq!(parse("grid"), CommandInput::Toggle(ToggleKind::Grid));
        assert_eq!(parse("ortho"), CommandInput::Toggle(ToggleKind::Ortho));
    }

    #[test]
    fn parses_zoom_forms() {
        assert_eq!(parse("zoom in"), CommandInput::Zoom(ZoomKind::In));
        assert_eq!(parse("ZOOM   In"), CommandInput::Zoom(ZoomKind::In));
        assert_eq!(parse("zoom out"), CommandInput::Zoom(ZoomKind::Out));
        assert_eq!(parse("zoom extents"), CommandInput::Zoom(ZoomKind::Extents));
        assert_eq!(parse("ze"), CommandInput::Zoom(ZoomKind::Extents));
        assert_eq!(parse("ZE"), CommandInput::Zoom(ZoomKind::Extents));
    }

    #[test]
    fn blank_input_is_empty() {
        assert_eq!(parse(""), CommandInput::Empty);
        assert_eq!(parse("   "), CommandInput::Empty);
        assert_eq!(parse("\t"), CommandInput::Empty);
    }

    #[test]
    fn rejects_malformed_input_as_unknown() {
        assert_eq!(
            parse("1,2,3"),
            CommandInput::Unknown("1,2,3".to_owned()),
            "never take the first two fields"
        );
        assert_eq!(parse("@"), CommandInput::Unknown("@".to_owned()));
        assert_eq!(parse("-"), CommandInput::Unknown("-".to_owned()));
        assert_eq!(parse("10,"), CommandInput::Unknown("10,".to_owned()));
        assert_eq!(parse(",10"), CommandInput::Unknown(",10".to_owned()));
        assert_eq!(parse("@,"), CommandInput::Unknown("@,".to_owned()));
        assert_eq!(parse("1,abc"), CommandInput::Unknown("1,abc".to_owned()));
        assert_eq!(parse("nan"), CommandInput::Unknown("nan".to_owned()));
        assert_eq!(parse("inf,0"), CommandInput::Unknown("inf,0".to_owned()));
        assert_eq!(parse("1e400"), CommandInput::Unknown("1e400".to_owned()));
        assert_eq!(parse("zoom"), CommandInput::Unknown("zoom".to_owned()));
        assert_eq!(
            parse("zoom sideways"),
            CommandInput::Unknown("zoom sideways".to_owned())
        );
        assert_eq!(parse("x"), CommandInput::Unknown("x".to_owned()));
        assert_eq!(parse("d"), CommandInput::Unknown("d".to_owned()));
        // "line" and "del" moved into `parses_tool_aliases` (LCV-131 AC 10):
        // both are now Tool(_), not Unknown. Not silently dropped — see that
        // test's word table.
        assert_eq!(
            parse(":draw a square"),
            CommandInput::Unknown(":draw a square".to_owned())
        );
    }

    #[test]
    fn unknown_payload_is_trimmed_and_keeps_original_case() {
        assert_eq!(parse("  Foo  "), CommandInput::Unknown("Foo".to_owned()));
    }

    /// LCV-131 AC 4 — the word axis must not shadow any existing form.
    /// `tool_alias` is consulted first in `parse`, so a careless row could
    /// silently steal an existing line: every toggle, every zoom form, a
    /// point, a relative offset, a distance, an empty line, and the five
    /// `Unknown` controls, all asserted against the same `parse` in one run.
    #[test]
    fn tool_words_do_not_shadow_other_forms() {
        assert_eq!(parse("snap"), CommandInput::Toggle(ToggleKind::Snap));
        assert_eq!(parse("grid"), CommandInput::Toggle(ToggleKind::Grid));
        assert_eq!(parse("ortho"), CommandInput::Toggle(ToggleKind::Ortho));

        assert_eq!(parse("ze"), CommandInput::Zoom(ZoomKind::Extents));
        assert_eq!(parse("zoom in"), CommandInput::Zoom(ZoomKind::In));
        assert_eq!(parse("zoom out"), CommandInput::Zoom(ZoomKind::Out));
        assert_eq!(parse("zoom extents"), CommandInput::Zoom(ZoomKind::Extents));

        assert_eq!(parse("50,25"), CommandInput::Point(Vec2::new(50.0, 25.0)));
        assert_eq!(parse("@10,0"), CommandInput::Relative(Vec2::new(10.0, 0.0)));
        assert_eq!(parse("37.5"), CommandInput::Distance(37.5));
        assert_eq!(parse(""), CommandInput::Empty);

        // Reserved by inaction (§The product call): none of these gets a
        // mechanism, and none may be quietly picked up by a careless row.
        for raw in ["zoom", "zoom sideways", "z", "1,2,3", "nan"] {
            assert_eq!(
                parse(raw),
                CommandInput::Unknown(raw.to_owned()),
                "{raw:?} must stay Unknown, payload unchanged"
            );
        }
    }

    /// Decision 1: `.` is always the decimal separator, `,` is always the
    /// coordinate separator — no locale detection, no `LANG` sniffing.
    /// `"10,5"` is the point (10, 5), never the number 10.5.
    #[test]
    fn comma_is_always_the_separator_never_a_decimal_point() {
        assert_eq!(parse("10,5"), CommandInput::Point(Vec2::new(10.0, 5.0)));
        assert_eq!(
            parse("10.5,20.25"),
            CommandInput::Point(Vec2::new(10.5, 20.25))
        );
        assert_eq!(parse("10.5"), CommandInput::Distance(10.5));
    }

    #[test]
    fn three_field_coordinates_are_rejected_not_truncated() {
        assert_eq!(parse("1,2,3"), CommandInput::Unknown("1,2,3".to_owned()));
    }

    #[test]
    fn aliases_are_case_insensitive() {
        assert_eq!(parse("l"), parse("L"));
        assert_eq!(parse("text"), parse("TEXT"));
        assert_eq!(parse("snap"), parse("SNAP"));
        assert_eq!(parse("ze"), parse("ZE"));
        assert_eq!(parse("zoom in"), parse("ZOOM IN"));

        // LCV-131 AC 2 — the word axis is case-insensitive too, exact
        // `ToolKind` asserted (not merely equal to each other), whitespace
        // padding included.
        assert_eq!(parse("LINE"), CommandInput::Tool(ToolKind::Line));
        assert_eq!(parse("Line"), CommandInput::Tool(ToolKind::Line));
        assert_eq!(parse("line"), CommandInput::Tool(ToolKind::Line));
        assert_eq!(parse("ERASE"), CommandInput::Tool(ToolKind::Delete));
        assert_eq!(parse("Del"), CommandInput::Tool(ToolKind::Delete));
        assert_eq!(parse("RECTANGLE"), CommandInput::Tool(ToolKind::Rect));
        assert_eq!(
            parse("  circle  "),
            CommandInput::Tool(ToolKind::Circle),
            "surrounding whitespace is still trimmed"
        );
    }
}
