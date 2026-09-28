//! LCV-128 — scans for `AGENTS.md`'s normative per-file and per-symbol
//! enumerations.
//!
//! `AGENTS.md` states several architecture rules as enumerations of specific
//! files: the kernel purity list, and the `src/agent/` purity buckets. Each
//! entry in those lists carries a rule *for that name*, and a new file can
//! silently escape the list with nothing failing. This file closes the two
//! gaps the demand's own coverage census found still open:
//!
//! - **AC 1** — the kernel purity rule (§Purity rule's five directories) has
//!   no scan at all.
//! - **AC 2** — the `src/agent/` purity buckets name eight of the directory's
//!   nine files; `mod.rs` is in neither, though `AGENTS.md` now (LCV-128)
//!   states a rule for it.
//!
//! **AC 3 is not duplicated here.** It overlaps LCV-129 AC 5 by one needle
//! (`agent.busy = false`), and that demand landed first: its scan in
//! `tests/lcv129_agent_timeout_and_cancel.rs::ac5_only_agent_poll_clears_the_busy_flag`
//! was extended in place with the other two single-writer needles
//! (`agent.rx = None`, `agent.busy = true`) rather than copied here, per the
//! demand's own sequencing note.
//!
//! Every scan follows the two rules that keep a scan able to fail (AGENTS.md;
//! the canonical example is `guard_is_runtime_not_cfg` in `src/io/dialogs.rs`):
//! needles are assembled with `concat!`, and every absence assertion carries a
//! positive control run through the same helper, because an absence assertion
//! over an empty or mis-sliced haystack passes for the wrong reason.
//!
//! **AC 1 is the one place in this file that scans whole files rather than
//! the `#[cfg(test)]`-bounded implementation slice** that every other scan in
//! this repository uses (`tests/lcv121_source_scans.rs`,
//! `tests/lcv122_source_scans.rs`, the extended AC 5 above). A kernel unit
//! test that imports `egui` breaks the same headless promise the purity rule
//! is about, so bounding the haystack at the test module would hide exactly
//! the violation the rule exists to catch. This is still safe from
//! self-matching: the scan lives in `tests/`, and its haystack is
//! `src/geometry/`, `src/document/`, `src/io/svg/`, `src/text/` and
//! `src/cmdline/`, none of which can ever contain this file. Comment lines
//! are still skipped in both scans below: every one of the kernel's ~15
//! files documents the rule it obeys in a `//!` header, and on `main` today
//! that header is the *only* occurrence of `egui`/`eframe`/`rfd` in any of
//! the five directories — a scan that read comments would fail every kernel
//! file that honestly states the rule it follows (see `AGENTS.md` §Purity
//! rule and this demand's §Out of scope).
//!
//! Relative paths are rebuilt from `components()` joined with `/` and sorted
//! on the rendered string: `Path::display()` emits `\` on Windows and turns a
//! comparison like this into a CI-only failure (AGENTS.md §Implementation
//! Rules).
//!
//! The walker and the matcher are `tests/harness/scan.rs`'s (LCV-132). The
//! unbounded, five-per-directory-control wrapper below stays here: scanning
//! whole files and failing by directory name are this demand's decisions, and
//! they are the ones that make its two scans mean what they say.

use crate::harness;

use harness::scan::{files_containing, rs_files};
use std::path::Path;

/// The five kernel directories AGENTS.md §Purity rule names.
const KERNEL_DIRS: [&str; 5] = ["geometry", "document", "io/svg", "text", "cmdline"];

/// Every `.rs` file's **whole** text (not the `#[cfg(test)]`-bounded slice —
/// see the module header) under each of the five kernel directories, as
/// (relative path, text) pairs sorted on the rendered path.
///
/// Each directory's file count is asserted individually — not just the total
/// — so a typo'd directory name fails naming that directory, rather than the
/// walk silently scanning nothing while the other four still carry it.
fn kernel_sections() -> Vec<(String, String)> {
    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for dir in KERNEL_DIRS {
        let root = src_root.join(dir);
        let mut files = Vec::new();
        rs_files(&root, &mut files);
        assert!(
            !files.is_empty(),
            "positive control: src/{dir} must contribute at least one file to the kernel \
             purity scan, saw 0 — a typo'd directory name scans nothing silently"
        );
        for path in files {
            let relative = path
                .strip_prefix(&src_root)
                .expect("every walked file is under src/")
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let text = std::fs::read_to_string(&path).expect("a readable source file");
            out.push((relative, text));
        }
    }
    out.sort();
    out
}

