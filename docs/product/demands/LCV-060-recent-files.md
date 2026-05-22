# LCV-060 — Recent files (store-backed)

- **Status**: Ready
- **Phase**: 5
- **Depends on**: LCV-058
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

After opening or saving a drawing, the user cannot re-open it without navigating the file system
from scratch. AutoCAD R14 and every competent desktop CAD tool maintain a "recent files" list so
that frequently used drawings are one or two keystrokes away. LaserCAD v2 already stores
`recent_files: Vec<String>` in `Settings` (LCV-058) and provides `push_recent_file()`, but there
is no module that exposes the list as a stable slice or that advances an entry to the front when
the user picks it. Without this data-layer function, the future File menu (LCV-065) and any
keyboard shortcut (LCV-070) must duplicate the index-validation and promotion logic themselves.

## Scope

- **New file `src/io/recent.rs`** containing:

  - `pub fn recent_files(settings: &Settings) -> &[String]`  
    Thin slice accessor. Returns `settings.recent_files.as_slice()`.  
    Provides a single stable symbol for downstream callers (LCV-065 menu, LCV-070 shortcuts) to
    import without reaching into the struct field directly.

  - `#[derive(Debug, thiserror::Error)] pub enum OpenRecentError`  
    Variants:
    - `#[error("index {index} is out of range (list has {len} entries)")] IndexOutOfRange { index: usize, len: usize }`

  - `pub fn open_recent(index: usize, settings: &mut Settings) -> Result<PathBuf, OpenRecentError>`  
    Contract (in order):
    1. If `index >= settings.recent_files.len()`, return
       `Err(OpenRecentError::IndexOutOfRange { index, len: settings.recent_files.len() })`.
    2. Clone the path string at `settings.recent_files[index]`.
    3. Call `settings.push_recent_file(path.clone())` — this deduplicates and moves the entry to
       `settings.recent_files[0]` using the logic already tested by LCV-058.
    4. Return `Ok(PathBuf::from(path))`.

    The caller is responsible for:
    - Calling `settings.save()` to persist the updated list.
    - Passing the returned `PathBuf` to the SVG importer and replacing `app.document`
      (wired in LCV-062 and consumed by LCV-065).

- **`src/io/mod.rs`** — add `pub mod recent;` and re-export:
  ```rust
  pub use recent::{open_recent, recent_files, OpenRecentError};
  ```

- **`src/io/recent.rs` purity** — MUST NOT import `egui`, `eframe`, or `rfd`.
  The only crate imports allowed are `std`, `thiserror`, and `crate::io::settings::Settings`.

## Out of scope

- SVG parsing or document replacement — that is LCV-057 + LCV-062.
- "Open Recent" submenu in the File menu — that is LCV-065.
- Keyboard shortcuts for recent-file slots — that is LCV-070.
- Removing stale paths (tombstoning paths that no longer exist on disk). A separate demand can
  add that if ever needed.
- Calling `settings.save()` inside `open_recent` — the caller owns persistence timing.
- Any GUI feedback (progress indicator, error toast) for a missing file.

## Acceptance criteria

1. `recent_files(&settings)` returns a `&[String]` identical to `settings.recent_files.as_slice()`
   for any settings value (empty list, one entry, ten entries).

2. `open_recent(index, settings)` returns
   `Err(OpenRecentError::IndexOutOfRange { index: 0, len: 0 })` when `settings.recent_files` is
   empty and `index` is `0`.

3. `open_recent(index, settings)` returns
   `Err(OpenRecentError::IndexOutOfRange { index, len })` for any `index >= len` where `len` is
   the current length of `settings.recent_files`.

4. `open_recent(0, settings)` where `settings.recent_files == ["a.svg"]` returns
   `Ok(PathBuf::from("a.svg"))`.

5. `open_recent(2, settings)` where `settings.recent_files == ["c.svg", "b.svg", "a.svg"]`
   returns `Ok(PathBuf::from("a.svg"))`.

6. After a successful `open_recent(index, settings)` call, `settings.recent_files[0]` equals
   the path that was at `settings.recent_files[index]` before the call (the opened entry is
   promoted to front).

7. After a successful `open_recent(index, settings)` call where the opened path was already
   present elsewhere in the list, the list contains that path exactly once (deduplication
   guaranteed by `push_recent_file`, no regression here).

8. `open_recent` does not call `settings.save()` — the caller is responsible for persistence.
   Verified by: after calling `open_recent`, the settings file on disk (if any) is unchanged
   until the caller explicitly calls `settings.save()` / `save_to()`.

