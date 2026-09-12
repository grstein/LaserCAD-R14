# LCV-080 — Agent panel — chat UI for AI assistant

- **Status**: Done
- **Phase**: 7
- **Depends on**: LCV-079
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: ee10f2a — feat(LCV-080): agent panel — chat UI with background thread and scroll history
- **Note**: this id was planned as the command-line ":" / "/ai" prefix wiring; it shipped as an agent chat side panel. Prefix routing is deferred.

## Problem

Once the multi-turn agent loop (LCV-079) exists it is reachable only from the
command-line prefix `!` buried in a single-line text widget — no conversation
history is visible, no error is surfaced, and there is no way to review what
the LLM replied beyond the last action it took. For a laser-cutting operator
who wants to ask "move all circles to x=50" and then follow up with "undo the
last step", this is unusable: context is lost after every turn and the operator
cannot tell whether the LLM understood the request. This demand adds a right
side-panel that surfaces the full chat history, accepts freeform input at the
bottom, drives `run_agent_turn` on a background thread, and shows a thinking
indicator while the LLM is working. The panel is opt-in: it is hidden by
default and does not affect viewport geometry unless the user opens it.

## Scope

- **`src/agent/panel.rs`** — new file; exports one public function:
  `pub fn draw_agent_panel(ui: &mut egui::Ui, app: &mut App)`.
- **`src/agent/panel.rs`** — also defines `AgentPanelMsg`, a `pub enum` used
  as the channel payload:
  ```rust
  pub enum AgentPanelMsg {
      Reply(String),
      Error(String),
  }
  ```
- **`src/app.rs`** — five new fields on `App` (all `Default`-derived; see §App
  fields below).
- **`src/app.rs`** — poll the `agent_rx` channel each frame inside `App::update`
  and update `agent_chat` / `agent_busy` accordingly.
- **`src/agent/mod.rs`** — re-export `draw_agent_panel` and `AgentPanelMsg`.
- **`src/ui/toolbar.rs`** — add one "🤖" toggle button that flips
  `app.agent_panel_open`; insert it below the existing tool buttons, separated
  by a `ui.separator()`.
- **`src/app.rs`** (`App::update`) — wrap the `SidePanel::right` call so it
  is rendered only when `app.agent_panel_open == true`.

### App fields

Add these five fields to the `App` struct (all satisfy `#[derive(Default)]`):

| Field | Type | Default | Purpose |
|---|---|---|---|
| `agent_panel_open` | `bool` | `false` | Drives the `SidePanel::right` render gate. |
| `agent_chat` | `Vec<(String, String)>` | `vec![]` | Chat history as `(role, content)` pairs. Role is one of `"user"`, `"assistant"`, `"error"`. |
| `agent_input_draft` | `String` | `""` | Live contents of the bottom text-input widget. Cleared on submit. |
| `agent_busy` | `bool` | `false` | Set `true` when a background thread is running; `false` on reply or error. |
| `agent_rx` | `Option<std::sync::mpsc::Receiver<AgentPanelMsg>>` | `None` | Polled each frame; `Some` while a turn is in flight. |

### Panel layout (`draw_agent_panel`)

The function renders an `egui::SidePanel::right("agent_panel")` with
`resizable(true)` and `default_width(300.0)`. Internal layout (top to bottom):

1. **Header row** — `ui.horizontal(|ui| { ui.heading("AI Assistant"); /* spacer */
   if ui.small_button("×").clicked() { app.agent_panel_open = false; } })`.
2. `ui.separator()`
3. **Scroll area** — `egui::ScrollArea::vertical().stick_to_bottom(true)` over
   `app.agent_chat`. For each `(role, content)`:
   - `"user"` → `ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| { ui.label(content); })` — right-aligned.
   - `"assistant"` → `ui.label(content)` — left-aligned, default colour.
   - `"error"` → `ui.colored_label(egui::Color32::RED, content)` — left-aligned, red.
4. **Thinking indicator** — shown only when `app.agent_busy == true`:
   `ui.horizontal(|ui| { ui.spinner(); ui.label("Thinking…"); })`.
5. `ui.separator()`
6. **Input row** — `ui.horizontal(|ui| { ... })` containing:
   - `ui.add(egui::TextEdit::singleline(&mut app.agent_input_draft).hint_text("Ask the AI…").desired_width(f32::INFINITY))` — expands to fill available width.
   - A `ui.button("Send")` that is disabled (`ui.add_enabled(false, ...)`) when
     `app.agent_busy == true` or `app.agent_input_draft.trim().is_empty()`.
   - **Submit logic** fires when:
     - The Send button is clicked, **or**
     - The text widget has keyboard focus and `ui.input(|i| i.key_pressed(egui::Key::Enter))` is true.

