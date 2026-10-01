//! SVG 2 ch. 5 structure reuse on import (LCV-178): the id index, `<use>`
//! reference resolution, cycle detection and the instance context.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`. Nothing here
//! reads a file or the network: only same-document `#id` references resolve.

use super::style::Style;
use super::walk::Walk;
use super::{SVG_NS, SvgImportError};
use crate::document::LayerId;
use crate::io::svg::length::{parse_length, to_user};
use crate::io::svg::matrix::Matrix;
use crate::io::svg::viewport::{Ctx, par, parse_view_box, view_box_map};
use std::collections::HashMap;

type Node<'a, 'input> = roxmltree::Node<'a, 'input>;

/// The report label of a `<use>` whose reference does not resolve (AC 8).
const UNRESOLVED: &str = "use (unresolved)";

/// The report label of a `<use>` that would re-enter itself (AC 7).
const CYCLE: &str = "use (cycle)";

/// The deepest `<use>` nesting imported (AC 9).
const MAX_DEPTH: usize = 32;

/// The most entities `<use>` instances may create in one file (AC 9).
const MAX_INSTANCED: usize = 100_000;

/// The deprecated XLink namespace of `xlink:href`.
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

/// The SVG elements of one document by `id`, the first in document order
/// winning (browser behaviour).
pub(super) struct Index<'a, 'input> {
    ids: HashMap<&'a str, Node<'a, 'input>>,
}

impl<'a, 'input> Index<'a, 'input> {
    /// Index every SVG-namespace element at or below `root` that has an `id`.
    pub(super) fn build(root: Node<'a, 'input>) -> Self {
        let mut ids = HashMap::new();
        let svg = root
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().namespace() == Some(SVG_NS));
        for node in svg {
            if let Some(id) = node.attribute("id") {
                ids.entry(id).or_insert(node);
            }
        }
        Self { ids }
    }
}

impl<'a, 'input> Walk<'a, 'input> {
    /// Import the instance `use_` places, given its context `ctx` (its own
    /// `transform` composed) and computed style `style`, on `layer`, the one
    /// the `<use>` lands on (AC 5). A `symbol` or `svg` target walks its
    /// children in its viewport; any other is imported as in place, so a
    /// nested `<use>` recurses (AC 6). Nesting deeper than [`MAX_DEPTH`] or
    /// more than [`MAX_INSTANCED`] instanced elements refuse the file
    /// (AC 9); each expansion counts as one, so a fan-out that draws
    /// nothing is bounded too.
    pub(super) fn expand(
        &mut self,
        use_: Node<'a, 'input>,
        layer: Option<LayerId>,
        ctx: &Ctx,
        style: &Style,
    ) -> Result<(), SvgImportError> {
        self.instanced += 1;
        if self.instanced > MAX_INSTANCED {
            return Err(SvgImportError::LimitExceeded("100000 instanced elements"));
        }
        let Some(target) = target(use_, &self.index) else {
            self.report.note(UNRESOLVED);
            return Ok(());
        };
        if is_cycle(target, use_, &self.uses) {
            self.report.note(CYCLE);
            return Ok(());
        }
        let Some(inner) = instance_ctx(use_, target, ctx) else {
            return Ok(());
        };
        if self.uses.len() >= MAX_DEPTH {
            return Err(SvgImportError::LimitExceeded("use nesting depth 32"));
        }
        self.uses.push(use_);
        let done = match target.tag_name().name() {
            "symbol" | "svg" => {
                let style = style.child(target, &self.sheet, &mut self.report);
                self.collect(target, layer, &inner, &style)
            }
            _ => self.element(target, layer, &inner, style),
        };
        self.uses.pop();
        done?;
        match self.instanced > MAX_INSTANCED {
            true => Err(SvgImportError::LimitExceeded("100000 instanced elements")),
            false => Ok(()),
        }
    }

    /// `LayerReader::enter`, except inside an instance, which stays on the
    /// `<use>`'s layer (AC 5): a referenced `<g data-layer>` declares no
    /// layer again.
    pub(super) fn enter(&mut self, node: Node<'_, '_>) -> Result<Option<LayerId>, SvgImportError> {
        match self.uses.is_empty() {
            true => self.layers.enter(node),
            false => Ok(None),
        }
    }
}

/// The element `use_` references: `href`, else `xlink:href` (AC 2), which
/// must be `#` and a non-empty indexed id once trimmed. `None` when it is
/// unresolved (AC 8); nothing is ever fetched.
pub(super) fn target<'a, 'input>(
    use_: Node<'a, 'input>,
    index: &Index<'a, 'input>,
) -> Option<Node<'a, 'input>> {
    let href = use_
        .attribute("href")
        .or_else(|| use_.attribute((XLINK_NS, "href")))?;
    let id = href.trim().strip_prefix('#').filter(|id| !id.is_empty())?;
    index.ids.get(id).copied()
}

