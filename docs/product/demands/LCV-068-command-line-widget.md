# LCV-068 — Command-line widget (bottom dock)

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-040 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Once drawing tools are active, the operator has no text channel for entering
precise numeric values — distance, angle, or absolute coordinates — without
clicking on the canvas. In a laser-cutting workflow this matters: a user
drawing a 47.5 mm guide line must be able to type `47.5` and press Enter
rather than trying to hit an exact pixel on screen. AutoCAD R14 solves this
with a persistent command-line strip at the bottom of the window: a prompt
showing the tool's current instruction, followed by a single-line text field
the user types into. This demand ships that strip.

## Scope

- **New file `src/ui/command_line.rs`** containing exactly one public item:
  `pub fn draw_command_line(ui: &mut egui::Ui, app: &mut App)`.
  - Renders a horizontal row: a read-only label showing
    `app.tool_manager.active_status_text()` followed by a single-line
    `egui::TextEdit` bound to `app.command_line_input`.
  - **Enter** (while the TextEdit has keyboard focus): submits the current
    text to `tool_manager.on_command_input(text, &mut doc, &mut history)` via
    the `std::mem::take` borrow-split pattern, then clears
    `app.command_line_input` to `""`.
  - **Escape** (while the TextEdit has keyboard focus): clears
    `app.command_line_input` to `""`, cancels the active tool via
    `tool_manager.handle_key(Key::Escape, app)` (same `std::mem::take`
    pattern already used in `App::update`), and consumes the Escape key event
    via `ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE,
    egui::Key::Escape))` so `App::update`'s existing Escape handler does not
    double-fire.
  - File is `≤ 150 LOC` (blank lines and comments included). No `rfd` import.

- **`src/ui/mod.rs`**: add `pub mod command_line;` and
  `pub use command_line::draw_command_line;`.

- **`src/tools/tool.rs`** — add one new method to `pub trait Tool`:
  ```rust
  fn on_command_input(
      &mut self,
      input: &str,
      doc: &mut Document,
      history: &mut History,
  ) {}
  ```
  Default body is empty (no-op). The trait must remain object-safe after the
  addition; no generic parameters or `Self` references.

- **`src/tools/manager.rs`** — add two new public methods to `ToolManager`:
  - `pub fn active_status_text(&self) -> &'static str` — delegates to
    `self.active.status_text()`.
  - `pub fn on_command_input(&mut self, input: &str, doc: &mut Document,
    history: &mut History)` — delegates to
    `self.active.on_command_input(input, doc, history)`.

- **`src/app.rs`** (`App` struct) — add one field:
  `pub command_line_input: String` initialised to `String::new()` by the
  `#[derive(Default)]`.

- **`src/app.rs`** (`App::update`) — add one panel call:
  ```rust
  egui::TopBottomPanel::bottom("command_line").show(ctx, |ui| {
      crate::ui::draw_command_line(ui, self);
  });
  ```
  This call must appear **after** `TopBottomPanel::bottom("statusbar")` and
  **before** `CentralPanel::default()`. In egui, bottom panels are stacked
  from the window edge up in declaration order, so declaring `"statusbar"`
  first places it at the very bottom; `"command_line"` declared second
  appears immediately above it.

## Out of scope

- **Tool command parsing** (coordinate formats `X,Y`, `@X,Y`, tool aliases
  `L`/`C`/`A`) — owned by the individual tool demands (LCV-043, LCV-046,
  LCV-047, …); this demand delivers the input channel only.
- **Command history** (Up/Down arrow to cycle previous inputs) — not in v0.1.0.
- **Auto-complete or inline hints** in the text field — not in v0.1.0.
- **Multi-line output area** above the input (classic AutoCAD prompt log) —
  not in v0.1.0; a single-line prompt label is sufficient.
- **Agent prefix routing** (`:` / `/ai` prefixes) — owned by LCV-075 and
  LCV-080.
- **Visual theme polish** (font size, background colour) — owned by LCV-071.
- **Any interaction when the TextEdit does not have keyboard focus** — typing
  into other egui widgets (e.g. a dialog) must not affect `command_line_input`.

