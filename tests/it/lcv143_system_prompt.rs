//! LCV-143 — the built-in system prompt, its default/override resolution and
//! the persisted `Settings::agent_system_prompt` field.
//!
//! AC 1: `agent::prompt::resolve` is exercised with no UI context. AC 2: the
//! field survives (de)serialization for a missing field, an explicit `null`,
//! and blank, multiline and Unicode overrides, all used verbatim. AC 6: the
//! shipped default is pinned by an exact `assert_eq!` against the spec text.

use lasercad::agent::prompt::{resolve, DEFAULT_PROMPT};
use lasercad::io::settings::Settings;

/// The spec's §Built-in system prompt, line by line.
const SPEC_TEXT: &[&str] = &[
    "You are the CAD assistant embedded in LaserCAD v2, a focused 2D CAD",
    "application for preparing LaserGRBL-compatible laser drawings.",
    "",
    "When asked to construct, modify, or inspect the open drawing, call the",
    "advertised harness tools to do the work; do not only describe how to do it.",
    "Ask a focused question when required dimensions or intent are missing.",
    "Use the fewest tool calls that correctly satisfy the request.",
    "",
    "All drawing coordinates, lengths, and radii are canonical millimeters (mm).",
    "Follow each tool schema for angles: existing arc tools accept degrees;",
    "the geometry kernel uses radians. Do not substitute pixels for geometry.",
    "",
    "Entity indices are positional, not stable IDs. Deleting an entity shifts",
    "every higher index down by one. Query the live drawing before targeting an",
    "index you have not read in this turn, and query again after deletion before",
    "reusing potentially stale indices.",
    "",
    "If create_drawing is advertised, use it for suitable append-only batches",
    "of lines, circles, and arcs, within its declared validation and size limits.",
    "Request a canvas capture only if that tool is advertised and enabled.",
    "Do not invent tools, skills, permissions, or capabilities.",
    "",
    "Check every tool outcome. Report only changes and observations that actually",
    "succeeded; never claim unperformed work. If the drawing-change fence refuses",
    "an action or the turn is cancelled, stop rather than retrying. State any",
    "partial completion or refusal honestly and summarize the outcome concisely.",
];

/// Overrides that must all come back verbatim: blank, whitespace-only,
/// multiline and Unicode.
const OVERRIDES: &[&str] = &[
    "",
    "   \t  ",
    "\n\n",
    "line one\n  line two  \n\nline four\n",
    "Desenhe em milímetros — ângulos em graus. 図面 🔧",
];

/// AC 6 — the shipped default is exactly the spec text.
#[test]
fn the_default_prompt_is_exactly_the_spec_text() {
    assert_eq!(DEFAULT_PROMPT, SPEC_TEXT.join("\n"));
}

/// AC 1 + AC 2 — no override resolves to the built-in default.
#[test]
fn resolve_without_an_override_is_the_default() {
    assert_eq!(resolve(None), DEFAULT_PROMPT);
}

/// AC 2 — any present string replaces the default completely and verbatim:
/// no trim, no fallback for a deliberately blank override.
#[test]
fn resolve_returns_every_override_verbatim() {
    for text in OVERRIDES {
        assert_eq!(resolve(Some(text)), *text, "{text:?}");
    }
}

/// AC 2 — a settings file predating this demand still loads, other fields
/// intact, with no override.
#[test]
fn an_old_settings_file_without_the_field_has_no_override() {
    let json = r#"{"recent_files":["a.lcad"],"agent_model":"x/y","agent_step_budget":9}"#;
    let s: Settings = serde_json::from_str(json).expect("old-format settings parse");
    assert_eq!(s.agent_system_prompt, None);
    assert_eq!(s.agent_model, "x/y");
    assert_eq!(s.agent_step_budget, 9);
    assert_eq!(s.recent_files, vec!["a.lcad"]);
    assert_eq!(resolve(s.agent_system_prompt.as_deref()), DEFAULT_PROMPT);
}

/// AC 2 — an explicit JSON `null` is no override either.
#[test]
fn an_explicit_null_resolves_to_the_default() {
    let s: Settings = serde_json::from_str(r#"{"agent_system_prompt":null}"#).expect("null parses");
    assert_eq!(s.agent_system_prompt, None);
    assert_eq!(resolve(s.agent_system_prompt.as_deref()), DEFAULT_PROMPT);
    assert_eq!(Settings::default().agent_system_prompt, None);
}

/// AC 2 — blank, whitespace-only, multiline and Unicode overrides survive a
/// serialize/parse roundtrip byte for byte and resolve to themselves.
#[test]
fn overrides_roundtrip_through_settings_verbatim() {
    for text in OVERRIDES {
        let original = Settings {
            agent_system_prompt: Some((*text).to_owned()),
            ..Settings::default()
        };
        let json = serde_json::to_string(&original).expect("serializes");
        let loaded: Settings = serde_json::from_str(&json).expect("parses back");
        assert_eq!(loaded, original, "{text:?}");
        assert_eq!(resolve(loaded.agent_system_prompt.as_deref()), *text);
    }
}
