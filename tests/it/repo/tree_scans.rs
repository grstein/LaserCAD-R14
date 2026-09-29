//! LCV-121 — tree-wide source scans.
//!
//! These claims are about the whole of `src/`, not about one file, so they live
//! in their own integration test rather than in any module's test block.
//!
//! Every scan follows the two rules that keep a scan able to fail (AGENTS.md;
//! the canonical example is `guard_is_runtime_not_cfg` in `src/io/dialogs.rs`):
//! the haystack stops at the bare `#[cfg(test)]` at column 0, so inline test
//! code is never searched, and every needle is assembled with `concat!`, so a
//! scan can never match the literal written next to it. Each one also carries a
//! positive control, because an absence assertion over an empty haystack passes
//! for the wrong reason.
//!
//! Relative paths are rebuilt from `components()` joined with `/` and sorted on
//! the rendered string: `Path::display()` emits `\` on Windows and turns a
//! comparison like this into a CI-only failure (AGENTS.md §Implementation
//! Rules).
//!
//! The walker and the matcher are `tests/harness/scan.rs`'s (LCV-132), which is
//! where those four rules are written down in full. The walk-and-slice wrapper
//! below stays here: its root, its bound, its return shape and its positive
//! control are this demand's decisions, not shareable ones.

use crate::harness;

use harness::scan::{files_containing, is_test_file, rs_files};
use std::path::{Path, PathBuf};

/// The implementation half of a source file: everything before the bare
/// `#[cfg(test)]` at column 0.
fn implementation_or_all(src: &str) -> &str {
    match src.find("\n#[cfg(test)]") {
        Some(at) => &src[..at],
        None => src,
    }
}

/// `src/` and every `.rs` file under it, as (relative path, implementation
/// text) pairs sorted on the rendered path.
fn implementation_sections() -> (PathBuf, Vec<(String, String)>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    files.retain(|path| !is_test_file(path));
    assert!(
        files.len() > 30,
        "positive control: the walk must see the whole tree, saw {}",
        files.len()
    );

    let mut sections: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .expect("every walked file is under src/")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let src = std::fs::read_to_string(path).expect("a readable source file");
            (relative, implementation_or_all(&src).to_owned())
        })
        .collect();
    sections.sort();
    (root, sections)
}

/// AC 9 — the model id is the caller's to choose, so the old hardcoded one is
/// gone from the tree. Pasting `"gpt-4o"` back into `loop_.rs::send_fn` fails
/// this by name.
#[test]
fn the_hardcoded_model_id_is_gone_from_src() {
    let (_, sections) = implementation_sections();
    assert!(
        !files_containing(&sections, concat!("chat_", "completion")).is_empty(),
        "positive control: the scanned slices must be real source"
    );
    let hits = files_containing(&sections, concat!("gpt", "-4o"));
    assert!(
        hits.is_empty(),
        "AC 9: the model id comes from Settings, but src/ still names one: {hits:?}"
    );
}

/// AC 10 — the fixed call cap was replaced by the configurable step budget.
#[test]
fn the_fixed_tool_call_cap_is_retired() {
    let (_, sections) = implementation_sections();
    assert!(
        !files_containing(&sections, concat!("AGENT_STEP_BUDGET", "_DEFAULT")).is_empty(),
        "positive control: the budget constants must exist in src/"
    );
    let hits = files_containing(&sections, concat!("MAX_TOOL_CALLS", "_PER_TURN"));
    assert!(
        hits.is_empty(),
        "AC 10: the constant is retired, but src/ still mentions it: {hits:?}"
    );
}

/// AC 2 — `transport::Message` is gone; the conversation type is
/// `wire::ChatMessage` everywhere.
#[test]
fn the_old_transport_message_type_is_gone() {
    let (_, sections) = implementation_sections();
    assert!(
        !files_containing(&sections, concat!("Chat", "Message")).is_empty(),
        "positive control: the replacement type must be referenced in src/"
    );
    let hits = files_containing(&sections, concat!("transport::", "Message"));
    assert!(
        hits.is_empty(),
        "AC 2: transport::Message is retired, but src/ still names it: {hits:?}"
    );
}

/// AC 15 — the transport is the crate's single HTTP boundary (ADR 0007 §D8).
/// The assertion is an equality on the whole list, so a second importer fails
/// it just as loudly as a missing one.
#[test]
fn only_the_transport_imports_reqwest() {
    let (_, sections) = implementation_sections();
    let hits = files_containing(&sections, concat!("req", "west"));
    assert!(
        hits.contains(&"agent/transport.rs".to_string()),
        "positive control: the transport really does speak HTTP, hits were {hits:?}"
    );
    assert_eq!(
        hits,
        ["agent/transport.rs"],
        "AC 15: exactly one file under src/ may speak HTTP"
    );
}

/// AC 12 — `io::settings` writes the default budget as a literal so it does
/// not have to import the agent module, which leaves the two numbers free to
/// drift. This is the test that stops them.
#[test]
fn the_settings_default_budget_matches_the_loop_constant() {
    assert_eq!(
        lasercad::io::settings::Settings::default().agent_step_budget,
        lasercad::agent::AGENT_STEP_BUDGET_DEFAULT,
        "the literal in io/settings.rs and the constant in agent/loop_.rs must agree"
    );
}