### Submit logic (called from within `draw_agent_panel`)

1. Take the trimmed text: `let text = app.agent_input_draft.trim().to_string();`.
   Return early if `text.is_empty()` or `app.agent_busy`.
2. Push `("user".into(), text.clone())` onto `app.agent_chat`.
3. Clear `app.agent_input_draft` → `String::new()`.
4. Build a snapshot: `let history_snap = app.agent_chat.clone();` (role/content
   pairs; the background thread uses this to reconstruct the conversation).
5. Create channel: `let (tx, rx) = std::sync::mpsc::channel::<AgentPanelMsg>();`.
6. Store `app.agent_rx = Some(rx)`.
7. Set `app.agent_busy = true`.
8. Call `std::thread::spawn(move || { ... })`. Inside the closure:
   - Call `crate::agent::run_agent_turn(history_snap)` (signature defined by
     LCV-079; see Notes).
   - On `Ok(reply)` → `tx.send(AgentPanelMsg::Reply(reply)).ok();`.
   - On `Err(e)` → `tx.send(AgentPanelMsg::Error(e.to_string())).ok();`.

### Channel polling in `App::update`

Inside `App::update`, **before** the `CentralPanel::default().show(…)` call,
add a polling block:

```rust
// Poll agent background thread (LCV-080).
if let Some(rx) = &self.agent_rx {
    if let Ok(msg) = rx.try_recv() {
        match msg {
            crate::agent::AgentPanelMsg::Reply(text) => {
                self.agent_chat.push(("assistant".into(), text));
            }
            crate::agent::AgentPanelMsg::Error(e) => {
                self.agent_chat.push(("error".into(), e));
            }
        }
        self.agent_busy = false;
        self.agent_rx = None;
    }
}
```

Also add `ctx.request_repaint()` when `self.agent_busy == true` so the
spinner animates continuously even without user interaction:
```rust
if self.agent_busy {
    ctx.request_repaint();
}
```

### `SidePanel::right` wiring in `App::update`

Add after the `SidePanel::left("toolbar")` block and before the
`CentralPanel::default()` block:

```rust
if self.agent_panel_open {
    crate::agent::draw_agent_panel(
        &mut egui::SidePanel::right("agent_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| ui)
            .inner,
        self,
    );
}
```

> **Note**: the standard egui pattern is to call `draw_agent_panel` inside the
> `SidePanel::right(…).show(ctx, |ui| { draw_agent_panel(ui, self); })` closure;
> use whichever form the borrow checker accepts without unsafe. Either form
> satisfies the acceptance criteria.

### LOC budgets

| File | Budget |
|---|---|
| `src/agent/panel.rs` | ≤ 300 LOC |
| `src/ui/toolbar.rs` | must not exceed 300 LOC after the addition |
| `src/app.rs` | if the polling block pushes the file past 370 LOC, extract all keyboard + agent polling into `src/app/keys.rs` (the `src/app/` directory already exists per LCV-075 note) |

## Out of scope

- **Markdown rendering** of assistant replies — plain text only.
- **Streaming tokens** / partial output — the reply arrives as one complete string.
- **Conversation persistence** across app restarts — `agent_chat` is in-memory only; cleared on next launch.
- **API key / model / endpoint configuration** — owned by LCV-076 (agent settings dialog).
- **Command-line prefix wiring** (`!` / `/ai` routes from the command-line widget
  to the agent loop) — this is the feature that PLAN.md originally called
  LCV-080; see Notes for the ID conflict. That feature is out of scope here.
- **Copy / clear history button** in the panel header.
- **Tool-call display** in the chat history (showing which CAD actions the LLM
  invoked) — deferred; `run_agent_turn` returns a single final summary string.
- **Abort / cancel** while the agent is thinking — the thread runs to completion;
  no cancel support in this demand.
- **Panel width persistence** across restarts.
- **Mobile / touch** — Linux desktop only.

## Acceptance criteria

1. `App::default().agent_panel_open == false`; `App::default().agent_chat` is
   empty; `App::default().agent_busy == false`; `App::default().agent_rx` is
   `None`; `App::default().agent_input_draft.is_empty() == true`.
