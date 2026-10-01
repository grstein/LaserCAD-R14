//! The CSS subset of SVG import (LCV-175): `<style>` sheets and `style`
//! declarations (SVG 2 ch. 6).
//!
//! A sheet keeps rules whose selector list is made only of compounds of a
//! type or `*`, `.class` and `#id`; a rule with a combinator, pseudo-class or
//! attribute selector is dropped and noted. At-rules are skipped, never
//! fetched, and noted. Comments are stripped.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

/// One `name: value [!important]` declaration, trimmed; the name keeps its
/// case (callers compare ASCII case-insensitively).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Decl<'a> {
    /// The property name, e.g. `stroke`.
    pub(super) name: &'a str,
    /// The value, `!important` removed.
    pub(super) value: &'a str,
    /// Whether the value ended in `!important`.
    pub(super) important: bool,
}

/// The declarations of a `style` attribute or a rule body, in written order.
pub(super) fn declarations(text: &str) -> impl DoubleEndedIterator<Item = Decl<'_>> {
    text.split(';').filter_map(|_| None)
}

/// A selector's weight: `(ids, classes, types)`, compared in that order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Specificity(pub(super) u32, pub(super) u32, pub(super) u32);

/// A compound selector: an optional type (`None` = `*`), ids and classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Selector {
    tag: Option<String>,
    ids: Vec<String>,
    classes: Vec<String>,
}

impl Selector {
    /// This selector's [`Specificity`].
    pub(super) fn specificity(&self) -> Specificity {
        Specificity::default()
    }

    /// Whether `node` matches: local name, `id` and `class` attributes.
    pub(super) fn matches(&self, _node: roxmltree::Node<'_, '_>) -> bool {
        false
    }
}

/// One kept rule: its selector list and its declaration block text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Rule {
    /// The comma list, every item supported.
    pub(super) selectors: Vec<Selector>,
    /// The text between the braces, for [`declarations`].
    pub(super) body: String,
}

impl Rule {
    /// The highest specificity among the selectors matching `node`.
    pub(super) fn specificity_for(&self, node: roxmltree::Node<'_, '_>) -> Option<Specificity> {
        self.selectors
            .iter()
            .filter(|s| s.matches(node))
            .map(Selector::specificity)
            .max()
    }
}

/// A parsed style sheet: kept rules in order, and report labels for what was
/// dropped (`style @x`, `style rule (unsupported selector)`,
/// `style rule (malformed)`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sheet {
    /// Kept rules, in document order (a later rule wins a tie).
    pub(super) rules: Vec<Rule>,
    /// One label per dropped rule or skipped at-rule.
    pub(super) notes: Vec<String>,
}

