//! LCV-151 — the built-in system prompt describes every tool, the index
//! contract, routing, the step budget and the reply style.
//!
//! AC 2: every tool in `tool_definitions()` has its own paragraph naming it
//! and every schema property as a whole word, with a positive control on a
//! doctored prompt. AC 1, 3, 4, 6, 7: ASCII-only, section needles in order.

use lasercad::agent::{
    AGENT_STEP_BUDGET_DEFAULT, AGENT_STEP_BUDGET_MAX, DEFAULT_PROMPT, tool_definitions,
};

/// The whole words of `text`: runs of ASCII letters, digits and `_`.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Every `(tool name, property keys)` pair advertised by `tool_definitions()`,
/// with the vision flag off and on (LCV-145 adds `capture_canvas` behind it),
/// each tool once.
fn tools() -> Vec<(String, Vec<String>)> {
    let mut seen = Vec::new();
    for vision in [false, true] {
        let defs = tool_definitions(vision);
        let defs = defs.as_array().expect("tool_definitions is an array");
        for def in defs {
            if !seen.contains(def) {
                seen.push(def.clone());
            }
        }
    }
    seen.iter()
        .map(|def| {
            let function = &def["function"];
            let name = function["name"].as_str().expect("tool name").to_owned();
            let props = function["parameters"]["properties"]
                .as_object()
                .expect("properties object")
                .keys()
                .cloned()
                .collect();
            (name, props)
        })
        .collect()
}

/// What `prompt` fails to say about the tools: a tool with no paragraph of its
/// own (one that starts with its name), or a property missing as a whole word
/// from that paragraph.
fn coverage_gaps(prompt: &str) -> Vec<String> {
    let paragraphs: Vec<&str> = prompt.split("\n\n").collect();
    let mut gaps = Vec::new();
    for (name, props) in tools() {
        let own: Vec<&&str> = paragraphs
            .iter()
            .filter(|p| words(p).first() == Some(&name.as_str()))
            .collect();
        let [paragraph] = own.as_slice() else {
            gaps.push(format!("{name}: {} paragraphs", own.len()));
            continue;
        };
        let found = words(paragraph);
        for prop in props {
            if !found.contains(&prop.as_str()) {
                gaps.push(format!("{name}: argument {prop}"));
            }
        }
    }
    gaps
}

/// AC 2 — each advertised tool has its own paragraph listing every argument.
#[test]
fn every_tool_has_a_paragraph_with_every_argument() {
    assert!(tools().len() >= 7, "control: the registry is not empty");
    assert!(
        tools().iter().any(|(name, _)| name == "capture_canvas"),
        "control: the vision-only tool is covered too"
    );
    assert_eq!(coverage_gaps(DEFAULT_PROMPT), Vec::<String>::new());
}

/// `text` with every whole-word `from` in the paragraph that starts with
/// `tool` replaced by `to`; the rest of the prompt is untouched.
fn drop_word(text: &str, tool: &str, from: &str, to: &str) -> String {
    let rewrite = |p: &str| {
        let mut out = String::new();
        let mut word = String::new();
        for c in p.chars().chain(['\0']) {
            if c.is_ascii_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            out.push_str(if word == from { to } else { &word });
            word.clear();
            if c != '\0' {
                out.push(c);
            }
        }
        out
    };
    text.split("\n\n")
        .map(|p| {
            if words(p).first() == Some(&tool) {
                rewrite(p)
            } else {
                p.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// AC 2 — positive control: dropping `r` from the create_circle paragraph
/// is caught, even though `r` still appears in the create_arc paragraph.
#[test]
fn a_prompt_missing_one_argument_is_caught() {
    let doctored = drop_word(DEFAULT_PROMPT, "create_circle", "r", "radius");
    assert_ne!(doctored, DEFAULT_PROMPT, "control: create_circle names r");
    assert_eq!(
        coverage_gaps(&doctored),
        vec!["create_circle: argument r".to_owned()]
    );
}

/// AC 1 — the prompt is ASCII-only.
#[test]
fn the_default_prompt_is_ascii_only() {
    let bad: Vec<char> = DEFAULT_PROMPT.chars().filter(|c| !c.is_ascii()).collect();
    assert!(bad.is_empty(), "non-ASCII characters: {bad:?}");
}

/// AC 1, 3, 4, 6, 7 — the sections of the spec's Scope appear, in order.
/// Line breaks are folded to spaces, so a needle may span a wrapped line.
#[test]
fn the_sections_appear_in_order() {
    let default = AGENT_STEP_BUDGET_DEFAULT.to_string();
    let range = format!("1 to {AGENT_STEP_BUDGET_MAX}");
    let lower = DEFAULT_PROMPT
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let needles: Vec<&str> = vec![
        // What LaserCAD is.
        "laserCAD",
        "laser cutting",
        "svg",
        "lasergrbl",
        // Units and the world.
        "millimeters",
        "degrees",
        "y-up",
        "bottom-left",
        // Tools.
        "create_line",
        "query_selection",
        // Entity indices (AC 3).
        "zero-based",
        "positional",
        "entity 0 is the first",
        "every higher index down by one",
        "query_entities to read the current indices",
        // Routing (AC 4).
        "\":\"",
        "\"/ai\"",
        "everything else",
        "cad command",
        // Fence.
        "fence",
        // Step budget (AC 6).
        "one tool call is one step",
        &default,
        &range,
        "no message",
        // Reply style (AC 7).
        "brief",
        "do not ask for confirmation",
    ];
    let mut from = 0;
    for needle in needles {
        let needle = needle.to_lowercase();
        let Some(at) = lower[from..].find(&needle) else {
            panic!("`{needle}` missing after byte {from}");
        };
        from += at + needle.len();
    }
}

/// LCV-156 AC 14 — a LAYERS section, after the index section, says that a
/// creation lands on the current layer unless `layer` names an existing one,
/// where to read the names, that an unknown name is refused, and that no
/// tool edits layers.
#[test]
fn the_prompt_describes_layers() {
    let folded = DEFAULT_PROMPT
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let section = folded
        .split_once("LAYERS ")
        .map(|(_, rest)| rest)
        .expect("a LAYERS section");
    assert!(
        folded.find("ENTITY INDICES") < folded.find("LAYERS "),
        "LAYERS follows ENTITY INDICES"
    );
    let section = section.split_once("COMMAND LINE").map_or(section, |s| s.0);
    for needle in [
        "current layer",
        "layer argument",
        "existing layer",
        "query_entities",
        "refused",
        "nothing is drawn",
        "no tool creates, renames or deletes layers",
    ] {
        let lower = section.to_lowercase();
        assert!(lower.contains(needle), "`{needle}` missing: {section}");
    }
}

/// LCV-185 AC 7 — the create_drawing paragraph says that keys of other types
/// may be omitted or null, and no longer demands exactly one type's keys.
#[test]
fn the_create_drawing_paragraph_tolerates_null_foreign_keys() {
    let paragraph = DEFAULT_PROMPT
        .split("\n\n")
        .find(|p| words(p).first() == Some(&"create_drawing"))
        .expect("a create_drawing paragraph");
    let folded = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        folded
            .to_lowercase()
            .contains("keys of other types may be omitted or null"),
        "{folded}"
    );
    assert!(!folded.contains("exactly that type's"), "{folded}");
}
