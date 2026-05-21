# LCV-058 — Settings store (JSON file via directories)

- **Status**: Ready
- **Phase**: 5
- **Depends on**: LCV-007
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

After a session ends, all user preferences — bed dimensions, snap toggle, grid
visibility, recently opened files — are lost. Every launch resets to hard-coded
defaults, forcing the operator to re-enter the bed size and re-enable snap before
doing any laser-cut geometry work. LaserCAD v2 needs a lightweight settings store
that reads a JSON file on startup and writes it back whenever preferences change,
so the working environment persists across restarts.

## Scope

- **Add `serde`, `serde_json`, and `directories` to `Cargo.toml`.**
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `directories = "5"` (or the latest stable 5.x semver)

- **New file `src/io/settings.rs`** containing:
  - `pub struct Settings` with exactly these fields:
    - `pub bed_width_mm: f64` — laser bed width in millimetres.
    - `pub bed_height_mm: f64` — laser bed height in millimetres.
    - `pub snap_enabled: bool` — snap engine on/off.
    - `pub grid_visible: bool` — grid on/off.
    - `pub recent_files: Vec<String>` — MRU list, capped at 10, most-recent at index 0.
  - Derives: `serde::Serialize`, `serde::Deserialize`, `Debug`, `Clone`, `PartialEq`.
  - `impl Default for Settings` returning:
    `bed_width_mm = 400.0`, `bed_height_mm = 400.0`,
    `snap_enabled = true`, `grid_visible = true`, `recent_files = vec![]`.
  - `pub fn push_recent_file(&mut self, path: String)`:
    - Removes any existing occurrence of `path` from `recent_files`.
    - Inserts `path` at index 0.
    - Truncates `recent_files` to at most 10 entries.
  - `pub(crate) fn load_from(path: &std::path::Path) -> Settings`:
    - Reads the file at `path`.
    - Deserializes with `serde_json::from_str`.
    - Returns `Settings::default()` on **any** error (file missing, IO error,
      parse error, unknown fields). Must not panic.
  - `pub(crate) fn save_to(settings: &Settings, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>>`:
    - Creates all ancestor directories with `std::fs::create_dir_all(parent)`.
    - Serializes with `serde_json::to_string_pretty`.
    - Writes to `path.with_extension("json.tmp")` (i.e. sibling `.tmp` file).
    - Renames the `.tmp` file to `path` atomically with `std::fs::rename`.
    - Returns `Ok(())` on success; propagates any IO error.
  - `pub fn load() -> Settings`:
    - Resolves the platform config path:
      `directories::ProjectDirs::from("", "", "LaserCAD")` →
      `project_dirs.config_dir().join("settings.json")`.
    - If `ProjectDirs::from` returns `None` (sandboxed environment), returns
      `Settings::default()` without panicking.
    - Otherwise delegates to `load_from(&path)`.
  - `pub fn save(settings: &Settings) -> Result<(), Box<dyn std::error::Error>>`:
    - Resolves the same platform config path as `load()`.
    - If `ProjectDirs::from` returns `None`, returns
      `Err("no config dir available".into())`.
    - Otherwise delegates to `save_to(settings, &path)`.
  - Module doc comment: `"Persistent settings — JSON file under the platform config dir."`
  - File size ≤ 200 LOC.
  - **Must NOT import `egui`, `eframe`, or `rfd`.**

- **Update `src/io/mod.rs`**:
  - Add `pub mod settings;`.
  - Re-export: `pub use settings::{Settings, load, save};`.
  - Remove (or update) the placeholder `pub const MODULE: &str = "io";` line if
    it becomes redundant (keeping it is also acceptable).

- **Update `src/app.rs`**:
  - Add `pub settings: Settings` to the `App` struct.
  - Wire `App::default()` so that `settings` initialises to `Settings::default()`
    (the `#[derive(Default)]` on `App` covers this automatically once the field
    is added, because `Settings` implements `Default`).
  - Add `use crate::io::Settings;` (or the full path) to the imports.
  - No other behaviour changes to `App` in this demand.

## Out of scope

- **Autosave** (LCV-059) — the demand establishes the store; the debounced
  autosave timer is a separate concern.
- **Recent files UI** (LCV-060) — `push_recent_file` ships here; the menu
  rendering is owned by LCV-060.
- **Agent settings** (API key, model, endpoint) — owned by LCV-076.
- **Syncing `settings.bed_width_mm / bed_height_mm` into `App::bed`** at
  startup — that wiring belongs to whichever demand introduces the settings
  dialog or the boot sequence (likely LCV-065 / LCV-076). This demand only
  adds the field to `App`; it does not yet read it back into `App::bed`.