/// Parse one `<style>` element's text.
pub(super) fn parse_sheet(_text: &str) -> Sheet {
    Sheet::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNSUPPORTED: &str = "style rule (unsupported selector)";

    fn specificities(text: &str) -> Vec<Vec<Specificity>> {
        let sheet = parse_sheet(text);
        assert!(sheet.notes.is_empty(), "{text}: {:?}", sheet.notes);
        sheet
            .rules
            .iter()
            .map(|r| r.selectors.iter().map(Selector::specificity).collect())
            .collect()
    }

    /// AC 3 — type, `*`, `.a`, `#b`, compounds and a comma list are kept
    /// with their specificity.
    #[test]
    fn supported_selectors_parse_with_their_specificity() {
        let cases: [(&str, Vec<Specificity>); 7] = [
            ("line{stroke:red}", vec![Specificity(0, 0, 1)]),
            ("*{stroke:red}", vec![Specificity(0, 0, 0)]),
            (".a{stroke:red}", vec![Specificity(0, 1, 0)]),
            ("#b{stroke:red}", vec![Specificity(1, 0, 0)]),
            ("g.a#b{stroke:red}", vec![Specificity(1, 1, 1)]),
            ("*.a.b-2_c{}", vec![Specificity(0, 2, 0)]),
            (
                " line , .a,#b {stroke:red}",
                vec![
                    Specificity(0, 0, 1),
                    Specificity(0, 1, 0),
                    Specificity(1, 0, 0),
                ],
            ),
        ];
        for (text, want) in cases {
            assert_eq!(specificities(text), vec![want], "{text}");
        }
    }

    /// AC 3 — a combinator, pseudo-class or attribute selector drops the
    /// whole rule (comma list included) and is noted; later rules survive.
    #[test]
    fn unsupported_selectors_drop_the_rule() {
        for bad in [
            "g path",
            "g>path",
            "g + path",
            "a ~ b",
            "a:hover",
            "[x]",
            "line, g path",
            "",
        ] {
            let sheet = parse_sheet(&format!("{bad}{{stroke:red}} .ok{{stroke:blue}}"));
            assert_eq!(sheet.notes, [UNSUPPORTED], "{bad}");
            assert_eq!(sheet.rules.len(), 1, "{bad}");
            assert_eq!(sheet.rules[0].body, "stroke:blue", "{bad}");
        }
    }

    /// AC 4 — at-rules are skipped (statement or block) and noted by name.
    #[test]
    fn at_rules_are_skipped_and_noted() {
        let text = "@import url(x.css); line{stroke:red} \
            @MEDIA print { line { stroke:blue } .a { b:c } } @font-face{x:y} .a{stroke:green}";
        let sheet = parse_sheet(text);
        assert_eq!(
            sheet.notes,
            ["style @import", "style @media", "style @font-face"]
        );
        let bodies: Vec<&str> = sheet.rules.iter().map(|r| r.body.trim()).collect();
        assert_eq!(bodies, ["stroke:red", "stroke:green"]);
    }

    /// Comments are stripped, inside selectors and bodies too.
    #[test]
    fn comments_are_stripped() {
        let sheet = parse_sheet("/* .x{stroke:red} */ line/* c */{stroke:/* d */red}");
        assert!(sheet.notes.is_empty(), "{:?}", sheet.notes);
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors.len(), 1);
        let decls: Vec<_> = declarations(&sheet.rules[0].body).collect();
        assert_eq!(decls.len(), 1);
        assert_eq!((decls[0].name, decls[0].value), ("stroke", "red"));
    }

    /// An unterminated block ends the sheet and is noted.
    #[test]
    fn an_unterminated_block_is_malformed() {
        let sheet = parse_sheet(".a{stroke:red} line{stroke:blue");
        assert_eq!(sheet.notes, ["style rule (malformed)"]);
        assert_eq!(sheet.rules.len(), 1);
    }

    /// `!important` (any case) is split off; names keep their case.
    #[test]
    fn declarations_split_important() {
        let got: Vec<_> =
            declarations(" stroke: red ; FILL:blue !important;color:green!IMPORTANT;;x").collect();
        let want = [
            Decl {
                name: "stroke",
                value: "red",
                important: false,
            },
            Decl {
                name: "FILL",
                value: "blue",
                important: true,
            },
            Decl {
                name: "color",
                value: "green",
                important: true,
            },
        ];
        assert_eq!(got, want);
    }

    /// Compounds match by local name, `id` and every class.
    #[test]
    fn selectors_match_name_id_and_classes() {
        let xml = r#"<svg xmlns="http://www.w3.org/2000/svg"><g id="b" class="x a"/><g class="a"/><line/></svg>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let nodes: Vec<_> = doc.root_element().children().collect();
        let sheet = parse_sheet("g.a#b{} .a{} *{} line{} .a.y{}");
        let matched = |i: usize| -> Vec<bool> {
            sheet
                .rules
                .iter()
                .map(|r| r.selectors[0].matches(nodes[i]))
                .collect()
        };
        assert_eq!(matched(0), [true, true, true, false, false]);
        assert_eq!(matched(1), [false, true, true, false, false]);
        assert_eq!(matched(2), [false, false, true, true, false]);
    }
}
