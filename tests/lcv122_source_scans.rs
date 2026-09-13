//! LCV-122 — tree-wide source scans for the bridge.
//!
//! Two claims that are about the whole tree rather than about any one file, so
//! they live here rather than in a module's test block:
//!
//! - **AC 3**, as tightened by LCV-123 AC 1 — no file under `src/agent/` holds
//!   document state, and the exception list that used to hold `panel.rs` is now
//!   asserted **empty**.
//! - **AC 4 / AC 5** — the two identifiers LCV-122 retired appear nowhere in
//!   `src/` or `tests/`. They are deliberately not spelled out here; the test
//!   below builds them from fragments.
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
/// and `History` precisely to state that no file here may hold one, and a scan
/// that counted those as uses would force the rule to go undocumented to stay
/// true.
///
/// The cost is that prose is **unscanned**, so a header can drift out of step
/// with the code it describes and nothing here notices — LCV-123's review
/// caught exactly that in `src/agent/mod.rs`. Skipping comments is still the
/// right trade; it just is not free.
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

/// The files allowed to name document state under `src/agent/`.
///
/// LCV-122 shipped this holding `panel.rs`, because `panel.rs` still built the
/// throwaway `Document` that made the agent's edits go nowhere. LCV-123 deleted
/// that, so the list is empty — and it is a list rather than an inlined
/// `is_empty()` so that the day someone wants an exception back, the diff says
/// so in one obvious place.
const AGENT_DOCUMENT_EXCEPTIONS: [&str; 0] = [];

/// The needles that spell "this file holds document state".
const DOCUMENT_NEEDLES: [&str; 5] = [
    concat!("crate::", "document"),
    concat!("Doc", "ument"),
    concat!("His", "tory"),
    concat!("Arc<", "Mutex"),
    concat!("Vec<", "Entity>"),
];

/// AC 3 / ADR 0007 §D1, tightened by LCV-123 AC 1 — the background thread owns
/// no document state, so **no** file it can reach names any.
///
/// ## How this scan is shown to discriminate
///
/// An absence assertion whose needles are misspelt passes for free, and the
/// obvious control — "the needle must find the one file that legitimately has
/// it" — died with the exception it pointed at. So the control is synthetic and
/// per-needle: each needle is first run against a witness section that spells
/// out what `panel.rs` used to contain, through the **same** `files_containing`
/// that does the real work. A needle that cannot find itself in the witness
/// fails here, before the real haystack is ever consulted.
///
/// The haystack gets its own control too: the walk must see at least eight
/// files, and `bridge.rs` — which is in the scan's own directory — must be
/// findable by a needle that is genuinely there.
#[test]
fn no_agent_file_holds_document_state() {
    let agent = sections("src/agent", true, 8);

    assert!(
        !files_containing(&agent, concat!("Agent", "Action")).is_empty(),
        "positive control: the haystack must really be src/agent's source"
    );

    // What `panel.rs` looked like before LCV-123, one line per needle.
    let witness: Vec<(String, String)> = vec![(
        "witness.rs".to_owned(),
        "use crate::document::{Document, History};\n         let held: Arc<Mutex<Document>> = todo!();\n         let snapshot: Vec<Entity> = doc.entities.clone();\n"
            .to_owned(),
    )];

    for needle in DOCUMENT_NEEDLES {
        assert_eq!(
            files_containing(&witness, needle),
            ["witness.rs"],
            "control: `{needle}` must be a needle that can still find document state"
        );
        let offenders = files_containing(&agent, needle);
        assert!(
            offenders
                .iter()
                .all(|f| AGENT_DOCUMENT_EXCEPTIONS.contains(&f.as_str())),
            "AC 3: `{needle}` may only appear in {AGENT_DOCUMENT_EXCEPTIONS:?}, found in {offenders:?}"
        );
    }

    // Bound through a slice so this reads as a runtime check: clippy rejects
    // `is_empty()` called straight on a `const`, and the point of the
    // assertion is the *policy*, not a compile-time fact.
    let exceptions: &[&str] = &AGENT_DOCUMENT_EXCEPTIONS;
    assert!(
        exceptions.is_empty(),
        "LCV-123 AC 1: the exception list is empty; re-opening it needs an ADR"
    );
}

/// LCV-123 AC 2 — `panel.rs` in particular starts no turn of its own.
///
/// Named separately from the scan above because the rule ADR 0007 §D8 states is
/// broader than "no document": the panel renders and reports, so it spawns no
/// thread and opens no channel either. Same witness technique.
#[test]
fn the_panel_spawns_nothing() {
    let agent = sections("src/agent", true, 8);
    let (_, panel) = agent
        .iter()
        .find(|(path, _)| path == "panel.rs")
        .expect("positive control: src/agent/panel.rs must exist");
    assert!(
        panel.contains(concat!("pub fn draw_agent", "_panel")),
        "positive control: the panel's render entry must be in panel.rs"
    );
    let witness = "let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();\n                   std::thread::spawn(move || run());\n";
    for needle in [concat!("thread", "::spawn"), concat!("mpsc", "::channel")] {
        assert!(
            witness.contains(needle),
            "control: `{needle}` must be able to match a real spawn"
        );
        assert!(
            !panel
                .lines()
                .any(|l| l.contains(needle) && !l.trim_start().starts_with("//")),
            "AC 2: panel.rs must not name `{needle}`"
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
/// Two things keep this scan from matching its own source, and it is worth
/// being precise about which does the work. First, `files_containing` ignores
/// any line whose first non-space characters are `//`, so a retired name
/// written in a doc comment — here or anywhere else in the tree — is invisible
/// to it. Second, and not relying on the first, the needles below are assembled
/// with `concat!` and the assertion message interpolates `{gone}` at runtime, so
/// neither retired name appears as a literal in this file's compiled text
/// either. The pairs are `(retired, replacement)`, and the replacement doubles
/// as the positive control: if the haystack were empty or mis-read, the control
/// fails first.
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
