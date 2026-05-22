# LCV-076 — Agent settings UI (API key + endpoint)

- **Status**: Ready
- **Phase**: 7
- **Depends on**: LCV-058 (Done), LCV-069 (Ready)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

The agent harness (Phase 7) must call an OpenAI-compatible HTTP endpoint with an
API key supplied by the operator. Neither value belongs in source code or the
`Cargo.toml`; both must survive process exit in the existing settings JSON
(LCV-058). Without a settings UI the operator has no way to configure the
endpoint or key at runtime — the agent transport (LCV-077) would hard-code a URL
and have no key. This demand closes the configuration gap: extend `Settings`
with the two agent fields and provide the egui panel that edits them, keeping the
kernel (`src/agent/` except `settings_ui.rs`) free of egui.

## Scope

### 1 — `src/io/settings.rs` — extend `Settings`

Add two fields to `pub struct Settings`:

```rust
/// OpenAI-compatible API base URL.
/// Default: `"https://api.openai.com/v1"`.
#[serde(default = "default_agent_endpoint")]
pub agent_endpoint: String,

/// API key sent in the `Authorization: Bearer …` header.
/// Default: `""` (agent disabled until set).
#[serde(default)]
pub agent_api_key: String,
```

Add a private helper used by serde:

```rust
fn default_agent_endpoint() -> String {
    "https://api.openai.com/v1".to_string()
}
```

Because `String::default()` is `""` — not the desired endpoint string — remove
`Default` from the `#[derive(…)]` list and write a manual `impl Default for
Settings`:

```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_files:   Vec::new(),
            agent_endpoint: default_agent_endpoint(),
            agent_api_key:  String::new(),
        }
    }
}
```

No other changes to `src/io/settings.rs`.

### 2 — `src/agent/settings_ui.rs` — new file

```rust
//! egui panel for editing agent connection settings.
//!
//! This is the ONLY file in `src/agent/` that may import `egui`.
//! All other `agent/` submodules must remain kernel-pure.

use egui;
use crate::io::settings::Settings;

/// Draw the agent-settings form into `ui`.
///
/// Renders two labelled rows in a two-column grid:
/// - **Endpoint URL** — a single-line text field editing `settings.agent_endpoint`.
/// - **API Key** — a password-masked single-line field editing `settings.agent_api_key`;
///   uses `egui::TextEdit::singleline(…).password(true)` so the value is shown
///   as bullet characters (•) and is excluded from egui's accessibility tree text.
///
/// Both fields have a minimum width of 320 logical pixels.
/// The function returns `true` if either field was modified in this frame
/// (`response.changed()` was true for at least one `TextEdit`), `false` otherwise.
pub fn draw_agent_settings(ui: &mut egui::Ui, settings: &mut Settings) -> bool { … }
```

> **Exact signature** (including the `bool` return):
> `pub fn draw_agent_settings(ui: &mut egui::Ui, settings: &mut Settings) -> bool`
>
> The `bool` return lets the caller trigger a settings save without storing
> intermediate copies of the struct.

File **must not** import `eframe` or `rfd`.  
File LOC ≤ 100.

### 3 — `src/agent/mod.rs` — declare submodule

Add exactly one line to `src/agent/mod.rs`:

```rust
pub mod settings_ui;
```

No other `agent/` submodule may import `egui`, `eframe`, or `rfd`.

### 4 — `src/app.rs` — dialog field + window wiring

**Field** — add to `App`:

```rust
/// Controls visibility of the Agent Settings dialog.
/// The LCV-065 menubar will set this to `true`; LCV-076 owns the field.
pub agent_settings_open: bool,
```

`agent_settings_open` must participate in `App::default()` as `false` — either
via `#[derive(Default)]` (bool defaults to false) or explicit assignment in
`App::new()`.

**Window call** — in `App::update`, immediately after the existing
`crate::ui::about_dialog(ctx, &mut self.about_open)` call, add:

```rust
// LCV-076 — Agent Settings dialog.
{
    let open = &mut self.agent_settings_open;
    let settings = &mut self.settings;
    let was_open = *open;
    egui::Window::new("Agent Settings")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            crate::agent::settings_ui::draw_agent_settings(ui, settings);
        });
    // Save on dialog close (× button or programmatic close).
    if was_open && !*open {
        settings.save().ok();
    }
}
```

