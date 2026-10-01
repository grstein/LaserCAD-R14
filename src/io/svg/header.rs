//! The root `<svg>` header (LCV-114, LCV-173): the bed size a file declares
//! and the map from its user units onto that bed.
//!
//! Lengths follow SVG 2 (see [`super::length`]): any absolute unit, unitless
//! being px at 96 px = 1 in. Millimetres stay canonical: a `mm` side is taken
//! as written, and LaserCAD's own `viewBox="0 0 W H"` on a `W mm × H mm` bed
//! maps with scale exactly 1 (LCV-173 AC 11).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::import::SvgImportError;
use super::length::{PX_PER_MM, parse_length};
use super::matrix::Matrix;
use super::viewport::{Ctx, par, parse_view_box, view_box_map};
use crate::io::svg::layers::attr as plain_attr;
use crate::util::{BED_MAX_MM, BED_MIN_MM, DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM};

/// What the root `<svg>` declares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Root {
    /// The bed `[width, height]` in mm.
    pub(super) bed_mm: [f64; 2],
    /// The walk's starting context: root user units onto the bed, Y-down.
    pub(super) ctx: Ctx,
}

/// Read the bed and the root context from a root `<svg>` element.
///
/// The bed (LCV-173 AC 2) is `width` × `height` when both are absolute;
/// otherwise the `viewBox` size read as px; otherwise
/// `[DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]` — a constant, never the
/// open document's bed (LCV-114 product decision 5). A side outside
/// `BED_MIN_MM ..= BED_MAX_MM` is refused, never clamped (AC 3).
///
/// The viewBox is mapped onto the bed rect per `preserveAspectRatio` (AC 4);
/// without one, one user unit is 1 px. A present side that does not parse,
/// or a viewBox that is not four finite numbers with positive size, is an
/// error even when unused: a header this module cannot honour is reported,
/// never half-read.
pub(super) fn parse_root(root: roxmltree::Node<'_, '_>) -> Result<Root, SvgImportError> {
    let width = side(root, "width")?;
    let height = side(root, "height")?;
    let view_box = match plain_attr(root, "viewBox") {
        Some(raw) => Some(parse_view_box(raw).ok_or_else(|| bad("viewBox", raw))?),
        None => None,
    };
    let bed_mm = match (width, height, view_box) {
        (Some((w, wa)), Some((h, ha)), _) => [checked(w, "width", wa)?, checked(h, "height", ha)?],
        (_, _, Some(vb)) => {
            let raw = plain_attr(root, "viewBox").unwrap_or_default();
            [
                checked(vb[2] / PX_PER_MM, "viewBox", raw)?,
                checked(vb[3] / PX_PER_MM, "viewBox", raw)?,
            ]
        }
        _ => [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM],
    };
    let ctx = match view_box {
        Some(vb) => Ctx {
            ctm: view_box_map(
                vb,
                [0.0, 0.0, bed_mm[0], bed_mm[1]],
                par(plain_attr(root, "preserveAspectRatio")),
            ),
            viewport: [vb[2], vb[3]],
        },
        None => Ctx {
            ctm: Matrix::scale(1.0 / PX_PER_MM, 1.0 / PX_PER_MM),
            viewport: bed_mm.map(|mm| mm * PX_PER_MM),
        },
    };
    Ok(Root { bed_mm, ctx })
}

/// One root side: `Ok(None)` when absent or relative (`%`, `em`, `ex`),
/// its mm and raw text when absolute, `Err` when it does not parse.
fn side<'a>(
    root: roxmltree::Node<'a, '_>,
    attr: &'static str,
) -> Result<Option<(f64, &'a str)>, SvgImportError> {
    let Some(raw) = plain_attr(root, attr) else {
        return Ok(None);
    };
    let len = parse_length(raw).ok_or_else(|| bad(attr, raw))?;
    Ok(len.to_mm().map(|mm| (mm, raw)))
}

/// `mm` when it is a usable bed side, else the error naming `attr`.
fn checked(mm: f64, attr: &'static str, raw: &str) -> Result<f64, SvgImportError> {
    if (BED_MIN_MM..=BED_MAX_MM).contains(&mm) {
        Ok(mm)
    } else {
        Err(bad(attr, raw))
    }
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
        parse_root(doc.root_element()).map(|root| root.bed_mm)
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
