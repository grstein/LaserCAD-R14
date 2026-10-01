//! The root `<svg>` header: the bed size a file declares (LCV-114 AC 8/AC 9).
//!
//! Split out of `import.rs` so both stay well inside the 300-line cap
//! (ADR 0004). One public function, [`parse_bed`]; everything else is the
//! numeric grammar it accepts.
//!
//! Millimetres are canonical (AGENTS.md), so the grammar is deliberately
//! narrow: a bare number or a number with an optional, case-insensitive `mm`
//! suffix, with whitespace tolerated around both. `pt`, `in`, `px`, `%` and
//! unitless-with-scale headers are **not** accepted — silently reading `210`
//! user units as 210 mm is the same class of bug this demand exists to kill.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::import::SvgImportError;
use crate::geometry::EPSILON;
use crate::util::{BED_MAX_MM, BED_MIN_MM, DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

/// Read the bed size `[width, height]` in mm from a root `<svg>` element.
///
/// Precedence (AC 8):
/// 1. `width` **and** `height` both present → that pair is the bed;
/// 2. otherwise a `viewBox="0 0 W H"` → `W`/`H` are the bed;
/// 3. otherwise `[DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]` — a constant,
///    never the currently open document's bed, so an import result never
///    depends on hidden state (LCV-114 product decision 5).
///
/// Any of the three attributes that is *present* is validated even when
/// precedence means it goes unused: a header this module cannot honour is
/// reported rather than half-read. Out-of-range values are rejected, not
/// clamped (AC 9) — a 5000 mm canvas quietly held to 2000 mm would place every
/// coordinate wrongly while still looking plausible.
pub fn parse_bed(root: roxmltree::Node<'_, '_>) -> Result<[f64; 2], SvgImportError> {
    let width = dimension(root, "width")?;
    let height = dimension(root, "height")?;
    let view_box = view_box(root)?;
    Ok(match (width, height, view_box) {
        (Some(w), Some(h), _) => [w, h],
        (_, _, Some(vb)) => vb,
        _ => [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM],
    })
}

/// One length attribute: `Ok(None)` when absent, `Err` when present but not a
/// usable bed dimension.
fn dimension(
    root: roxmltree::Node<'_, '_>,
    attr: &'static str,
) -> Result<Option<f64>, SvgImportError> {
    let Some(raw) = root.attribute(attr) else {
        return Ok(None);
    };
    parse_mm(raw).map(Some).ok_or_else(|| bad(attr, raw))
}

/// The `viewBox`, restricted to the `"0 0 W H"` shape this app writes.
///
/// A non-zero origin would need a transform stack (explicitly out of scope),
/// so it is an error rather than a silent offset.
fn view_box(root: roxmltree::Node<'_, '_>) -> Result<Option<[f64; 2]>, SvgImportError> {
    let Some(raw) = root.attribute("viewBox") else {
        return Ok(None);
    };
    let tok: Vec<&str> = raw
        .split([',', ' ', '\t', '\n', '\r'])
        .filter(|t| !t.is_empty())
        .collect();
    if tok.len() != 4 {
        return Err(bad("viewBox", raw));
    }
    let origin_at_zero = tok[..2]
        .iter()
        .all(|t| t.parse::<f64>().is_ok_and(|v| v.abs() < EPSILON));
    let (Some(w), Some(h)) = (parse_mm(tok[2]), parse_mm(tok[3])) else {
        return Err(bad("viewBox", raw));
    };
    if !origin_at_zero {
        return Err(bad("viewBox", raw));
    }
    Ok(Some([w, h]))
}

/// The accepted numeric grammar: optional whitespace, a number, an optional
/// case-insensitive `mm`, optional whitespace — and the result must be a
/// usable bed dimension (finite, inside `BED_MIN_MM ..= BED_MAX_MM`).
///
/// `NaN` and the infinities fail the range test rather than propagating:
/// `RangeInclusive::contains` is false for `NaN`.
fn parse_mm(raw: &str) -> Option<f64> {
    let t = raw.trim();
    let t = match t.len().checked_sub(2) {
        Some(cut) if t[cut..].eq_ignore_ascii_case("mm") => t[..cut].trim_end(),
        _ => t,
    };
    let v = t.parse::<f64>().ok()?;
    (BED_MIN_MM..=BED_MAX_MM).contains(&v).then_some(v)
}

fn bad(attr: &'static str, value: &str) -> SvgImportError {
    SvgImportError::MalformedBedDimension {
        attr,
        value: value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse `attrs` as a root `<svg>` and return the bed it declares.
    fn bed_of(attrs: &str) -> Result<[f64; 2], SvgImportError> {
        let src = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" {attrs}/>"#);
        let doc = roxmltree::Document::parse(&src).expect("fixture must be valid XML");
        parse_bed(doc.root_element())
    }

    fn attr_of(e: &SvgImportError) -> &'static str {
        match e {
            SvgImportError::MalformedBedDimension { attr, .. } => attr,
            other => panic!("expected MalformedBedDimension, got {other:?}"),
        }
    }

    fn near(got: [f64; 2], want: [f64; 2]) -> bool {
        (got[0] - want[0]).abs() < 1e-9 && (got[1] - want[1]).abs() < 1e-9
    }

    /// LCV-173 AC 2 — absolute `width` + `height` are the bed, in any unit,
    /// unitless being px; `mm` is taken as written.
    #[test]
    fn an_absolute_pair_is_the_bed() {
        assert_eq!(
            bed_of(r#"width="300mm" height="180mm" viewBox="0 0 111 222""#).unwrap(),
            [300.0, 180.0]
        );
        assert_eq!(
            bed_of(r#"width=" 400.5 mm " height="128MM""#).unwrap(),
            [400.5, 128.0]
        );
        for (attrs, want) in [
            (r#"width="1in" height="72pt""#, [25.4, 25.4]),
            (r#"width="96px" height="960""#, [25.4, 254.0]),
            (r#"width="10cm" height="100Q""#, [100.0, 25.0]),
            (r#"width="6pc" height="5000""#, [25.4, 5000.0 * 25.4 / 96.0]),
        ] {
            assert!(near(bed_of(attrs).unwrap(), want), "{attrs}");
        }
    }

    /// LCV-173 AC 2 — a relative or absent side takes both from the viewBox,
    /// read as px, whatever its origin.
    #[test]
    fn a_relative_or_absent_side_falls_back_to_the_view_box_as_px() {
        for attrs in [
            r#"viewBox="0 0 960 480""#,
            r#"viewBox="10,-20,960,480""#,
            r#"width="300mm" viewBox="0 0 960 480""#,
            r#"height="180mm" viewBox="0 0 960 480""#,
            r#"width="50%" height="180mm" viewBox="0 0 960 480""#,
            r#"width="2em" height="3ex" viewBox="0 0 960 480""#,
        ] {
            assert!(near(bed_of(attrs).unwrap(), [254.0, 127.0]), "{attrs}");
        }
    }

    /// LCV-173 AC 2 — no usable pair and no viewBox: the default bed, never
    /// the open document's.
    #[test]
    fn no_pair_and_no_view_box_is_the_default_bed() {
        let default = [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM];
        for attrs in ["", r#"width="300mm""#, r#"width="100%" height="100%""#] {
            assert_eq!(bed_of(attrs).unwrap(), default, "{attrs}");
        }
    }

    /// LCV-173 AC 3 — a bed side outside 1..=2000 mm, or an unparseable
    /// side, is refused with the attribute it came from and its raw value.
    #[test]
    fn rejects_unusable_dimensions() {
        for raw in [
            "0", "-10mm", "5000mm", "0.5mm", "3", "abc", "", "inf", "NaN", "10km",
        ] {
            let e = bed_of(&format!(r#"width="{raw}" height="400mm""#)).unwrap_err();
            assert_eq!(attr_of(&e), "width", "width={raw:?} must be rejected");
            assert!(
                e.to_string().contains(raw),
                "the raw value must survive into the message: {e}"
            );
            let e = bed_of(&format!(r#"width="400mm" height="{raw}""#)).unwrap_err();
            assert_eq!(attr_of(&e), "height", "height={raw:?} must be rejected");
        }
        assert_eq!(
            bed_of(r#"width="1mm" height="2000mm""#).unwrap(),
            [1.0, 2000.0]
        );
    }

    /// LCV-173 AC 3 — a viewBox-derived bed out of range, or a viewBox that
    /// is not four finite numbers with positive size, is refused; the latter
    /// even when the pair is used, since the viewBox maps every coordinate.
    #[test]
    fn rejects_unusable_view_boxes() {
        for raw in ["0 0 9000 180", "0 0 2 180"] {
            let e = bed_of(&format!(r#"viewBox="{raw}""#)).unwrap_err();
            assert_eq!(attr_of(&e), "viewBox", "viewBox={raw:?}");
        }
        for raw in [
            "0 0 300",
            "0 0 300 180 90",
            "0 0 0 180",
            "0 0 300 -1",
            "a b c d",
        ] {
            let e =
                bed_of(&format!(r#"width="300mm" height="180mm" viewBox="{raw}""#)).unwrap_err();
            assert_eq!(attr_of(&e), "viewBox", "viewBox={raw:?}");
        }
    }
}