## Acceptance criteria

1. `src/ui/command_line.rs` exists, exports `pub fn draw_command_line(ui:
   &mut egui::Ui, app: &mut App)`, is `≤ 150 LOC`, and contains no `rfd`
   import.

2. `src/ui/mod.rs` declares `pub mod command_line;` and re-exports
   `pub use command_line::draw_command_line;`.

3. `App` struct in `src/app.rs` carries `pub command_line_input: String`.
   `App::default().command_line_input` is the empty string `""`.

4. `pub trait Tool` in `src/tools/tool.rs` declares `fn on_command_input(&mut
   self, input: &str, doc: &mut Document, history: &mut History) {}` with a
   default no-op body. The trait is still object-safe: `let _: Box<dyn Tool> =
   Box::new(SelectTool);` compiles.

5. `ToolManager` exposes `pub fn active_status_text(&self) -> &'static str`
   that returns `self.active.status_text()`. When `SelectTool` is active,
   `active_status_text()` returns `"Select"` (the `status_text()` default
   delegates to `name()`).

6. `ToolManager` exposes `pub fn on_command_input(&mut self, input: &str, doc:
   &mut Document, history: &mut History)` that delegates to
   `self.active.on_command_input(input, doc, history)`.

7. `draw_command_line` renders a horizontal row containing: (a) a read-only
   label whose text equals `app.tool_manager.active_status_text()`, and (b)
   a single-line `egui::TextEdit` whose mutable buffer is
   `app.command_line_input`.

8. When the TextEdit has keyboard focus and Enter is pressed, the widget calls
   `tool_manager.on_command_input` with the current input text,
   then sets `app.command_line_input` to `""`. The active tool's
   `on_command_input` is called exactly once per Enter press with a `&str`
   equal to the text that was in the field at the time of the press.

9. When the TextEdit has keyboard focus and Escape is pressed, the widget: (a)
   sets `app.command_line_input` to `""`; (b) calls `tool_manager.handle_key(
   egui::Key::Escape, app)` to cancel the active tool; (c) consumes the
   Escape key event via `ui.input_mut` so `App::update`'s existing Escape
   handler does not fire a second time in the same frame.

10. `App::update` calls `draw_command_line` inside
    `egui::TopBottomPanel::bottom("command_line")`. That panel call appears in
    source **after** `TopBottomPanel::bottom("statusbar")` and **before**
    `CentralPanel::default()`, causing the command-line strip to render above
    the status bar and below the viewport.

11. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All unit tests live in `#[cfg(test)] mod tests` inside
`src/ui/command_line.rs` unless noted otherwise.

- **AC 3 — field default** (`src/app.rs` tests): add
  `fn app_default_command_line_input_is_empty` asserting
  `App::default().command_line_input.is_empty()`.

- **AC 4 — trait object-safety** (`src/tools/` tests or compile-only):
  `fn tool_with_command_input_is_object_safe` — `let _: Box<dyn Tool> =
  Box::new(SelectTool);` compiles after the new method is added.

- **AC 4 — default no-op** (`src/tools/` tests): `fn
  select_tool_on_command_input_is_noop` — construct `SelectTool::default()`,
  call `on_command_input("50", &mut Document::default(), &mut
  History::default())`, assert no panic and `Document::default().entity_count()
  == 0`.

- **AC 5 — active_status_text default** (`src/tools/manager.rs` tests): `fn
  tool_manager_active_status_text_defaults_to_name` — `assert_eq!(
  ToolManager::default().active_status_text(), "Select")`.

- **AC 5 — active_status_text override** (`src/tools/manager.rs` tests): `fn
  tool_manager_active_status_text_reflects_override` — construct a mock `Tool`
  where `status_text()` returns `"LINE: Click start point"`, wrap in a
  `ToolManager`, assert `active_status_text() == "LINE: Click start
  point"`.

