# LCV-058 — Settings store (JSON file via directories)

- **Status**: Done
- **Phase**: 5
- **Depends on**: LCV-007
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 30f9ca0 — feat(LCV-058): settings store — JSON persistence via serde + directories

## Problem

LaserCAD v2 needs a place to persist user preferences that survive process exit —
recent files list (LCV-060), autosave path (LCV-059), agent API key (LCV-076), and
future per-user settings. Without a settings store, every downstream demand must
invent its own ad-hoc persistence.

## Scope

- **`src/io/settings.rs`** — the settings store module:
  - `pub struct Settings` — serializable with `serde::Serialize/Deserialize`.
    Fields (all with `#[serde(default)]`):
    - `pub recent_files: Vec<String>` — capped at 10 entries.
  - `impl Default for Settings` — all-empty / zero defaults.
  - `pub fn push_recent_file(&mut self, path: String)` — deduplicates, inserts at
    front, truncates to 10.
  - `pub fn load() -> Settings` — infallible; finds the platform config dir via
    `directories::ProjectDirs`, loads and parses JSON; returns `Settings::default()`
    on any error (missing file, parse failure, no platform dir).
  - `pub fn save(&self) -> Result<(), SettingsError>` — atomic: serialises to
    JSON, writes to `.tmp` sibling, renames to final path.
  - `pub(crate) fn load_from(path: &Path) -> Settings` — path-injected variant for
    tests; same infallible semantics.
  - `pub(crate) fn save_to(settings: &Settings, path: &Path) -> Result<(), SettingsError>`
    — path-injected variant for tests; same atomic-write semantics.
  - `pub enum SettingsError` — `thiserror`-derived; variants: `NoPlatformPath`,
    `Json(serde_json::Error)`, `Io(std::io::Error)`.

- **`src/io/mod.rs`** — add `pub mod settings;`.

- **`src/app.rs`** — add `pub settings: Settings` field to `App` struct.

- **`Cargo.toml`** — add `serde = { version = "1", features = ["derive"] }`,
  `serde_json = "1"`, `directories = "5"`, `thiserror = "2"`.

## Out of scope

- Recent files UI (LCV-060).
- Autosave logic (LCV-059).
- Agent settings (LCV-076).
- Migration between settings schema versions.

## Acceptance criteria

1. `Settings::default()` has `recent_files` empty.
2. `push_recent_file` deduplicates (existing occurrence removed before re-insert).
3. `push_recent_file` inserts at position 0 (most-recent first).
4. `push_recent_file` truncates to exactly 10 entries when more are pushed.
5. `load_from` with a nonexistent path returns `Settings::default()`.
6. `load_from` with a valid JSON file returns the deserialised settings.
7. `save_to` + `load_from` round-trip preserves all fields.
8. `save_to` creates intermediate directories if they don't exist.
9. `save_to` uses an atomic rename (writes `.tmp`, then renames).
10. `App` struct carries `pub settings: Settings`; `App::default().settings` equals
    `Settings::default()`.
11. No `egui`/`eframe`/`rfd` imports in `src/io/settings.rs`.
12. `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all`
    all exit 0.

## Open questions

(none)
