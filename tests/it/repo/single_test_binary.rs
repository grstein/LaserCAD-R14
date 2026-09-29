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

/// The files in `dir` not declared as `mod <name>;` in `decl` (its
/// `main.rs`/`mod.rs`), and the count of module files checked.
fn undeclared_in(dir: &Path, decl: &str) -> (Vec<String>, usize) {
    let declares = std::fs::read_to_string(dir.join(decl)).expect("a module root");
    let (_, files) = entries(dir);
    let modules: Vec<&String> = files.iter().filter(|f| f.as_str() != decl).collect();
    let undeclared = modules
        .iter()
        .filter(|f| {
            let module = f.strip_suffix(".rs").unwrap_or(f);
            !declares.lines().any(|l| l == format!("mod {module};"))
        })
        .map(|f| {
            format!(
                "{}/{f}",
                dir.file_name().unwrap_or_default().to_string_lossy()
            )
        })
        .collect();
    (undeclared, modules.len())
}

/// AC 1 — every module file in `tests/it/` and its area directories (LCV-155)
/// is declared by its directory's `main.rs`/`mod.rs`; areas hold no deeper
/// directory.
#[test]
fn ac1_every_file_in_tests_it_is_a_declared_module() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/it");
    let (areas, _) = entries(&dir);
    let (mut undeclared, mut total) = undeclared_in(&dir, "main.rs");
    let main = std::fs::read_to_string(dir.join("main.rs")).expect("tests/it/main.rs");
    for area in &areas {
        if !main.lines().any(|l| l == format!("mod {area};")) {
            undeclared.push(format!("{area}/"));
        }
        let area_dir = dir.join(area);
        assert_eq!(
            entries(&area_dir).0,
            Vec::<String>::new(),
            "tests/it/{area}/ is one level: no deeper directory"
        );
        let (missing, count) = undeclared_in(&area_dir, "mod.rs");
        undeclared.extend(missing);
        total += count;
    }
    assert!(
        areas.len() > 5 && total > 20,
        "positive control: tests/it/ holds the whole suite, saw {} areas, {total} files",
        areas.len()
    );
    assert_eq!(
        undeclared,
        Vec::<String>::new(),
        "these files are never compiled: add `mod <name>;` to their main.rs/mod.rs"
    );
}
