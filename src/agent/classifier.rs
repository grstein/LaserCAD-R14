//! LCV-124 — where one submitted command line goes: the CAD grammar, or the
//! agent (ADR 0007 §D9).
//!
//! [`classify`] is a pure function of two things: the raw text and whether an
//! API key is configured. It reads no `App`, opens no panel, spawns no thread
//! and sends nothing — the caller (`src/app/cmdline.rs::submit`) turns its
//! answer into effects. That split is what makes the precedence testable as a
//! table with no UI context at all.
//!
//! **Raw-input mode is not in this file, on purpose.** ADR 0007 §D9 rule 1 —
//! "raw means raw" (LCV-112 decision 1) — is enforced at the call site, which
//! returns *before* `classify` is reached while the active tool wants the
//! command line as a free-text field. Moving the call above that early return
//! posts the string an operator is typing into a TEXT entity to a language
//! model; `tests/it/cmdline/agent_routing.rs` guards it by name.
//!
//! Purity: this file's implementation imports nothing but `std`. `crate::cmdline`
//! is a permitted import (ADR 0003 §A2a) that, since LCV-148, lives only in
//! `#[cfg(test)] mod tests`. No `egui`, no `eframe`, no `rfd`, no `reqwest`, no
//! `crate::app` — asserted by [`tests::classifier_is_kernel_pure`].

/// The `:` escape hatch: everything after the first colon is the prompt.
const COLON_PREFIX: char = ':';

/// The long-form escape hatch, matched case-insensitively and only when what
/// follows it is a space or the end of the line (so `/aim` is not a prefix).
const AI_PREFIX: &str = "/ai";

/// Where one submitted command line goes (ADR 0007 §D9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// The command line's own grammar owns this line. Dispatch it exactly as
    /// it was dispatched before the agent existed — including the
    /// `Unknown command: "…"` message, which is what an unrecognised line
    /// gets unconditionally (ADR 0007 §D9 rule 4, amendment (5)).
    Cad,
    /// Send the payload to the agent as a prompt. A `:` or `/ai` prefix is
    /// now its only producer (ADR 0007 §D9 rule 4, amendment (5)).
    ///
    /// The payload is already trimmed. An **empty** payload is a prefix with
    /// nothing behind it (`":"`, `"/ai   "`): it is a refusal for the caller
    /// to report, never a prompt to send.
    Agent(String),
    /// The line asked for the agent by name, but no API key is configured.
    Unavailable,
}

/// Decide where `raw` goes. Precedence is ADR 0007 §D9, highest first:
///
/// 1. **raw-input mode wins over everything** — enforced by the caller, which
///    returns before this function is reached (see the module header).
/// 2. a `:` or `/ai` prefix is **absolute**: `":line"`, `":50,25"` and
///    `":snap"` all go to the agent, even though the remainder would parse as
///    a CAD command. This is the escape hatch and nothing overrides it.
/// 3. everything else is **CAD, always** (this collapses the old rules 3 and
///    4 into one — amendment (5)): whatever `agent_available` says, an
///    unprefixed line never reaches the agent. The grammar's own verdict,
///    recognised or `Unknown command: "…"`, is all it ever gets, and
///    `classify` does not consult the grammar to know this.
///
/// `agent_available` is data, not a global: the caller computes it as
/// "a non-whitespace API key is configured" and passes it in; it still
/// separates [`Route::Agent`] from [`Route::Unavailable`] on a prefixed line.
pub fn classify(raw: &str, agent_available: bool) -> Route {
    match agent_prompt(raw.trim()) {
        // An empty prompt is refused before availability is even consulted:
        // `:` with nothing behind it is a malformed line whatever the settings
        // say, and the caller answers it without reaching the network.
        Some("") => Route::Agent(String::new()),
        Some(_) if !agent_available => Route::Unavailable,
        Some(prompt) => Route::Agent(prompt.to_owned()),
        // Rule 4 (amendment (5)): no prefix is always CAD, whatever
        // `agent_available` says.
        None => Route::Cad,
    }
}

