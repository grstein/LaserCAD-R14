//! SVG 2 ch. 5 structure reuse on import (LCV-178): the id index, `<use>`
//! reference resolution, cycle detection and the instance context.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`. Nothing here
//! reads a file or the network: only same-document `#id` references resolve.

use super::SVG_NS;
use std::collections::HashMap;

type Node<'a, 'input> = roxmltree::Node<'a, 'input>;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
