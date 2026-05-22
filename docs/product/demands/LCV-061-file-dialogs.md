# LCV-061 — File dialogs (rfd wrapper)

- **Status**: Ready
- **Phase**: 5
- **Depends on**: LCV-007 (done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Without native dialogs the File menu's Open and Save As actions would require
the user to type full absolute paths into the command line — a poor match for a
desktop CAD tool whose primary workflow is open-SVG → edit → save-SVG for
LaserGRBL. `src/io/dialogs.rs` provides three thin blocking wrappers around
`rfd::FileDialog` so that LCV-062 (New / Open / Save / Save As / Exit actions)
can invoke them with a single call and receive an `Option<PathBuf>`. The
wrappers own nothing beyond dialog configuration; all file I/O lives in the
SVG pipeline (LCV-056 / LCV-057).

## Scope

- **`src/io/dialogs.rs`** — three `pub` functions, all blocking:
  - `pub fn open_file_dialog() -> Option<PathBuf>` — file-open dialog filtered
    to "SVG files" (`*.svg`) and "All files" (`*.*`); returns the chosen path
    or `None` on cancel.
  - `pub fn save_file_dialog(default_name: &str) -> Option<PathBuf>` — file-
    save dialog with the same two filters; the filename input is pre-filled
    with `default_name`; returns the chosen path or `None` on cancel.
  - `pub fn pick_folder_dialog() -> Option<PathBuf>` — folder-picker dialog
    (no file filter); reserved for future batch-export; returns the chosen
    directory path or `None` on cancel.
- **`src/io/mod.rs`** — add `pub mod dialogs;` and re-export all three
  functions: `pub use dialogs::{open_file_dialog, save_file_dialog, pick_folder_dialog};`.
- **`Cargo.toml`** — add `rfd = "0.14"` (sync API; do **not** enable the
  `tokio` or `async-std` features of `rfd`).

## Out of scope

- Async / tokio dialog variants (`rfd::AsyncFileDialog`).
- Any egui rendering or menu integration — that is LCV-062 (actions) and
  LCV-065 (menubar).
- File reading or writing — owned by LCV-056 (SVG export) and LCV-057
  (SVG import).
- Remembering or filtering by recently opened files — owned by LCV-060.
- Confirm-save-changes modal or error dialogs — owned by LCV-069.
- Windows or macOS platform testing — primary platform is Linux; other
  platforms are Phase 9.

## Acceptance criteria

1. `open_file_dialog()` presents at least two filter groups — "SVG files"
   mapping to extension `svg`, and "All files" mapping to `*` — and returns
   `Some(PathBuf)` when the user confirms a selection, `None` when the user
   cancels.
2. `save_file_dialog(default_name)` pre-fills the dialog filename input with
   the exact string passed as `default_name`; the same two filters as AC#1 are
   present; it returns `Some(PathBuf)` on confirm, `None` on cancel.
3. `pick_folder_dialog()` opens a folder-picker (no file-extension filter)
   and returns `Some(PathBuf)` on confirm, `None` on cancel.
4. All three functions use `rfd::FileDialog` (the synchronous API); no call to
   `rfd::AsyncFileDialog` appears anywhere in `src/io/dialogs.rs`.
5. `src/io/dialogs.rs` does **not** import `egui`, `eframe`, or any module from
   `crate::geometry`, `crate::document`, `crate::io::svg`, `crate::agent`, or
   `crate::text`. It imports only `std::path::PathBuf` and `rfd`.
6. All three functions are accessible as `crate::io::open_file_dialog`,
   `crate::io::save_file_dialog`, and `crate::io::pick_folder_dialog` from
   any crate module (re-exported through `src/io/mod.rs`).
7. `rfd = "0.14"` (or a later semver-compatible version) appears in
   `[dependencies]` in `Cargo.toml`; the `rfd` entry does NOT enable any
   async runtime feature.
8. `src/io/dialogs.rs` stays within the 300-LOC cap.
9. `cargo fmt --all && cargo clippy --all-targets -- -D warnings &&
   cargo test --all` all exit 0.

## Expected tests

Tests must not open actual dialogs (CI has no display). Use type-assertion
compile tests only.

- **Unit test `dialog_fn_signatures`** (in `src/io/dialogs.rs` under
  `#[cfg(test)] mod tests`): asserts the three function pointers match their
  declared signatures without invoking them:
  ```rust
  let _: fn() -> Option<std::path::PathBuf> = open_file_dialog;
  let _: fn(&str) -> Option<std::path::PathBuf> = save_file_dialog;
  let _: fn() -> Option<std::path::PathBuf> = pick_folder_dialog;
  ```
  Passes when the functions compile with the exact signatures in AC#1–3.

- **Unit test `dialogs_reexported_from_io`** (in `src/io/mod.rs` or
  `src/lib.rs` test block): confirms the re-exports compile:
  ```rust
  let _: fn() -> Option<std::path::PathBuf> = crate::io::open_file_dialog;
  let _: fn(&str) -> Option<std::path::PathBuf> = crate::io::save_file_dialog;
  let _: fn() -> Option<std::path::PathBuf> = crate::io::pick_folder_dialog;
  ```

- **Static check (AC#4)**: `grep -n 'AsyncFileDialog' src/io/dialogs.rs`
  returns no matches.

- **Static check (AC#5)**: `grep -n 'egui\|eframe' src/io/dialogs.rs`
  returns no matches.

- **Static check (AC#7)**: `grep 'rfd' Cargo.toml` shows an entry without
  `tokio` or `async-std` in its feature list.

- **Build gate (AC#9)**: `cargo fmt --all -- --check && cargo clippy
  --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- `rfd::FileDialog` API (v0.14): `.add_filter(label, &[ext])` chains filter
  groups; `.set_file_name(name)` sets the pre-filled filename; `.pick_file()`,
  `.save_file()`, and `.pick_folder()` are the three blocking pickers, all
  returning `Option<PathBuf>`. The all-files filter extension is `"*"`.
- On Linux, `rfd` 0.14 links against GTK3 (`gtk3` feature, the default) or
  uses XDG portal if the `xdg-portal` feature is enabled. The default feature
  set is sufficient; do not override features unless the build fails.
- `rfd` is explicitly listed under "Dialogs: rfd" in `AGENTS.md` §Architecture
  summary and is therefore an approved dependency — no separate ADR needed.
- The purity rule (`AGENTS.md` §Purity rule) prohibits `rfd` only in
  `geometry/`, `document/`, `io/svg/`, `agent/classifier.rs`, and `text/`.
  `src/io/dialogs.rs` is outside that boundary; the import is legal.
- LCV-062 is the direct consumer; it will call these three wrappers from
  `App`-level action handlers. Keep the wrapper signatures stable across
  the phase boundary.
- Precedent for `io/` module structure: LCV-058 (`src/io/settings.rs`).