2. `cargo build --all` exits 0 with `agent_panel_open`, `agent_chat`,
   `agent_input_draft`, `agent_busy`, and `agent_rx` all present on `App`.
3. `AgentPanelMsg` is defined in `src/agent/panel.rs` and re-exported from
   `src/agent/mod.rs`; the variants are `Reply(String)` and `Error(String)`.
4. `src/agent/panel.rs` exports `pub fn draw_agent_panel(ui: &mut egui::Ui,
   app: &mut App)` and is declared as `pub mod panel;` in `src/agent/mod.rs`.
5. When `app.agent_panel_open == false`, no `SidePanel::right("agent_panel")`
   call is made — verified by code review (the `SidePanel::right` call is
   inside `if self.agent_panel_open { … }`).
6. When `app.agent_panel_open == true`, the right panel renders; the header
   contains the text "AI Assistant" and a "×" close button; clicking "×" sets
   `app.agent_panel_open = false` and the panel disappears on the next frame.
7. The 🤖 button in `src/ui/toolbar.rs` toggles `app.agent_panel_open`
   (true→false, false→true) on each click. Its tooltip reads `"AI Assistant"`.
8. With `app.agent_chat = vec![("user".into(), "hello".into()),
   ("assistant".into(), "Hi there!".into()),
   ("error".into(), "timeout".into())]`: the panel's scroll area renders
   exactly three rows; the "timeout" row is coloured red; the "hello" row is
   right-aligned; the "Hi there!" row is left-aligned.
9. The Send button is disabled when `agent_input_draft.trim().is_empty()` — verified by code review.
10. The Send button is disabled when `agent_busy == true` — verified by code review.
11. Submitting (click Send or press Enter with focus on the text input) with
    non-empty `agent_input_draft`:
    a. pushes a `("user", text)` entry onto `agent_chat`;
    b. clears `agent_input_draft` to `""`;
    c. sets `agent_busy = true`;
    d. sets `agent_rx = Some(…)`.
12. When `agent_busy == true`, a spinner and the text "Thinking…" are visible
    in the panel — verified by code review (the thinking block is gated on
    `app.agent_busy`).
13. When `AgentPanelMsg::Reply("done".into())` arrives via the channel: a
    `("assistant", "done")` pair is appended to `agent_chat`; `agent_busy`
    becomes `false`; `agent_rx` becomes `None`.
14. When `AgentPanelMsg::Error("err".into())` arrives: an `("error", "err")`
    pair is appended; `agent_busy` becomes `false`; `agent_rx` becomes `None`.
15. `ctx.request_repaint()` is called each frame when `agent_busy == true` —
    verified by code review (the `if self.agent_busy { ctx.request_repaint(); }`
    guard is present in `App::update`).
16. `src/agent/panel.rs` is ≤ 300 LOC (`wc -l src/agent/panel.rs`).
17. `grep -nE '^use (eframe|rfd)' src/agent/panel.rs` returns no matches
    (panel may import `egui` and `crate::app::App`; it must not import `eframe`
    or `rfd`).
18. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.
19. **Manual smoke** — `cargo run` → click 🤖 button → panel appears on the
    right → type "hello" in the input → press Enter → "hello" appears
    right-aligned in the history, spinner + "Thinking…" appears, Send button is
    disabled → after the thread returns (stub or real), the reply (or error)
    appears, spinner disappears, Send button re-enables → click "×" in the
    panel header → panel closes.

## Expected tests

