# LCV-075 — Undo/Redo keyboard shortcuts (Ctrl+Z / Ctrl+Y) + status bar flash

- **Status**: Ready
- **Phase**: 6
- **Depends on**: LCV-026 (done), LCV-030 (done), LCV-067 (done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

`History::undo` and `History::redo` (LCV-026) are fully implemented and `App::commit`
(LCV-030) is the sole mutation entry point, yet Ctrl+Z and Ctrl+Y do nothing — the
keys are absent from `App::update`. In a laser-cutting CAD session, undo is a
non-negotiable safety net: an operator who places a line in the wrong position,
trims the wrong segment, or deletes the wrong entity must be able to recover in
one keystroke. Without keyboard wiring the 200-deep history stack is invisible
and every accidental mutation is permanent. This demand closes that gap:
wire Ctrl+Z → `history.undo` and Ctrl+Y → `history.redo`, and confirm each
activation with a brief "Undo" / "Redo" label in the existing status bar.

## Scope

### 1 — `App` gains a `status_flash` field (`src/app.rs`)

Add to `App`:

```rust
/// Transient status-bar message and its egui-time expiry (seconds).
/// Set to `Some(("Undo".into(), now + 1.5))` or `Some(("Redo".into(), …))`
/// by the keyboard handlers; cleared each frame when expired.
pub status_flash: Option<(String, f64)>,
```

`Option<(String, f64)>` implements `Default` as `None`, so `#[derive(Default)]`
on `App` continues to work unchanged.

### 2 — Two testable free functions (`src/app.rs`)

Following the existing `handle_wheel_zoom` / `handle_pan` / `handle_zoom_extents`
pattern, add two free functions at the bottom of `src/app.rs`:

```rust
/// Call `history.undo`; if it returns `true`, set `app.status_flash`.
/// `current_time` is `ctx.input(|i| i.time)` at the call site.
/// Returns `true` when a command was undone.
pub fn handle_undo(app: &mut App, current_time: f64) -> bool { … }

/// Call `history.redo`; if it returns `true`, set `app.status_flash`.
/// Returns `true` when a command was redone.
pub fn handle_redo(app: &mut App, current_time: f64) -> bool { … }
```

Each function:
- calls `app.history.undo(&mut app.document)` / `app.history.redo(&mut app.document)`,
- if the call returns `true`, sets
  `app.status_flash = Some(("Undo".into(), current_time + 1.5))` or
  `Some(("Redo".into(), current_time + 1.5))`,
- returns the `bool` from `history.undo` / `history.redo`.

### 3 — Keyboard handling in `App::update` (`src/app.rs`)

Inside the `CentralPanel` closure, **after** the Backspace block and **before**
`ctx.request_repaint()`, add three new blocks:

```rust
// Flash expiry — clear before any panel paints.
if let Some((_, exp)) = self.status_flash {
    if ctx.input(|i| i.time) >= exp {
        self.status_flash = None;
    }
}

// Ctrl+Z — undo (LCV-075).
if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
    handle_undo(self, ctx.input(|i| i.time));
}

// Ctrl+Y — redo (LCV-075).
if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
    handle_redo(self, ctx.input(|i| i.time));
}
```

Both handlers fire **unconditionally** inside `App::update` — they are NOT
gated by `response.hovered()` or any cursor-over-viewport check.  Add a
`// TODO LCV-070: Ctrl+Shift+Z redo alias` comment after the Ctrl+Y block.

### 4 — Status bar flash segment (`src/ui/statusbar.rs`)

Extract a small pure helper and extend `draw_statusbar`:

```rust
/// Return the flash message if `flash` is `Some` and not yet expired.
/// Pure function — no egui context needed; unit-testable.
pub(crate) fn flash_label<'a>(
    flash: Option<&'a (String, f64)>,
    current_time: f64,
) -> Option<&'a str> {
    flash
        .filter(|(_, exp)| current_time < *exp)
        .map(|(msg, _)| msg.as_str())
}
```

In `draw_statusbar`, append after the "Entities: N" segment:

```rust
if let Some(label) = flash_label(
    app.status_flash.as_ref(),
    ui.ctx().input(|i| i.time),
) {
    ui.separator();
    ui.label(label);
}
```

`draw_statusbar`'s signature stays `pub fn draw_statusbar(ui: &mut egui::Ui, app: &App)`.
The file must remain ≤ 150 LOC.

### 5 — LOC budget

`src/app.rs` currently exceeds 300 LOC. If adding the three blocks from §3 plus
two helpers from §2 would push the file to ≥ 370 LOC, the implementer **must**
extract all keyboard handling (Escape, Delete, Backspace, zoom-extents keys,
Ctrl+Z/Y) into `src/app/keys.rs` as a new submodule. The `src/app/` directory
already exists (hosts `snap.rs`); no architectural consultation is required.
The submodule must re-export nothing — `app.rs` calls it via
`crate::app::keys::handle_keys(self, ctx, rect)` or inline calls.

## Out of scope

- **Edit menu "Undo" / "Redo" items** (greyed-out when stack is empty) —
  deferred to LCV-065 (Menubar).
- **Undo/redo labels in the flash** ("Undo CreateLine") — `History::peek` is
  explicitly deferred in LCV-026 Notes; not in this demand.
- **Ctrl+Z inside the command-line text widget** cancelling mid-entry input —
  that is egui's native text-edit behaviour; owned by LCV-068.
- **Ctrl+Shift+Z** as an additional redo alias — LCV-070 can add it later.
- **Cmd+Z / Cmd+Y on macOS** — cross-platform concerns are Phase 9.
- **Configurable flash duration** — 1.5 s is fixed.
- **Command grouping or coalescing** — explicitly out of scope per LCV-026.
- **Undo history panel** listing the last N command labels.

## Acceptance criteria

1. `App` has `pub status_flash: Option<(String, f64)>` and
   `App::default().status_flash == None`.
2. `handle_undo(&mut app, 0.0)` on a fresh `App::default()` returns `false` and
   leaves `app.status_flash == None` (undo on empty history must not set the
   flash).
3. After `app.commit(Box::new(CreateLine::new(line_a())))`, calling
   `handle_undo(&mut app, 0.0)` returns `true`, `app.document.entities` is
   empty, and `app.status_flash == Some(("Undo".to_string(), 1.5))`.
4. Continuing from AC#3, calling `handle_redo(&mut app, 0.0)` returns `true`,
   `app.document.entities.len() == 1`, and
   `app.status_flash == Some(("Redo".to_string(), 1.5))`.
5. `handle_undo` on an empty undo stack (no commands committed) returns `false`;
   `handle_redo` on an empty redo stack returns `false`; neither sets
   `status_flash`.
6. `flash_label(Some(&("Undo".to_string(), 10.0)), 9.9)` returns `Some("Undo")`.
7. `flash_label(Some(&("Undo".to_string(), 10.0)), 10.0)` returns `None`
   (expiry is exclusive: `current_time < exp`).
8. `flash_label(None, 5.0)` returns `None`.
9. The Ctrl+Z and Ctrl+Y handlers in `App::update` are **not** nested inside
   `if response.hovered()` — verified by code review (grep / static read).
10. `src/ui/statusbar.rs` remains ≤ 150 LOC; `grep -nE '^use (eframe|rfd)'
    src/ui/statusbar.rs` returns no matches.
11. **Manual smoke**: `cargo run` → draw two lines with LineTool → press Ctrl+Z
    → first line disappears, "Undo" appears in the status bar for ~1.5 s, then
    vanishes → press Ctrl+Z → second line disappears → press Ctrl+Y → second
    line reappears, "Redo" appears briefly → pressing Ctrl+Y again with a full
    redo stack does nothing visible.
12. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All in `#[cfg(test)] mod tests` inside their respective source files.

**`src/app.rs`**

- `app_default_status_flash_is_none` (AC#1): `assert_eq!(App::default().status_flash, None)`.
- `handle_undo_on_empty_history_returns_false_and_no_flash` (AC#2): fresh
  `App::default()`, call `handle_undo(&mut app, 0.0)`, assert return is `false`,
  `app.status_flash == None`.
- `handle_undo_on_non_empty_history_returns_true_and_sets_flash` (AC#3):
  `app.commit(Box::new(CreateLine::new(line_a())))`, call
  `handle_undo(&mut app, 0.0)`, assert `true`, empty entities,
  `app.status_flash == Some(("Undo".to_string(), 1.5))`.
- `handle_redo_after_undo_returns_true_and_sets_flash` (AC#4): continue from
  above, call `handle_redo(&mut app, 0.0)`, assert `true`, one entity,
  `app.status_flash == Some(("Redo".to_string(), 1.5))`.
- `handle_redo_on_empty_redo_stack_returns_false_and_no_flash` (AC#5):
  fresh app (no undo to replay), `handle_redo(&mut app, 0.0)` returns `false`,
  `status_flash == None`.

**`src/ui/statusbar.rs`**

- `flash_label_live_message_returned` (AC#6): assert
  `flash_label(Some(&("Undo".to_string(), 10.0)), 9.9) == Some("Undo")`.
- `flash_label_expired_returns_none` (AC#7): assert
  `flash_label(Some(&("Undo".to_string(), 10.0)), 10.0) == None`.
- `flash_label_boundary_exact_expiry_returns_none` (AC#7): assert
  `flash_label(Some(&("X".to_string(), 5.0)), 5.0) == None`
  (boundary: `5.0 < 5.0` is `false`).
- `flash_label_none_returns_none` (AC#8): assert `flash_label(None, 5.0) == None`.

**Static / CI checks**

- **AC#9 — not hover-gated**: `grep -n "Ctrl\|key_pressed(egui::Key::Z)"
  src/app.rs` followed by code review confirms neither check is inside
  `if response.hovered()`.
- **AC#10 — LOC + purity**: `wc -l src/ui/statusbar.rs` ≤ 150;
  `grep -nE '^use (eframe|rfd)' src/ui/statusbar.rs` exits empty.
- **AC#12 — build gate**: `cargo fmt --all -- --check &&
  cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Testable-helper pattern**: `handle_undo` / `handle_redo` follow the
  `handle_wheel_zoom` / `handle_pan` / `handle_zoom_extents` convention already
  in `app.rs` — free functions that take explicit arguments and return a `bool`
  so they can be exercised in `#[cfg(test)]` without an egui context.
  The `current_time: f64` argument stands in for `ctx.input(|i| i.time)` at the
  call site.

- **`app.rs` LOC situation**: `src/app.rs` is already at 336 LOC (measured at
  demand-write time), exceeding the 300-LOC soft cap in AGENTS.md. Adding §2
  + §3 content amounts to roughly 30–40 lines. If the result exceeds 370 LOC the
  implementer must extract all keyboard dispatch into `src/app/keys.rs`; the
  `src/app/` directory already exists (hosts `snap.rs`). No architect escalation
  is needed — this is routine within-component decomposition.

- **egui time source**: `ctx.input(|i| i.time)` returns `f64` seconds since egui
  startup. It is monotonically increasing and consistent across one frame. Storing
  it as an expiry value works without wall-clock or `std::time` imports.

- **Flash expiry placement**: the expiry check (`if time >= exp { flash = None }`)
  runs **inside** the `CentralPanel` closure, before `ctx.request_repaint()`.
  Running it there — rather than at the very top of `update` — ensures the
  check fires after the status bar panel has already been submitted (the panel
  is submitted outside and before the central panel). The flash persists for one
  extra frame at most if it expires between the status-bar render and the
  central-panel key check; this is imperceptible (< 16 ms).

- **Ctrl+Shift+Z redo alias**: some platforms (and AutoCAD itself) treat
  Ctrl+Shift+Z as redo. This demand does not add that alias. Leave a
  `// TODO LCV-070: Ctrl+Shift+Z redo alias` comment so LCV-070 (full
  keyboard shortcut sweep) can complete it.

- **LCV-070 relationship**: LCV-070 covers the full shortcut suite
  (L / P / R / C / A, F3 / F7 / F8, Ctrl+Z / Y / N / O / S). This demand
  scopes strictly to Ctrl+Z and Ctrl+Y to unblock undo without waiting for
  the full Phase-6 shortcut overhaul. When LCV-070 lands it may refactor the
  Ctrl+Z/Y wiring; the `handle_undo` / `handle_redo` helpers make that
  refactoring safe.

- **ID conflict**: PLAN.md currently lists LCV-075 as "Agent classifier
  (regex routing)" in Phase 7. This demand file uses the same ID at the user's
  explicit instruction. The `project-manager` and `demand-manager` must
  reconcile the ID assignment (renumber the agent classifier or resequence
  this demand) before `implementer-rust` picks up either ticket.
