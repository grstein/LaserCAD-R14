# LCV-059 — Autosave (debounced, restore on boot)

- **Status**: Ready
- **Phase**: 5
- **Depends on**: LCV-058, LCV-021
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

If the application crashes or is force-quit while the operator is mid-drawing,
all unsaved work is lost. For a laser-cutting workflow this means re-drawing
from scratch, which can represent significant precision work. A debounced
autosave that writes the document to disk silently and restores it on next boot
closes this gap without changing the user's save/open workflow.

## Scope

### 1. Serde derives on kernel types

The geometry primitives carry no serde derives today. Add
`#[derive(serde::Serialize, serde::Deserialize)]` to:

- `Vec2` in `src/geometry/vec2.rs`
- `Line` in `src/geometry/line.rs`
- `Circle` in `src/geometry/circle.rs`
- `Arc` in `src/geometry/arc.rs`
- `Entity` in `src/document/entity.rs`

These are pure derive macros — no runtime cost when serialization is not
invoked, no behaviour change. `Cargo.toml` already carries
`serde = { version = "1", features = ["derive"] }`.

### 2. `src/io/autosave.rs` — autosave module

New file following the same two-layer API pattern as `src/io/settings.rs`.

**On-disk envelope** (JSON):

```json
{ "schema_version": 1, "entities": [ … ] }
```

Introduce a private `DocumentEnvelope { schema_version: u32, entities:
Vec<Entity> }` struct inside `autosave.rs` that derives
`Serialize, Deserialize`. `Document` itself is **not** given serde derives;
the envelope is the serialization boundary. `SCHEMA_VERSION` (already
`pub const` in `src/document/entity.rs`) is used to stamp and validate.

**Public API:**

```rust
/// Write `doc` to the platform autosave file. Atomic (`.tmp` rename).
pub fn save_autosave(doc: &Document) -> Result<(), AutosaveError>;

/// Read and deserialise the autosave file.
/// Returns `None` if the file is absent, unreadable, or the
/// `schema_version` does not match `SCHEMA_VERSION`.
pub fn load_autosave() -> Option<Document>;

/// Delete the autosave file. No-op if absent.
pub fn clear_autosave();
```

**`pub(crate)` path-injected variants (for tests):**

```rust
pub(crate) fn save_autosave_to(doc: &Document, path: &Path)
    -> Result<(), AutosaveError>;
pub(crate) fn load_autosave_from(path: &Path) -> Option<Document>;
pub(crate) fn clear_autosave_at(path: &Path);
```

**`AutosaveError`** (thiserror-derived):

```rust
pub enum AutosaveError {
    NoPlatformPath,
    Json(#[from] serde_json::Error),
    Io(#[from] std::io::Error),
}
```

**Platform path:** resolve via `directories::ProjectDirs::from("", "",
"lasercad")`, use `data_dir()` (not `config_dir()`), join `"autosave.json"`.
Returns `None` if `ProjectDirs` cannot be constructed (e.g. no `$HOME`).

**Atomic write** (same contract as `settings.rs`):
1. Create intermediate directories.
2. Serialise to `<path>.tmp`.
3. `fs::rename` tmp → final path.

**Module-level doc comment** (`//!`) must reference this demand (LCV-059),
describe the two-layer API, and state the kernel-purity constraint.

**No `egui`, `eframe`, or `rfd` imports** in `autosave.rs`.

**File ≤ 300 LOC.**

### 3. `src/io/mod.rs` — register the new module

Add `pub mod autosave;`.

### 4. `App` — dirty tracking

In `src/app.rs`:

- Add field `dirty_since: Option<std::time::Instant>` to `App`. `Option<Instant>`
  is `Default` → `None`, so `#[derive(Default)]` on `App` continues to work
  unchanged.

- In `App::commit`, after `self.history.commit(cmd, &mut self.document)`,
  add:
  ```rust
  self.dirty_since = Some(std::time::Instant::now());
  ```