All unit tests live in `#[cfg(test)] mod tests` at the bottom of their
respective source files. Tests that require an egui context are manual (see
AC#19); pure logic tests can be unit-tested headlessly.

**`src/agent/panel.rs`**

- `agent_panel_msg_variants_compile` (AC#3): construct both `AgentPanelMsg::Reply("x".into())` and `AgentPanelMsg::Error("e".into())`; assert their discriminants differ. This is a compile-time proof the enum exists with those variants.

**`src/app.rs`**

- `app_default_agent_fields` (AC#1): `let a = App::default(); assert!(!a.agent_panel_open); assert!(a.agent_chat.is_empty()); assert!(!a.agent_busy); assert!(a.agent_rx.is_none()); assert!(a.agent_input_draft.is_empty());`
- `agent_rx_reply_updates_chat_and_clears_busy` (AC#13): construct an
  `mpsc::channel`, store `rx` in `app.agent_rx`, set `app.agent_busy = true`,
  send `AgentPanelMsg::Reply("done".into())` via `tx`, then run the polling
  block manually (extracted into a helper `pub fn poll_agent_rx(app: &mut App)`)
  and assert `app.agent_chat.last() == Some(&("assistant".into(), "done".into()))`,
  `app.agent_busy == false`, `app.agent_rx.is_none()`.
- `agent_rx_error_updates_chat_and_clears_busy` (AC#14): same pattern with
  `AgentPanelMsg::Error("err".into())`, assert role is `"error"`.

**Static / CI checks (all ACs)**

- **AC#5** — panel gated: `grep -n "agent_panel_open" src/app.rs` shows the
  `SidePanel::right` call is inside the conditional block.
- **AC#9, AC#10** — Send disabled: `grep -n "add_enabled" src/agent/panel.rs`
  confirms the disabled guard.
- **AC#12** — thinking block: `grep -n "agent_busy" src/agent/panel.rs`
  confirms the spinner is guarded by `agent_busy`.
- **AC#15** — repaint: `grep -n "request_repaint" src/app.rs` confirms the
  `if self.agent_busy` guard.
- **AC#16** — LOC: `wc -l src/agent/panel.rs` ≤ 300.
- **AC#17** — purity: `grep -nE '^use (eframe|rfd)' src/agent/panel.rs` exits empty.
- **AC#18** — build gate: `cargo fmt --all -- --check && cargo clippy
  --all-targets -- -D warnings && cargo test --all` exits 0.
- **AC#19** — manual smoke: described in acceptance criteria above.

## Open questions

(none)

## Notes

### Interface contract with LCV-079

This demand calls `crate::agent::run_agent_turn(history_snap)`. The exact
function signature — including whether it takes `Vec<(String, String)>`,
a structured `ChatMessage` type, `&AgentSettings`, or a `tokio` runtime
handle — is defined by LCV-079. When LCV-079 is implemented, the implementer
of LCV-080 must adapt the closure to match. If LCV-079's function is `async`,
the background thread must create a single-threaded `tokio` runtime:
```rust
std::thread::spawn(move || {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let result = rt.block_on(crate::agent::run_agent_turn(history_snap));
    tx.send(match result { Ok(s) => AgentPanelMsg::Reply(s), Err(e) => AgentPanelMsg::Error(e.to_string()) }).ok();
});
```
Do **not** use `tokio::spawn` here — the agent HTTP call lives in the spawned
OS thread, not the eframe tokio runtime (which does not exist by default).

### `poll_agent_rx` testable helper

To keep AC#13 / AC#14 testable without an egui context, extract the polling
block into a free function `pub fn poll_agent_rx(app: &mut App)` in `src/app.rs`
(following the `handle_undo` / `handle_redo` pattern from LCV-075).  Call it
from `App::update` instead of inlining the block.

### `ScrollArea::stick_to_bottom`

`egui::ScrollArea::vertical().stick_to_bottom(true)` auto-scrolls to the
newest message. This is the correct API in egui ≥ 0.27. Verify the egui
version in `Cargo.toml` before using; if the method is absent, use
`scroll_to_cursor` or keep a frame-delayed scroll state.

### Panel width default

300 px is a suggested `default_width`. The implementer may choose a different
value if 300 px is too wide on a 1080 p monitor with the toolbar; any value in
the range 240–360 px satisfies the acceptance criteria.

### ID conflict — PLAN.md

`PLAN.md` currently assigns LCV-080 to *"Command-line wires `:` / `/ai`
prefixes to agent"* (Phase 7). That is a distinct feature from this panel.
This demand has been written at LCV-080 at the user's explicit instruction.
`demand-manager` must renumber the command-line wiring feature to a new ID
(e.g., LCV-081) and update the PLAN.md table before either feature is
scheduled for implementation.

### No `eframe` in `panel.rs`

`src/agent/panel.rs` receives `ui: &mut egui::Ui` directly; it does not need
`eframe` or `rfd`. The purity check (`grep -nE '^use (eframe|rfd)'`) must pass.
`egui` itself is permitted.

### LOC pressure on `src/app.rs`

As of LCV-075 demand-write time, `src/app.rs` was at 336 LOC. The five new
fields (≈10 LOC), polling block (≈15 LOC), repaint guard (≈3 LOC), and
`SidePanel::right` wiring (≈6 LOC) add ≈34 lines. If this pushes `app.rs`
past 370 LOC, extract all non-`App`-struct logic into `src/app/agent_poll.rs`
(or merge into the existing `src/app/keys.rs`). No architecture review is
required — this is routine within-component decomposition.
