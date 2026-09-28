//! LCV-152 — the suite links as one integration binary.
//!
//! Cargo turns every `tests/<name>.rs` into its own binary, each linked against
//! egui; a stray one silently brings back the per-file relink this demand
//! removed. So `tests/` may hold only the `it/` binary and the shared
//! `harness/`, and every file in `tests/it/` must be a declared module of
//! `tests/it/main.rs` — an undeclared one is never compiled and its tests never
//! run, with nothing red to say so.

use std::path::Path;

/// The names of the entries directly in `dir`, split into `(dirs, files)`, sorted.
fn entries(dir: &Path) -> (Vec<String>, Vec<String>) {
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    for entry in std::fs::read_dir(dir).expect("the directory must be readable") {
        let path = entry.expect("a readable directory entry").path();
        let name = path
            .file_name()
            .expect("an entry has a name")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            dirs.push(name);
        } else {
            files.push(name);
        }
    }
    dirs.sort();
    files.sort();
    (dirs, files)
}

/// AC 1 — `tests/` holds no top-level `.rs` file, only `it/` and `harness/`.
#[test]
fn ac1_tests_holds_only_the_one_binary_and_the_harness() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let (dirs, files) = entries(&root);
    assert_eq!(
        files,
        Vec::<String>::new(),
        "a file directly in tests/ is its own test binary (or clutter); put the \
         test in tests/it/ and declare it in tests/it/main.rs"
    );
    assert_eq!(
        dirs,
        ["harness", "it"],
        "tests/ holds only it/ and harness/"
    );
}

/// AC 1 — every module file in `tests/it/` is declared in `tests/it/main.rs`.
#[test]
fn ac1_every_file_in_tests_it_is_a_declared_module() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/it");
    let main = std::fs::read_to_string(dir.join("main.rs")).expect("tests/it/main.rs");
    let (dirs, files) = entries(&dir);
    assert_eq!(dirs, Vec::<String>::new(), "tests/it/ is flat");
    let undeclared: Vec<&String> = files
        .iter()
        .filter(|f| f.as_str() != "main.rs")
        .filter(|f| {
            let module = f.strip_suffix(".rs").unwrap_or(f);
            !main.lines().any(|l| l == format!("mod {module};"))
        })
        .collect();
    assert!(
        files.len() > 20,
        "positive control: tests/it/ holds the whole suite, saw {}",
        files.len()
    );
    assert_eq!(
        undeclared,
        Vec::<&String>::new(),
        "these files are never compiled: add `mod <name>;` to tests/it/main.rs"
    );
}
