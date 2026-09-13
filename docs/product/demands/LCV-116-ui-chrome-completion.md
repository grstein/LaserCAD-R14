# LCV-116 — Chrome completion: clickable mode toggles, autosave indicator, F1 shortcuts dialog, Ortho in the View menu

- **Status**: Done
- **Phase**: 11
- **Depends on**: LCV-102 (Done), LCV-104 (Done), LCV-105 (Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (commits fd9b4e4, 900f0c7 — the latter a Windows-only CI fix comparing a path against a forward-slash literal, no production code wrong; CI run 34747953068 green; reviewed, APPROVED — the reviewer independently reproduced both of the implementer's disclosed deviations and found no surviving mutation among more than twenty probes)

## Problem

v2's chrome hides state the operator needs and offers no mouse path to state
they change constantly.

- **Modes are invisible until they are on, and only one of them.** The status
  bar shows `ORTHO` when ortho is on and nothing at all for snap or grid
  (`src/ui/statusbar.rs::format_ortho`). Snap silently changing where a click
  lands, with no on-screen evidence of whether it is armed, is the single most
  confusing state in a CAD app. And none of the three can be toggled with the
  mouse from where they are displayed: F3/F7/F8 are the only path for snap and
  ortho (grid and snap also have View-menu checkboxes).
- **Autosave is entirely silent.** LCV-102 debounces a write at 800 ms and
  never tells the operator it happened. After a crash the operator has no idea
  whether there is anything to recover.
- **F1 does nothing.** There is no in-app list of keybindings anywhere; the
  eleven tools' letters are discoverable only from the Tools menu, and nothing
  documents Escape, Enter, `F`, `Ctrl+0` or the F-keys.
- **Ortho is missing from the View menu.** Grid (`F7`) and Snap (`F3`) have
  checkboxes there; Ortho (`F8`) does not, so it is the one mode with no mouse
  path at all.

Four small, independent pieces of chrome. Each has its own acceptance criteria
below and can be reviewed separately, but they share one file
(`src/ui/statusbar.rs`) and one constraint (ADR 0002's single-keyboard-reader
rule), which is why they ship together.

### Correction to the roadmap item

The roadmap lists "View > Zoom in / Zoom out" as missing. **It is already
shipped**: `src/ui/menubar.rs::view_menu` has `Zoom In`, `Zoom Out` and
`Fit to Bed`, backed by `do_zoom_in` / `do_zoom_out` / `do_fit_to_bed`. Part (d)
of this demand is therefore a regression test over the existing items plus the
one real gap in that menu — the missing `Ortho\tF8` checkbox.

## Scope

- **(a)** Three always-visible, clickable `SNAP` / `GRID` / `ORTHO` indicators
  in the status bar, flipping the same flags as F3/F7/F8.
- **(b)** An autosave indicator in the status bar, plus the repaint scheduling
  that makes autosave actually fire on an idle app.
- **(c)** A Help > Keyboard shortcuts… / F1 dialog listing the *real* bindings,
  with the tool rows derived from `toolbar::TOOLS`.
- **(d)** `Ortho\tF8` in the View menu, and a regression test pinning the four
  existing View items.

## Out of scope

- **New keyboard bindings other than F1.** No key is rebound, added or removed;
  the shortcuts dialog documents what exists.
- **A configurable / user-editable keymap, and a search box in the dialog.**
  Thirty rows in a scroll area.
- **Editing ADR 0002.** The ADR's gate table is a record of a decision; the
  living gate table is the module doc in `src/app/input.rs`, and the implementer
  updates that one when F1 lands.
- **A "saved N seconds ago" counter or a timed fade for the autosave
  indicator.** Both require a repaint every second forever (battery, and a
  90-minute-old app burning frames to animate a label), and a relative timestamp
  goes stale the moment repaints stop. Three permanent states instead — see the
  decisions.
- **Showing autosave *failures* in the indicator.** `save_autosave`'s error is
  swallowed today (LCV-102). Surfacing it is a real demand — error toast,
  retry policy, what the operator is supposed to do — and not this one. The
  indicator distinguishes "an autosave has happened this session" from "none
  has", which is honest with the information available.
- **Restyling the status bar, icons, colours, a theme, or reordering the
  existing segments.** New segments append; `format_coords`, the tool name and
  the entity count keep their current text and order.
- **Making the coordinate readout or the entity count clickable.**

## Product decisions (do not re-open these)

1. **The three mode indicators are always visible**, whether on or off — a
   badge that disappears when off cannot tell the operator "snap is off"; it
   looks identical to "this app has no snap". `format_ortho` and its two tests
   are retired.
2. **On/off is conveyed by egui's own selected state**
   (`ui.selectable_label(is_on, "SNAP")`), not by ASCII dots. v1 used `◉`/`○`
   because it rendered HTML; egui gives a real toggled-button look for free and
   it reads at a glance.
3. **The autosave indicator has three permanent states**, driven by a pure
   function, with no timer and no animation: a pending write, at least one
   successful write this session, or none yet.
4. **The app must schedule a repaint while a write is pending.**
   `flush_if_due` only runs inside a frame; on an idle app egui stops
   repainting, so today a document can sit dirty and unsaved indefinitely — and
   the new indicator would sit on "pending" forever. One
   `ctx.request_repaint_after(remaining_debounce)` while `dirty_since.is_some()`
   fixes both the indicator and the latent autosave defect.
5. **F1 is dispatched in `dispatch_shortcuts`, in the F3/F7/F8 class**: a bare
   key that fires even when a text widget has keyboard focus. It opens a
   read-only dialog, steals no character, and the operator who is lost mid-text
   is exactly the one who needs it. `dispatch_shortcuts` remains the sole
   keyboard reader for this class (ADR 0002 §A6); the dialog itself reads no
   keys.
6. **The dialog's tool rows are generated from `toolbar::TOOLS`.** That table is
   already the single source of truth for the toolbar and the Tools menu
   (LCV-104); a hand-typed third copy would drift on the first tool change.

## Acceptance criteria

### (a) Clickable mode indicators

1. `draw_statusbar(ui: &mut egui::Ui, app: &mut App)` — the signature changes
   from `&App`; `src/app/panels.rs::draw_chrome` already holds `&mut App` and
   needs no other change.
2. After the entity count, the bar renders three `selectable_label`s in the
   order `SNAP`, `GRID`, `ORTHO`, separated in the existing style, always
   visible, each selected iff its flag is `true`.
3. Clicking one flips exactly the flag that F3 / F7 / F8 flips —
   `app.snap_enabled`, `app.grid_enabled`, `app.ortho_enabled` — and nothing
   else: no document mutation, no history entry, no change to `dirty_since`.
4. A click on `SNAP` that turns snap off has the same effect as F3: the next
   frame's `suppress_snap_if_disabled` clears `active_snap`, so a stale snap
   marker cannot survive the toggle.
5. `format_ortho` and its two unit tests are deleted;
   `grep -rn "format_ortho" src/` returns nothing.
6. **No key reading enters the status bar.**
   `grep -nE "key_pressed|key_down|events|consume_key|input\(" src/ui/statusbar.rs`
   returns nothing. The two readers stay `dispatch_shortcuts` and
   `src/app/input.rs` (ADR 0002 §A6).

### (b) Autosave indicator

7. `App::last_autosave_at: Option<std::time::Instant>`, `None` by default, set
   to `Instant::now()` in `autosave::flush_if_due` **only when
   `crate::io::save_autosave` returned `Ok`**. LCV-102 AC 18 still holds:
   `mark_clean()` is still called unconditionally after the attempt, so a failing
   write cannot spin the debounce.
8. `pub(crate) fn format_autosave(write_pending: bool, ever_saved: bool) -> &'static str`
   in `src/ui/statusbar.rs`, pure, returning:
   - `write_pending == true` → `"● autosave pending"`;
   - else `ever_saved == true` → `"○ autosaved"`;
   - else → `"○ no autosave yet"`.

   `draw_statusbar` calls it with `app.dirty_since.is_some()` and
   `app.last_autosave_at.is_some()` and renders the result as a plain,
   non-interactive label in the last segment.
9. `App::update_ui` requests a follow-up repaint while a write is pending:
   when `dirty_since` is `Some(t)`, call
   `ctx.request_repaint_after(AUTOSAVE_DEBOUNCE.saturating_sub(t.elapsed()))`.
   Placed in `autosave::flush_if_due` (which already owns the debounce
   constant and would need `ctx` passed in) or in a sibling function called from
   the same phase of `update_ui` — the implementer picks, but the existing
   `if self.agent_busy { ctx.request_repaint(); }` line must keep working and no
   unconditional per-frame repaint may be introduced.
10. Consequence, and the point of AC 9: a document left dirty with no further
    input is autosaved within ~1 s and the indicator settles on `"○ autosaved"`
    without the operator touching anything.

### (c) F1 shortcuts dialog

11. `App::shortcuts_open: bool` (default `false`) and
    `pub fn shortcuts_dialog(ctx: &Context, open: &mut bool)` in
    `src/ui/dialogs.rs`, following `about_dialog` exactly: `Window::new("Keyboard shortcuts").open(open)`,
    centre-anchored, not collapsible, egui's × closes it. It is rendered from
    `panels::draw_dialogs` and its body sits in a vertically scrollable area so
    the list is reachable on a short window.
12. `Help > Keyboard shortcuts…` sets `shortcuts_open = true`, placed above
    `About`.
13. `dispatch_shortcuts` handles `Key::F1` alongside `F3` / `F7` / `F8` — bare
    modifiers, **not** gated on `wants_kbd` — and sets
    `app.shortcuts_open = true`. No other code reads `F1`.
14. **Tool rows are derived, not typed.** The dialog iterates
    `toolbar::TOOLS`, emitting one row per entry whose `shortcut` is `Some`
    (ten rows today: `L` Line, `P` Polyline, `R` Rect, `C` Circle, `A` Arc,
    `D` Text, `M` Move, `T` Trim, `X` Extend, `E` Delete — `Select` has no
    binding and is skipped). Adding or removing a `ToolEntry` changes the dialog
    with no edit here.
15. **The static rows list the real bindings**, verified against
    `src/ui/shortcuts.rs` and `src/app/input.rs`, grouped with headings, in this
    order:
    - *File* — `Ctrl+N` New, `Ctrl+O` Open, `Ctrl+S` Save, `Ctrl+Shift+S` Save As
    - *Edit* — `Ctrl+Z` Undo, `Ctrl+Y` Redo
    - *View* — `F` Zoom extents, `Ctrl+0` Zoom extents
    - *Modes* — `F3` Snap, `F7` Grid, `F8` Ortho
    - *Drawing* — `Enter` Accept / finish the current tool step, `Esc` Cancel
      the current operation, `Delete` / `Backspace` sent to the active tool
    - *Help* — `F1` This dialog

    No binding may appear that `grep` cannot find in those two files, and none of
    the twenty above may be missing. (For the record, the brief that spawned this
    demand listed only the ten tool letters, the three F-keys and the Ctrl
    combinations; `Esc`, `Enter`, `Delete`, `Backspace`, `F` and `Ctrl+0` are
    real bindings that were missing from it.)
16. The dialog is read-only: no button in it changes app state, and
    `grep -nE "key_pressed|events|input\(" ` over the dialog's source returns
    nothing (the × is egui's own `open` flag).
17. The implementer updates the gate table in `src/app/input.rs`'s module doc
    with the `F1` row (ungated, like F3/F7/F8). ADR 0002 is not edited.

### (d) View menu

18. `View` gains `ui.checkbox(&mut app.ortho_enabled, "Ortho\tF8")` immediately
    below the existing `Snap\tF3` checkbox, and toggling it has the same effect
    as F8 and as the status-bar `ORTHO` indicator (one flag, three paths).
19. A regression test pins the View menu's contents: `Zoom In`, `Zoom Out`,
    `Fit to Bed` (all three already exist and are backed by `do_zoom_in`,
    `do_zoom_out`, `do_fit_to_bed`), `Grid\tF7`, `Snap\tF3`, `Ortho\tF8`.
    `do_zoom_in` / `do_zoom_out` behaviour is unchanged by this demand.

### Cross-cutting

20. Every touched file stays ≤ 300 implementation lines (to the first
    `#[cfg(test)]`) — `src/ui/statusbar.rs` is at 76, `src/ui/dialogs.rs` at 112,
    `src/ui/menubar.rs` at 189, `src/ui/shortcuts.rs` at 142, `src/app/mod.rs` at
    **293** (LCV-113 moves code out of it; if that has not landed, the
    `request_repaint_after` call must not push `mod.rs` over 300 — put it in
    `src/app/autosave.rs`).
21. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` exit 0.

## Expected tests

**Quality bar (Marco 1).** Every criterion is covered by an automated test or
tagged **[manual]**. This demand changes input dispatch and the frame body, so
the headless tests drive the real `App::update_ui(ctx)` through
`tests/harness/mod.rs` (`SCREEN`, `key_events`, `raw_input`, `frame`, `tap`);
the harness's five-item surface does not change. The egui 0.29.1 traps —
key-repeat rewrite needing press+release, pointer tests needing a warm-up frame,
`wants_keyboard_input` lagging one frame — are documented in ADR 0002 §A4;
follow it rather than rediscovering it. egui is pinned at **0.29.1**:
`Ui::selectable_label`, `Context::request_repaint_after(Duration)` and
`Window::open` all exist there; propose nothing that needs a newer version.

- **Unit (a)** in `src/ui/statusbar.rs`:
  `toggle_click_flips_only_its_own_flag` over a small pure helper
  (`fn apply_toggle(app, which)` or equivalent) — one case per mode, asserting
  the other two flags and `document.entity_count()` are untouched; the AC 5 and
  AC 6 greps as static checks.
- **Integration (a, AC 2/3/4)** in `tests/lcv116_statusbar.rs` with
  `mod harness;`: `statusbar_renders_all_three_indicators_without_panic` — drive
  two frames with snap on / ortho off and assert no panic and the flags are
  unchanged by rendering alone (rendering must not toggle anything);
  `f3_and_the_statusbar_agree` — `tap(F3)` through the real `update_ui`, assert
  `snap_enabled` flipped and `active_snap` is cleared on the following frame
  (AC 4). The pointer click itself is **[manual]** — clicking a specific
  `selectable_label` by position is brittle; the pure helper covers the logic.
- **Unit (b, AC 8)**: `format_autosave_states` — all four input combinations,
  asserting the three exact strings.
- **Unit (b, AC 7)**: `last_autosave_at_is_set_only_on_success` — exercise
  `flush_if_due`'s success path against a temp `XDG_DATA_HOME`-style location
  (or the existing LCV-102 test seam) and assert `last_autosave_at.is_some()`
  and `dirty_since.is_none()`; plus `default_app_has_no_autosave_timestamp`.
- **Integration (b, AC 9/10)** in `tests/lcv116_autosave_repaint.rs`: mark the
  app dirty, run one frame via `ctx.run`, and assert
  `FullOutput.viewport_output[&ViewportId::ROOT].repaint_delay` is greater than
  zero and at most 800 ms (the debounce — note `AUTOSAVE_DEBOUNCE` is a private
  `const` in `src/app/autosave.rs`, so either assert against the literal or widen
  it to `pub(crate)`); then assert a clean app's frame
  does **not** request a sub-second repaint (guarding against a blanket
  per-frame repaint). Verified available in egui 0.29.1: `ViewportOutput::repaint_delay`
  is public.
- **Integration (c, AC 11/13)** in `tests/lcv116_shortcuts_dialog.rs`:
  `f1_opens_the_shortcuts_dialog` — `tap(F1)` through the real `update_ui`,
  assert `app.shortcuts_open` and that the following frame renders without
  panic; `f1_opens_the_dialog_even_with_keyboard_focus` — call
  `dispatch_shortcuts(Key::F1, Modifiers::NONE, /* wants_kbd */ true, &mut app)`
  directly and assert it still opens (the F3/F7/F8 class contract);
  `closing_the_dialog_leaves_no_state_behind` — set `shortcuts_open = false`,
  frame, assert nothing else changed.
- **Unit (c, AC 14/15)** in `src/ui/dialogs.rs` (or wherever the row table
  lives): `tool_rows_match_the_toolbar_table` — the generated tool rows equal
  `toolbar::TOOLS.iter().filter(|t| t.shortcut.is_some()).count()` and every
  label/shortcut pair matches its `ToolEntry`;
  `static_rows_cover_every_documented_binding` — asserts the twenty rows of
  AC 15 are present by exact string, so deleting one fails the build.
- **Unit (d, AC 18/19)**: extend the existing menubar test module —
  `view_menu_items_are_stable` (the six labels) and
  `ortho_checkbox_flips_the_same_flag_as_f8`.
- **Static checks**: AC 5, 6, 16 greps; `wc -l` to the first `#[cfg(test)]` for
  every touched file; a grep that `src/app/input.rs`'s gate table mentions `F1`
  (AC 17).
- **[manual] smoke**: `cargo run`. The status bar shows `SNAP`, `GRID`, `ORTHO`
  (unselected) and `○ no autosave yet`. Click `SNAP` → it lights up and snapping
  starts working; press `F3` → it goes dark again; `View > Snap` agrees with
  both. Same for `GRID`/F7 and `ORTHO`/F8, the latter now also togglable from
  `View > Ortho`. Draw a line and **stop touching the app**: within ~1 s and with
  no further input, the indicator goes from `● autosave pending` to
  `○ autosaved` (this is AC 10, and it fails today). Press `F1` → the dialog
  lists the tool letters, the Ctrl combinations and `Esc` / `Enter` / `F` /
  `Ctrl+0`; scroll it; close it with the ×. Start the Text tool, type a few
  characters, press `F1` → it still opens; press `L` while typing → it does
  **not** switch tools (the gate still holds).

## Risks

- **The `&App` → `&mut App` change in `draw_statusbar` is a public signature
  change**; `draw_chrome` is the only caller in `src/`, but check
  `grep -rn "draw_statusbar" src/ tests/` for test callers before assuming.
- **A blanket `ctx.request_repaint()` would "fix" AC 10 and burn a core
  forever.** The integration test in (b) asserts the clean-app case precisely to
  catch that shortcut.
- **F1 while a text tool has focus** is deliberate (decision 5). If it ever
  feels wrong, the fix is a gate change in one place, not a second reader.
- **Four pieces, one file.** `src/ui/statusbar.rs` grows by (a) and (b)
  together; at 76 implementation lines it has room, but re-measure before
  claiming it.
- **Merge overlap with LCV-113** (`src/app/mod.rs`, `src/ui/dialogs.rs`,
  `panels::draw_dialogs`, `dispatch_shortcuts`) and **LCV-115**
  (the status bar's preset label, `src/ui/menubar.rs`). All three add to the
  same lists rather than changing each other's behaviour; whoever lands later
  re-reads the file.

## Notes

- v1 reference: `../LaserCAD-R14/src/ui/statusbar.ts` — clickable `SNAP`/`GRID`/
  `ORTHO` with `◉`/`○` dots and a `● saved Ns ago` / `● not yet` autosave badge
  refreshed on a 1 s interval. v2 keeps the clickable modes, drops the relative
  timestamp and its timer (decision 3), and uses egui's selected state instead of
  dots (decision 2).
- v1's `F1` (`src/app/shortcuts.ts:67`) opened a stub dialog; v2's lists the real
  bindings.
- `about_dialog(ctx, open: &mut bool)` is the pattern for AC 11 — copy its shape,
  including the `.open(open)` × handling, so the two dialogs behave identically.
- `toolbar::TOOLS` is `pub(crate)`; the dialog lives in `src/ui/`, so no
  visibility change is needed.
- LCV-102 owns `dirty_since` and `mark_clean`; this demand adds a *reader* and a
  timestamp, and must not add a second writer to `dirty_since`.
- LCV-113 introduces `saved_revision` for the discard guard. The autosave
  indicator deliberately does **not** use it: "a write is pending" and "you would
  lose work if you quit" are different questions, and this indicator answers the
  first.