> The explicit borrow-split (`open` + `settings` as separate `&mut` bindings)
> avoids a partial-borrow compile error. Do not refactor to `self.X` inside
> the closure.

No other changes to `src/app.rs`.

## Out of scope

- Endpoint URL validation (the transport layer in LCV-077 handles connection
  errors at runtime).
- "Clear API key" confirmation dialog (LCV-069 provides `confirm_dialog`; a
  future polish demand can wire it).
- A "Test connection" button.
- Model name / temperature / token-cap fields — those belong to LCV-077 or
  later.
- The "Agent Settings" menu item that sets `agent_settings_open = true` — that
  is LCV-065 (Menubar).
- Masking / redacting the key in log output — LCV-077's transport layer concern.
- Multi-account or per-project keys.

## Acceptance criteria

1. `Settings::default().agent_endpoint == "https://api.openai.com/v1"`.

2. `Settings::default().agent_api_key == ""`.

3. Existing LCV-058 `Settings` tests still pass without modification (the new
   fields carry `#[serde(default)]`, so old JSON without those keys deserialises
   to the correct defaults).  
   _Verify_: `cargo test -p lasercad -- io::settings` exits 0.

4. Round-trip: `save_to(&settings_with_custom_agent_fields, path)` followed by
   `load_from(path)` returns a `Settings` equal to the original, preserving
   non-default values for both `agent_endpoint` and `agent_api_key`.  
   _Verify_: unit test (§Expected tests §T2).

5. `draw_agent_settings` is declared `pub` in `src/agent/settings_ui.rs` with
   the exact signature `pub fn draw_agent_settings(ui: &mut egui::Ui, settings: &mut Settings) -> bool`.  
   _Verify_: `grep -n 'pub fn draw_agent_settings' src/agent/settings_ui.rs`
   returns exactly one match.

6. Calling `draw_agent_settings` with a fresh `egui::Context` and
   `Settings::default()` without synthesising any input returns `false`
   (no change).  
   _Verify_: unit test (§Expected tests §T3).

7. The endpoint field renders as a plain (non-password) `egui::TextEdit::singleline`.
   The API-key field renders as `egui::TextEdit::singleline(…).password(true)`.  
   _Verify_: `grep -n 'password(true)' src/agent/settings_ui.rs` returns exactly
   one match; `grep -c 'TextEdit::singleline' src/agent/settings_ui.rs` returns 2.

8. `grep -nE '^use (eframe|rfd)' src/agent/settings_ui.rs` returns no matches.  
   No other file under `src/agent/` (except `settings_ui.rs`) imports `egui`,
   `eframe`, or `rfd`.  
   _Verify_: `grep -rn 'egui\|eframe\|rfd' src/agent/ --include='*.rs' | grep -v settings_ui` returns empty.

9. `App::default().agent_settings_open == false`.  
   _Verify_: unit test (§Expected tests §T4).

10. `App::update` renders the "Agent Settings" `egui::Window` in the same frame
    that `self.agent_settings_open` is `true`, and does NOT render it when
    `agent_settings_open` is `false`.  
    _Verify_: code review (static read of `src/app.rs`; confirm the window call
    is guarded by the existing `open` flag via `egui::Window::open()`).

11. When the dialog's × button is clicked (i.e., egui sets `agent_settings_open`
    to `false`), `settings.save()` is called in that same frame.  
    _Verify_: code review confirms `settings.save().ok()` is called in the
    `was_open && !*open` branch immediately after the window block.

12. `wc -l src/agent/settings_ui.rs` ≤ 100.  
    _Verify_: static check exits with count ≤ 100.

13. **Manual smoke**: `cargo run` → open a Rust debugger or temporarily set
    `agent_settings_open: true` in `App::default()` → confirm the "Agent
    Settings" window appears centred → change the endpoint field to
    `"https://openrouter.ai/api/v1"` → close with × → reopen → confirm the
    new value persists (was saved to JSON) → confirm the API key field shows
    bullet characters when text is entered.  Record pass/fail in the
    `Implementation:` line.

14. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All tests live in `#[cfg(test)] mod tests` blocks inside their respective files.
egui tests use `egui::Context::default()` + `ctx.run(egui::RawInput::default(), |ctx| { … })`.

**`src/io/settings.rs`**

- **§T1 — AC 1 + AC 2 — field defaults**  
  ```rust
  let s = Settings::default();
  assert_eq!(s.agent_endpoint, "https://api.openai.com/v1");
  assert_eq!(s.agent_api_key, "");
  ```

