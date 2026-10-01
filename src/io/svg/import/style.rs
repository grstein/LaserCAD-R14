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

use super::SVG_NS;
use super::report::Report;
use crate::document::LayerId;
use crate::io::svg::css::{Decl, Rule, Sheet, declarations, parse_sheet};
use crate::io::svg::css_color::parse_css_color;
use crate::io::svg::layers::Slot;

/// The report label of an element `display:none` hides, subtree included
/// (AC 7).
const HIDDEN_DISPLAY: &str = "hidden (display:none)";

/// The report label of an imported element whose `visibility` is `hidden`
/// or `collapse` (AC 7).
const HIDDEN_VISIBILITY: &str = "hidden (visibility)";

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
        node: roxmltree::Node<'_, '_>,
        sheet: &Sheet,
        report: &mut Report,
    ) -> Self {
        let rules = matching(node, sheet);
        let values = |prop| candidates(node, &rules, prop);
        let mut invalid = |prop: &str| report.note(&format!("{prop} (invalid color)"));
        let stroke = cascade(&values("stroke"), self.stroke, paint, || invalid("stroke"));
        let fill = cascade(&values("fill"), self.fill, paint, || invalid("fill"));
        let parent_color = self.color;
        let color = cascade(
            &values("color"),
            parent_color,
            |v| match v.to_ascii_lowercase().as_str() {
                "currentcolor" => Some(parent_color),
                "initial" => Some(None),
                _ => parse_css_color(v).map(Some),
            },
            || invalid("color"),
        );
        let visibility = |v: &str| match v.to_ascii_lowercase().as_str() {
            "visible" | "initial" => Some(false),
            "hidden" | "collapse" => Some(true),
            _ => None,
        };
        let none = |v: &str| Some(v.eq_ignore_ascii_case("none"));
        Self {
            stroke,
            fill,
            color,
            invisible: cascade(&values("visibility"), self.invisible, visibility, || ()),
            display_none: cascade(&values("display"), false, none, || ()),
        }
    }

    /// The resolved stroke color, if it is a color.
    pub(super) fn stroke(&self) -> Option<[u8; 3]> {
        self.resolve(self.stroke)
    }

    /// The resolved fill color, if it is a color.
    pub(super) fn fill(&self) -> Option<[u8; 3]> {
        self.resolve(self.fill)
    }

    /// Where an entity styled `self` belongs: `layer` (its innermost layer
    /// group) if any, else its stroke color, else its fill color (AC 8, AC 9),
    /// else the first layer (AC 10).
    pub(super) fn slot(&self, layer: Option<LayerId>) -> Slot {
        match (layer, self.stroke().or_else(|| self.fill())) {
            (Some(id), _) => Slot::Layer(id),
            (None, Some(rgb)) => Slot::Color(rgb),
            (None, None) => Slot::First,
        }
    }

    /// The report label when an element styled `self` is not imported (AC 7):
    /// `display:none` hides any element with its subtree; `visibility` hides
    /// an `imported` (geometry) element only, so a descendant may revert it.
    pub(super) fn hidden(&self, imported: bool) -> Option<&'static str> {
        if self.display_none {
            Some(HIDDEN_DISPLAY)
        } else if self.invisible && imported {
            Some(HIDDEN_VISIBILITY)
        } else {
            None
        }
    }

    fn resolve(&self, paint: Paint) -> Option<[u8; 3]> {
        match paint {
            Paint::None => None,
            Paint::Color(rgb) => Some(rgb),
            Paint::Current => self.color,
        }
    }
}

/// The rules of `sheet` matching `node`, lowest precedence first.
fn matching<'s>(node: roxmltree::Node<'_, '_>, sheet: &'s Sheet) -> Vec<&'s Rule> {
    let mut ranked: Vec<_> = (sheet.rules.iter().enumerate())
        .filter_map(|(i, rule)| Some((rule.specificity_for(node)?, i, rule)))
        .collect();
    ranked.sort_by_key(|&(spec, i, _)| (spec, i));
    ranked.into_iter().map(|(.., rule)| rule).collect()
}

/// `node`'s own highest-precedence declared value of `prop` (from `style`,
/// `sheet` rules or the presentation attribute), unvalidated and trimmed;
/// `None` when it declares none (LCV-179 text properties).
pub(super) fn declared(node: roxmltree::Node<'_, '_>, sheet: &Sheet, prop: &str) -> Option<String> {
    let rules = matching(node, sheet);
    candidates(node, &rules, prop)
        .first()
        .map(|v| v.trim().to_owned())
}

/// `prop`'s declared values for `node`, highest precedence first (module
/// docs); `rules` are the matching rules, lowest precedence first.
fn candidates<'a>(node: roxmltree::Node<'a, '_>, rules: &[&'a Rule], prop: &str) -> Vec<&'a str> {
    let named = |d: &Decl<'_>| d.name.eq_ignore_ascii_case(prop);
    let styled: Vec<_> = (node.attribute("style").into_iter())
        .flat_map(declarations)
        .filter(named)
        .collect();
    let ruled: Vec<_> = (rules.iter())
        .flat_map(|rule| declarations(&rule.body))
        .filter(named)
        .collect();
    let mut out = Vec::new();
    for important in [true, false] {
        let tier = |d: &&Decl<'a>| d.important == important;
        out.extend(styled.iter().rev().filter(tier).map(|d| d.value));
        out.extend(ruled.iter().rev().filter(tier).map(|d| d.value));
    }
    out.extend(node.attribute(prop));
    out
}

/// The first candidate `parse` accepts (`inherit`/`unset` = `parent`),
/// calling `invalid` for each one it refuses; `parent` when none applies.
fn cascade<T: Copy>(
    values: &[&str],
    parent: T,
    parse: impl Fn(&str) -> Option<T>,
    mut invalid: impl FnMut(),
) -> T {
    for value in values.iter().map(|v| v.trim()) {
        if value.eq_ignore_ascii_case("inherit") || value.eq_ignore_ascii_case("unset") {
            return parent;
        }
        match parse(value) {
            Some(t) => return t,
            None => invalid(),
        }
    }
    parent
}

/// A `stroke`/`fill` value; `None` for an unsupported color.
fn paint(value: &str) -> Option<Paint> {
    match value.to_ascii_lowercase().as_str() {
        "none" | "initial" => Some(Paint::None),
        "currentcolor" => Some(Paint::Current),
        _ => parse_css_color(value).map(Paint::Color),
    }
}

/// Every SVG `<style>` of the document, in document order, as one sheet;
/// what each drops is noted in `report`.
pub(super) fn collect_sheet(root: roxmltree::Node<'_, '_>, report: &mut Report) -> Sheet {
    let mut sheet = Sheet::default();
    let styles = root.descendants().filter(|n| {
        n.is_element() && n.tag_name().name() == "style" && n.tag_name().namespace() == Some(SVG_NS)
    });
    for node in styles {
        let text: String = node
            .children()
            .filter_map(|c| c.is_text().then(|| c.text()).flatten())
            .collect();
        let part = parse_sheet(&text);
        for note in &part.notes {
            report.note(note);
        }
        sheet.rules.extend(part.rules);
    }
    sheet
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