9. `src/io/recent.rs` contains no `use egui`, `use eframe`, or `use rfd` import.

10. `crate::io::open_recent`, `crate::io::recent_files`, and `crate::io::OpenRecentError` are
    all accessible through the `io` module re-exports.

11. `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all` all
    exit 0.

## Expected tests

All tests live in `src/io/recent.rs` under `#[cfg(test)]`.

- **`recent_files_empty`** — AC 1 (empty list):  
  `Settings::default()` → `recent_files(&s)` is `&[] as &[String]`.

- **`recent_files_with_entries`** — AC 1 (non-empty list):  
  Push three paths; verify `recent_files(&s)` equals `s.recent_files.as_slice()`.

- **`open_recent_index_out_of_range_empty`** — AC 2:  
  Empty settings, `open_recent(0, &mut s)` → `Err(IndexOutOfRange { index: 0, len: 0 })`.

- **`open_recent_index_out_of_range_nonzero_list`** — AC 3:  
  Two entries, `open_recent(5, &mut s)` → `Err(IndexOutOfRange { index: 5, len: 2 })`.

- **`open_recent_single_entry_returns_path`** — AC 4:  
  `settings.recent_files == ["a.svg"]`; `open_recent(0, &mut s)` → `Ok(PathBuf::from("a.svg"))`.

- **`open_recent_middle_entry_returns_correct_path`** — AC 5:  
  Three entries `["c.svg", "b.svg", "a.svg"]`; `open_recent(2, &mut s)` →
  `Ok(PathBuf::from("a.svg"))`.

- **`open_recent_promotes_to_front`** — AC 6:  
  Three entries; call `open_recent(2, &mut s)`; assert `s.recent_files[0] == "a.svg"`.

- **`open_recent_deduplication_preserved`** — AC 7:  
  Entries `["b.svg", "a.svg"]`; `open_recent(1, &mut s)` → `s.recent_files == ["a.svg", "b.svg"]`
  (length remains 2, no duplicate).

- **`open_recent_does_not_save_settings`** — AC 8:  
  Use `save_to(&settings, &tmp_path)` to write an initial settings file. Then call
  `open_recent(0, &mut settings)`. Read the file back with `load_from(&tmp_path)`. Assert the
  file still reflects the state *before* the `open_recent` call (i.e., the promotion is not yet
  persisted until the caller saves).

- **`io_reexports_recent_symbols`** — AC 10:  
  Compile-time assertion (function-pointer assignment) that `crate::io::open_recent`,
  `crate::io::recent_files`, and `crate::io::OpenRecentError` are accessible.
  Pattern follows the existing `dialogs_reexported_from_io` test in `src/io/mod.rs`.

- **Manual smoke** — AC 4–6 end-to-end:  
  In a debug build, add `"/path/to/real/drawing.svg"` to `app.settings.recent_files` via the
  Rust REPL or a temporary test harness. Call `open_recent(0, &mut app.settings)` and verify
  the returned `PathBuf` matches. Pass the path to the SVG importer (once LCV-057 is done) and
  confirm the document is replaced correctly.

## Open questions

*(none)*

## Notes

- `Settings::push_recent_file` (already shipped by LCV-058) handles deduplication, front-insert,
  and the 10-entry cap. `open_recent` delegates entirely to it; do not re-implement that logic.
- `recent_files()` is intentionally a standalone function rather than a `Settings` method — it
  belongs to the `io::recent` module so that LCV-065 imports from one stable place.
- The caller pattern for a full "open recent" user action (once LCV-057 and LCV-062 exist) is:
  ```rust
  // in app.rs or a ui action handler:
  let path = crate::io::open_recent(index, &mut self.settings)?;
  let doc  = crate::io::svg::import::from_file(&path)?;
  self.document = doc;
  self.settings.save().unwrap_or_else(|e| eprintln!("settings save: {e}"));
  ```
- `src/io/recent.rs` is NOT a kernel module (kernel = `geometry/`, `document/`, `io/svg/`,
  `agent/classifier.rs`, `text/`). It may import `std`, `thiserror`, and sibling `io` types,
  but must not import `egui`/`eframe`/`rfd` to keep it testable headlessly.
- `thiserror` is already a dependency (brought in by LCV-058 via `SettingsError`); no new
  `Cargo.toml` changes are required.
- `PathBuf` is in `std::path`; no new dependencies.
- File existence checking (is the path still on disk?) is deliberately excluded from this demand.
  If a path is stale, the open attempt in LCV-062 will produce a clear `std::io::Error`; no
  pre-flight check is needed here.
