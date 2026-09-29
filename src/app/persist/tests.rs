use super::*;
use crate::document::{Document, Entity};
use crate::geometry::{Line, Vec2};
use crate::io::autosave::load_autosave_from;
use crate::io::settings::load_from;
use std::path::{Path, PathBuf};

/// A private, empty directory under the system temp dir, named after the
/// test that owns it so parallel tests never share one. Any leftovers
/// from a previous run are removed first, which is what makes the
/// `read_dir(..).count() == 0` assertions below meaningful.
fn tempdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lcv119_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn doc_with_a_line() -> Document {
    let mut doc = Document::default();
    doc.entities.push(Entity::Line(Line::new(
        Vec2::new(1.0, 2.0),
        Vec2::new(30.0, 40.0),
    )));
    doc
}

// --- AC 6 --------------------------------------------------------------

/// LCV-119 AC 6 — both halves in one test so neither can be read without
/// the other: an injected path really persists (the positive control),
/// and a `None` path writes **nothing at all**, proved by counting the
/// directory rather than by checking one expected filename — a stray
/// `.tmp` staging file or a `.bak` sibling would fail this too.
#[test]
fn an_injected_path_persists_and_none_persists_nothing() {
    let dir = tempdir("persist_roundtrip");

    // --- injected: the writes land -------------------------------------
    let mut app = App {
        settings_path: Some(dir.join("settings.json")),
        autosave_path: Some(dir.join("autosave.json")),
        document: doc_with_a_line(),
        ..App::default()
    };
    app.settings.default_bed_mm = [321.0, 123.0];
    app.settings.push_recent_file("/tmp/lcv119.svg".into());

    app.persist_settings();
    assert!(
        dir.join("settings.json").is_file(),
        "an injected settings_path must produce a file"
    );
    assert_eq!(
        load_from(&dir.join("settings.json")),
        app.settings,
        "and the bytes on disk must parse back to what the App carries"
    );

    assert!(
        app.write_autosave(),
        "an injected autosave_path must report a write"
    );
    assert_eq!(
        load_autosave_from(&dir.join("autosave.json"))
            .expect("the autosave file must be readable back")
            .entities,
        app.document.entities,
    );

    // --- None: nothing happens -----------------------------------------
    let dir = tempdir("persist_nothing");
    let app = App {
        settings_path: None,
        autosave_path: None,
        document: doc_with_a_line(),
        ..App::default()
    };

    app.persist_settings();
    assert!(
        !app.write_autosave(),
        "a pathless App must report that it wrote nothing"
    );
    app.clear_autosave();

    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        0,
        "a pathless App must leave the filesystem untouched"
    );
}

// --- AC 7 --------------------------------------------------------------

/// LCV-119 AC 7 — `clear_autosave` removes the injected file, and calling
/// it again on the now-missing file is a silent no-op (the file action
/// that calls it must not fail because there was nothing to tidy up).
#[test]
fn clear_autosave_removes_the_injected_file_and_tolerates_a_missing_one() {
    let dir = tempdir("clear_autosave");
    let path = dir.join("autosave.json");
    std::fs::write(&path, b"{}").unwrap();

    let app = App {
        autosave_path: Some(path.clone()),
        ..App::default()
    };

    app.clear_autosave();
    assert!(!path.exists(), "the injected autosave file must be removed");

    app.clear_autosave();
    assert!(!path.exists(), "a second call must not resurrect or panic");
}

/// LCV-119 AC 7, the other side of the guard — a pathless `App` must not
/// delete *anything*, which is the exact defect this demand closes:
/// `cargo test` was unlinking the developer's real recovery file.
#[test]
fn clear_autosave_with_no_path_deletes_nothing() {
    let dir = tempdir("clear_no_path");
    let bystander = dir.join("autosave.json");
    std::fs::write(&bystander, b"{\"probe\":\"lcv119\"}").unwrap();

    App::default().clear_autosave();

    assert_eq!(
        std::fs::read(&bystander).unwrap(),
        b"{\"probe\":\"lcv119\"}",
        "a pathless App must not touch a file it was never given"
    );
}

// --- AC 1, 4, 5, 8, 9 — the static invariants --------------------------

/// Every `.rs` file under `src/`, paired with its **implementation**
/// section only: everything before the bare `#[cfg(test)]` at column 0
/// (ADR 0004's anchor). Bounding every scan below to this is what stops a
/// test body — including this file's own — from satisfying its own
/// assertion.
///
/// The tree is walked from `CARGO_MANIFEST_DIR`, baked in at compile
/// time, so the scans cover files that do not exist yet rather than a
/// hand-maintained list that a new module could quietly escape.
fn implementation_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
                && !path.ends_with("tests.rs")
            {
                let src = std::fs::read_to_string(&path).unwrap();
                let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
                let name = path.to_string_lossy().replace('\\', "/");
                let name = name.rsplit_once("/src/").unwrap().1.to_owned();
                out.push((name, src[..end].to_owned()));
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &mut out);
    assert!(
        out.len() > 30,
        "positive control: the walk must actually find the source tree (found {})",
        out.len()
    );
    out
}

