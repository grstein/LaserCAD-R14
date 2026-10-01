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
    text.split(';').filter_map(|decl| {
        let (name, value) = decl.split_once(':')?;
        let value = value.trim();
        let cut = value.len().checked_sub(IMPORTANT.len());
        let bang = cut
            .filter(|&i| value.is_char_boundary(i) && value[i..].eq_ignore_ascii_case(IMPORTANT));
        let (value, important) = match bang {
            Some(i) => (value[..i].trim(), true),
            None => (value, false),
        };
        Some(Decl {
            name: name.trim(),
            value,
            important,
        })
    })
}

/// The suffix that marks an important declaration.
const IMPORTANT: &str = "!important";

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
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Specificity(
            count(self.ids.len()),
            count(self.classes.len()),
            u32::from(self.tag.is_some()),
        )
    }

    /// Whether `node` matches: local name, `id` and `class` attributes.
    pub(super) fn matches(&self, node: roxmltree::Node<'_, '_>) -> bool {
        let tag_ok = self
            .tag
            .as_deref()
            .is_none_or(|t| t == node.tag_name().name());
        let id = node.attribute("id");
        let classes = node
            .attribute("class")
            .unwrap_or("")
            .split_ascii_whitespace();
        tag_ok
            && self.ids.iter().all(|i| id == Some(i.as_str()))
            && self.classes.iter().all(|c| classes.clone().any(|k| k == c))
    }

    /// One comma-list item: `type|*` then `.class`/`#id`, nothing else.
    fn parse(item: &str) -> Option<Self> {
        let mut sel = Self {
            tag: None,
            ids: Vec::new(),
            classes: Vec::new(),
        };
        let mut rest = item.trim();
        if rest.is_empty() {
            return None;
        }
        if let Some(r) = rest.strip_prefix('*') {
            rest = r;
        } else {
            let (tag, r) = ident(rest);
            sel.tag = Some(tag.to_owned()).filter(|t| !t.is_empty());
            rest = r;
        }
        while !rest.is_empty() {
            let (name, r) = ident(rest.get(1..)?);
            if name.is_empty() {
                return None;
            }
            match rest.as_bytes()[0] {
                b'.' => sel.classes.push(name.to_owned()),
                b'#' => sel.ids.push(name.to_owned()),
                _ => return None,
            }
            rest = r;
        }
        Some(sel)
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
pub(super) fn parse_sheet(text: &str) -> Sheet {
    let text = strip_comments(text);
    let mut sheet = Sheet::default();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        if let Some(at) = rest.strip_prefix('@') {
            let (name, _) = ident(at);
            sheet
                .notes
                .push(format!("style @{}", name.to_ascii_lowercase()));
            let end = match at.find([';', '{']) {
                Some(i) if at.as_bytes()[i] == b'{' => block_end(&at[i + 1..]).map(|e| i + 1 + e),
                other => other,
            };
            rest = end.map_or("", |e| &at[e + 1..]);
        } else {
            let open = rest.find('{');
            let close = open.and_then(|o| Some(o + 1 + block_end(&rest[o + 1..])?));
            let (Some(open), Some(close)) = (open, close) else {
                sheet.notes.push(MALFORMED.to_owned());
                break;
            };
            let selectors: Option<Vec<Selector>> =
                rest[..open].split(',').map(Selector::parse).collect();
            match selectors {
                Some(selectors) => sheet.rules.push(Rule {
                    selectors,
                    body: rest[open + 1..close].to_owned(),
                }),
                None => sheet.notes.push(UNSUPPORTED_SELECTOR.to_owned()),
            }
            rest = &rest[close + 1..];
        }
        rest = rest.trim_start();
    }
    sheet
}

/// Report label of a rule whose selector list has an unsupported item.
const UNSUPPORTED_SELECTOR: &str = "style rule (unsupported selector)";

/// Report label of a rule left open at the end of the sheet.
const MALFORMED: &str = "style rule (malformed)";

/// `text` with every `/* … */` replaced by a space; an unterminated comment
/// runs to the end.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        out.push(' ');
        rest = rest[start + 2..]
            .find("*/")
            .map_or("", |e| &rest[start + 2 + e + 2..]);
    }
    out.push_str(rest);
    out
}

/// The index of the `}` closing a block whose `{` precedes `text`, nested
/// blocks skipped; `None` when it never closes.
fn block_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, b) in text.bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' if depth == 0 => return Some(i),
            b'}' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The leading CSS identifier of `text` (ASCII alphanumerics, `-`, `_`,
/// non-ASCII) and the rest.
fn ident(text: &str) -> (&str, &str) {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()))
        .unwrap_or(text.len());
    text.split_at(end)
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