- **§T2 — AC 4 — agent-field round-trip**  
  Create `Settings { agent_endpoint: "https://openrouter.ai/api/v1".into(), agent_api_key: "sk-test".into(), ..Settings::default() }`.  
  Call `save_to(&s, &tmp_path)`. Call `load_from(&tmp_path)`. Assert the result
  equals the original `s`.

- **§T2b — AC 3 — legacy JSON compatibility**  
  Write `{"recent_files":[]}` (no agent fields) to a tmp file. Call
  `load_from(&tmp_path)`. Assert `result.agent_endpoint == "https://api.openai.com/v1"` and `result.agent_api_key == ""`.

**`src/agent/settings_ui.rs`**

- **§T3 — AC 6 — no-input returns false**  
  ```rust
  let ctx = egui::Context::default();
  let mut settings = Settings::default();
  let changed = ctx.run(egui::RawInput::default(), |ctx| {
      egui::CentralPanel::default().show(ctx, |ui| {
          draw_agent_settings(ui, &mut settings)
      }).inner
  }).1; // extract the return value from the inner closure
  assert!(!changed);
  ```
  _(egui 0.29: `ctx.run` returns `(PlatformOutput, T)` where `T` is the closure
  return; extract accordingly.)_

**`src/app.rs`**

- **§T4 — AC 9 — agent_settings_open defaults to false**  
  ```rust
  let app = App::default();
  assert!(!app.agent_settings_open);
  ```

**Static / CI checks**

- **AC 7**: `grep -n 'password(true)' src/agent/settings_ui.rs` → exactly 1 match;
  `grep -c 'TextEdit::singleline' src/agent/settings_ui.rs` → 2.
- **AC 8**: `grep -rn 'egui\|eframe\|rfd' src/agent/ --include='*.rs' | grep -v settings_ui` → empty.
- **AC 12**: `wc -l src/agent/settings_ui.rs` → ≤ 100.
- **AC 14**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **`Default` derivation change**: `Settings` currently uses `#[derive(Default)]`.
  Adding `agent_endpoint` with a non-empty default requires switching to a manual
  `impl Default`. This is a mechanical change; the existing LCV-058 tests remain
  valid because they only assert `recent_files.is_empty()` and round-trip equality,
  not the specific form of `Default`.

- **Borrow split pattern**: `App::update` must split `&mut self.agent_settings_open`
  and `&mut self.settings` into two named `&mut` bindings before passing them to
  the window and closure respectively. Rust allows disjoint field borrows at the
  statement level; the named-binding pattern makes this explicit and avoids the
  compiler's partial-borrow diagnostic.

- **Save-on-close, not save-on-keystroke**: writing the JSON on every keystroke
  would thrash the filesystem. The save fires once — in the frame the dialog closes
  (either the × button or a future "OK" button). `settings.save()` is infallible at
  the call site (errors are swallowed with `.ok()`); LCV-077 (transport) can surface
  missing-key errors at connection time.

- **`egui::Window::open()` semantics (egui 0.29)**: `Window::open(&mut bool)` adds
  a × button; when clicked egui sets `*bool = false` _before_ your post-window code
  runs in the same frame. The `was_open && !*open` pattern therefore fires exactly
  once — on the close frame.

- **Password masking**: `egui::TextEdit::singleline(&mut text).password(true)` in
  egui 0.29 replaces each character with a bullet glyph (•) in the rendered widget.
  The underlying `String` is still stored in plaintext in `Settings`; at-rest
  encryption is an explicit non-goal for v0.1.0.

- **Default endpoint**: `"https://api.openai.com/v1"` is the OpenAI default, but the
  product README describes the target as "OpenAI-compatible LLM (default OpenRouter)".
  The PO has chosen `api.openai.com` as the field default because it is the most
  widely recognised base URL; operators using OpenRouter will update it once in
  the settings dialog. A future demand can change the compiled-in default if the
  product shifts.

- **LCV-077 dependency**: this demand does not add `reqwest`, `tokio`, or any HTTP
  code. LCV-077 reads `settings.agent_endpoint` and `settings.agent_api_key` from
  the `App`-owned `Settings` struct.

- **Menubar hook (LCV-065)**: the "Agent Settings" menu item under a future "Tools"
  or "Agent" menu will do `self.agent_settings_open = true;`. The field and window
  call land here so that LCV-065 needs no further coordination.
