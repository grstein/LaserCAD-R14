//! The import report (LCV-171): what [`super::import_svg`] met but did not
//! turn into geometry, as `(label, count)` entries in order of first
//! occurrence, one entry per label (AC 8).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::io::svg::css;

/// Properties LaserCAD does not apply yet, reported by name whenever an
/// imported or descended element carries one, as an attribute or a `style`
/// declaration (AC 7); `fill:none` is exempt. `transform` is applied since
/// LCV-173, `display` and `visibility` since LCV-175, and no longer listed.
pub(super) const REPORTED_PROPERTIES: [&str; 9] = [
    "fill",
    "clip-path",
    "mask",
    "filter",
    "marker-start",
    "marker-mid",
    "marker-end",
    "stroke-dasharray",
    "opacity",
];

/// `node`'s `style` declarations as trimmed `(property, value)` pairs in
/// written order, a trailing `!important` dropped ([`css::declarations`]).
pub(in crate::io::svg) fn style_decls<'a>(
    node: roxmltree::Node<'a, '_>,
) -> impl DoubleEndedIterator<Item = (&'a str, &'a str)> {
    node.attribute("style")
        .into_iter()
        .flat_map(css::declarations)
        .map(|decl| (decl.name, decl.value))
}

/// Report builder: [`Report::note`] bumps a label's count or appends it.
#[derive(Debug, Default)]
pub(super) struct Report {
    entries: Vec<(String, usize)>,
}

impl Report {
    /// Count one more `label`.
    pub(super) fn note(&mut self, label: &str) {
        match self.entries.iter_mut().find(|(known, _)| known == label) {
            Some((_, count)) => *count += 1,
            None => self.entries.push((label.to_owned(), 1)),
        }
    }

    /// Count each [`REPORTED_PROPERTIES`] entry `node` carries: attributes in
    /// the null namespace by exact name, then `style` declarations ASCII
    /// case-insensitively. `fill` whose value is `none` is not counted.
    pub(super) fn note_properties(&mut self, node: roxmltree::Node<'_, '_>) {
        let attrs = node
            .attributes()
            .filter(|attr| attr.namespace().is_none())
            .filter_map(|attr| {
                let name = attr.name();
                let prop = REPORTED_PROPERTIES.iter().find(|&&p| p == name)?;
                Some((*prop, attr.value()))
            });
        let styled = style_decls(node).filter_map(|(name, value)| {
            let prop = REPORTED_PROPERTIES
                .iter()
                .find(|p| p.eq_ignore_ascii_case(name))?;
            Some((*prop, value))
        });
        for (prop, value) in attrs.chain(styled) {
            if prop != "fill" || !value.trim().eq_ignore_ascii_case("none") {
                self.note(prop);
            }
        }
    }

    /// The entries in order of first occurrence.
    pub(super) fn finish(self) -> Vec<(String, usize)> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC 8 — first-occurrence order, one entry per label.
    #[test]
    fn note_keeps_first_occurrence_order_and_merges_repeats() {
        let mut report = Report::default();
        for label in ["a", "b", "a"] {
            report.note(label);
        }
        let want = vec![("a".to_owned(), 2), ("b".to_owned(), 1)];
        assert_eq!(report.finish(), want);
    }
}
