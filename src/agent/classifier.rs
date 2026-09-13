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
//! model; `tests/lcv124_command_line_routing.rs` guards it by name.
//!
//! Purity: this file may import nothing but `crate::cmdline` and `std`. No
//! `egui`, no `eframe`, no `rfd`, no `reqwest`, no `crate::app` — asserted by
//! [`tests::classifier_is_kernel_pure`].

use crate::cmdline::{parse, CommandInput};

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
    /// still gets when no API key is configured.
    Cad,
    /// Send the payload to the agent as a prompt.
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
///    `":snap"` all go to the agent, even though the remainder parses as a CAD
///    command. This is the escape hatch and nothing overrides it.
/// 3. [`parse`] returning anything other than [`CommandInput::Unknown`] is
///    CAD: verbs, aliases, toggles, zoom, points, offsets, bare distances and
///    a bare Enter always win.
/// 4. an `Unknown` line reaches the agent only when `agent_available`;
///    otherwise it stays CAD and gets the same `Unknown command: "…"` it has
///    always got.
///
/// `agent_available` is data, not a global: the caller computes it as
/// "a non-whitespace API key is configured" and passes it in.
pub fn classify(raw: &str, agent_available: bool) -> Route {
    match agent_prompt(raw.trim()) {
        // An empty prompt is refused before availability is even consulted:
        // `:` with nothing behind it is a malformed line whatever the settings
        // say, and the caller answers it without reaching the network.
        Some("") => Route::Agent(String::new()),
        Some(_) if !agent_available => Route::Unavailable,
        Some(prompt) => Route::Agent(prompt.to_owned()),
        None => match parse(raw) {
            // The payload is the trimmed original text in its original case,
            // so the caller echoes it without keeping a second copy.
            CommandInput::Unknown(text) if agent_available => Route::Agent(text),
            _ => Route::Cad,
        },
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
    /// Comment lines are skipped, exactly as `tests/lcv122_source_scans.rs`
    /// does it and for the same reason: this file's header names the crates it
    /// may not import precisely in order to forbid them, and a scan that read
    /// prose as an import would force the rule to go undocumented to stay true.
    fn code_contains(haystack: &str, needle: &str) -> bool {
        haystack
            .lines()
            .any(|line| line.contains(needle) && !line.trim_start().starts_with("//"))
    }

    /// AC 1 — the classifier is pure and takes availability as data. Needles
    /// are built with `concat!` so the scan can never match the literal
    /// written next to it, the haystack stops at the test module, and each
    /// absence is backed by a positive control run through the **same**
    /// `code_contains` over the **same** slice: if the scan were reading the
    /// wrong bytes, or skipping every line, the control fails first.
    #[test]
    fn classifier_is_kernel_pure() {
        let implementation = implementation();
        for control in [
            concat!("pub fn ", "classify"),
            concat!("crate::", "cmdline"),
            concat!("Command", "Input"),
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

    /// AC 2 — the whole precedence table, every row asserted with
    /// `agent_available` both `true` and `false`, so the flag is *proved* to
    /// matter: the only rows that move with it are the ones rule 4 owns.
    #[test]
    fn the_precedence_table_holds_for_both_availabilities() {
        // (line, route when a key is configured, route without one)
        let rows: [(&str, Route, Route); 15] = [
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
            // Rule 4 — the rows the availability flag moves. `"z"` sits here
            // and not above on purpose: see `z_is_not_a_zoom_word` below.
            ("lien", Route::Agent("lien".into()), Route::Cad),
            ("z", Route::Agent("z".into()), Route::Cad),
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
            moved_with_availability, 7,
            "the five prefixed rows, `lien` and `z` must depend on availability"
        );
    }

    /// `"z"` is **not** a zoom word. The zoom forms are `"ze"` and
    /// `"zoom <in|out|extents>"` (ADR 0003, `src/cmdline/parse.rs`), so a bare
    /// `z` parses as `Unknown` and belongs to rule 4, not rule 3.
    ///
    /// The LCV-124 demand lists `"z"` among the lines that "still behave
    /// exactly as they do today", which reads as a claim that it is a zoom
    /// command. It never was one: today a bare `z` answers
    /// `Unknown command: "z"`, and rule 4 therefore sends it to the agent when
    /// a key is configured, exactly like `lien` — with no key it keeps that
    /// same message, so it *does* still behave exactly as it does today. This
    /// test records the grammar the table row above is asserted against.
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

    /// AC 3 — `/ai` needs a word boundary, so `/aim` is not a prefix: it falls
    /// through to rules 3/4 as an unrecognised line.
    #[test]
    fn slash_ai_requires_a_boundary() {
        assert_eq!(classify("/aim", true), Route::Agent("/aim".into()));
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

    /// AC 3 — a multi-byte first character cannot panic the prefix match.
    #[test]
    fn a_multibyte_line_is_classified_without_panicking() {
        assert_eq!(classify("é", true), Route::Agent("é".into()));
        assert_eq!(classify("é", false), Route::Cad);
        assert_eq!(classify("/é", true), Route::Agent("/é".into()));
    }
}
