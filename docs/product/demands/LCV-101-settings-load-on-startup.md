# LCV-101 — Load persisted settings at startup (+ OpenRouter default endpoint)

- **Status**: Ready
- **Phase**: 10
- **Depends on**: LCV-058 (Done), LCV-076 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

`Settings::load()` (`src/io/settings.rs`) is fully implemented, tested, and
**never called**. `App::new()` builds its state from `Self::default()`, which
sets `settings: Settings::default()`. Everything the settings file exists for is
therefore write-only: the operator saves a drawing, `action_save_as` pushes the
path into `recent_files` and writes `settings.json` to disk — and on the next
launch File → Open Recent says "No recent files". The same applies to the agent
endpoint and API key entered in the Agent Settings dialog: they are written on
dialog close and silently discarded on restart, so the operator re-types an API
key every session. Separately, the shipped default endpoint points at OpenAI
while `docs/product/README.md` commits the product to "OpenAI-compatible LLM
(**default OpenRouter**)", so a fresh install is pointed at the wrong service.

## Scope

- **`App::new()` loads the persisted settings**: `Self::default()`, then
  `app.settings = Settings::load()`, then the existing autosave restore
  (LCV-059). `App::default()` is untouched and keeps `Settings::default()` — it
  is the test constructor and must stay filesystem-free (ADR 0002 §A2).
- **Default agent endpoint becomes `https://openrouter.ai/api/v1`** in
  `default_agent_endpoint()`, with the doc comment on `Settings::agent_endpoint`
  updated to match.
- **Corrupt-settings policy** in `load_from`: a settings file that exists but
  does not parse is **preserved**, not overwritten — renamed to a `.bak` sibling
  before defaults are returned. A missing file stays a silent, side-effect-free
  default.
- **Doc comments** on `App::new` / `App::default` recording which constructor
  touches the user's real data directories (config dir via `Settings::load`,
  data dir via `load_autosave`) and which one does not.

## Out of scope

- **Reacting to settings changes at runtime** (file watching, reload-on-change).
  Load happens once, at construction.
- **Migrating existing settings files.** A file that already stores
  `agent_endpoint: "https://api.openai.com/v1"` keeps that value — it is an
  explicit user choice, not a default. Only the fallback for a field that is
  absent or a file that does not exist changes.
- **New settings fields** (theme, grid spacing, window geometry, model name,
  bed size). The struct keeps exactly the three fields it has today.
- **Changing when settings are saved.** The existing save sites (file actions,
  Agent Settings dialog close) stay as they are; no save-on-quit, no autosave of
  settings, no write during `App::new`.
- **Surfacing the corrupt-file event in the UI** (error modal, statusbar toast).
  The `.bak` file is the user-visible artefact for Marco 0.
- **A logging dependency.** `tracing` is not in `Cargo.toml` and this demand does
  not add it or any other dependency.
- **Stale-path pruning in the recent-files list.** A recent entry whose file was
  deleted keeps its menu row; `action_open_path` already reports the read error.

## Acceptance criteria

1. `App::new()` assigns `Settings::load()` to `self.settings`. In the file that
   defines `App::new` (`src/app.rs` today, `src/app/mod.rs` if LCV-105 has
   landed first), `grep -n "Settings::load()"` returns exactly one match and it
   is inside the body of `App::new`.

2. `App::new()` still restores the autosaved document (LCV-059 behaviour is
   unchanged) and performs **no write**: no `settings.save()`, no
   `save_autosave`, no file creation on the startup path.

3. `App::default()` still yields `settings == Settings::default()` and touches
   no filesystem. The existing test
   `app_default_settings_equals_settings_default` stays green unmodified.

4. The doc comment on `App::new` states that it reads the platform config
   directory (`Settings::load`) and the platform data directory
   (`load_autosave`) and MUST NOT be called from tests; the doc comment on
   `App::default` states it is the test constructor and touches no filesystem.
   (ADR 0002 §A2. If LCV-103 has already added these comments, extend the
   `App::new` one with the config-directory clause rather than duplicating it.)

5. `Settings::default().agent_endpoint == "https://openrouter.ai/api/v1"`, and
   the `///` doc comment on the field states that default.

