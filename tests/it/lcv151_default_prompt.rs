//! LCV-151 — the built-in system prompt describes every tool, the index
//! contract, routing, the step budget and the reply style.
//!
//! AC 2: every tool in `tool_definitions()` has its own paragraph naming it
//! and every schema property as a whole word, with a positive control on a
//! doctored prompt.

use lasercad::agent::{
    tool_definitions, AGENT_STEP_BUDGET_DEFAULT, AGENT_STEP_BUDGET_MAX, DEFAULT_PROMPT,
};

/// The whole words of `text`: runs of ASCII letters, digits and `_`.
fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect()
}

/// Every `(tool name, property keys)` pair advertised by `tool_definitions()`.
fn tools() -> Vec<(String, Vec<String>)> {
    let defs = tool_definitions();
    let defs = defs.as_array().expect("tool_definitions is an array");
    defs.iter()
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
    assert_eq!(coverage_gaps(DEFAULT_PROMPT), Vec::<String>::new());
}

/// AC 2 — positive control: dropping one argument from its tool's paragraph
/// is caught, even though the same word appears elsewhere in the prompt.
#[test]
fn a_prompt_missing_one_argument_is_caught() {
    let doctored = DEFAULT_PROMPT.replacen("end_deg", "stop", 1);
    assert_ne!(
        doctored, DEFAULT_PROMPT,
        "control: the prompt names end_deg"
    );
    let gaps = coverage_gaps(&doctored);
    assert!(
        gaps.iter().any(|g| g == "create_arc: argument end_deg"),
        "{gaps:?}"
    );
}
