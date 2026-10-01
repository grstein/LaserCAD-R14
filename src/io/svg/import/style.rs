//! The style cascade of SVG import (LCV-175, SVG 2 ch. 6): `stroke`, `fill`,
//! `color`, `visibility` (inherited) and `display` (not inherited) of one
//! element, from its presentation attributes, the document's `<style>` rules
//! and its `style` attribute.
//!
//! Precedence, highest first: important `style` declarations, important
//! rules, `style` declarations, rules, the presentation attribute, the
//! parent's value. Rules rank by specificity, then document order; within a
//! `style` attribute the last declaration wins. The first *valid* candidate
//! wins: an unsupported color is dropped and noted `<prop> (invalid color)`.
//! `inherit`/`unset` take the parent's value; `currentColor` is kept as a
//! keyword and resolves against the element's own `color`. SVG's initial
//! `fill: black` is not applied: only declared colors count.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use super::report::Report;
use crate::io::svg::css::Sheet;

/// A `stroke` or `fill` value after the cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paint {
    /// `none`, `initial`, or never declared.
    None,
    /// A CSS color.
    Color([u8; 3]),
    /// `currentColor`: the element's own `color`.
    Current,
}

/// The computed style of one element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Style {
    stroke: Paint,
    fill: Paint,
    color: Option<[u8; 3]>,
    /// `visibility` is `hidden` or `collapse` (inherited).
    pub(super) invisible: bool,
    /// `display` is `none` (not inherited).
    pub(super) display_none: bool,
}

impl Style {
    /// The style above the root element: nothing declared, visible.
    pub(super) fn root() -> Self {
        Self {
            stroke: Paint::None,
            fill: Paint::None,
            color: None,
            invisible: false,
            display_none: false,
        }
    }

    /// The style of `node`, a child of an element styled `self`.
    pub(super) fn child(
        &self,
        _node: roxmltree::Node<'_, '_>,
        _sheet: &Sheet,
        _report: &mut Report,
    ) -> Self {
        *self
    }

    /// The resolved stroke color, if it is a color.
    pub(super) fn stroke(&self) -> Option<[u8; 3]> {
        None
    }

    /// The resolved fill color, if it is a color.
    pub(super) fn fill(&self) -> Option<[u8; 3]> {
        None
    }
}