6. `Settings::default().agent_api_key == ""` and
   `Settings::default().recent_files.is_empty()` — unchanged.

7. A settings file that stores an explicit endpoint keeps it: `load_from` on a
   temp file containing `{"agent_endpoint":"https://api.openai.com/v1"}` returns
   a `Settings` whose `agent_endpoint` is `https://api.openai.com/v1`.

8. A legacy settings file with no `agent_endpoint` key (e.g.
   `{"recent_files":[]}`) loads with `agent_endpoint ==
   "https://openrouter.ai/api/v1"`.

9. **Missing file**: `load_from` on a path that does not exist returns
   `Settings::default()`, does not panic, and leaves the path non-existent
   afterwards (no file is created, no `.bak` is created).

10. **Corrupt file**: `load_from` on a path holding unparsable bytes (e.g.
    `{ this is not json }`, or an empty file) returns `Settings::default()`,
    does not panic, and moves the original file to a sibling whose file name is
    the original file name with `.bak` appended (`settings.json` →
    `settings.json.bak`, mirroring the `.tmp` convention already used by
    `save_to`). The `.bak` file's bytes are byte-identical to the original
    input. An existing `.bak` is replaced.

11. **Corrupt file, unwritable directory**: if the `.bak` rename fails,
    `load_from` still returns `Settings::default()` and does not panic
    (the rename result is ignored, not unwrapped).

12. `load_from` never truncates, creates, or rewrites the file it reads; after a
    successful load the file's bytes are unchanged.

13. **No test touches the user's real config directory**: `Settings::load()` and
    `Settings::save()` (the no-argument platform-path variants) are not called
    from any `#[cfg(test)]` block or any file under `tests/`. Every settings
    test uses `load_from` / `save_to` with a path under
    `std::env::temp_dir()`. `grep -rn "Settings::load()" tests/` returns no
    matches.

14. The two existing unit tests that assert the OpenAI default
    (`agent_field_defaults`, `legacy_json_without_agent_fields_uses_defaults` in
    `src/io/settings.rs`) are updated to the OpenRouter default rather than
    deleted. `agent_fields_round_trip` keeps a non-default endpoint so it still
    proves persistence rather than tautologically matching the default — change
    its stored value to `https://api.openai.com/v1` if needed.