- **AC 6 — on_command_input delegation** (`src/tools/manager.rs` tests): `fn
  tool_manager_on_command_input_delegates` — construct a mock `Tool` that
  records `on_command_input` calls into a `RefCell<Vec<String>>`, wrap in a
  `ToolManager`, call `manager.on_command_input("42", &mut doc, &mut hist)`,
  assert the recorded string is `"42"`.

- **AC 8 — Enter submits and clears** (manual smoke): `cargo run`, activate
  the LINE tool, click in the command-line field, type `50`, press Enter —
  assert that `command_line_input` is empty afterward (no panic; the tool
  receives `"50"` as input).

- **AC 9 — Escape clears and cancels** (manual smoke): `cargo run`, activate
  the LINE tool, click the first point on the canvas, click in the
  command-line field, type some text, press Escape — assert the text field is
  cleared and the LINE tool resets to idle (no in-progress preview line).

- **AC 10 — panel ordering** (static review): `grep` confirms
  `"command_line"` appears after `"statusbar"` and before `CentralPanel` in
  `src/app.rs`.

- **AC 11 — build gate**: `cargo fmt --all -- --check && cargo clippy
  --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

*(none)*

## Notes

- **egui Enter detection** — the standard egui single-line TextEdit pattern
  for submit-on-Enter is:
  ```rust
  let response = ui.add(egui::TextEdit::singleline(&mut app.command_line_input));
  if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
      // submit
  }
  ```
  Pressing Enter in a TextEdit causes the widget to lose focus, so
  `response.lost_focus()` is `true` on that frame.

- **Escape key consumption** — `App::update` reads
  `ctx.input(|i| i.key_pressed(egui::Key::Escape))` inside `CentralPanel`.
  Because `draw_command_line` runs inside a `TopBottomPanel` that is
  processed **before** `CentralPanel`, calling `ui.input_mut(|i|
  i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))` in the command
  line handler will suppress the key before the viewport block reads it.
  The implementer must verify this behaviour with the egui version pinned in
  `Cargo.lock`.

- **Borrow split for Enter** — `draw_command_line` takes `&mut App`. Calling
  `app.tool_manager.on_command_input(&text, &mut app.document, &mut
  app.history)` creates a simultaneous mutable alias. Use the same
  `std::mem::take` pattern already present in `App::update`:
  ```rust
  let text = std::mem::take(&mut app.command_line_input);
  let mut tm = std::mem::take(&mut app.tool_manager);
  tm.on_command_input(&text, &mut app.document, &mut app.history);
  app.tool_manager = tm;
  // command_line_input is already "" after mem::take
  ```

- **Borrow split for Escape** — same pattern; after clearing
  `command_line_input`, take `tool_manager` out, call `handle_key(Escape,
  app)`, restore:
  ```rust
  app.command_line_input.clear();
  let mut tm = std::mem::take(&mut app.tool_manager);
  tm.handle_key(egui::Key::Escape, app);
  app.tool_manager = tm;
  ```

- **`status_text()` already exists on `Tool`** (shipped in LCV-040,
  `src/tools/tool.rs` line 75). Its default returns `self.name()`. No tool
  needs to be changed to satisfy the prompt display; tools that want richer
  prompts (e.g., `"LINE: Click to set end point"`) override `status_text()`
  in their own demands.

- **`ToolManager` must not import `eframe` or `rfd`** (purity rule in
  `AGENTS.md`). The two new methods on `ToolManager` only import from
  `crate::document`, which is already present.

- **`command_line.rs` must not import `eframe` or `rfd`** (same rule). It
  imports `egui` and `crate::app::App` only.

- **v1 reference** — v1.0.0 had a bottom command bar showing a text prompt
  and an `<input>` field. The same UX intent carries over. The multi-line
  prompt history log visible in some AutoCAD screenshots is explicitly out
  of scope for v0.1.0 (see Out of scope above).

- **Agent harness hook** — LCV-075 (agent classifier) and LCV-080 (`:` / `/ai`
  prefix routing) depend on this demand. The `on_command_input` method is the
  integration point they will use; LCV-068 must not pre-implement any
  classification logic.