- In `App::update`, **after** all rendering and input handling (i.e. at the
  end of the `CentralPanel` closure body), add the debounce flush:
  ```rust
  if let Some(since) = self.dirty_since {
      if since.elapsed() >= std::time::Duration::from_secs(5) {
          let _ = crate::io::autosave::save_autosave(&self.document);
          self.dirty_since = None;
      }
  }
  ```
  Errors from `save_autosave` are silently discarded (the `_` binding) so
  they never interrupt the UI frame. A future demand may surface them.

### 5. `App::new()` — boot restore

Add an associated function:

```rust
/// Construct the application, restoring an autosave if one exists.
///
/// Any autosave found on disk is silently loaded and then deleted so that
/// subsequent clean exits do not re-offer the restored document.
/// A user-visible restore dialog is deferred to LCV-069.
pub fn new() -> Self {
    let mut app = Self::default();
    if let Some(doc) = crate::io::autosave::load_autosave() {
        app.document = doc;
        crate::io::autosave::clear_autosave();
    }
    app
}
```

### 6. `src/lib.rs` — use `App::new()` at startup

Change the eframe creation closure from:

```rust
Box::new(|_cc| Ok(Box::<app::App>::default()))
```

to:

```rust
Box::new(|_cc| Ok(Box::new(app::App::new())))
```

No other change to `lib.rs`.

## Out of scope

- User-visible restore dialog (deferred to LCV-069).
- Autosave path stored in `Settings` (not required; the path is derived
  deterministically from the platform data dir).
- Schema migration for `schema_version > 1` files — `load_autosave_from`
  simply returns `None` for unknown versions.
- Undo history persistence — only `entities` are saved; the history stack
  is not snapshotted.
- Async / off-thread I/O — the JSON payload is tiny (entity geometry only);
  synchronous write in `update()` is acceptable for v0.1.0.
- Compression of the autosave file.
- Multiple autosave slots or rotation.
- Dirty indicator in the title bar or status bar.
- `Document::Clone` — not introduced by this demand.

## Acceptance criteria

1. `src/io/autosave.rs` exists; `grep -nE '^use (egui|eframe|rfd)'
   src/io/autosave.rs` returns no matches.

2. `save_autosave_to(doc, path)` writes a valid JSON file whose top-level
   keys are `"schema_version"` (integer `1`) and `"entities"` (array).

3. `load_autosave_from(path)` over a file written by `save_autosave_to`
   returns `Some(doc)` whose `entity_count()` equals the original and whose
   entity values match (all fields within `EPSILON` for `f64`).

4. `load_autosave_from(nonexistent_path)` returns `None` without panicking.

5. `load_autosave_from(path)` where the file contains
   `{ "schema_version": 999, "entities": [] }` returns `None`.

6. `load_autosave_from(path)` where the file is not valid JSON returns
   `None` without panicking.

7. `save_autosave_to` creates intermediate directories if they do not exist.

8. After a successful `save_autosave_to`, no `.tmp` sibling file remains.

9. `clear_autosave_at(path)` removes the file when it exists; calling it on
   a nonexistent path does not panic or return an error.

10. `App::default().dirty_since` is `None`.

11. After calling `App::commit(cmd)` on a fresh `App`, `dirty_since` is
    `Some(_)`.

12. `App::new()` with an autosave present at the platform path loads and
    clears it, leaving `app.document.entity_count() > 0` (for a non-empty
    autosave) and no autosave file on disk.

13. `App::new()` with no autosave present constructs identically to
    `App::default()` (empty document, no `dirty_since`).

14. `src/io/autosave.rs` is ≤ 300 LOC.

15. `cargo fmt --all && cargo clippy --all-targets -- -D warnings &&
    cargo test --all` all exit 0.

## Expected tests

All unit tests live in `#[cfg(test)] mod tests` inside `src/io/autosave.rs`
unless noted.

- **AC 2 + 3** — `round_trip_empty_document`: save a `Document::default()`,
  load back, assert `entity_count() == 0`.

