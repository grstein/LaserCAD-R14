//! LCV-180 — dependency refresh. The renderer stays `glow`, the agent
//! transport stays blocking on `native-tls`, and `tokio` is never a direct
//! dependency. Scans of `Cargo.toml` and `Cargo.lock`, read fresh from disk.

use std::fs;
use std::path::Path;

/// Contents of a repository file; panics with the path when unreadable.
fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("must be able to read {}: {e}", path.display()))
}

/// The one-line declaration `<name> = …` of a dependency in `Cargo.toml`.
fn dependency_line(manifest: &str, name: &str) -> String {
    let prefix = format!("{name} =");
    manifest
        .lines()
        .find(|l| l.trim_start().starts_with(&prefix))
        .unwrap_or_else(|| panic!("Cargo.toml must declare {name} on one line"))
        .to_owned()
}

/// LCV-180 AC 5 — the lock file resolves no `wgpu` crate: eframe runs on glow.
#[test]
fn cargo_lock_resolves_no_wgpu() {
    let lock = read("Cargo.lock");
    for line in lock.lines() {
        let Some(name) = line.strip_prefix("name = ") else {
            continue;
        };
        let name = name.trim_matches('"');
        assert!(
            !name.starts_with("wgpu") && name != "egui-wgpu",
            "Cargo.lock must not resolve {name}"
        );
    }
}

/// LCV-180 AC 1/5 — eframe drops its default features (which pull wgpu) and
/// lists `glow`; `winit` is declared for feature unification.
#[test]
fn eframe_uses_glow_without_default_features() {
    let manifest = read("Cargo.toml");
    let eframe = dependency_line(&manifest, "eframe");
    assert!(
        eframe.contains("default-features = false"),
        "eframe must set default-features = false: {eframe}"
    );
    assert!(
        eframe.contains("\"glow\""),
        "eframe must list glow: {eframe}"
    );
    assert!(
        !eframe.contains("wgpu"),
        "eframe must not list wgpu: {eframe}"
    );
    dependency_line(&manifest, "winit");
}

/// LCV-180 AC 6 — the agent transport keeps the blocking reqwest client on
/// native-tls, and `tokio` is not a direct dependency.
#[test]
fn reqwest_stays_blocking_on_native_tls_without_tokio() {
    let manifest = read("Cargo.toml");
    let reqwest = dependency_line(&manifest, "reqwest");
    for feature in ["\"blocking\"", "\"native-tls\""] {
        assert!(
            reqwest.contains(feature),
            "reqwest must list {feature}: {reqwest}"
        );
    }
    assert!(
        !manifest
            .lines()
            .any(|l| l.trim_start().starts_with("tokio")),
        "Cargo.toml must not declare tokio"
    );
}
