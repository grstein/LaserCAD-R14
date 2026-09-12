# LCV-104 — Reachability: TEXT tool, full toolbar, Tools menu, Agent settings item

- **Status**: Ready
- **Phase**: 10
- **Depends on**: LCV-103 (keyboard gate + Enter routing)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Five tools exist in `src/tools/` and cannot be reached from the running
application. `TextTool` is the worst case: it has no toolbar button, no
keyboard binding and no menu item, so the TEXT command — listed as an explicit
v0.1.0 scope target in `docs/product/README.md` ("TEXT command: ASCII strings as
engravable line geometry via Hershey font") — does not exist for the operator.
The toolbar (`src/ui/toolbar.rs:29-54`) still carries the six tools that existed
when LCV-066 shipped, and its module doc (lines 8-9) still says Rect, Text and
Move "will be added when those tools land" — they landed months ago. The
menubar has File / Edit / View / Help but no **Tools** menu, although
`PLAN.md:115` and `docs/product/backlog.md:78` both record LCV-065 as
"Menubar (File / Edit / View / **Tools** / Help)". And the Agent Settings window
is rendered every frame behind `app.agent_settings_open`, a flag that no code
path ever sets to `true` — the operator cannot enter an API key, so the agent
harness is configuration-locked.

Net effect for a laser-cutting operator: engraved text is impossible, Rect /
Move / Trim / Extend are mouse-unreachable, and the AI assistant cannot be
configured. This demand makes every shipped v0.1.0 tool reachable by at least
two routes (pointer + keyboard) and unlocks the agent settings dialog.

## Scope

- **One tool table, two consumers.** `src/ui/toolbar.rs::ToolEntry` gains a
  `shortcut: Option<&'static str>` field; `TOOLS` becomes `pub(crate)` and is
  extended to the eleven v0.1.0 tools. `make_tool` becomes `pub(crate)` and
  covers every entry. The Tools menu iterates the same table — no second list.
- **Toolbar** renders, in this order, with a separator between the draw group
  and the modify group, and the existing 🤖 toggle last:

  | # | label | `Tool::name()` | constructor | shortcut hint |
  |---|---|---|---|---|
  | 1 | `Select` | `Select` | `SelectTool::default()` | — |
  | 2 | `Line` | `LINE` | `LineTool::default()` | `L` |
  | 3 | `Polyline` | `PLINE` | `PolylineTool::default()` | `P` |
  | 4 | `Rect` | `RECT` | `RectTool::default()` | `R` |
  | 5 | `Circle` | `CIRCLE` | `CircleTool::default()` | `C` |
  | 6 | `Arc` | `ARC` | `ArcTool::default()` | `A` |
  | 7 | `Text` | `TEXT` | `TextTool::default()` | `D` |
  | — | *separator* | | | |
  | 8 | `Move` | `MOVE` | `MoveTool::default()` | `M` |
  | 9 | `Trim` | `TRIM` | `TrimTool` | `T` |
  | 10 | `Extend` | `EXTEND` | `ExtendTool::default()` | `X` |
  | 11 | `Delete` | `ERASE` | `DeleteTool` | `E` |

- **Tools menu** in `src/ui/menubar.rs`, placed between `View` and `Help`
  (final bar order: File, Edit, View, Tools, Help). One `ui.button` per table
  entry, labelled `"<label>\t<shortcut>"` when a shortcut exists and `"<label>"`
  otherwise; clicking closes the menu and calls
  `ToolManager::set_tool(make_tool(label))`.
- **TEXT keyboard binding**: `Key::D` is added to `TOOL_KEYS` in
  `src/ui/shortcuts.rs`, constructing `TextTool::default()`. It joins the
  ADR 0002 gate table's "tool activation" class — bare key only, suppressed when
  a text widget has focus. Reasoning is recorded in Notes; this is a settled
  product decision, not an option.
- **Help > Agent settings…** — a new item in the Help menu backed by
  `pub(crate) fn do_agent_settings(app: &mut App)` which sets
  `app.agent_settings_open = true`, matching the existing `do_about` pattern.
- **Stale doc comment** at `src/ui/toolbar.rs:8-9` rewritten to describe the
  shipped table.

## Out of scope

- **OffsetTool gets no toolbar button, no menu item and no shortcut.** OFFSET is
  a `docs/product/README.md` non-goal; LCV-106 decides its fate.
- **Making the agent actually mutate the document** — Marco 2. This demand only
  opens the settings window.
- **Command-line alias dispatch** (typing `TEXT` or `L` into the command line to
  activate a tool). Not implemented today; it is a separate demand.
- **Icons, images or styling on toolbar buttons.** Text labels only — same
  `SelectableLabel` widget LCV-066 shipped.
- **Rebinding any existing shortcut.** `L P R C A M E T X` keep their current
  meanings exactly.
- **TEXT tool options** (height, spacing, justification UI). `TextTool` keeps
  its defaults; exposing them is a later demand if the workflow demands it.
- **A `Select` keyboard shortcut.** Escape already returns to a usable state and
  the toolbar/menu cover it; adding a letter is unearned complexity.
- **Statusbar, command line, dialogs** — untouched.

## Acceptance criteria

1. `src/ui/toolbar.rs::TOOLS` contains exactly the eleven entries above, in that
   order, with those `label` / `tool_name` / `shortcut` triples.

2. `make_tool(label)` returns `Some` for every entry and the constructed tool's
   `name()` equals the entry's `tool_name`; `make_tool` returns `None` for an
   unknown label.

3. `make_tool("Text")` returns a tool whose `name()` is `"TEXT"` — TextTool is
   reachable from the pointer for the first time.

4. Toolbar labels are unique and `tool_name`s are unique (the LCV-066 invariants
   still hold with eleven entries).

5. The menubar renders five top-level menus in the order File, Edit, View,
   Tools, Help.

6. The Tools menu is generated by iterating `toolbar::TOOLS`; there is no second
   tool list in `src/ui/menubar.rs`. Verified by `grep -n '"LINE"\|"TEXT"\|"RECT"'
   src/ui/menubar.rs` returning no matches.

7. Activating a Tools-menu item sets the active tool: for every entry,
   `make_tool(entry.label)` followed by `ToolManager::set_tool` leaves
   `active_tool_name() == entry.tool_name`.

8. Bare `D` with no text-widget focus activates TEXT: after one `D` tap driven
   through `App::update_ui`, `app.tool_manager.active_tool_name() == "TEXT"`.

9. Bare `D` while a text widget has focus does **not** change the active tool
   (`dispatch_shortcuts(Key::D, Modifiers::NONE, true, &mut app)` leaves
   `active_tool_name()` unchanged) — the ADR 0002 "tool activation" gate row.

10. `Ctrl+D` and `Shift+D` do not activate TEXT (bare-key-only rule).

11. `TOOL_KEYS` in `src/ui/shortcuts.rs` contains exactly the keys
    `{L, P, R, C, A, M, E, T, X, D}`, with no duplicate key, and every entry's
    constructed tool `name()` matches the `tool_name` of the `TOOLS` entry that
    advertises the same shortcut letter.

12. Help > "Agent settings…" sets `app.agent_settings_open = true`:
    `do_agent_settings(&mut app)` flips the flag from `false` to `true`, and the
    existing LCV-076 window then renders (unchanged code path).

13. `App::default().agent_settings_open` is still `false` — no dialog opens on
    boot.

14. The module doc of `src/ui/toolbar.rs` no longer claims Rect / Text / Move
    "will be added"; `grep -n "will be added" src/ui/toolbar.rs` returns no
    match.

15. Neither `src/ui/toolbar.rs` nor `src/ui/menubar.rs` references `OffsetTool`
    or the string `"OFFSET"`.

16. `src/ui/toolbar.rs` and `src/ui/menubar.rs` each stay ≤ 300 implementation
    lines (lines before the first `#[cfg(test)]`).

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

- **(AC 1, 4)**: extend the existing `toolbar_labels_are_unique` and
  `toolbar_tool_names_are_unique`; add `toolbar_table_is_the_v010_tool_set` —
  asserts `TOOLS.len() == 11` and the ordered label list.
- **(AC 2, 3)**: extend `make_tool_covers_all_toolbar_entries`; add
  `text_tool_is_reachable_from_toolbar` asserting `make_tool("Text")` yields
  `"TEXT"`.
- **(AC 5)**: `menubar_has_five_menus_in_order` — render `draw_menubar` through
  `egui::Context::run` (existing `run_menubar` helper) and assert no panic, plus
  a structural test asserting the call order of the five `*_menu` helpers is
  File, Edit, View, Tools, Help.
- **(AC 6)**: static check — `grep` on `src/ui/menubar.rs` for hard-coded tool
  names returns nothing.
- **(AC 7)**: `tools_menu_entries_activate_their_tool` — loop over
  `toolbar::TOOLS`, call `set_tool(make_tool(label).unwrap())`, assert
  `active_tool_name()`.
- **(AC 8)**: headless test in `tests/lcv104.rs` using `mod harness;` —
  `d_activates_text_tool` drives `harness::tap(&ctx, &mut app, Key::D,
  Modifiers::NONE)` and asserts `active_tool_name() == "TEXT"`.
- **(AC 9)**: `d_suppressed_while_text_widget_focused` — direct
  `dispatch_shortcuts(Key::D, Modifiers::NONE, true, &mut app)` call, asserting
  the active tool is unchanged (same style as `tests/lcv070.rs`).
- **(AC 10)**: `d_with_modifiers_does_not_activate_text`.
- **(AC 11)**: `tool_keys_are_unique_and_match_toolbar_shortcuts` — unit test in
  `src/ui/shortcuts.rs`.
- **(AC 12)**: `help_agent_settings_sets_flag` — unit test in
  `src/ui/menubar.rs`, mirroring `help_about_sets_about_open`.
- **(AC 13)**: existing `app_default_agent_settings_open_is_false` still passes.
- **(AC 14, 15, 16)**: static checks (`grep`, `wc -l` to the first
  `#[cfg(test)]`).
- **(AC 17)**: the three build gates.
- **Manual smoke**: `cargo run`. The toolbar shows eleven buttons; click `Text`,
  click a point on the canvas, type `LASER`, press Enter → Hershey line geometry
  appears at the click point in millimetre coordinates, and Ctrl+Z removes it.
  Press `D` with the canvas focused → the Text button highlights. Open
  Tools > Rect → the Rect button highlights. Open Help > Agent settings… → the
  settings window appears and the API-key field is editable.

## Risks

- **`D` as a mnemonic for Delete.** An operator may press `D` expecting "delete".
  Mitigation: DELETE is bound to `E` (AutoCAD `ERASE`) and to the Delete key,
  both unchanged; and `D` activating a tool is non-destructive and instantly
  reversible with Escape or another tool key. Accepted.
- **Enter dependency.** The TEXT workflow is only usable once LCV-103 routes
  Enter to the active tool. Shipping this demand first would put a Text button
  in front of the user that cannot commit — hence the hard `Depends on`.
- **Focus timing.** `D` is in the gated class, so it is suppressed for one frame
  after a text widget loses focus (`wants_keyboard_input` lags by one frame).
  This matches every other tool key; no special handling.
- **Table coupling.** Making `TOOLS` the single source for both toolbar and
  menu means a malformed entry breaks two surfaces at once. AC 2 and AC 7 walk
  the whole table, so the failure is a test failure, not a runtime surprise.

## Open questions

*(none — demand is Ready)*

## Notes

### Decision — TEXT binds to `D`

Product-owner call, recorded here so no one re-opens it:

- The natural letters are taken: `T` = TRIM, `X` = EXTEND, `E` = ERASE,
  `M` = MOVE, `R` = RECT (verified in `src/ui/shortcuts.rs:49-59`). Rebinding any
  of them to free `T` for TEXT would break muscle memory and the LCV-070 tests
  for a cosmetic gain — rejected.
- `D` is free and carries an AutoCAD R14 mnemonic: `DT` / `DTEXT` is R14's
  single-line text command, so "D for dynamic text" is the closest thing to a
  native binding still available.
- "Toolbar + menu only, no shortcut" was considered and rejected: product
  principle #1 is keyboard-first, and TEXT is the one v0.1.0 tool a user would
  otherwise have to reach with the mouse every single time.
- Consequence: the ADR 0002 gate table's "tool activation" row becomes
  `L P R C A M E T X D`. The row's *behaviour* (bare key, suppressed under
  keyboard focus) is unchanged, so the ADR is extended, not contradicted.

### Other notes

- `TextTool::name()` returns `"TEXT"` (`src/tools/text.rs:45`); the toolbar's
  `tool_name` column must match it exactly or the active-tool highlight breaks.
- `TrimTool` and `DeleteTool` are unit structs — `TrimTool`, `DeleteTool`, not
  `::default()` — while the rest use `::default()`. `make_tool` already handles
  the mixed shapes.
- The Agent Settings window (`src/app.rs:426`) saves settings on close; opening
  it from the Help menu needs no other change.
- `src/ui/menubar.rs` must keep its "no `eframe` / no `rfd`" property; the Tools
  menu only touches `ToolManager`.
- Keep menu item labels ASCII plus the existing `\t` shortcut-hint convention
  (`"New\tCtrl+N"`), so the Tools menu reads like the rest of the bar.
