# LCV-110 — Command-line parser: a pure kernel module for `X,Y`, `@X,Y`, distance, aliases, toggles and zoom

- **Status**: Ready
- **Phase**: 11
- **Depends on**: none (ADR 0003 is the binding contract; Marco 0 is Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

v2 ships a command line that cannot parse a command. `src/ui/command_line.rs`
renders a prompt and a `TextEdit`, and on Enter hands the raw string to
`ToolManager::on_command_input`, which forwards it to
`Tool::on_command_input(&str, …)` — a trait method with a **no-op default body
and zero implementors**. LCV-068 shipped the pipe; no water ever ran through it.
An operator who types `50,25` and presses Enter gets silence: no geometry, no
error, no echo.

That is the single largest parity gap with v1.0.0, and it matters more here than
in a general drawing tool. A laser part is *dimensioned*, not sketched: a 100 mm
slot has to be 100 mm or the part does not fit, and no amount of zooming makes a
mouse click land on an exact millimetre. The command line is the only exact
input path in the product, and the product's first principle is
"prefer command line and keyboard-first flows".

This demand builds **only the kernel**: the grammar, the alias tables and the
recall ring, as a pure Rust module with no `egui` anywhere near it. Nothing the
operator can see changes when this lands. LCV-111 wires it to the tools and to
the widget; LCV-112 completes TEXT.

### The identity problem this demand also closes

The tree currently holds **three** independent maps from "which tool" to "a
`Box<dyn Tool>`", keyed on three different types:

| site | key | map |
|---|---|---|
| `src/ui/shortcuts.rs:54` | `egui::Key` | `TOOL_KEYS: &[(Key, fn() -> Box<dyn Tool>)]` |
| `src/ui/toolbar.rs:112` | `&str` label | `make_tool(label) -> Option<Box<dyn Tool>>` |
| *(this demand)* | a command alias | the parser's alias table |

Adding the alias table as a fourth stringly-typed lookup is how `e` ends up
meaning ERASE on the keyboard and EXTEND on the command line — see product
decision 2 below, where exactly that collision was found. ADR 0003 §A3 introduces
`ToolKind` and `tools::make(kind)` as the one map and converts `TOOL_KEYS`;
this demand also retires `toolbar::make_tool`, because leaving a `match` on a
string literal that silently returns `None` on a typo defeats the point of
having the enum at all.

## Scope

- **A new kernel module `src/cmdline/`** (ADR 0003 §A1), three files:
  `mod.rs` (types + re-exports), `parse.rs` (`parse`, `parse_number`, the alias
  tables), `history.rs` (`CommandHistory`). It imports `crate::geometry` and
  `std` and nothing else, and joins the AGENTS.md purity list.
- **`CommandInput`** and its supporting enums `ToolKind`, `ToggleKind`,
  `ZoomKind`.
- **`parse(raw: &str) -> CommandInput`** — total, case-insensitive,
  whitespace-tolerant, never panics, never returns `Err`.
- **`parse_number(raw: &str) -> Option<f64>`** — the product's single number
  grammar, in millimetres, rejecting non-finite values.
- **`CommandHistory`** — the 50-entry recall ring, owned by the pure module.
- **`ToolKind` → instance**: `pub fn make(kind: ToolKind) -> Box<dyn Tool>` in
  `src/tools/mod.rs`; `TOOL_KEYS` becomes `&[(Key, ToolKind)]`;
  `ToolEntry` gains a `kind` field and `toolbar::make_tool` is deleted.
- **`pub mod cmdline;`** in `src/lib.rs`.
- **The AGENTS.md amendment** (ADR 0003 §A1): `src/cmdline/*` added to the
  purity list, a `cmdline/` row in the module tree, and ADR 0003 added to the
  reference-documentation list.

## Out of scope

- **`ToolInput`, any change to the `Tool` trait, and any tool implementation.**
  The parser's output is consumed by nobody in this demand.
  `Tool::on_command_input` keeps its current `&str` signature until LCV-111
  replaces it. LCV-111 declares `ToolInput` (ADR 0003 §B1).
- **Any change to `App`, `src/app/*` or `src/ui/command_line.rs`.** No new `App`
  field, no widget change, no focus behaviour, no feedback line. LCV-111 owns
  all of it. `App` does **not** gain `command_history` here — the ring is built
  and unit-tested, then wired next demand.
- **Raw-input mode** (`wants_raw_input` / `on_raw_input`) — LCV-112.
- **Full-word aliases** (`line`, `circle`, `rect`, `move`, `del`, …), and the
  letters `x` and `d`. v1 had them; the alias table is explicitly *not* frozen
  (ADR 0003 §A2), so each is one table row plus one test row in a later demand.
  Ten aliases cover every tool the operator reaches by letter today.
- **`z e`** as a third spelling of zoom-extents (v1 accepted it). `ze` and
  `zoom extents` are enough.
- **Polar entry `@50<30`, `SPACE` as Enter, empty-Enter "repeat last command",
  LINE's `c`/`u` options, `zoom window` / `zoom previous`, `pan`.** Each is one
  parser arm or one table row; none is needed for parity and none reopens
  ADR 0003.
- **The agent prefixes `:` and `/ai`.** Marco 2 / LCV-122. The parser must not
  grow an agent arm now; `:foo` parses as `Unknown(":foo")`.
- **Locale-aware number parsing.** See product decision 1.
- **Command completion, a dropdown, a history panel, syntax colouring.**

## Product decisions (do not re-open these)

1. **The decimal separator is always `.` and the comma is always the coordinate
   separator.** No locale detection, no `LANG` sniffing, no heuristic. `10,5`
   is the point *(10, 5)*, never the number *10.5*; `10.5` is the number;
   `10.5,20.25` is the point *(10.5, 20.25)*. A parser whose meaning depends on
   an environment variable is not deterministic geometry, and a CAD file that
   means something different on the operator's laptop than on the shop machine
   is a defect factory. This is the rule v1 used and the rule AutoCAD uses.
2. **`e` activates DELETE (ERASE), not EXTEND.** v1's table
   (`../LaserCAD-R14/src/ui/command-line.ts:23`) mapped `e` → extend, but v2
   binds the bare `E` key to `DeleteTool` (`TOOL_KEYS`, LCV-070) and labels it
   `Delete` / `"ERASE"` in the toolbar — as does AutoCAD R14, where `E` is
   ERASE and EXTEND is `EX`. One letter must not mean two tools depending on
   whether the command line has focus. **Every single-letter alias must resolve
   to the same `ToolKind` as the `TOOL_KEYS` entry for the same letter**, and
   AC 12 makes that a test rather than a convention. EXTEND consequently has no
   alias in this demand (ADR 0003 §A2 already excludes it).
3. **The recall ring does not deduplicate** (ADR 0003 §A4). Consecutive
   duplicates are kept. `l` `l` `l` is three entries; Up walks back through all
   three. Suppression would mean the ring's contents depend on comparisons the
   operator cannot see, and the ring is a transcript, not a set.
4. **`Empty` is a variant of `CommandInput`, not an `Unknown("")`.** Pressing
   Enter on a blank field is a *meaningful* keystroke in R14 ("accept"), and
   LCV-111 routes it to the active tool. Reporting `Unknown command: ""` for it
   would be a lie.
5. **`parse` is total.** It returns `Unknown`, never `Err`, never panics. A
   `Result` here would force every caller to invent a policy for a case that has
   exactly one sensible answer.
6. **Rejected input is still recall-able.** `CommandHistory::push` is called by
   the app for anything the operator submitted, including a typo — recall exists
   so a typo can be fixed, which is only possible if the typo was stored.
   (LCV-111 owns the call site; this demand owns the ring's behaviour.)
7. **The parser does not validate magnitudes.** `Distance(-37.5)` and
   `Distance(0.0)` are valid parses; whether a negative number is a legal radius
   is the tool's business (LCV-111), not the grammar's.

## Acceptance criteria

### The module and its purity

1. `src/cmdline/mod.rs`, `src/cmdline/parse.rs` and `src/cmdline/history.rs`
   exist; `src/lib.rs` declares `pub mod cmdline;`; `src/cmdline/mod.rs` carries
   a `//!` module header naming ADR 0003 and re-exports the module's whole
   public surface, so no caller outside the module uses a deep path
   (`crate::cmdline::parse`, never `crate::cmdline::parse::parse`).

2. **Purity.** `grep -rnE '^\s*use (egui|eframe|rfd)' src/cmdline/` returns
   nothing, and neither does a grep for those crate names anywhere in the
   module. `src/cmdline/` imports only `crate::geometry` and `std`. In
   particular it does **not** import `crate::document`, `crate::tools` or
   `crate::app`.

3. **AGENTS.md is amended in the same commit**: `src/cmdline/*` is listed under
   §"Purity rule" and under §"Implementation Rules" ("Keep the kernel pure:
   …"), a `cmdline/` row is added to the §"Architecture → Module tree" listing
   with a one-line description, and
   [`docs/adr/0003-command-line-input-contract.md`](../../adr/0003-command-line-input-contract.md)
   is added to §"Reference documentation". No other line of AGENTS.md changes.

### The types

4. `src/cmdline/mod.rs` declares, with doc comments on every public item:

   ```rust
   pub enum CommandInput {
       Point(Vec2),        // absolute, world mm
       Relative(Vec2),     // offset from the active anchor, mm
       Distance(f64),      // a bare magnitude, mm
       Tool(ToolKind),
       Toggle(ToggleKind),
       Zoom(ZoomKind),
       Empty,
       Unknown(String),    // payload = the trimmed raw text, original case
   }

   pub enum ToolKind { Select, Line, Polyline, Rect, Circle, Arc, Move, Delete, Trim, Extend, Text }
   pub enum ToggleKind { Snap, Grid, Ortho }
   pub enum ZoomKind { In, Out, Extents }
   ```

   `CommandInput` derives `Debug, Clone, PartialEq`; the three fieldless enums
   derive `Debug, Clone, Copy, PartialEq, Eq`. Coordinates are **millimetres in
   world space**, Y-up, and the doc comments say so.

5. **`pub fn parse_number(raw: &str) -> Option<f64>`** trims ASCII whitespace,
   parses with `str::parse::<f64>`, and returns `None` unless the result
   `is_finite()`. Therefore `parse_number("nan")`, `("NaN")`, `("inf")`,
   `("-inf")`, `("infinity")` and `("1e400")` are all `None`, while
   `parse_number(" -3.5 ")` is `Some(-3.5)`, `parse_number("+5")` is `Some(5.0)`
   and `parse_number("5.")` is `Some(5.0)`. Every number in the grammar —
   coordinates, offsets, distances, and LCV-112's text height — goes through
   this function; a second `str::parse::<f64>` anywhere in `src/cmdline/` or
   `src/app/cmdline.rs` is a review blocker.

### The grammar

6. **`pub fn parse(raw: &str) -> CommandInput`** is total: for every possible
   `&str` it returns a value, panics for none, and allocates only for the
   `Unknown` payload. It trims surrounding whitespace, matches keywords and
   aliases **case-insensitively**, and tolerates whitespace around the comma
   and between `zoom` and its argument (split on ASCII whitespace, so
   `"zoom   in"` parses).

7. **This table is the grammar.** Every row is a test case (AC in §Expected
   tests), and the `Unknown` payload is the **trimmed original text with its
   original case**, not the lowercased form:

   | input | result |
   |---|---|
   | `"50,25"` | `Point((50.0, 25.0))` |
   | `"  50 , 25  "` | `Point((50.0, 25.0))` |
   | `"-3.5,0"` | `Point((-3.5, 0.0))` |
   | `"0,0"` | `Point((0.0, 0.0))` |
   | `"10,5"` | `Point((10.0, 5.0))` — **not** the number 10.5 (decision 1) |
   | `"@10,-5"` | `Relative((10.0, -5.0))` |
   | `"@ 10 , -5"` | `Relative((10.0, -5.0))` |
   | `"37.5"` | `Distance(37.5)` |
   | `"-37.5"` | `Distance(-37.5)` |
   | `"0"` | `Distance(0.0)` |
   | `"l"`, `"L"`, `"  L  "` | `Tool(ToolKind::Line)` |
   | `"p"` | `Tool(Polyline)` |
   | `"r"` | `Tool(Rect)` |
   | `"c"` | `Tool(Circle)` |
   | `"a"` | `Tool(Arc)` |
   | `"s"` | `Tool(Select)` |
   | `"t"` | `Tool(Trim)` |
   | `"e"`, `"E"` | `Tool(Delete)` (decision 2) |
   | `"m"` | `Tool(Move)` |
   | `"text"`, `"TEXT"`, `" Text "` | `Tool(Text)` |
   | `"snap"`, `"SNAP"` | `Toggle(Snap)` |
   | `"grid"` | `Toggle(Grid)` |
   | `"ortho"` | `Toggle(Ortho)` |
   | `"zoom in"`, `"ZOOM   In"` | `Zoom(In)` |
   | `"zoom out"` | `Zoom(Out)` |
   | `"zoom extents"` | `Zoom(Extents)` |
   | `"ze"`, `"ZE"` | `Zoom(Extents)` |
   | `""`, `"   "`, `"\t"` | `Empty` |
   | `"1,2,3"` | `Unknown("1,2,3")` — never "take the first two" |
   | `"@"` | `Unknown("@")` |
   | `"-"` | `Unknown("-")` |
   | `"10,"` | `Unknown("10,")` |
   | `",10"` | `Unknown(",10")` |
   | `"@,"` | `Unknown("@,")` |
   | `"1,abc"` | `Unknown("1,abc")` |
   | `"nan"` | `Unknown("nan")` |
   | `"inf,0"` | `Unknown("inf,0")` |
   | `"1e400"` | `Unknown("1e400")` |
   | `"zoom"` | `Unknown("zoom")` |
   | `"zoom sideways"` | `Unknown("zoom sideways")` |
   | `"x"`, `"d"`, `"line"`, `"del"` | `Unknown(…)` (§Out of scope) |
   | `":draw a square"` | `Unknown(":draw a square")` (agent prefixes are LCV-122) |
   | `"  Foo  "` | `Unknown("Foo")` — trimmed, original case preserved |

8. **Precedence is unambiguous and order-independent in effect.** No input in
   the table above matches two rules. The implementer may order the arms freely,
   but a keyword must never shadow a number and vice versa: there is no alias
   that is also a valid number, and `parse_number` is only consulted after the
   keyword and alias tables have missed.

### The recall ring

9. `pub struct CommandHistory` lives in `src/cmdline/history.rs`, derives
   `Debug, Default, Clone`, and exposes:

   ```rust
   pub const CAPACITY: usize = 50;
   pub fn push(&mut self, entry: &str);
   pub fn older(&mut self) -> Option<String>;   // Up
   pub fn newer(&mut self) -> Option<String>;   // Down
   pub fn len(&self) -> usize;
   pub fn is_empty(&self) -> bool;
   ```

   It returns owned `String`s, not `&str`, so a caller never fights a borrow of
   two `App` fields (ADR 0003 §A4).

10. **`push` semantics.** It trims the entry; if the trimmed entry is empty it
    is ignored entirely (`len()` unchanged). Otherwise the trimmed entry is
    appended as the newest; if the ring already held `CAPACITY` entries the
    **oldest is evicted first**, so `len()` never exceeds 50. Every `push`
    resets the recall cursor to "no recall in progress". Duplicates are stored
    (decision 3).

11. **Cursor semantics**, mirroring v1
    (`../LaserCAD-R14/src/ui/command-line.ts:202-217`). With entries pushed in
    the order `a`, `b`, `c`:

    | call sequence | returns |
    |---|---|
    | `older()` | `Some("c")` |
    | `older()`, `older()` | `Some("c")`, `Some("b")` |
    | `older()` ×3 | `…`, `Some("a")` |
    | `older()` ×4 (past the oldest) | 4th call → `Some("a")` — it **stays**, it does not wrap and does not return `None` |
    | `newer()` with no recall in progress | `None` |
    | `older()` ×2 then `newer()` | `Some("c")` |
    | `older()` then `newer()` | `None` — walked past the newest; the caller clears the field, and the cursor is back to "no recall in progress" |
    | any call on an empty ring | `None` |
    | `push("d")` mid-recall | resets the cursor; the next `older()` returns `Some("d")` |

12. **One map from tool identity to tool instance.**
    - `src/tools/mod.rs` gains `pub fn make(kind: ToolKind) -> Box<dyn Tool>` —
      an exhaustive `match` with no `_` arm, so adding a `ToolKind` variant is a
      compile error until it is mapped.
    - `src/ui/shortcuts.rs::TOOL_KEYS` becomes
      `const TOOL_KEYS: &[(Key, ToolKind)]` and the `dispatch_shortcuts` loop
      calls `tools::make(kind)`. The `ToolCtor` type alias is deleted. The key
      set is unchanged: `L P R C A M E T X D`, same tools as today.
    - `src/ui/toolbar.rs::ToolEntry` gains `pub(crate) kind: ToolKind`, all
      eleven rows are filled in, and `make_tool(label)` is **deleted**. Its two
      call sites (`toolbar.rs:160`, `menubar.rs:133`) call
      `tools::make(entry.kind)`, which is infallible — the `if let Some(tool)`
      wrapper disappears with it.
    - `grep -rn "make_tool" src/ tests/` returns nothing.

13. **The three tables agree.** A test asserts, for every `TOOL_KEYS` entry,
    that `parse(&letter.to_lowercase())` — where a single-letter alias exists
    for that letter — yields `Tool(kind)` with the **same** `kind` the key maps
    to; and that `tools::make(kind).name()` equals the `tool_name` of the
    `TOOLS` row advertising that letter as its `shortcut`. This is the criterion
    that pins product decision 2: making `e` mean EXTEND again fails the build.
    The existing LCV-104 test
    `tool_keys_are_unique_and_match_toolbar_shortcuts` survives with
    `ctor()` replaced by `make(kind)` and keeps asserting the ten-letter set.

14. **Behaviour is unchanged for the user.** No `App` field, no widget, no
    prompt and no keystroke behaves differently after this demand. The
    `TOOL_KEYS`, toolbar and Tools-menu conversions are refactors: the ten keys
    still activate the same ten tools, and the eleven toolbar buttons and eleven
    Tools-menu items still activate the same eleven tools.

15. Every touched file stays **≤ 300 implementation lines** (lines before the
    first `#[cfg(test)]` attribute; an inline test module does not count —
    ADR 0002 §"The 300-LOC cap"). Today: `src/tools/mod.rs` 34,
    `src/ui/shortcuts.rs` 141, `src/ui/toolbar.rs` ~175, `src/ui/menubar.rs`
    188. No `unsafe`, no `unwrap()` / `expect()` in `src/`.

16. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0.

## Expected tests

**Quality bar (Marco 1).** Every acceptance criterion above is covered by an
automated test or tagged **[manual]** below. The May-2026 drive shipped 72
demands with a green unit gate and ten integration defects that made "Done"
features non-functional; a green `cargo test` that never drives the real frame
body is not evidence. This demand is overwhelmingly pure, so its coverage is
exhaustive inline unit tests — **but it edits `src/ui/shortcuts.rs`, which is
input handling**, so it also carries one headless regression test driving the
real `App::update_ui(ctx)` through `tests/harness/mod.rs` (`SCREEN`,
`key_events`, `raw_input`, `frame`, `tap`). Follow ADR 0003 §F (recipes and the
five egui 0.29.1 traps, including trap 4: always emit press **and** release, via
`tap`) rather than rediscovering them. egui is pinned at **0.29.1**
(`Cargo.toml` requests `"0.29"`, `Cargo.lock` resolves 0.29.1); do not upgrade,
and `egui_kittest` (needs ≥ 0.30) stays out of scope.

- **Unit — the grammar (AC 6, 7, 8)** in `src/cmdline/parse.rs`: one test per
  logical group, with **every row of the AC 7 table asserted**. Group them as
  `parses_absolute_points`, `parses_relative_offsets`, `parses_bare_distances`,
  `parses_tool_aliases`, `parses_toggles`, `parses_zoom_forms`,
  `blank_input_is_empty`, `rejects_malformed_input_as_unknown`,
  `unknown_payload_is_trimmed_and_keeps_original_case`,
  `comma_is_always_the_separator_never_a_decimal_point` (the locale case, with
  a doc comment stating decision 1),
  `three_field_coordinates_are_rejected_not_truncated`,
  `aliases_are_case_insensitive`.
- **Unit — numbers (AC 5)** in `src/cmdline/parse.rs`:
  `parse_number_accepts_plain_decimal_and_signed`,
  `parse_number_rejects_non_finite` (`nan`, `NaN`, `inf`, `-inf`, `infinity`,
  `1e400`), `parse_number_trims_whitespace`,
  `parse_number_rejects_empty_and_garbage`.
- **Unit — the ring (AC 9, 10, 11)** in `src/cmdline/history.rs`:
  `push_ignores_blank_entries`, `push_trims`, `push_keeps_duplicates` (named
  after decision 3 so deleting it is visible),
  `ring_evicts_the_oldest_past_fifty` — push 60 entries, assert `len() == 50`,
  assert the first `older()` is entry 60 and that walking back 50 times reaches
  entry 11 and stays there — `older_stops_at_the_oldest`,
  `newer_past_the_newest_returns_none`, `newer_without_recall_returns_none`,
  `push_resets_the_cursor`, `empty_ring_returns_none_both_ways`,
  `capacity_is_fifty` (asserts the constant).
- **Unit — the single map (AC 12, 13)**: in `src/tools/mod.rs`,
  `make_covers_every_tool_kind` (construct every variant, assert the returned
  `name()`); in `src/ui/shortcuts.rs`, the amended
  `tool_keys_are_unique_and_match_toolbar_shortcuts`; and a new
  `alias_table_agrees_with_tool_keys` asserting AC 13's cross-check for all ten
  letters — the test that pins `e` → DELETE.
- **Unit — purity and structure (AC 1, 2, 15)**: static checks run by the
  implementer and recorded in the commit message — the AC 2 greps, the AC 12
  `make_tool` grep, and `wc -l` to the first `#[cfg(test)]` for every touched
  file.
- **Integration — reachability is preserved (AC 14)** in `tests/lcv110.rs` with
  `mod harness;`: `every_tool_key_still_activates_its_tool` — for each of the
  ten letters, a fresh `App::default()`, one `tap(key, Modifiers::NONE)` through
  the **real** `App::update_ui`, asserting `app.tool_manager.active_tool_name()`
  equals the expected name. Unit tests that call `dispatch_shortcuts` directly
  prove the table is correct and prove nothing about whether it is *reachable*
  (ADR 0002 §Context) — this is the test that would catch a `TOOL_KEYS`
  conversion that compiled but was never wired.
- **[manual] smoke** — the only thing a human needs to confirm, because this
  demand is deliberately invisible: `cargo run`; click every one of the eleven
  toolbar buttons and every Tools-menu item and confirm each activates its tool
  (the status bar tool name changes); press each of `L P R C A M E T X D` and
  confirm the same. Nothing else about the app may behave differently — the
  command line still does nothing when you press Enter, and that is correct at
  this point in the milestone.

## Risks

- **`src/ui/toolbar.rs` and `src/ui/menubar.rs` are shared surfaces.** LCV-116
  iterates `toolbar::TOOLS` to generate the F1 dialog's tool rows, and LCV-113 /
  LCV-114 / LCV-115 all edit `src/ui/menubar.rs`. This demand only *adds a
  field* to `ToolEntry` and replaces a function call; whoever lands second
  re-reads the file. Landing LCV-110 first (drive order 1) keeps the overlap
  smallest.
- **The `e` decision is a deliberate divergence from v1** and will look like a
  bug to anyone who diffs the two alias tables. AC 13's test and the doc comment
  on the alias table are the record; do not "fix" it back.
- **`CommandHistory` has no consumer until LCV-111.** Rust will not warn (it is
  `pub` in a lib crate), so the only thing keeping it honest is its unit tests.
  If LCV-111 slips, this module is dead code in a shipped binary — which is why
  LCV-111 is the next demand in the drive order and not a "later".
- **Whitespace tolerance is a trap for `Unknown`.** `"  foo  "` must yield
  `Unknown("foo")`, not `Unknown("  foo  ")` and not `Unknown("foo")` with a
  lowercased payload from a shared `to_lowercase()` binding. Keep the trimmed
  original and the lowercased match key as two separate values.

## Open questions

*(none — demand is Ready)*

## Notes

- Binding contract: [ADR 0003 §A](../../adr/0003-command-line-input-contract.md)
  (module placement, types, `ToolKind`, the ring) and §F4 (minimum coverage).
  Where this demand is more specific than the ADR — the grammar table, the
  cursor table, the `e` resolution, the locale rule — the ADR left those to
  `product-owner` by name.
- v1 reference: `../LaserCAD-R14/src/ui/command-line.ts:8-31` (alias and toggle
  tables), `:100-161` (the grammar, as four regexes), `:202-217` (the recall
  cursor). v1's regexes are the shape being reproduced; its alias table is
  reproduced only where product decision 2 allows.
- ADR 0003 §A2 states the alias set is "exactly the v1 set". Strictly, v1 also
  carried full-word aliases and `del`; this demand ships the ten single-token
  aliases the ADR enumerates and defers the rest as table rows (§Out of scope),
  which is what the ADR's own "the alias table is not frozen" sentence
  anticipates.
- `Vec2` is `crate::geometry::Vec2 { x: f64, y: f64 }`, world space, Y-up,
  millimetres. `EPSILON` is `1e-9` mm and the parser does not use it — parsing
  is exact.
- The module is named `cmdline`, **not** `command`: `Command` already means
  "undoable mutation" in `src/document/commands.rs` (ADR 0003 §A1).
- `Tool::on_command_input`'s current `&str` signature and its two existing unit
  tests in `src/tools/tool.rs` are untouched by this demand; LCV-111 replaces
  both.