/// The prompt a prefixed line carries, or `None` when `trimmed` has no prefix.
///
/// `trimmed` must already be trimmed — the prefix is matched on the trimmed
/// line (AC 3), and the remainder is trimmed again before it becomes a prompt.
fn agent_prompt(trimmed: &str) -> Option<&str> {
    if let Some(rest) = trimmed.strip_prefix(COLON_PREFIX) {
        // Everything after the *first* colon, so `":a:b"` prompts `a:b`.
        return Some(rest.trim());
    }
    // `get` rather than a slice: a line whose third byte falls inside a
    // multi-byte character must answer `None`, not panic.
    let head = trimmed.get(..AI_PREFIX.len())?;
    if !head.eq_ignore_ascii_case(AI_PREFIX) {
        return None;
    }
    let rest = &trimmed[AI_PREFIX.len()..];
    // A word boundary is required, or `/aim` would be a prefix carrying the
    // prompt `m` instead of an unrecognised line.
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // LCV-148: the implementation above no longer calls `parse` or names a
    // `CommandInput` variant — `classify` decides on the prefix alone (ADR
    // 0003 §A2a, amendment (3)). This import survives here, and only here,
    // for `z_is_not_a_zoom_word`, which records what the grammar does with
    // `z`, `ze` and `zoom in` independently of how `classify` routes them.
    use crate::cmdline::{parse, CommandInput};

    /// The implementation section of this file: everything before the bare
    /// `#[cfg(test)]` at column 0. Scanning the whole file would let this test
    /// module's own needles satisfy the scan below.
    fn implementation() -> &'static str {
        let src = include_str!("classifier.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("classifier.rs must have a bare #[cfg(test)] marker to bound the scan");
        &src[..at]
    }

    /// Does any **code** line of `haystack` contain `needle`?
    ///
    /// Comment lines are skipped, exactly as `tests/it/repo/bridge_scans.rs`
    /// does it and for the same reason: this file's header names the crates it
    /// may not import precisely in order to forbid them, and a scan that read
    /// prose as an import would force the rule to go undocumented to stay true.
    fn code_contains(haystack: &str, needle: &str) -> bool {
        haystack
            .lines()
            .any(|line| line.contains(needle) && !line.trim_start().starts_with("//"))
    }

    /// AC 5 (LCV-148) — the classifier is pure and, since the rule-4 flip, no
    /// longer needs the grammar to decide anything. Needles are built with
    /// `concat!` so the scan can never match the literal written next to it,
    /// the haystack stops at the test module, and each absence is backed by a
    /// positive control run through the **same** `code_contains` over the
    /// **same** slice: if the scan were reading the wrong bytes, or skipping
    /// every line, the control fails first. The positive controls are
    /// re-derived from the post-flip implementation section: `crate::cmdline`
    /// and `CommandInput` are no longer in it and must not be asserted as
    /// present (ADR 0003 §A2a, amendment (3)); `crate::cmdline` is **not**
    /// added to the forbidden list — this is not a new ban, the classifier
    /// simply no longer needs the import.
    #[test]
    fn classifier_is_kernel_pure() {
        let implementation = implementation();
        for control in [
            concat!("pub fn ", "classify"),
            concat!("agent_", "available"),
            concat!("pub enum ", "Route"),
        ] {
            assert!(
                code_contains(implementation, control),
                "positive control: the scanned slice must contain {control}"
            );
        }
        for forbidden in [
            concat!("eg", "ui"),
            concat!("ef", "rame"),
            concat!("rf", "d"),
            concat!("req", "west"),
            concat!("crate::", "app"),
            concat!("crate::", "document"),
        ] {
            assert!(
                !code_contains(implementation, forbidden),
                "classifier.rs must not import {forbidden}"
            );
        }
    }

    /// AC 12 — this file stays under the 300 **implementation** LOC cap
    /// (ADR 0004): total lines minus the inline test module.
    #[test]
    fn the_implementation_section_is_under_the_loc_cap() {
        let lines = implementation().lines().count();
        assert!(
            lines <= 300,
            "ADR 0004: classifier.rs has {lines} implementation LOC"
        );
    }

    /// AC 2 (LCV-148) — `classify` keeps its two-argument signature. The
    /// coercion below only type-checks against `fn(&str, bool) -> Route`, so a
    /// change that "simplifies" `classify` to drop `agent_available` fails to
    /// compile here rather than silently losing rule 2's discrimination
    /// between [`Route::Agent`] and [`Route::Unavailable`].
    #[test]
    fn classify_keeps_its_pinned_two_argument_signature() {
        let f: fn(&str, bool) -> Route = classify;
        assert_eq!(f("lien", true), Route::Cad);
    }

    /// AC 1 / AC 7 (LCV-148) — the whole precedence table, every row asserted
    /// with `agent_available` both `true` and `false`, so the flag is
    /// *proved* to matter for exactly the rows it still owns. Since the
    /// rule-4 flip only the five prefixed rows may move with availability;
    /// every unprefixed line — recognised by the grammar or not — is
    /// `Route::Cad` under both.
    #[test]
    fn the_precedence_table_holds_for_both_availabilities() {
        // (line, route when a key is configured, route without one)
        let rows: [(&str, Route, Route); 24] = [
            // Rule 2 — the prefix is absolute, even over a real CAD command.
            (":line", Route::Agent("line".into()), Route::Unavailable),
            ("/ai line", Route::Agent("line".into()), Route::Unavailable),
            ("/AI line", Route::Agent("line".into()), Route::Unavailable),
            (":50,25", Route::Agent("50,25".into()), Route::Unavailable),
            (":snap", Route::Agent("snap".into()), Route::Unavailable),
            // Rule 3 — the grammar wins, with or without a key.
            ("l", Route::Cad, Route::Cad),
            ("snap", Route::Cad, Route::Cad),
            ("ze", Route::Cad, Route::Cad),
            ("zoom in", Route::Cad, Route::Cad),
            ("50,25", Route::Cad, Route::Cad),
            ("@10,0", Route::Cad, Route::Cad),
            ("37.5", Route::Cad, Route::Cad),
            ("", Route::Cad, Route::Cad),
            // Rule 4, post-flip (ADR 0007 §D9, amendment (5)): every
            // unrecognised line is CAD too, whatever the settings say. `lien`
            // and `z` are the two the flip is named after; the rest are the
            // words refinement found the old rule 4 also fired on (a full
            // AutoCAD command name), plus the multi-byte and malformed inputs
            // that must not panic on the way to the same verdict.
            ("lien", Route::Cad, Route::Cad),
            ("z", Route::Cad, Route::Cad),
            ("zoom", Route::Cad, Route::Cad),
            ("zoom sideways", Route::Cad, Route::Cad),
            ("/aim", Route::Cad, Route::Cad),
            ("é", Route::Cad, Route::Cad),
            ("x", Route::Cad, Route::Cad),
            ("d", Route::Cad, Route::Cad),
            ("1,2,3", Route::Cad, Route::Cad),
            ("nan", Route::Cad, Route::Cad),
            ("Foo", Route::Cad, Route::Cad),
        ];
        let mut moved_with_availability = 0;
        for (line, with_key, without_key) in rows {
            assert_eq!(
                classify(line, true),
                with_key,
                "`{line}` with a key configured"
            );
            assert_eq!(
                classify(line, false),
                without_key,
                "`{line}` with no key configured"
            );
            if with_key != without_key {
                moved_with_availability += 1;
            }
        }
        assert_eq!(
            moved_with_availability, 5,
            "only the five prefixed rows — `:line`, `/ai line`, `/AI line`, \
             `:50,25`, `:snap` — may depend on availability"
        );
    }

    /// `"z"` is **not** a zoom word. The zoom forms are `"ze"` and
    /// `"zoom <in|out|extents>"` (ADR 0003, `src/cmdline/parse.rs`), so a bare
    /// `z` parses as `Unknown` — a CAD row like every other unrecognised line
    /// since LCV-148, not the "rule 4 row" this test recorded before the flip.
    ///
    /// A bare `z` has always answered `Unknown command: "z"` locally; what
    /// LCV-148 changed is that it now answers that way whatever the settings
    /// say, exactly like `lien`. This test records the grammar the table row
    /// above is asserted against.
    #[test]
    fn z_is_not_a_zoom_word() {
        assert_eq!(parse("z"), CommandInput::Unknown("z".into()));
        assert_eq!(
            parse("ze"),
            CommandInput::Zoom(crate::cmdline::ZoomKind::Extents)
        );
        assert_eq!(
            parse("zoom in"),
            CommandInput::Zoom(crate::cmdline::ZoomKind::In)
        );
        // `l` is CAD the other way: a tool alias rule 3 recognises.
        assert_eq!(
            parse("l"),
            CommandInput::Tool(crate::cmdline::ToolKind::Line)
        );
    }

    /// AC 3 / AC 7 (LCV-148) — `/ai` needs a word boundary, so `/aim` is not a
    /// prefix: it falls through to rule 4 and is now CAD under both
    /// availabilities.
    #[test]
    fn slash_ai_requires_a_boundary() {
        assert_eq!(classify("/aim", true), Route::Cad);
        assert_eq!(classify("/aim", false), Route::Cad);
        assert_eq!(classify("/ai", true), Route::Agent(String::new()));
        assert_eq!(classify("/ai hello", true), Route::Agent("hello".into()));
        assert_eq!(classify("/ai\thello", true), Route::Agent("hello".into()));
    }

    /// AC 3 — the prefix is matched on the trimmed line, the remainder is
    /// trimmed, and only the *first* colon is the prefix.
    #[test]
    fn the_prompt_is_trimmed_and_only_the_first_colon_is_the_prefix() {
        assert_eq!(classify("  :  draw  ", true), Route::Agent("draw".into()));
        assert_eq!(classify(":a:b", true), Route::Agent("a:b".into()));
        assert_eq!(
            classify("   /AI   draw a square   ", true),
            Route::Agent("draw a square".into())
        );
    }

    /// AC 4 — the four empty-prompt spellings. They answer the same with and
    /// without a key: a prefix with nothing behind it is malformed either way,
    /// and the caller reports it without sending anything.
    #[test]
    fn an_empty_prompt_is_refused_not_sent() {
        for line in [":", ":   ", "/ai", "/ai   "] {
            assert_eq!(classify(line, true), Route::Agent(String::new()), "{line}");
            assert_eq!(classify(line, false), Route::Agent(String::new()), "{line}");
        }
    }

    /// AC 2 rule 2 — a prefixed line never reaches the parser's verdict, so a
    /// prompt that happens to be a whole CAD command is still a prompt. The
    /// control is the same text without the prefix.
    #[test]
    fn a_prefixed_cad_command_is_a_prompt() {
        for line in ["line", "l", "snap", "50,25", "@10,0", "37.5", "zoom in"] {
            assert_eq!(
                classify(line, false),
                Route::Cad,
                "control: `{line}` is CAD"
            );
            assert_eq!(
                classify(&format!(":{line}"), true),
                Route::Agent(line.to_owned()),
                "`:{line}` is a prompt"
            );
        }
    }

    /// AC 3 / AC 7 (LCV-148) — a multi-byte first character cannot panic the
    /// prefix match, and since the flip an unprefixed multi-byte line is CAD
    /// under both availabilities, exactly like any other unrecognised line.
    #[test]
    fn a_multibyte_line_is_classified_without_panicking() {
        assert_eq!(classify("é", true), Route::Cad);
        assert_eq!(classify("é", false), Route::Cad);
        assert_eq!(classify("/é", true), Route::Cad);
    }
}