- **Snap toggle consumption** — `settings.snap_enabled` is stored but not yet
  read by the snap engine (LCV-054 / LCV-016).
- **Grid visibility toggle consumption** — `settings.grid_visible` is stored
  but not yet read by the grid renderer (LCV-033).
- **Settings dialog / UI** — no egui widgets in this demand.
- **Migration / versioning** of the JSON schema — unknown fields are silently
  ignored by `serde`; schema versioning is out of scope.
- **Windows / macOS path edge cases** — Linux is the primary platform; the
  `directories` crate handles cross-platform resolution but manual testing on
  those platforms is not required here.

## Acceptance criteria

1. `Cargo.toml` lists `serde` (with `features = ["derive"]`), `serde_json`, and
   `directories` as dependencies. `cargo build` succeeds.

2. `src/io/settings.rs` exists and defines `pub struct Settings` with exactly
   the five fields listed in Scope, each with the correct type. Verified by
   inspecting the file and by `cargo build`.

3. `Settings` derives `Serialize`, `Deserialize`, `Debug`, `Clone`, `PartialEq`.
   Verified by a unit test that calls `format!("{:?}", Settings::default())` and
   `.clone()` without compile error, and by a round-trip serialization test.

4. `Settings::default()` returns `bed_width_mm = 400.0`, `bed_height_mm = 400.0`,
   `snap_enabled = true`, `grid_visible = true`, `recent_files` is empty. Each
   field asserted in a unit test.

5. `push_recent_file` inserts the path at index 0, deduplicates (same path pushed
   twice leaves exactly one entry), and caps the list at 10 (pushing an 11th entry
   drops the 11th, keeping 10). Each behaviour asserted in a dedicated unit test.

6. `load_from(path)` returns `Settings::default()` when `path` does not exist
   (no panic, no `unwrap`). Asserted in a unit test using a path that is known
   not to exist (e.g. a temp-dir path that was never written).

7. `load_from(path)` returns `Settings::default()` when `path` contains invalid
   JSON (e.g. `"not json"`). Asserted in a unit test that writes a garbage string
   to a temp file and calls `load_from`.

8. `save_to(settings, path)` followed by `load_from(path)` returns a value equal
   to the original `settings` (round-trip). Asserted in a unit test using a temp
   dir path. Specifically: `save_to(&s, &tmp_path)` → `load_from(&tmp_path)` →
   `assert_eq!(loaded, s)`.

9. `save_to` creates parent directories that do not yet exist. Asserted in a unit
   test that passes a path inside a non-existent subdirectory of `std::env::temp_dir()`.

10. `save_to` writes via an atomic rename: the `settings.json.tmp` sibling is
    written first, then renamed. Asserted indirectly — after `save_to` returns
    `Ok(())`, no `.tmp` file remains alongside the target path.

11. `pub fn load() -> Settings` has the signature `fn load() -> Settings` (not
    `Result`). Verified by `cargo build` and a unit test calling `let _ =
    crate::io::load();` without pattern-matching a `Result`.

12. `src/io/mod.rs` re-exports `Settings`, `load`, and `save` at the `io` module
    root. Verified by a `use crate::io::{Settings, load, save};` import that
    compiles.

13. `App` struct has `pub settings: Settings`. `App::default().settings` equals
    `Settings::default()`. Asserted in a unit test in `src/app.rs`.

14. `src/io/settings.rs` does not import `egui`, `eframe`, or `rfd`. Verified by
    `grep -nE 'use (egui|eframe|rfd)' src/io/settings.rs` returning no matches.

15. `src/io/settings.rs` is ≤ 200 LOC (`wc -l src/io/settings.rs`).

16. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0.

## Expected tests

All unit tests live in a `#[cfg(test)] mod tests` block at the bottom of
`src/io/settings.rs` (and one in `src/app.rs`).

- **`settings_default_values`** (AC 4): `assert_eq!` on each field of
  `Settings::default()`.

- **`settings_debug_and_clone`** (AC 3): `format!("{:?}", Settings::default())` is
  non-empty; `Settings::default().clone() == Settings::default()`.

- **`push_recent_file_inserts_at_front`** (AC 5): push one path, assert
  `recent_files[0]` equals that path.

- **`push_recent_file_deduplicates`** (AC 5): push `"a"` twice, assert
  `recent_files.len() == 1` and `recent_files[0] == "a"`.

- **`push_recent_file_caps_at_ten`** (AC 5): push 11 distinct paths, assert
  `recent_files.len() == 10`.