/// Every SVG `<style>` of the document, in document order, as one sheet;
/// what each drops is noted in `report`.
pub(super) fn collect_sheet(_root: roxmltree::Node<'_, '_>, _report: &mut Report) -> Sheet {
    Sheet::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 3] = [255, 0, 0];
    const LIME: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    /// The style of the element `id="t"` in an `<svg>` around `body`, and the
    /// report the cascade made on the way down.
    fn resolve(body: &str) -> (Style, Vec<(String, usize)>) {
        let xml = format!(r#"<svg xmlns="http://www.w3.org/2000/svg">{body}</svg>"#);
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let mut report = Report::default();
        let sheet = collect_sheet(doc.root_element(), &mut report);
        let target = doc
            .descendants()
            .find(|n| n.attribute("id") == Some("t"))
            .unwrap();
        let chain: Vec<_> = target.ancestors().filter(|n| n.is_element()).collect();
        let mut style = Style::root();
        for node in chain.iter().rev() {
            style = style.child(*node, &sheet, &mut report);
        }
        (style, report.finish())
    }

    fn stroke(body: &str) -> Option<[u8; 3]> {
        resolve(body).0.stroke()
    }

    /// AC 1 — the attribute loses to a rule, which loses to `style`.
    #[test]
    fn style_beats_rule_beats_attribute() {
        let rule = "<style>line{stroke:blue}</style>";
        assert_eq!(stroke(r#"<line id="t" stroke="red"/>"#), Some(RED));
        assert_eq!(
            stroke(&format!(r#"{rule}<line id="t" stroke="red"/>"#)),
            Some(BLUE)
        );
        let styled = format!(r#"{rule}<line id="t" stroke="red" style="stroke:lime"/>"#);
        assert_eq!(stroke(&styled), Some(LIME));
        let last = r#"<line id="t" style="stroke:lime;stroke:red"/>"#;
        assert_eq!(stroke(last), Some(RED), "the last style declaration wins");
    }

    /// AC 1 — an `!important` rule beats `style`; an important `style`
    /// beats it back.
    #[test]
    fn important_beats_everything() {
        let rule = "<style>line{stroke:blue!important} line{stroke:red}</style>";
        let body = format!(r#"{rule}<line id="t" style="stroke:lime"/>"#);
        assert_eq!(stroke(&body), Some(BLUE));
        let body = format!(r#"{rule}<line id="t" style="stroke:lime !important"/>"#);
        assert_eq!(stroke(&body), Some(LIME));
    }

    /// AC 2 — id > class > type > `*`, whatever the order; a tie goes to
    /// the later rule.
    #[test]
    fn specificity_then_order_ranks_rules() {
        let el = r#"<line id="t" class="c"/>"#;
        let rules = [
            "#t{stroke:red}",
            ".c{stroke:lime}",
            "line{stroke:blue}",
            "*{stroke:navy}",
        ];
        for skip in 0..3 {
            let mut kept: Vec<&str> = rules[skip..].to_vec();
            let want = [RED, LIME, BLUE][skip];
            assert_eq!(
                stroke(&format!("<style>{}</style>{el}", kept.join(""))),
                Some(want)
            );
            kept.reverse();
            assert_eq!(
                stroke(&format!("<style>{}</style>{el}", kept.join(""))),
                Some(want)
            );
        }
        let tie = format!("<style>.c{{stroke:red}}</style><style>.c{{stroke:blue}}</style>{el}");
        assert_eq!(stroke(&tie), Some(BLUE));
        let list = format!("<style>#t, x{{stroke:red}} .c{{stroke:blue}}</style>{el}");
        assert_eq!(stroke(&list), Some(RED), "a list item's own specificity");
    }

    /// AC 5 — `stroke`, `fill`, `color` inherit from the nearest ancestor
    /// that sets them; `inherit` takes the parent's over the element's own
    /// attribute.
    #[test]
    fn paints_and_color_inherit() {
        let body =
            r#"<g stroke="blue"><g stroke="red" fill="lime" color="blue"><line id="t"/></g></g>"#;
        let (style, report) = resolve(body);
        assert_eq!(
            (style.stroke(), style.fill(), style.color),
            (Some(RED), Some(LIME), Some(BLUE))
        );
        assert!(report.is_empty(), "{report:?}");
        let body = r#"<g stroke="red"><line id="t" style="stroke:inherit" stroke="blue"/></g>"#;
        assert_eq!(stroke(body), Some(RED));
        assert_eq!(
            stroke(r#"<g stroke="red"><line id="t" stroke="none"/></g>"#),
            None
        );
    }

    /// AC 6 — `currentColor` uses the element's own resolved `color`.
    #[test]
    fn current_color_takes_color() {
        let body =
            r#"<g color="lime"><line id="t" stroke="currentColor" fill="CurrentColor"/></g>"#;
        let (style, _) = resolve(body);
        assert_eq!((style.stroke(), style.fill()), (Some(LIME), Some(LIME)));
        let body = r#"<g stroke="currentColor" color="red"><line id="t" color="blue"/></g>"#;
        assert_eq!(
            stroke(body),
            Some(BLUE),
            "the keyword inherits, not the color"
        );
        assert_eq!(stroke(r#"<line id="t" stroke="currentColor"/>"#), None);
    }

    /// AC 11 — an unsupported color is dropped and noted; the next
    /// candidate, else the inherited value, applies.
    #[test]
    fn an_invalid_color_falls_back_and_is_noted() {
        let body =
            r#"<style>line{stroke:blue}</style><line id="t" style="stroke:#zz" stroke="red"/>"#;
        let (style, report) = resolve(body);
        assert_eq!(style.stroke(), Some(BLUE));
        assert_eq!(report, [("stroke (invalid color)".to_owned(), 1)]);
        let body = r#"<g stroke="red" color="lime"><line id="t" stroke="url(#g)" fill="transparent" color="bogus"/></g>"#;
        let (style, report) = resolve(body);
        assert_eq!((style.stroke(), style.color), (Some(RED), Some(LIME)));
        let labels: Vec<&str> = report.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(
            labels,
            [
                "stroke (invalid color)",
                "fill (invalid color)",
                "color (invalid color)"
            ]
        );
    }

    /// AC 3, AC 4 — what a `<style>` drops reaches the report.
    #[test]
    fn sheet_notes_reach_the_report() {
        let body =
            "<defs><style>@import url(x.css); g path{stroke:red}</style></defs><line id=\"t\"/>";
        let (_, report) = resolve(body);
        let labels: Vec<&str> = report.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(
            labels,
            ["style @import", "style rule (unsupported selector)"]
        );
    }

    /// AC 7 — `visibility` inherits and a `visible` descendant reverts it;
    /// `display` does not inherit.
    #[test]
    fn visibility_inherits_display_does_not() {
        let (style, _) = resolve(r#"<g visibility="hidden"><line id="t"/></g>"#);
        assert!(style.invisible && !style.display_none);
        let (style, _) =
            resolve(r#"<g style="visibility:collapse"><line id="t" visibility="visible"/></g>"#);
        assert!(!style.invisible);
        let (style, _) = resolve(r#"<g display="none"><line id="t"/></g>"#);
        assert!(!style.display_none);
        let (style, _) = resolve(r#"<style>.h{display:none}</style><line id="t" class="h"/>"#);
        assert!(style.display_none);
    }
}