/// AC 1 — the kernel purity rule (AGENTS.md §Purity rule, ADR 0001): no file
/// under the five kernel directories names `egui`, `eframe` or `rfd` on a
/// code line.
///
/// Each needle is run against a fixed witness line — built independently of
/// the needle itself, so a misspelt `concat!` (e.g. `"eg", "ux"`) cannot
/// accidentally validate itself — through the *same* `files_containing` that
/// does the real work. A needle that cannot find a real import in the
/// witness fails here, before the (expected-empty) kernel haystack is ever
/// consulted.
#[test]
fn kernel_directories_do_not_import_egui_eframe_or_rfd() {
    let sections = kernel_sections();

    let witness = vec![(
        "witness.rs".to_owned(),
        "use eframe::egui; let f = rfd::FileDialog::new();".to_owned(),
    )];

    for needle in [
        concat!("eg", "ui"),
        concat!("efr", "ame"),
        concat!("rf", "d"),
    ] {
        assert_eq!(
            files_containing(&witness, needle),
            ["witness.rs"],
            "control: `{needle}` must be a needle that can match a real import"
        );

        let offenders = files_containing(&sections, needle);
        assert!(
            offenders.is_empty(),
            "AC 1 / AGENTS.md §Purity rule: no file under the kernel directories may name \
             `{needle}` on a code line, found in {offenders:?}"
        );
    }
}

/// The text of `AGENTS.md` between a `### <heading>` line and the next `###`
/// line (or end of file). Returns an empty string if the heading is not
/// found, so a broken extraction fails the *content* assertions that follow
/// rather than silently scanning the whole document.
fn agents_md_section(heading: &str) -> String {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"))
        .expect("AGENTS.md must be readable");
    let marker = format!("### {heading}");
    let Some(start) = text.find(&marker) else {
        return String::new();
    };
    let after = &text[start + marker.len()..];
    let end = after.find("\n### ").unwrap_or(after.len());
    after[..end].to_owned()
}

/// AC 2 — every file directly under `src/agent/` is named somewhere inside
/// `AGENTS.md`'s `### Purity rule` section.
///
/// **One-directional, deliberately** (the demand's §Scope / §Out of scope): a
/// file that exists and is unnamed fails this test; a name in the section for
/// a file that does not yet exist does not — the section named
/// `classifier.rs` for days before LCV-124 created it, and that forward
/// commitment is a feature. Section-bounded substring only: this does not
/// parse the bullets, does not care which bucket a name sits in, and does not
/// compare wording, so a reviewer's clarity edit to the section's prose
/// cannot turn this red.
#[test]
fn every_file_under_agent_is_named_in_the_purity_section() {
    let section = agents_md_section("Purity rule");

    // Controls: the extraction is real (non-empty, names a file known to be
    // there) and bounded (shorter than the whole document) — so a broken
    // extraction (e.g. a heading that does not exist) fails one of these two
    // assertions rather than the real check passing by scanning all of
    // AGENTS.md, which names every one of these files in the module tree too.
    assert!(
        !section.is_empty() && section.contains("transport.rs"),
        "positive control: the extracted §Purity rule section must be non-empty and name \
         transport.rs, got {section:?}"
    );
    let whole = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"))
        .expect("AGENTS.md must be readable");
    assert!(
        section.len() < whole.len(),
        "control: the extracted section ({} bytes) must be a bounded slice of the whole \
         document ({} bytes), not the whole document itself",
        section.len(),
        whole.len()
    );

    let agent_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/agent");
    let mut files: Vec<String> = std::fs::read_dir(&agent_dir)
        .expect("src/agent must be readable")
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| {
            path.file_name()
                .expect("a file under src/agent/ has a file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert!(
        files.len() > 5,
        "positive control: src/agent/ has more than a handful of files, saw {}",
        files.len()
    );
    files.sort();

    let unnamed: Vec<&String> = files
        .iter()
        .filter(|name| !section.contains(name.as_str()))
        .collect();
    assert!(
        unnamed.is_empty(),
        "AC 2 / AGENTS.md §Purity rule: every file directly under src/agent/ must be named \
         somewhere in the section; unnamed: {unnamed:?}"
    );
}