- **`push_recent_file_moves_existing_to_front`** (AC 5): push `["a","b","c"]` in
  order, then push `"b"` again; assert `recent_files == ["b","a","c"]`.

- **`load_from_missing_file_returns_default`** (AC 6): call `load_from` with
  `Path::new("/tmp/lcv058_nonexistent_xyz/settings.json")`; assert result equals
  `Settings::default()`.

- **`load_from_invalid_json_returns_default`** (AC 7): write `"{{not valid"` to a
  temp file path; call `load_from`; assert result equals `Settings::default()`.

- **`save_to_and_load_from_roundtrip`** (AC 8): construct a non-default
  `Settings { bed_width_mm: 300.0, bed_height_mm: 200.0, snap_enabled: false,
  grid_visible: false, recent_files: vec!["foo.lcad".into()] }`, call
  `save_to(&s, &path)`, then `load_from(&path)`, assert equality.

- **`save_to_creates_parent_dirs`** (AC 9): pass a path inside a fresh temp
  subdirectory that does not exist, call `save_to`; assert `path.exists()` is
  true afterwards.

- **`save_to_no_tmp_file_after_success`** (AC 10): after `save_to` returns `Ok(())`,
  assert that `path.with_extension("json.tmp")` does not exist.

- **`settings_json_is_pretty_printed`** (implicit quality): after `save_to`, read
  the raw file content and assert it contains a newline (`\n`), confirming
  `to_string_pretty` was used (allows manual editing).

- **`app_default_settings_equals_settings_default`** (AC 13): placed in
  `src/app.rs` tests block — `assert_eq!(App::default().settings, Settings::default())`.

- **Static check (AC 14)**: `grep -nE 'use (egui|eframe|rfd)' src/io/settings.rs`
  must return no output (CI / manual verification).

- **Build gate (AC 16)**: `cargo fmt --all -- --check && cargo clippy --all-targets
  -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **LCV-007 status**: PLAN.md shows LCV-007 as `Draft`, but the egui window is
  demonstrably running (LCV-030, LCV-031, LCV-032, LCV-040 are Done and all
  depend on it). Treat LCV-007 as Done for this demand.

- **`directories` crate**: `ProjectDirs::from("qualifier", "organisation",
  "application")`. For LaserCAD the call is
  `ProjectDirs::from("", "", "LaserCAD")`. On Linux this resolves to
  `~/.config/LaserCAD/`; on Windows `%APPDATA%\LaserCAD\config\`; on macOS
  `~/Library/Application Support/LaserCAD/`. The `config_dir()` accessor returns
  the right subdir on each platform.

- **Atomic write**: `std::fs::rename` is POSIX-atomic on Linux (same filesystem).
  On Windows rename-over-existing may fail; that edge case is acceptable for now
  (Linux is primary). The implementer may call `std::fs::remove_file` on Windows
  before rename if needed but is not required to for this demand.

- **`serde` unknown fields**: `#[derive(Deserialize)]` ignores unknown fields by
  default when `#[serde(deny_unknown_fields)]` is absent. Do NOT add
  `deny_unknown_fields` — forward-compatibility matters more than strict parsing.

- **`recent_files` as `Vec<String>`**: file paths on Linux are bytes, not always
  UTF-8. `String` is a Linux-first simplification consistent with the rest of the
  codebase. Non-UTF-8 paths are out of scope.

- **Bed dimensions vs `App::bed`**: `App` already has `pub bed: Bed` with
  `size_mm: [400.0, 400.0]`. The settings store captures the user's preferred bed
  size but does not yet synchronise it into `App::bed` at startup. That wiring
  lands with the settings dialog (LCV-076) or the boot sequence. The implementer
  must not add that wiring here — it would constitute scope creep.

- **`App::default()` and `#[derive(Default)]`**: `App` currently uses
  `#[derive(Default)]`. Adding `pub settings: Settings` is sufficient — `Settings`
  implements `Default`, so the derive handles it automatically. No manual
  `Default` impl needed.

- **`save()` error handling upstream**: callers (LCV-059 autosave, LCV-076 settings
  dialog) will decide whether to log, display, or silently ignore the error.
  `save()` only propagates; it does not log or panic.

- **PLAN.md persistence line**: the architecture summary in PLAN.md already lists
  `serde_json + directories` as the persistence stack. Adding these deps is not a
  new architectural decision — no ADR required.

- **`pub use` in `io/mod.rs`**: downstream consumers (`app.rs`, `agent/`) should
  be able to write `use crate::io::{Settings, load, save}` without knowing the
  submodule path. The re-exports enforce this.
