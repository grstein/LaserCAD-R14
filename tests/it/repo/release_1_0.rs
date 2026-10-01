//! LCV-202 — the 1.0 release paperwork: the v1 parity table, the smoke
//! checklist, the README and the version. Text scans of files read fresh
//! from disk, in the style of `packaging.rs`.

use std::fs;
use std::path::{Path, PathBuf};

/// Absolute path of a repository-relative file.
fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Contents of a repository file; panics with the path when unreadable.
fn read(rel: &str) -> String {
    let path = repo(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {e}", path.display()))
}

/// The lines of `text` under the `## <heading>…` heading, up to the next `## `.
fn section<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    text.lines()
        .skip_while(|l| !l.starts_with(&format!("## {heading}")))
        .skip(1)
        .take_while(|l| !l.starts_with("## "))
        .collect()
}

/// The cells of every body row of the first Markdown table in `lines`.
fn table_rows(lines: &[&str]) -> Vec<Vec<String>> {
    lines
        .iter()
        .map(|l| l.trim())
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .skip(2) // header and separator
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_owned())
                .collect()
        })
        .collect()
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The backticked words of `cell`.
fn backticked(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

/// LCV-202 AC 1 — `docs/product/parity-1-0.md` has one row per bullet of
/// `docs/product/README.md` §Scope target, no empty cell, and every test it
/// cites exists as a `fn` under `src/` or `tests/`.
#[test]
fn parity_table_covers_every_v1_capability_with_real_tests() {
    let readme = read("docs/product/README.md");
    let bullets = section(&readme, "Scope target")
        .iter()
        .filter(|l| l.starts_with("- "))
        .count();
    assert!(bullets > 0, "§Scope target lists the v1 capabilities");
    let parity = read("docs/product/parity-1-0.md");
    let lines: Vec<&str> = parity.lines().collect();
    let rows = table_rows(&lines);
    assert_eq!(
        rows.len(),
        bullets,
        "one parity row per §Scope target bullet"
    );
    let mut sources = Vec::new();
    rust_files(&repo("src"), &mut sources);
    rust_files(&repo("tests"), &mut sources);
    let code: Vec<String> = sources
        .iter()
        .filter_map(|p| fs::read_to_string(p).ok())
        .collect();
    for row in &rows {
        assert_eq!(
            row.len(),
            3,
            "capability | v2 command or menu | tests: {row:?}"
        );
        assert!(row.iter().all(|c| !c.is_empty()), "empty cell: {row:?}");
        let tests = backticked(&row[2]);
        assert!(!tests.is_empty(), "no test cited: {row:?}");
        for name in tests {
            let decl = format!("fn {name}(");
            assert!(
                code.iter().any(|c| c.contains(&decl)),
                "cited test `{name}` does not exist ({})",
                row[0]
            );
        }
    }
}

/// LCV-202 AC 6 — `docs/release/smoke-1-0.md` is a numbered checklist for
/// Linux, Windows and macOS covering draw, edit, snap, layers, save,
/// reopen, export layers and opening the result in LaserGRBL.
#[test]
fn smoke_checklist_covers_the_three_systems_and_the_whole_flow() {
    let smoke = read("docs/release/smoke-1-0.md");
    let steps: Vec<String> = smoke
        .lines()
        .map(str::trim_start)
        .filter(|l| {
            let digits = l.chars().take_while(char::is_ascii_digit).count();
            digits > 0 && l[digits..].starts_with(". ")
        })
        .map(str::to_lowercase)
        .collect();
    assert!(steps.len() >= 8, "a numbered checklist: {steps:?}");
    for os in ["Linux", "Windows", "macOS"] {
        assert!(smoke.contains(os), "smoke checklist must name {os}");
    }
    for topic in [
        "draw",
        "edit",
        "snap",
        "layer",
        "save",
        "reopen",
        "export layers",
        "lasergrbl",
    ] {
        assert!(
            steps.iter().any(|s| s.contains(topic)),
            "no numbered step covers {topic:?}"
        );
    }
}