- **AC 2 + 3** — `round_trip_with_entities`: build a `Document` with one
  `Entity::Line`, one `Entity::Circle`, one `Entity::Arc`; save; load; assert
  `entity_count() == 3` and that the line endpoints, circle center/radius,
  and arc center/radius/angles all match within `EPSILON`.

- **AC 4** — `load_from_nonexistent_returns_none`: call
  `load_autosave_from` on a path that does not exist; assert `None`.

- **AC 5** — `load_schema_mismatch_returns_none`: write
  `{"schema_version":999,"entities":[]}` to a temp file; assert `None`.

- **AC 6** — `load_malformed_json_returns_none`: write `{ not json }` to a
  temp file; assert `None`.

- **AC 7** — `save_creates_intermediate_dirs`: call `save_autosave_to` with
  a path under a freshly-removed nested directory; assert the file exists
  after the call.

- **AC 8** — `save_no_tmp_after_success`: assert the `.tmp` sibling is
  absent after a successful save.

- **AC 9** — `clear_existing_file`: save then clear; assert the file is
  absent. Also call `clear_autosave_at` on a nonexistent path; assert no
  panic.

- **AC 10 + 11** (unit in `src/app.rs`) — `dirty_since_none_on_default`:
  `App::default().dirty_since` is `None`. `commit_sets_dirty_since`:
  construct an `App`, commit a `CreateLine` command, assert `dirty_since`
  is `Some`.

- **AC 12 + 13** (unit in `src/app.rs`) — `new_restores_autosave`:
  write a known Document via `save_autosave_to` to the temp path; construct
  `App` via logic that uses `load_autosave_from` on the injected path;
  verify `document.entity_count() > 0`. `new_without_autosave_matches_default`:
  when no autosave exists, `App::new()` equals `App::default()` in entity
  count and `dirty_since`.
  *(Tests for AC 12 + 13 may use the `pub(crate)` path-injected variants
  rather than the platform path to avoid filesystem pollution in CI.)*

## Open questions

*(none)*

## Notes

- **Why `data_dir()` not `config_dir()`?** Autosave is transient working
  data, not a user preference. On Linux this resolves to
  `~/.local/share/lasercad/autosave.json`; on macOS to
  `~/Library/Application Support/lasercad/autosave.json`.
- **Why `DocumentEnvelope` not `Document: Serialize`?** `Document` owns
  `Selection` (LCV-027), which carries UI-specific transient state
  (highlighted indices, window-selection in progress). Serialising
  `Document` directly would either persist that transient state or require
  `Selection` to impl a `Default`-on-load dance. The envelope approach
  serialises only `entities: Vec<Entity>` and deserialises into a clean
  `Document::default()` with entities injected — simpler and safer.
- **Why silent discard on save error?** A failed autosave should not
  interrupt the drawing session; the operator has done nothing wrong. A
  future demand (LCV-069) can surface non-fatal errors in the status bar.
- **Debounce period**: 5 s. The product README says "debounced 800 ms" as an
  aspirational figure; this demand sets the canonical value at 5 s, which
  avoids hammering the filesystem during rapid draw-undo-redo sequences.
- **No tokio in scope**: `Cargo.toml` has no tokio dependency. Synchronous
  I/O in `update()` is acceptable because the autosave JSON payload is
  bounded by entity count and a few hundred entities produce < 50 KB, well
  under 1 ms for a local write. Async autosave is deferred until tokio is
  added by a later demand.
- **v1 reference**: `LaserCAD R14 v1` (TypeScript/Tauri) autosaved to
  `localStorage` on every change with a 300 ms debounce. v2 moves to a
  proper platform data file for cross-session durability and uses a longer
  debounce to reduce I/O frequency.
- `SCHEMA_VERSION` constant is already `pub` in `src/document/entity.rs`
  and re-exported via `src/document/schema.rs`. Import as
  `use crate::document::schema::SCHEMA_VERSION;` or
  `use crate::document::entity::SCHEMA_VERSION;` — either path compiles.
