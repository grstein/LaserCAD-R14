//! LCV-201 — Windows and macOS packaging. Packaging is shell scripts, CI YAML
//! and docs, so most criteria are text scans of those files, read fresh from
//! disk. `release.sh --list-assets` is the one behavioural check (Unix only).

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

/// LCV-201 AC 2 — release builds on Windows use the GUI subsystem (no console
/// window); debug builds keep the console.
#[test]
fn main_rs_hides_console_in_windows_release_builds() {
    let main = read("src/main.rs");
    assert!(
        main.contains(
            r#"#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]"#
        ),
        "src/main.rs must gate windows_subsystem on all(windows, not(debug_assertions))"
    );
}

/// LCV-201 AC 1 — `scripts/build-zip.ps1` reads the version from `Cargo.toml`,
/// stages the executable, both licences and `FIRST-RUN.txt`, and writes the
/// versioned `.zip`; the unverified WiX/MSI path is gone.
#[test]
fn build_zip_ps1_stages_the_portable_windows_zip() {
    let script = read("scripts/build-zip.ps1");
    for needle in [
        "Cargo.toml",
        "lasercad.exe",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "assets/FIRST-RUN.txt",
        "Compress-Archive",
        "lasercad-$Version-windows-x86_64.zip",
    ] {
        assert!(script.contains(needle), "build-zip.ps1 must mention {needle:?}");
    }
    assert!(repo("assets/FIRST-RUN.txt").is_file(), "assets/FIRST-RUN.txt must exist");
    assert!(repo("LICENSE-APACHE").is_file() && repo("LICENSE-MIT").is_file());
    assert!(!repo("scripts/build-msi.ps1").exists(), "build-msi.ps1 must be deleted");
    assert!(!repo("wix").exists(), "wix/ must be deleted");
}

/// LCV-201 AC 3 — `scripts/build-dmg.sh` builds `LaserCAD.app` on Apple
/// Silicon only, ad-hoc signs it, ships `FIRST-RUN.txt` and writes the
/// versioned `.dmg`. On a Linux host it must refuse before building anything:
/// it runs from a scratch copy with no `Cargo.toml`, so a broken guard fails
/// fast instead of starting a release build.
#[test]
fn build_dmg_sh_bundles_signs_and_refuses_non_arm64() {
    let script = read("scripts/build-dmg.sh");
    for needle in [
        "LaserCAD.app",
        "CFBundleShortVersionString",
        "iconutil",
        "codesign --force --deep -s -",
        "FIRST-RUN.txt",
        "lasercad-${VERSION}-macos-aarch64.dmg",
    ] {
        assert!(script.contains(needle), "build-dmg.sh must mention {needle:?}");
    }
    assert!(
        !script.contains("x86_64) ARCH"),
        "build-dmg.sh must not accept Intel hosts"
    );
    if cfg!(target_os = "linux") {
        let syntax = std::process::Command::new("bash")
            .args(["-n", "scripts/build-dmg.sh"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .status()
            .expect("bash must run");
        assert!(syntax.success(), "bash -n scripts/build-dmg.sh must pass");

        let scratch = std::env::temp_dir().join(format!("lcv201-dmg-{}", std::process::id()));
        fs::create_dir_all(scratch.join("scripts")).expect("scratch dir");
        fs::copy(repo("scripts/build-dmg.sh"), scratch.join("scripts/build-dmg.sh"))
            .expect("copy build-dmg.sh");
        let out = std::process::Command::new("bash")
            .arg("scripts/build-dmg.sh")
            .current_dir(&scratch)
            .output()
            .expect("bash must run");
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let built = scratch.join("dist").exists() || scratch.join("build").exists();
        let _ = fs::remove_dir_all(&scratch);
        assert!(!out.status.success(), "build-dmg.sh must refuse a Linux host");
        assert!(stderr.contains("arm64"), "refusal must name arm64: {stderr}");
        assert!(!built, "build-dmg.sh must refuse before creating build/ or dist/");
    }
}

/// The text of one top-level job in `ci.yml`: from `  <name>:` at two-space
/// indent to the next two-space job key (or end of file).
fn ci_job(ci: &str, name: &str) -> String {
    let head = format!("\n  {name}:\n");
    let start = ci
        .find(&head)
        .unwrap_or_else(|| panic!("ci.yml must have a `{name}` job"));
    let body = &ci[start + head.len()..];
    let end = body
        .lines()
        .scan(0usize, |off, line| {
            let at = *off;
            *off += line.len() + 1;
            Some((at, line))
        })
        .find(|(_, l)| l.starts_with("  ") && !l.starts_with("   ") && l.ends_with(':'))
        .map_or(body.len(), |(at, _)| at);
    body[..end].to_owned()
}

/// LCV-201 AC 4, AC 9 — the `package` job runs on a tag or a manual dispatch,
/// Windows packages with `build-zip.ps1` (no `cargo-wix`), uploads and the
/// release use the new artifact names, and the `test` matrix covers
/// `windows-2022` and `macos-15` on dispatch.
#[test]
fn ci_packages_on_dispatch_with_zip_and_dmg() {
    let ci = read(".github/workflows/ci.yml");
    assert!(ci.contains("\n  workflow_dispatch:"), "ci.yml must accept workflow_dispatch");
    assert!(!ci.contains("wix") && !ci.contains(".msi"), "ci.yml must not use WiX/MSI");

    let package = ci_job(&ci, "package");
    assert!(
        package.contains(
            "if: startsWith(github.ref, 'refs/tags/') || github.event_name == 'workflow_dispatch'"
        ),
        "package must run on tags and on workflow_dispatch"
    );
    assert!(package.contains("./scripts/build-zip.ps1"), "Windows step must run build-zip.ps1");
    assert!(package.contains("./scripts/build-dmg.sh"), "macOS step must run build-dmg.sh");
    for path in ["dist/lasercad-*-windows-x86_64.zip", "dist/lasercad-*-macos-aarch64.dmg"] {
        assert!(package.contains(path), "package must upload {path}");
    }

    let release = ci_job(&ci, "release");
    assert!(release.contains("if: startsWith(github.ref, 'refs/tags/')\n"));
    for path in ["dist/lasercad-*-windows-x86_64.zip", "dist/lasercad-*-macos-aarch64.dmg"] {
        assert!(release.contains(path), "release must attach {path}");
    }

    let test = ci_job(&ci, "test");
    let matrix = test
        .lines()
        .find(|l| l.trim_start().starts_with("os:"))
        .expect("test job must have an os matrix");
    for needle in ["github.event_name == 'workflow_dispatch'", "windows-2022", "macos-15"] {
        assert!(matrix.contains(needle), "test matrix must include {needle:?}");
    }
}