/// Whether expanding `target` for `use_` re-enters an element already being
/// expanded (AC 7): `target` is `use_` itself, contains it, or contains a
/// `<use>` on the expansion `stack` (`ancestors` includes self).
pub(super) fn is_cycle(target: Node<'_, '_>, use_: Node<'_, '_>, stack: &[Node<'_, '_>]) -> bool {
    let mut uses = stack.iter().chain([&use_]);
    uses.any(|u| u.ancestors().any(|a| a == target))
}

/// The context of an instance of `target` placed by `use_`, given `ctx`
/// (the `<use>`'s, its own `transform` already composed): then
/// `translate(x, y)` (AC 1); for a `symbol` or `svg` target, its `viewBox`
/// mapped per its `preserveAspectRatio` onto `width`/`height` of the
/// `<use>`, else the target's, else 100% (AC 3). `None` when the instance
/// renders nothing. The target's own `x`/`y` place that viewport; nothing
/// is clipped.
pub(super) fn instance_ctx(use_: Node<'_, '_>, target: Node<'_, '_>, ctx: &Ctx) -> Option<Ctx> {
    let [pw, ph] = ctx.viewport;
    let len = |node: Node<'_, '_>, attr: &str, reference: f64| {
        node.attribute(attr)
            .and_then(parse_length)
            .map(|l| to_user(l, reference))
    };
    let at = |attr, reference| len(use_, attr, reference).unwrap_or(0.0);
    let ctm = Matrix::translate(at("x", pw), at("y", ph)).then(ctx.ctm);
    let placed = Ctx { ctm, ..*ctx };
    if !matches!(target.tag_name().name(), "symbol" | "svg") {
        return Some(placed);
    }
    let size = |attr, reference| {
        len(use_, attr, reference)
            .or_else(|| len(target, attr, reference))
            .unwrap_or(reference)
    };
    let rect = [
        len(target, "x", pw).unwrap_or(0.0),
        len(target, "y", ph).unwrap_or(0.0),
        size("width", pw),
        size("height", ph),
    ];
    if !(rect[2] > 0.0 && rect[3] > 0.0) {
        return None;
    }
    let (map, viewport) = match target.attribute("viewBox").and_then(parse_view_box) {
        Some(vb) => {
            let par = par(target.attribute("preserveAspectRatio"));
            (view_box_map(vb, rect, par), [vb[2], vb[3]])
        }
        None => (Matrix::translate(rect[0], rect[1]), [rect[2], rect[3]]),
    };
    let ctm = map.then(placed.ctm);
    (!ctm.is_singular()).then_some(Ctx { ctm, viewport })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;
    use crate::io::svg::matrix::parse_transform;

    const DOC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
      <defs>
        <line id="a" x2="1"/>
        <circle id="a" r="1"/>
        <g id="b"><use id="ub" href="#c"/></g>
        <g id="c"><use id="uc" href="#b"/></g>
        <g id="self"><use id="us" href="#self"/></g>
        <g id="outer"><g><use id="deep" href="#outer"/></g></g>
        <x:g xmlns:x="urn:x" id="foreign"/>
        <g id=""/>
      </defs>
      <use id="u1" href="#a" xlink:href="#self"/>
      <use id="u2" xlink:href="#b"/>
      <use id="u3"/>
      <use id="u4" href="#nope"/>
      <use id="u5" href="other.svg#a"/>
      <use id="u6" href="http://x/#a"/>
      <use id="u7" href="#"/>
      <use id="u8" href="#foreign"/>
      <use id="u9" href=" #b "/>
    </svg>"##;

    /// The element with `id` (first occurrence) in `doc`.
    fn by_id<'a, 'i>(doc: &'a roxmltree::Document<'i>, id: &str) -> Node<'a, 'i> {
        doc.descendants()
            .find(|n| n.attribute("id") == Some(id))
            .expect("test id")
    }

    /// AC 8 — the first element with an id wins; non-SVG elements are not
    /// indexed.
    #[test]
    fn index_keeps_the_first_svg_element_per_id() {
        let doc = roxmltree::Document::parse(DOC).expect("test XML");
        let index = Index::build(doc.root_element());
        let a = target(by_id(&doc, "u1"), &index).expect("resolves");
        assert_eq!(a.tag_name().name(), "line");
        assert_eq!(target(by_id(&doc, "u8"), &index), None);
    }

    /// AC 2 — `href` beats `xlink:href`; `xlink:href` alone resolves;
    /// surrounding blanks are trimmed.
    #[test]
    fn href_beats_xlink_href() {
        let doc = roxmltree::Document::parse(DOC).expect("test XML");
        let index = Index::build(doc.root_element());
        assert_eq!(target(by_id(&doc, "u1"), &index), Some(by_id(&doc, "a")));
        assert_eq!(target(by_id(&doc, "u2"), &index), Some(by_id(&doc, "b")));
        assert_eq!(target(by_id(&doc, "u9"), &index), Some(by_id(&doc, "b")));
    }

    /// AC 8 — no href, an unknown id, another file, a URL or a bare `#`
    /// are unresolved.
    #[test]
    fn missing_unknown_and_external_references_are_unresolved() {
        let doc = roxmltree::Document::parse(DOC).expect("test XML");
        let index = Index::build(doc.root_element());
        for id in ["u3", "u4", "u5", "u6", "u7"] {
            assert_eq!(target(by_id(&doc, id), &index), None, "{id}");
        }
    }

    /// AC 7 — a target that contains the `<use>` (self or ancestor), or any
    /// `<use>` on the expansion stack, is a cycle; an unrelated one is not.
    #[test]
    fn self_ancestor_and_stacked_cycles_are_detected() {
        let doc = roxmltree::Document::parse(DOC).expect("test XML");
        let n = |id| by_id(&doc, id);
        assert!(is_cycle(n("self"), n("us"), &[]));
        assert!(is_cycle(n("outer"), n("deep"), &[]));
        // u2 → b ∋ ub → c ∋ uc → b: b contains ub, which is on the stack.
        assert!(!is_cycle(n("b"), n("u2"), &[]));
        assert!(!is_cycle(n("c"), n("ub"), &[n("u2")]));
        assert!(is_cycle(n("b"), n("uc"), &[n("u2"), n("ub")]));
        assert!(!is_cycle(n("a"), n("u1"), &[n("u2"), n("ub")]));
    }

    /// Where `instance_ctx` of `<use {use_attrs}/>` on `<{target}/>` sends
    /// each of `points`, in a 100 × 100 viewport under `transform`.
    fn placed(transform: &str, use_attrs: &str, target: &str, points: &[(f64, f64)]) -> Vec<Vec2> {
        let src = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><use {use_attrs}/><{target}/></svg>"#
        );
        let doc = roxmltree::Document::parse(&src).expect("test XML");
        let mut kids = doc.root_element().children().filter(|n| n.is_element());
        let (use_, target) = (kids.next().expect("use"), kids.next().expect("target"));
        let ctx = Ctx {
            ctm: parse_transform(transform).unwrap_or(Matrix::IDENTITY),
            viewport: [100.0, 100.0],
        };
        let inner = instance_ctx(use_, target, &ctx).expect("renders");
        points
            .iter()
            .map(|&(x, y)| inner.ctm.apply(Vec2::new(x, y)))
            .collect()
    }

    fn near(got: &[Vec2], want: &[(f64, f64)]) -> bool {
        got.len() == want.len()
            && (got.iter().zip(want)).all(|(g, &(x, y))| g.approx_eq(Vec2::new(x, y), 1e-9))
    }

    /// AC 1 — the `<use>`'s transform, then `translate(x, y)`; a non-symbol
    /// target ignores `width`/`height`.
    #[test]
    fn transform_then_translate() {
        let got = placed(
            "rotate(90)",
            r#"x="10" y="5" width="3""#,
            "line",
            &[(0.0, 0.0), (1.0, 0.0)],
        );
        assert!(near(&got, &[(-5.0, 10.0), (-5.0, 11.0)]), "{got:?}");
    }

    /// AC 3 — a symbol's viewBox into the `<use>`'s `width`/`height`,
    /// `xMidYMid meet` by default, after the translate.
    #[test]
    fn symbol_view_box_meets_the_use_viewport() {
        let sym = r#"symbol viewBox="0 0 10 10""#;
        let got = placed(
            "",
            r#"x="1" width="20" height="40""#,
            sym,
            &[(0.0, 0.0), (10.0, 10.0)],
        );
        assert!(near(&got, &[(1.0, 10.0), (21.0, 30.0)]), "{got:?}");
        let none = r#"symbol viewBox="0 0 10 10" preserveAspectRatio="none""#;
        let got = placed("", r#"width="20" height="40""#, none, &[(10.0, 10.0)]);
        assert!(near(&got, &[(20.0, 40.0)]), "{got:?}");
    }

    /// AC 3 — without `width`/`height` on the `<use>`, the symbol's own,
    /// else 100% of the viewport.
    #[test]
    fn symbol_size_falls_back_to_its_own_then_100_percent() {
        let own = r#"symbol viewBox="0 0 10 10" width="5" height="5""#;
        let got = placed("", "", own, &[(10.0, 10.0)]);
        assert!(near(&got, &[(5.0, 5.0)]), "{got:?}");
        let got = placed(
            "",
            r#"height="50%""#,
            r#"symbol viewBox="0 0 10 10""#,
            &[(10.0, 10.0)],
        );
        assert!(near(&got, &[(75.0, 50.0)]), "{got:?}");
        let got = placed("", "", r#"symbol viewBox="0 0 10 10""#, &[(10.0, 10.0)]);
        assert!(near(&got, &[(100.0, 100.0)]), "{got:?}");
    }
}
