//! LCV-122 — tree-wide source scans for the bridge.
//!
//! Two claims that are about the whole tree rather than about any one file, so
//! they live here rather than in a module's test block:
//!
//! - **AC 3** — no file under `src/agent/` holds document state, except
//!   `panel.rs`, and that exception list is exactly one file long.
//! - **AC 4** — the identifier `AgentPanelMsg` appears nowhere in `src/` or
//!   `tests/`.
//!
//! Every scan follows the two rules that keep a scan able to fail (AGENTS.md;
//! the canonical example is `guard_is_runtime_not_cfg` in `src/io/dialogs.rs`):
//! the haystack stops at the bare `#[cfg(test)]` at column 0, so inline test
//! code is never searched, and every needle is assembled with `concat!`, so a
//! scan can never match the literal written next to it. Each one carries a
//! positive control, because an absence assertion over an empty haystack passes
//! for the wrong reason.
//!
//! Relative paths are rebuilt from `components()` joined with `/` and sorted on
//! the rendered string: `Path::display()` emits `\` on Windows and turns a
//! comparison like this into a CI-only failure (AGENTS.md §Implementation
//! Rules).

use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively.
fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("the source tree must be readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The implementation half of a source file: everything before the bare
/// `#[cfg(test)]` at column 0.
fn implementation_or_all(src: &str) -> &str {
    match src.find("\n#[cfg(test)]") {
        Some(at) => &src[..at],
        None => src,
    }
}

/// Every `.rs` file under `<manifest>/<dir>` as (relative path, text) pairs,
/// sorted on the rendered path. `bound` decides whether the text is the
/// implementation section or the whole file.
fn sections(dir: &str, bound: bool, minimum: usize) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    assert!(
        files.len() >= minimum,
        "positive control: the walk over {dir} must see at least {minimum} files, saw {}",
        files.len()
    );

    let mut out: Vec<(String, String)> = files
        .iter()
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .expect("every walked file is under the root")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let src = std::fs::read_to_string(path).expect("a readable source file");
            let body = if bound {
                implementation_or_all(&src).to_owned()
            } else {
                src
            };
            (relative, body)
        })
        .collect();
    out.sort();
    out
}

/// Files whose section contains `needle` on a **code** line.
///
/// Comment lines are skipped, so these scans are about what the compiler sees
/// rather than about prose: `src/agent/mod.rs`'s module header names `Document`
/// precisely to say which file is allowed to hold one, and a scan that counted
/// that as a use would force the rule to go undocumented to stay true.
fn files_containing(sections: &[(String, String)], needle: &str) -> Vec<String> {
    sections
        .iter()
        .filter(|(_, body)| {
            body.lines()
                .any(|line| line.contains(needle) && !line.trim_start().starts_with("//"))
        })
        .map(|(path, _)| path.clone())
        .collect()
}

/// The one file allowed to name document state under `src/agent/`, until
/// LCV-123 deletes the throwaway pair inside it.
const AGENT_DOCUMENT_EXCEPTIONS: [&str; 1] = ["panel.rs"];

/// AC 3 / ADR 0007 §D1 — the background thread owns no document state, so no
/// file it can reach names any.
///
/// The exception list is asserted by **length as well as contents**: an
/// exception list that silently grows is the thing this AC exists to stop, and
/// a `contains` check would let a second file join it unnoticed.
#[test]
fn no_agent_file_but_panel_holds_document_state() {
    let agent = sections("src/agent", true, 8);

    // Positive control, and a tight one: the needle set below must really find
    // `panel.rs`, or the whole scan is vacuous. `panel.rs` names `Document` in
    // code today; when LCV-123 removes it, this control fails loudly and the
    // exception list comes out with it.
    let panel = files_containing(&agent, concat!("Doc", "ument"));
    assert_eq!(
        panel, AGENT_DOCUMENT_EXCEPTIONS,
        "positive control: the scan must still see panel.rs's throwaway document"
    );
    assert_eq!(
        AGENT_DOCUMENT_EXCEPTIONS.len(),
        1,
        "AC 3 names exactly one exception; a longer list is a review blocker"
    );

    for needle in [
        concat!("crate::", "document"),
        concat!("Doc", "ument"),
        concat!("His", "tory"),
        concat!("Arc<", "Mutex"),
        concat!("Vec<", "Entity>"),
    ] {
        let offenders = files_containing(&agent, needle);
        assert!(
            offenders.iter().all(|f| AGENT_DOCUMENT_EXCEPTIONS.contains(&f.as_str())),
            "AC 3: `{needle}` may only appear in {AGENT_DOCUMENT_EXCEPTIONS:?}, found in {offenders:?}"
        );
    }
}

/// AC 3 — the bridge in particular holds nothing at all, not even the
/// exception's worth. Stated separately because `bridge.rs` is the file the
/// worker thread shares, and it is the one that must stay clean forever rather
/// than until LCV-123.
#[test]
fn the_bridge_names_no_document_and_no_ui() {
    let agent = sections("src/agent", true, 8);
    let (_, bridge) = agent
        .iter()
        .find(|(path, _)| path == "bridge.rs")
        .expect("positive control: src/agent/bridge.rs must exist");
    assert!(
        bridge.contains(concat!("pub enum ", "AgentEvent")),
        "positive control: the protocol must be declared in bridge.rs"
    );
    for needle in [
        concat!("Doc", "ument"),
        concat!("His", "tory"),
        concat!("e", "gui"),
        concat!("e", "frame"),
        concat!("r", "fd"),
        concat!("req", "west"),
    ] {
        assert!(
            !bridge
                .lines()
                .any(|l| l.contains(needle) && !l.trim_start().starts_with("//")),
            "AC 2/3: bridge.rs must not name `{needle}`"
        );
    }
}

/// AC 4 and AC 5 — the two identifiers LCV-122 retired are gone from the whole
/// tree, tests included: a stale reference in a test is exactly as wrong as one
/// in the implementation, and there is no reason for either name to survive.
///
/// The retired names are never written out in this test — not in a needle, not
/// in an assertion message — because either one would make the file match
/// itself and the scan would report its own source forever. The pairs below are
/// `(retired, replacement)`, and the replacement doubles as the positive
/// control: if the haystack were empty or mis-read, the control fails first.
#[test]
fn the_retired_identifiers_are_gone_from_src_and_tests() {
    let retired = [
        (concat!("Agent", "PanelMsg"), concat!("Agent", "Event")),
        (
            concat!("dispatch_", "tool_call"),
            concat!("parse_", "tool_call"),
        ),
    ];
    for (dir, minimum) in [("src", 30), ("tests", 5)] {
        let all = sections(dir, false, minimum);
        for (gone, replacement) in retired {
            if dir == "src" {
                assert!(
                    !files_containing(&all, replacement).is_empty(),
                    "positive control: {dir}/ must name `{replacement}`"
                );
            }
            let survivors = files_containing(&all, gone);
            assert!(
                survivors.is_empty(),
                "AC 4/5: `{gone}` must not appear under {dir}/, found in {survivors:?}"
            );
        }
    }
    // The `tests/` half needs its own control, since only one replacement is
    // named there: `tests/skeleton.rs` witnesses the renamed channel payload.
    let tests = sections("tests", false, 5);
    assert!(
        !files_containing(&tests, concat!("Agent", "Event")).is_empty(),
        "positive control: tests/ must witness the replacement type"
    );
}
