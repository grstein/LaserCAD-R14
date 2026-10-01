//! SVG 2 §5.8 conditional processing (LCV-178 AC 11): whether an element's
//! `requiredExtensions` and `systemLanguage` let it render.
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

use crate::io::svg::layers::attr as plain_attr;

/// Whether `node`'s conditions pass: a present, non-empty (after trim)
/// `requiredExtensions` fails, since LaserCAD supports no extension;
/// `systemLanguage` passes iff one of its comma-separated tags is `en` or
/// starts with `en-` (ASCII case-insensitive, trimmed), so an empty one
/// fails; `requiredFeatures` is ignored (SVG 2 dropped it).
pub(super) fn passes(node: roxmltree::Node<'_, '_>) -> bool {
    let extensions = plain_attr(node, "requiredExtensions");
    if extensions.is_some_and(|e| !e.trim().is_empty()) {
        return false;
    }
    plain_attr(node, "systemLanguage").is_none_or(|langs| langs.split(',').any(english))
}

/// Whether one language tag is `en` or `en-*`.
fn english(tag: &str) -> bool {
    let tag = tag.trim().as_bytes();
    tag.eq_ignore_ascii_case(b"en") || (tag.len() > 3 && tag[..3].eq_ignore_ascii_case(b"en-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `passes` on `<g {attrs}/>`.
    fn on(attrs: &str) -> bool {
        let src = format!("<g {attrs}/>");
        let doc = roxmltree::Document::parse(&src).expect("test XML");
        passes(doc.root_element())
    }

    /// AC 11 — an empty `requiredExtensions` passes, any extension fails.
    #[test]
    fn required_extensions_fail_unless_empty() {
        assert!(on(""));
        assert!(on(r#"requiredExtensions="""#));
        assert!(on(r#"requiredExtensions="  ""#));
        assert!(!on(r#"requiredExtensions="x""#));
    }

    /// AC 11 — `systemLanguage` matches English only.
    #[test]
    fn system_language_matches_english_only() {
        for ok in ["en", "en-US", "fr, en-GB", " EN ", "de,En-au"] {
            assert!(on(&format!(r#"systemLanguage="{ok}""#)), "{ok}");
        }
        for bad in ["fr", "english", "", " ", "fr-en", "enx"] {
            assert!(!on(&format!(r#"systemLanguage="{bad}""#)), "{bad}");
        }
    }

    /// AC 11 — `requiredFeatures` never decides.
    #[test]
    fn required_features_is_ignored() {
        assert!(on(r#"requiredFeatures="x""#));
        assert!(!on(r#"requiredFeatures="x" systemLanguage="fr""#));
    }
}
