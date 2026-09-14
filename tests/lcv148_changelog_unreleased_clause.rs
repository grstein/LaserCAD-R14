//! LCV-148 AC 11 — the CHANGELOG no longer tells operators the pre-flip rule:
//! that a line the CAD grammar does not recognise reaches the model whenever
//! an API key is configured. ADR 0007 §D9 rule 4 (amendment (5)) closed that
//! for the *code*; this test is the one check that the *prose* caught up too.
//! The routing itself is pinned in `src/agent/classifier.rs` and
//! `tests/lcv124_command_line_routing.rs` — this file only checks the words
//! that describe it in `CHANGELOG.md`.
//!
//! **Bounded, not whole-file.** The haystack is the `## [Unreleased]` section:
//! from that heading to the next bare `\n## ` at column 0, or to end of file
//! when there is none. Today there is exactly **one** column-0 `## ` heading
//! in the whole file (`## [Unreleased]`), so the section runs to EOF — a
//! bound that assumed a second heading always exists would silently scan an
//! empty slice on the file as it stands today, which is bug class 8: a
//! criterion that is green on the exact defect it exists to catch. The
//! `slice_is_shorter_than_the_whole_file` control below is what would catch
//! that regression the day a `## [1.0.0]` heading is finally added.
//!
//! **The needle is split inside a word.** `concat!("goes to the model inst",
//! "ead of drawing")` only reads as the stale phrase once concatenated; no
//! literal in *this* file's own source can satisfy the scan it performs on a
//! document it does not itself embed.

use std::fs;
use std::path::Path;

/// The repository-root `CHANGELOG.md`, read fresh from disk (not
/// `include_str!`): this test's own source text never enters the haystack it
/// scans, so bug class 1 — a scan that matches its own needle — cannot apply
/// here regardless of how the needle is built.
fn changelog() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("CHANGELOG.md");
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {e}", path.display()))
}

/// The `## [Unreleased]` section: from that heading to the next bare
/// `"\n## "` at column 0, or to the end of the file when there is none.
///
/// A line reading `### Added` can never satisfy `"\n## "`: the third
/// character after the newline is `#`, not a space, so a level-3 heading
/// cannot be mistaken for the level-2 boundary this function is looking for.
fn unreleased_section(changelog: &str) -> &str {
    let start = changelog
        .find("## [Unreleased]")
        .expect("CHANGELOG.md must have an `## [Unreleased]` heading");
    let rest = &changelog[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}

/// AC 11 — the stale clause is gone from the section that carries it, shown
/// to discriminate by two positive controls run over the **same** slice with
/// the **same** kind of search: if `unreleased_section` silently returned
/// nothing (or the whole file), one of them fails before the real assertion
/// is trusted.
#[test]
fn the_unreleased_section_no_longer_promises_a_paid_round_trip_for_unknown_text() {
    let changelog = changelog();
    let section = unreleased_section(&changelog);

    assert!(
        !section.is_empty(),
        "positive control: the section must not be empty"
    );
    assert!(
        section.contains("LCV-124"),
        "positive control: the section must still name LCV-124"
    );
    assert!(
        section.len() < changelog.len(),
        "positive control: the section must be a strict subset of the whole file \
         (today the file has exactly one column-0 `## ` heading; this is the \
         guard against a bound that silently returns the whole document)"
    );

    let stale = concat!("goes to the model inst", "ead of drawing");
    assert!(
        !section.contains(stale),
        "the [Unreleased] section still promises the pre-LCV-148 rule: {stale:?}"
    );
}