15. `src/io/settings.rs` module header documents the corrupt-file `.bak`
    behaviour (it currently promises only "returns `Settings::default()` on any
    error").

16. No new entry in `[dependencies]` or `[dev-dependencies]`; kernel purity of
    `src/io/settings.rs` unchanged
    (`grep -nE '^use (egui|eframe|rfd)' src/io/settings.rs` → no matches).

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0.

## Expected tests

Unit tests live in `#[cfg(test)] mod tests` of `src/io/settings.rs` and
`src/app.rs`.

- **Static check (AC 1)**: the `Settings::load()` grep, plus reviewer read of
  `App::new`.
- **Static check (AC 2)**: `grep -n "save\|write" ` over the body of `App::new`
  shows no write call; reviewer confirms the autosave restore is still present.
- **Unit (AC 3)**: existing `app_default_settings_equals_settings_default` runs
  unmodified.
- **Static check (AC 4, 15)**: reviewer reads the three doc comments.
- **Unit (AC 5, 6)**: `agent_field_defaults` — asserts the OpenRouter endpoint,
  empty key, empty recent list.
- **Unit (AC 7)**: `explicit_endpoint_in_file_is_preserved` — temp file with the
  OpenAI endpoint; assert it survives the load.
- **Unit (AC 8)**: `legacy_json_without_agent_fields_uses_defaults` — updated.
- **Unit (AC 9)**: `load_from_nonexistent_path_returns_default_and_creates_nothing`
  — assert `!path.exists()` and `!bak.exists()` after the call.
- **Unit (AC 10)**: `load_from_corrupt_file_backs_it_up` — write
  `{ this is not json }`, call `load_from`, assert the return is
  `Settings::default()`, assert `settings.json.bak` exists with identical bytes,
  assert the original path no longer exists; second case: a pre-existing `.bak`
  is replaced; third case: an empty file takes the same path. Clean up the temp
  dir at the end.
- **Unit (AC 11)**: `load_from_corrupt_file_survives_failed_backup` — point
  `load_from` at a path inside a read-only temp directory (create the dir,
  `set_permissions` to `0o500`, restore afterwards); assert the call returns
  defaults and does not panic. If the platform makes this unreliable (test runs
  as root), the test may assert the weaker property that `load_from` returns
  `Settings::default()` for a corrupt file in a directory it cannot write, and
  must document why.
- **Unit (AC 12)**: `load_from_does_not_modify_a_valid_file` — write valid JSON,
  load, assert the file's bytes are unchanged.
- **Unit (AC 14)**: `agent_fields_round_trip` — updated, still asserts a
  non-default value survives `save_to` + `load_from`.
- **Static check (AC 13, 16)**: the greps; `git diff Cargo.toml` is empty.
- **Build gate (AC 17)**: the three cargo commands.
- **Manual smoke (the reason the demand exists)**: launch the app, open the
  Agent Settings dialog, set the API key to `sk-smoke` and close it; File → Save
  As `/tmp/lcv101.svg`; quit; relaunch. Expect: File → Open Recent lists
  `/tmp/lcv101.svg`, and the Agent Settings dialog still shows `sk-smoke` and
  the OpenRouter endpoint. Then corrupt the settings file
  (`echo 'garbage' > ~/.config/lasercad/settings.json`), relaunch, and confirm
  the app starts with defaults and that `settings.json.bak` holds the corrupt
  bytes.

## Risks

- **The corrupt-file `.bak` deletes the live file by renaming it.** That is the
  intent (a file the parser rejects is not recoverable in-place), but it means a
  half-written settings file loses whatever was still readable in it. Accepted:
  the atomic `.tmp`-then-rename write in `save_to` makes a torn settings file
  very unlikely, and the bytes survive in `.bak`.
- **The endpoint default change is silent for existing users** whose file has no
  `agent_endpoint` key: their next agent call goes to OpenRouter with an OpenAI
  key and fails with a 401. There is no installed base (LCV-089 unreleased), and
  the Agent Settings dialog exposes the field. Accepted.
- **Line-number drift with LCV-103 / LCV-105**: both touch `src/app.rs` around
  `App::new` (doc comments, file split). The edit here is three lines; whichever
  lands second rebases trivially.
- **`App::new` remains untestable by construction** — that is ADR 0002's
  decision, not a gap. AC 1, 2 and 4 are static checks for the reviewer, and the
  behaviour under them is covered by the `load_from` unit tests.

## Open questions

*(none — demand is Ready)*

## Notes

- `src/io/recent.rs` already reads the list out of `app.settings`
  (`recent_files(&app.settings)`), and `src/ui/menubar.rs` renders it — so
  loading the settings is the only missing link for Open Recent. No menubar or
  recent-files change is needed.
- Settings are already written back in four places: `action_open`,
  `action_open_path`, `action_save_as` (each calls `app.settings.save()`), and
  the Agent Settings dialog on close. Only the read path was missing.
- `load_from` / `save_to` are `pub(crate)`, so these tests must live **inside**
  the crate (`src/io/settings.rs`), not under `tests/`.
- `save_to` derives its temp name with
  `p.set_file_name(format!("{stem}.tmp"))` where `stem` is the full file name;
  the `.bak` name is derived the same way, so `settings.json` yields
  `settings.json.bak` and never collides with `settings.json.tmp`.
- `std::fs::rename` replaces an existing destination on Unix, which gives AC 10's
  "an existing `.bak` is replaced" for free.
- The historical demand `docs/product/demands/LCV-076-agent-settings-ui.md`
  records the original OpenAI-default decision. It is a shipped record — do not
  edit it. This demand supersedes that choice going forward.
- ADR 0002 (`docs/adr/0002-headless-input-tests-and-dirty-tracking.md`) §A2 is
  the authority for "`App::new()` is boot-only, `App::default()` is the test
  constructor"; this demand makes `App::new` read one more real directory, which
  is exactly why that rule exists.
