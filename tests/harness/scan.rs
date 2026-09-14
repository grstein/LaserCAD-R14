//! tests/harness/scan.rs — the source-scan walker and matcher, once.
//!
//! Five integration tests carried a byte-identical `rs_files` and four carried
//! a matcher that differed only in whether it returned presence or counts.
//! Both live here now. What does **not** live here is the walk-and-slice
//! wrapper each caller builds on top: its root, its bound, its return shape and
//! the shape of its positive control are decisions its own demand made, and a
//! `sections(dir, bound, minimum, per_directory_controls, return_root)` would
//! be a worse artefact than the duplication it deletes (LCV-132 §Out of scope).
//!
//! Four hard rules, each with the reason it exists. A scan that breaks one of
//! them still passes; that is the whole problem with scans.
//!
//! 1. **Bound the haystack at the bare `#[cfg(test)]` at column 0.** A scan
//!    whose haystack includes its own inline tests matches the needle it just
//!    wrote down and reports success forever. The anchor is `"\n#[cfg(test)]"`
//!    — column 0, nothing else on the line — because a pattern matching
//!    `#[cfg(test)]` anywhere also matches doc-comment prose (ADR 0004).
//!    The one documented exception is LCV-128 AC 1, which scans whole files on
//!    purpose: a kernel *unit test* that imports `egui` breaks the same
//!    headless promise the purity rule is about, so bounding there would hide
//!    exactly the violation being looked for.
//! 2. **Build every needle with `concat!`.** Otherwise the literal sits in the
//!    scanning file's own compiled text, and the day that file is added to its
//!    own haystack the scan matches itself. `concat!` splices at compile time
//!    and leaves no whole-string copy behind.
//! 3. **Every absence assertion carries a positive control run through the
//!    same helper.** An absence assertion over an empty or mis-sliced haystack
//!    passes for the wrong reason, and it passes silently. The canonical
//!    example is `guard_is_runtime_not_cfg` in `src/io/dialogs.rs`.
//! 4. **Rebuild a compared path from `components()` joined with `/`.** Never
//!    `Path::display()` or `to_string_lossy()` on the whole path: those emit
//!    `\` on Windows, so the comparison passes on Linux and macOS and fails
//!    only in CI. That has broken this repository's CI twice. Rendering a path
//!    into a *failure message* is fine; comparing one is not. Sort on the
//!    rendered string too, so the order cannot depend on where the separator
//!    sorts.
//!
//! ### The comment-skipping trade-off, and what it costs
//!
//! [`occurrences`] ignores any line whose first non-space characters are `//`,
//! so these scans are about what the compiler sees rather than about prose.
//! That is deliberate and load-bearing: `src/agent/mod.rs`'s module header
//! names `reqwest`, `Document` and `History` *precisely in order to say that no
//! file there may hold one*, and a scan that counted a header as a use would
//! force every rule in this repository to go undocumented in order to stay
//! true.
//!
//! The cost is real and was measured: prose is **unscanned**, so a header can
//! drift out of step with the code it describes and nothing here notices —
//! LCV-123's review caught exactly that in `src/agent/mod.rs`. Skipping
//! comments is still the right trade; it just is not free.

use std::path::{Path, PathBuf};

/// Every `.rs` file under `dir`, recursively.
///
/// Recursion is not a detail: `src/io/svg/` and `src/document/commands/` are
/// two levels down, so a flattened walk scans neither while every positive
/// control that counts whole-tree files still passes.
pub fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("the source tree must be readable") {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every `(path, count)` whose section contains `needle` on a **code** line,
/// in the order `sections` was given.
///
/// Comment lines are skipped — see the module header for why that trade is
/// deliberate and what it costs. A count rather than a flag, because
/// "`agent_busy = false` appears once, in `agent_poll.rs`" is a strictly
/// stronger claim than "it appears in `agent_poll.rs`", and a second write
/// smuggled into the owning file is exactly the regression the weaker claim
/// misses.
pub fn occurrences(sections: &[(String, String)], needle: &str) -> Vec<(String, usize)> {
    sections
        .iter()
        .filter_map(|(path, body)| {
            let hits: usize = body
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .map(|line| line.matches(needle).count())
                .sum();
            (hits > 0).then(|| (path.clone(), hits))
        })
        .collect()
}

/// The paths [`occurrences`] reported, dropping the counts.
///
/// **Derived, never re-implemented.** The rule about which lines count exists
/// once, in [`occurrences`]; a second independent filter here is how the two
/// drift apart, and a drifted comment rule is invisible to every test that
/// only ever calls one of them.
pub fn files_containing(sections: &[(String, String)], needle: &str) -> Vec<String> {
    occurrences(sections, needle)
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}
