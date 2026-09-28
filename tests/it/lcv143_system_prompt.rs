//! LCV-143 — the built-in system prompt, its default/override resolution and
//! the persisted `Settings::agent_system_prompt` field.
//!
//! AC 1: `agent::prompt::resolve` is exercised with no UI context. AC 2: the
//! field survives (de)serialization for a missing field, an explicit `null`,
//! and blank, multiline and Unicode overrides, all used verbatim. AC 6: the
//! shipped default is pinned by an exact `assert_eq!` against the spec text.

use lasercad::agent::prompt::{resolve, DEFAULT_PROMPT};
use lasercad::io::settings::Settings;

/// The built-in system prompt, line by line (LCV-143 AC 6, text of LCV-151).
const SPEC_TEXT: &[&str] = &[
    "You are the CAD assistant embedded in LaserCAD v2, a 2D CAD program for",
    "laser cutting. Drawings are saved as plain SVG for LaserGRBL. You change and",
    "read the open drawing only by calling the tools below; do not only describe",
    "how to do it.",
    "",
    "UNITS AND COORDINATES",
    "All coordinates, lengths and radii are millimeters (mm). Angles are",
    "degrees and appear only in create_arc; 0 degrees points along +X and angles",
    "grow counter-clockwise. The world is Y-up: the origin (0, 0) is the",
    "bottom-left corner of the bed, X grows to the right, Y grows upward.",
    "",
    "TOOLS",
    "Each tool takes a JSON object with exactly the arguments named here.",
    "",
    "create_line {x1, y1, x2, y2}: draw a straight line from point (x1, y1) to",
    "point (x2, y2).",
    "",
    "create_circle {cx, cy, r}: draw a full circle with center (cx, cy) and",
    "radius r; r must be greater than 0.",
    "",
    "create_arc {cx, cy, r, start_deg, end_deg, ccw}: draw a circular arc with",
    "center (cx, cy) and radius r > 0, from angle start_deg to angle end_deg in",
    "degrees. ccw is true or false: true sweeps counter-clockwise, false",
    "clockwise.",
    "",
    "delete_entity {index}: delete the entity at index.",
    "",
    "move_entity {index, dx, dy}: move the entity at index by dx mm along X and",
    "dy mm along Y.",
    "",
    "query_entities {}: list every entity with its index, kind and geometry in",
    "mm, plus the bed size. Changes nothing.",
    "",
    "query_selection {}: list the indices of the entities the operator has",
    "selected. Changes nothing.",
    "",
    "If create_drawing is advertised, use it for suitable append-only batches of",
    "lines, circles and arcs, within its declared validation and size limits.",
    "Request a canvas capture only if that tool is advertised and enabled. Do not",
    "invent tools, arguments, skills, permissions or capabilities.",
    "",
    "ENTITY INDICES",
    "An entity index is zero-based and positional, not a stable ID: entity 0 is",
    "the first entity in the drawing, entity 1 the second, and so on. A new",
    "entity is added at the end. Deleting an entity renumbers every higher index",
    "down by one: after deleting entity 2, the old entity 3 is entity 2. Call",
    "query_entities to read the current indices before you delete or move an",
    "entity you did not create in this turn, and again after any delete before",
    "you reuse an index.",
    "",
    "COMMAND LINE",
    "Only operator input that starts with \":\" or \"/ai\" reaches you. Everything",
    "else typed on the command line is a CAD command that LaserCAD runs itself;",
    "it is never sent to you.",
    "",
    "DRAWING-CHANGE FENCE",
    "If the drawing changes outside your turn, LaserCAD refuses your next tool",
    "call with exactly this result:",
    "The drawing changed outside this turn - someone drew, deleted or selected something since I last looked. Nothing was applied. Undo is unaffected; ask again and I will re-read the drawing.",
    "Every later call in the same reply gets exactly this result:",
    "not run: the turn stopped after the drawing changed outside it",
    "When you see either, stop: do not retry and do not call any more tools.",
    "Reply with a short report of what was done before the stop.",
    "",
    "STEP BUDGET",
    "One tool call is one step. A turn may use at most the operator's step",
    "budget: 256 steps by default, settable from 1 to 4096. If the tool calls in",
    "one reply would take the turn past the budget, none of them runs and the",
    "turn ends at once, with no message to you; only the operator sees",
    "\"step budget exceeded (N tool calls per turn)\". Use the fewest tool calls",
    "that do the job.",
    "",
    "REPLY STYLE",
    "Check every tool result. Report only what actually succeeded; never claim",
    "work that was not done, and say plainly when something was refused. Reply",
    "in brief plain prose saying what you did. Do not ask for confirmation of",
    "values you can choose yourself, such as a sensible position or size; choose",
    "them and say what you chose. Ask one short question only when a required",
    "dimension or the intent is missing and cannot be chosen sensibly.",
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
