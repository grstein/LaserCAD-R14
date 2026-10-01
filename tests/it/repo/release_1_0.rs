//! LCV-202 — the 1.0 release paperwork: the parity table, the smoke
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
fn parity_table_covers_every_original_capability_with_real_tests() {
    let readme = read("docs/product/README.md");
    let bullets = section(&readme, "Scope target")
        .iter()
        .filter(|l| l.starts_with("- "))
        .count();
    assert!(bullets > 0, "§Scope target lists the original capabilities");
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
            "capability | command or menu | tests: {row:?}"
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

/// LCV-202 AC 8 — the README calls 1.0 stable, links the install and user
/// guides, and lists the non-goals (DXF, G-code, fillet/chamfer/offset,
/// blocks).
#[test]
fn readme_says_stable_links_the_guides_and_lists_the_non_goals() {
    let readme = read("README.md");
    let status = section(&readme, "Status").join("\n");
    assert!(status.contains("Stable"), "README §Status calls 1.0 stable");
    for link in ["(docs/install.md)", "(docs/user-guide.md)"] {
        assert!(readme.contains(link), "README must link {link}");
    }
    let non_goals = section(&readme, "Non-goals").join("\n").to_lowercase();
    for item in ["DXF", "G-code", "fillet", "chamfer", "offset", "blocks"] {
        assert!(
            non_goals.contains(&item.to_lowercase()),
            "README §Non-goals must list {item}"
        );
    }
}

/// LCV-202 AC 8 — the release commit carried version `1.0.0` and a dated
/// `[1.0.0]` CHANGELOG section with entries. Later 1.x releases keep the
/// `[1.0.0]` section, and the current version's dated section follows
/// `[Unreleased]`.
#[test]
fn version_is_1_x_with_a_changelog_section() {
    let version = env!("CARGO_PKG_VERSION");
    assert!(version.starts_with("1."), "1.x release: {version}");
    let cargo = read("Cargo.toml");
    assert!(
        cargo
            .lines()
            .any(|l| l.trim() == format!(r#"version = "{version}""#)),
        "Cargo.toml [package] version"
    );
    let changelog = read("CHANGELOG.md");
    let headings: Vec<&str> = changelog
        .lines()
        .filter(|l| l.starts_with("## ["))
        .take(2)
        .collect();
    assert_eq!(headings.first(), Some(&"## [Unreleased]"));
    assert!(
        headings
            .get(1)
            .is_some_and(|h| h.starts_with(&format!("## [{version}] - "))),
        "[{version}] follows [Unreleased]: {headings:?}"
    );
    let entries = section(&changelog, "[1.0.0]")
        .iter()
        .filter(|l| l.starts_with("- "))
        .count();
    assert!(entries > 0, "[1.0.0] lists its changes");
}