/// LCV-119 AC 5 — this file and `App::new` are the only readers of the
/// two injected path fields. If a fourth reader appears, the one-grep
/// invariant that keeps `ProjectDirs` out of product logic is gone.
///
/// The needles are assembled with `concat!` so neither field name appears
/// whole in this test's own source; combined with the implementation-only
/// haystacks, the scan cannot match itself twice over.
#[test]
fn the_two_path_fields_have_only_three_readers() {
    let settings_needle = concat!("settings", "_path");
    let autosave_needle = concat!("autosave", "_path");
    let allowed = ["app/mod.rs", "app/init.rs", "app/persist.rs"];

    let mut seen = Vec::new();
    for (name, implementation) in implementation_sources() {
        let mentions =
            implementation.contains(settings_needle) || implementation.contains(autosave_needle);
        if allowed.contains(&name.as_str()) {
            assert!(
                mentions,
                "positive control: {name} must mention the path fields — if it no \
                     longer does, this scan has stopped proving anything"
            );
            seen.push(name);
        } else {
            assert!(
                !mentions,
                "{name} reads an injected path field; only {allowed:?} may (AC 5)"
            );
        }
    }
    assert_eq!(seen.len(), 3, "all three allowed files must exist");
}

/// LCV-119 AC 1 — `ProjectDirs` is resolved in exactly two files. A third
/// occurrence anywhere under `src/` is the defect coming back.
#[test]
fn project_dirs_is_resolved_in_exactly_two_files() {
    let needle = concat!("ProjectDirs", "::from");
    let resolvers: Vec<String> = implementation_sources()
        .into_iter()
        .filter(|(_, implementation)| implementation.contains(needle))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        resolvers,
        vec![
            "io/autosave.rs".to_owned(),
            "io/settings_store.rs".to_owned()
        ],
        "only the two boot resolvers may construct ProjectDirs (AC 1)"
    );
}

/// LCV-119 AC 4, AC 8, AC 9 — the five pathless wrappers are gone from
/// the tree, definitions and call sites alike. Each absence assertion
/// sits next to a presence assertion over the same haystack, so a scan
/// that has stopped looking at real code fails loudly instead of passing
/// vacuously.
#[test]
fn the_pathless_wrappers_have_no_definition_and_no_call_site() {
    let banned = [
        concat!("crate::io::", "save_autosave("),
        concat!("crate::io::", "load_autosave("),
        concat!("crate::io::", "clear_autosave("),
        concat!("io::", "clear_autosave();"),
        concat!("settings", ".save()"),
        concat!("Settings::", "load()"),
    ];
    for (name, implementation) in implementation_sources() {
        for needle in banned {
            assert!(
                !implementation.contains(needle),
                "{name} still reaches the pathless wrapper `{needle}` (AC 4/8/9)"
            );
        }
    }

    // Definition-level: the wrappers themselves, in the io modules. The
    // settings file I/O moved to `settings_store.rs` (LCV-143, ADR 0007
    // §D8); `settings.rs` keeps the struct and must not regrow them.
    for (file, src, present, absent) in [
        (
            "io/settings_store.rs",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/io/settings_store.rs"
            )),
            concat!("pub(crate) fn ", "save_to("),
            vec![
                concat!("pub fn ", "save(&self)"),
                concat!("pub fn ", "load()"),
            ],
        ),
        (
            "io/settings.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/settings.rs")),
            concat!("pub struct ", "Settings"),
            vec![
                concat!("pub fn ", "save(&self)"),
                concat!("pub fn ", "load()"),
            ],
        ),
        (
            "io/autosave.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/autosave.rs")),
            concat!("pub(crate) fn ", "save_autosave_to("),
            vec![
                concat!("pub fn ", "save_autosave("),
                concat!("pub fn ", "load_autosave("),
                concat!("pub fn ", "clear_autosave("),
            ],
        ),
    ] {
        let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
        let implementation = &src[..end];
        assert!(
            implementation.contains(present),
            "positive control: {file} must still define `{present}`"
        );
        for needle in absent {
            assert!(
                !implementation.contains(needle),
                "{file} must not define the pathless wrapper `{needle}` (AC 4)"
            );
        }
    }
}

/// LCV-119 AC 4 — the `NoPlatformPath` variants describe a failure that
/// can no longer occur: nothing below boot resolves a platform path, so
/// no write can fail for want of one. An error variant for an impossible
/// failure is a documentation lie.
#[test]
fn no_platform_path_error_variants_remain() {
    for (file, src) in [
        (
            "io/settings.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/settings.rs")),
        ),
        (
            "io/autosave.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/io/autosave.rs")),
        ),
    ] {
        let end = src.find("\n#[cfg(test)]").unwrap_or(src.len());
        let implementation = &src[..end];
        assert!(
            implementation.contains("    Io(#[from] std::io::Error),"),
            "positive control: {file}'s error enum must still be in the haystack"
        );
        assert!(
            !implementation.contains(concat!("NoPlatform", "Path")),
            "{file} must not keep an error variant for an impossible failure (AC 4)"
        );
    }
}
