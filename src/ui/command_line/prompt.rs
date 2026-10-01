//! Split a tool prompt into its verb, request and options (LCV-184 AC 6).
//!
//! Pure string work: the dock paints each part in its own colour. The spans
//! always concatenate back to the prompt, byte for byte.

/// Which part of a prompt a span is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PromptPart {
    /// The leading command word, e.g. `LINE`.
    Verb,
    /// What the tool asks for.
    Request,
    /// A `[Options]` list or a `<default>`.
    Option,
}

/// Split `prompt` into `(part, text)` spans, in order.
///
/// The verb is the leading run of two or more ASCII capitals, ending at a
/// space or `:` (`LINE Specify…`, `TRIM: Click…`). After it, every balanced
/// `[…]` or `<…>` is an option and the rest is request. A prompt without a
/// verb (`Command:`) is one request span; an unclosed bracket stays request.
pub(crate) fn prompt_spans(prompt: &str) -> Vec<(PromptPart, &str)> {
    let caps = prompt.len()
        - prompt
            .trim_start_matches(|c: char| c.is_ascii_uppercase())
            .len();
    let ends_verb = matches!(prompt.as_bytes().get(caps), Some(b' ' | b':'));
    if caps < 2 || !ends_verb {
        return request(prompt);
    }
    let mut spans = vec![(PromptPart::Verb, &prompt[..caps])];
    let mut rest = &prompt[caps..];
    while let Some((open, close)) = next_option(rest) {
        spans.extend(request(&rest[..open]));
        spans.push((PromptPart::Option, &rest[open..=close]));
        rest = &rest[close + 1..];
    }
    spans.extend(request(rest));
    spans
}

/// `text` as one request span, or none when it is empty.
fn request(text: &str) -> Vec<(PromptPart, &str)> {
    match text {
        "" => Vec::new(),
        _ => vec![(PromptPart::Request, text)],
    }
}

/// Byte offsets of the first balanced `[…]` or `<…>` in `text`.
fn next_option(text: &str) -> Option<(usize, usize)> {
    text.char_indices().find_map(|(open, c)| {
        let close = match c {
            '[' => ']',
            '<' => '>',
            _ => return None,
        };
        text[open..].find(close).map(|len| (open, open + len))
    })
}

#[cfg(test)]
mod tests {
    use super::PromptPart::{Option as Opt, Request as Req, Verb};
    use super::*;

    fn check(prompt: &str, expected: &[(PromptPart, &str)]) {
        let spans = prompt_spans(prompt);
        assert_eq!(spans, expected, "{prompt:?}");
        let joined: String = spans.iter().map(|s| s.1).collect();
        assert_eq!(joined, prompt, "lossless");
    }

    #[test]
    fn a_verb_then_a_request() {
        check(
            "LINE Specify first point:",
            &[(Verb, "LINE"), (Req, " Specify first point:")],
        );
    }

    #[test]
    fn options_and_default_are_options() {
        check(
            "MIRROR Erase source objects? [Yes/No] <N>:",
            &[
                (Verb, "MIRROR"),
                (Req, " Erase source objects? "),
                (Opt, "[Yes/No]"),
                (Req, " "),
                (Opt, "<N>"),
                (Req, ":"),
            ],
        );
        check(
            "TEXT Specify height <5>:",
            &[
                (Verb, "TEXT"),
                (Req, " Specify height "),
                (Opt, "<5>"),
                (Req, ":"),
            ],
        );
    }

    #[test]
    fn a_verb_may_end_at_a_colon() {
        check(
            "TRIM: Click on a segment to trim",
            &[(Verb, "TRIM"), (Req, ": Click on a segment to trim")],
        );
    }

    #[test]
    fn no_verb_is_all_request() {
        check("Command:", &[(Req, "Command:")]);
        check("A point", &[(Req, "A point")]);
        check("LINE", &[(Req, "LINE")]);
        check("", &[]);
    }

    #[test]
    fn an_unbalanced_bracket_stays_request() {
        check("MIRROR Pick [Yes", &[(Verb, "MIRROR"), (Req, " Pick [Yes")]);
        check(
            "MIRROR Pick <N [Yes/No]",
            &[(Verb, "MIRROR"), (Req, " Pick <N "), (Opt, "[Yes/No]")],
        );
    }
}
