# ADR 0003 — The command-line input contract

- **Status**: Accepted
- **Amended (1)**: 2026-09-13 — §F3 trap 2 rewritten to follow
  [ADR 0002](0002-headless-input-tests-and-dirty-tracking.md) §A4 rule 2, whose
  premise died at `2d81a14` (LCV-119). It used to read *"Never let the 800 ms
  autosave debounce elapse — a fired autosave writes to the user's real data
  directory."* Nothing else in this ADR changes.
- **Amended (2)**: 2026-09-13 — LCV-131's refinement read §A2's sentence *"the
  alias set for LCV-110 is exactly the v1 set"* as a **freeze**. It is not one,
  and §A2's own next sentence says so. §A2a is new and states the two axes
  explicitly — variants frozen, letters closed, words open — so the misreading
  cannot recur, and corrects one factual error in §A2 about v1's `extend`. No
  decision in §A1..§A4 is reversed; nothing in §B..§F changes.
- **Amended (3)**: 2026-09-14 — §A2a described *how* `src/agent/classifier.rs`
  obeys the single-alias-table rule ("by calling `parse`") and listed it among
  the readers of the `CommandInput` variants. Both were true when amendment (2)
  was written and stop being true when **LCV-148** lands: deleting `classify`'s
  one `agent_available`-guarded arm collapses its inner `match parse(raw)` to a
  bare `Route::Cad`, after which the production path of that file neither calls
  `parse` nor names a `CommandInput` variant. Only the *account* is corrected —
  the rule is unchanged and compliance with it gets **stronger**, not weaker.
  This amendment was written before LCV-148's code landed and describes the
  post-LCV-148 tree; until that commit lands, the file still shows the
  amendment (2) mechanism. No decision in §A1..§A4 is reversed; nothing in
  §B..§F changes.
- **Amended (4)**: 2026-09-29 — [ADR 0012](0012-document-layers-and-per-layer-export.md) §7
  (LCV-156) adds the fieldless variant `CommandInput::Layers` (words `layer`, `la`), the
  variant-level change §A2a reserves for an ADR. The letter axis is unchanged.
- **Amended (5)**: 2026-09-29 — the v0.3 edit sprint adds five `ToolKind` variants, the
  variant-level change §A2a reserves for an ADR: `Copy` (words `copy`, `co`, `cp`; LCV-157),
  `Rotate` (`rotate`, `ro`; LCV-158), `Mirror` (`mirror`, `mi`; LCV-181), `Scale` (`scale`,
  `sc`; LCV-182) and `Dist` (`dist`, `di`; LCV-159). Command words only: the letter axis is
  unchanged, with no bare key and no `TOOL_KEYS` row. Polar input `@d<a` / `d<a` (LCV-159) is
  the "one parser arm" the revisit criteria foresaw: `parse` maps it to the existing
  `CommandInput::Relative` / `Point` in mm, degrees never leave the parser, and no
  `CommandInput` variant is added. `Tool` gains one default method,
  `take_message(&mut self) -> Option<String>` (default `None`), a single-shot sibling of
  `take_successor` that hands one line to `command_feedback`, drained before succession
  (LCV-159 DIST); tools that do not override it are unaffected. `geometry::Transform`
  (LCV-158) is an additive kernel type below this contract and needs no ADR. Nothing in
  §A..§F is reversed.
- **Date**: 2026-09-12
- **Deciders**: architect (Marco 1 / LCV-110, LCV-111, LCV-112)

## Context

Marco 1 is functional parity with v1.0.0. Three of its demands are one feature
cut into three slices:

- **LCV-110** — a pure command-line parser: `X,Y`, `@X,Y`, a bare distance,
  tool aliases, toggles, zoom, "Unknown command", and a 50-entry recall ring.
- **LCV-111** — wire the parser into Line, Polyline, Rect, Circle, Arc and Move
  through `Tool::on_command_input`, add per-phase prompts, and make typing focus
  the command line.
- **LCV-112** — TEXT: click an anchor, type the string, type the height
  (default 5 mm), one undo entry for the whole operation.

They cannot be specified independently: the parser's output type is the tool
contract, and the tool contract decides what the app must resolve before a tool
is called. This ADR fixes all of it once so `product-owner` can write three
demand bodies that do not contradict each other.

Binding precedent: **ADR 0002 §A6** made `src/ui/shortcuts.rs::dispatch_shortcuts`
plus `src/app/input.rs::process_input` the only readers of key presses, behind
one `ctx.wants_keyboard_input()` gate, and declared that "a second reader of a
key is a double dispatch". LCV-111's "any alphanumeric key focuses the command
line" collides head-on with the bare tool-activation keys `L P R C A M E T X D`.
This ADR resolves that collision without adding a reader.

### What exists today (verified in the tree, not assumed)

- `src/ui/command_line.rs` (68 lines) already renders
  `ToolManager::active_status_text()` as a prompt, owns an `egui::TextEdit`
  bound to `App::command_line_input`, clears on Escape and submits on Enter via
  `ToolManager::on_command_input(&str, &mut Document, &mut History)`.
- `Tool::on_command_input(&mut self, &str, &mut Document, &mut History)` exists
  with a **no-op default** and **zero implementors** (LCV-068 shipped the pipe,
  not the water).
- `Tool::status_text(&self) -> &'static str` already exists and already
  implements the pull model. `LineTool`, `PolylineTool`, `RectTool`, `MoveTool`
  and `TextTool` override it; **`CircleTool` and `ArcTool` do not** — they
  inherit `name()`.
- `Tool::anchor(&self) -> Option<Vec2>` (LCV-053) already exposes exactly the
  base point relative input needs.
- `App` already carries `last_cursor_world`, `ortho_enabled`, `snap_enabled`,
  `grid_enabled`; `Camera` already has `zoom_in`, `zoom_out`, `zoom_extents`.
- `src/ui/shortcuts.rs::TOOL_KEYS` maps ten bare keys to `fn() -> Box<dyn Tool>`.
- egui resolves to **0.29.1** in `Cargo.lock`; `Cargo.toml` requests `"0.29"`,
  so a `cargo update` can move inside 0.29.x but cannot reach 0.30. Not
  upgrading; `egui_kittest` stays out of scope (ADR 0002).

### What was verified before deciding

A throwaway probe crate against egui 0.29.1 (a stand-in `update_ui` mirroring
the gate + the command-line panel, driven through `egui::Context::run`), five
tests, all passing:

1. A frame whose only event is `Event::Text("5")`, arriving while the field is
   **not** focused, is seen by the top-of-frame gate; the gate appending the
   character to the buffer and the widget calling `response.request_focus()`
   afterwards yields `buf == "5"` — **no double insert**. The widget's event
   handling runs before `request_focus()` takes effect, so the same frame's text
   cannot reach it twice.
2. `ctx.wants_keyboard_input()` is `false` at the top of the seeding frame and
   `true` from the next frame on — ADR 0002's one-frame focus lag, confirmed
   again for this path.
3. With focus established, further `Event::Text` frames land in the `TextEdit`
   normally (`"5"` → `"50,7"`), with the gate seeding exactly once.
4. **Enter only submits if the widget had focus in the previous frame.** The
   widget's submit is `response.lost_focus() && key_pressed(Enter)`; a
   single-frame Enter tap with an unfocused field submits nothing.
5. `Escape` clears the field **and** releases focus (`wants_keyboard_input()`
   drops back to `false` on the following frame), while the gate still sees the
   Escape event at the top of that same frame — both effects, one press.
6. A focused single-line `TextEdit` does **not** modify its buffer on
   `ArrowUp` / `ArrowDown`, and those key events are visible to the top-of-frame
   gate. Command recall needs no `consume_key` and no second reader.

## Decision

### A. The parser is a new kernel module: `src/cmdline/`

**A1. Placement.** A new top-level module `src/cmdline/`, three files:

| file | contents |
|---|---|
| `src/cmdline/mod.rs` | module header, the types below, re-exports |
| `src/cmdline/parse.rs` | `parse`, `parse_number`, the alias tables |
| `src/cmdline/history.rs` | `CommandHistory` — the 50-entry recall ring |

It is **kernel**: it imports `crate::geometry` and `std`, nothing else. It joins
the AGENTS.md purity list, which becomes `geometry/`, `document/`, `io/svg/`,
`text/`, `cmdline/`. That one-line amendment to AGENTS.md §"Purity rule" and
§"Module tree" ships with LCV-110.

Not `src/ui/` (the whole point is that it is testable without egui), not
`src/tools/` (`tools/` imports `App` through `Tool::on_key`, so anything in it
is transitively egui-coupled), not `src/geometry/` (parsing text is not
geometry). The name is `cmdline`, **not** `command`: `Command` already means
"undoable mutation" in `src/document/commands.rs`, and a module named `command`
next to that trait is a permanent reading hazard.

**A2. Public types.** The shape proposed in the brief, with three refinements:

```rust
pub enum CommandInput {
    Point(Vec2),        // "50,25"   — absolute, world mm
    Relative(Vec2),     // "@10,-5"  — offset from the active anchor
    Distance(f64),      // "37.5"    — a bare magnitude in mm
    Tool(ToolKind),     // "l", "p", "r", "c", "a", "s", "t", "e", "m", "text"
    Toggle(ToggleKind), // "snap" | "grid" | "ortho"
    Zoom(ZoomKind),     // "zoom in" | "zoom out" | "zoom extents" | "ze"
    Empty,              // the field was blank
    Unknown(String),    // anything else; payload is the trimmed raw text
}

pub enum ToolKind { Select, Line, Polyline, Rect, Circle, Arc, Move, Delete, Trim, Extend, Text }
pub enum ToggleKind { Snap, Grid, Ortho }
pub enum ZoomKind { In, Out, Extents }

pub fn parse(raw: &str) -> CommandInput;
pub fn parse_number(raw: &str) -> Option<f64>;   // trims, rejects NaN / ±inf
```

Refinements over the brief:

- **`Empty` is a variant.** Without it, pressing Enter on a blank field yields
  `Unknown("")` and the operator gets a spurious "Unknown command" on a
  keystroke that is meaningful (see B5). This is the one addition that is not
  cosmetic.
- **`Unknown(String)` carries the trimmed raw text**, so the app can echo
  `Unknown command: "foo"` without keeping its own copy.
- **`ToolKind` is a fieldless enum declared here, not `Box<dyn Tool>`.** The
  parser must not know what a `Tool` is; `tools/` maps the kind to an instance
  (A3). `ToolKind` names **every** tool, not only the aliased ones, because it
  also becomes the key-binding table's value type.

`parse` is **total** — it returns `Unknown`, never `Err`, never panics. Parsing
is case-insensitive and trims surrounding whitespace, including around the comma
(`"10, 20"` parses). `zoom` takes one argument; `ze` is the extents alias. The
alias set for LCV-110 is exactly the v1 set `l p r c a s t e m text`: EXTEND and
the `d`/`x` letters have no v1 alias and do not get one now. Following ADR 0002's
precedent for `TOOL_KEYS`: **the variants are the contract; the alias table is
not frozen** — adding `line`, `x` or `d` later is a table row, not an ADR.

**A2a. The alias table has two axes. One is closed, one is open, and neither is
the contract.**

*(Added by amendment (2). §A2's last sentence already said the table is not
frozen and already named `line` as the example of a later row. LCV-131's draft
read the preceding sentence — a scope statement for LCV-110 — as a freeze and
came here to unfreeze it. Nothing needs unfreezing. What was missing is the
statement of which axis is which, which is written here once.)*

**The contract is the variants.** `CommandInput`, `ToolKind`, `ToggleKind` and
`ZoomKind` are the types every caller matches on; adding, removing or renaming a
variant is an ADR-level change because `tools::make`'s exhaustive `match`
(§A3), `src/app/cmdline.rs::submit`'s exhaustive `match` (§B5) and
`src/ui/shortcuts.rs::TOOL_KEYS` all read them. *(Amendment (3): this list named
`src/agent/classifier.rs` as a fourth reader. It was one until **LCV-148**;
after it, the variants survive there only in that file's `#[cfg(test)] mod
tests`, and a test module is not a caller the contract is owed to.)* **The alias
table is a table.** A row is a row.

Two axes map text onto those variants, and they are independent:

- **The letter axis is closed.** `l p r c a s t e m` is the set, and it does not
  grow. A letter is a scarce, memorised, one-keystroke resource shared with the
  bare tool-activation keys (§E), so a new letter is a binding decision, not a
  row. **`e` is Delete, not Extend** — a v2 decision that deliberately departs
  from v1, where `e` is `extend` (verified 2026-09-13 in
  `../LaserCAD-R14/src/ui/command-line.ts`, `TOOL_ALIASES`). §A2's claim that
  *"EXTEND … has no v1 alias"* is wrong on the facts; the letter is free in v2
  because v2 reassigned it, not because v1 left it empty. That reassignment
  stands and is not revisited here. Extend, Delete and Text are reachable from
  the command line by **word**.
- **The word axis is open.** A word that spells a command this product already
  ships is a row, added by a demand, needing no ADR. The bound is not taste: a
  word may only be added if it maps to an **existing** `ToolKind` /
  `ToggleKind` / `ZoomKind` variant. `offset` has no variant to map to — the
  tool was rejected as LCV-054 — so it is not excluded by preference, it is
  unspellable. The same bound is why a word may never be the way a new tool
  enters the product.

**The alias table lives in `src/cmdline/parse.rs` and nothing outside
`src/cmdline/` may hold a second copy of it.** *(Amendment (3) rewrites the
sentence that stood here. It read: "`src/agent/classifier.rs` already obeys this
by calling `parse` rather than re-deciding a rule of it, and its module header
says so." That was the mechanism until **LCV-148**, which deletes the one arm
that call fed and leaves `classify` deciding on the `:` / `/ai` prefix alone.
From LCV-148 on the classifier obeys the rule by **not needing the grammar at
all**, which is the stronger compliance: a file that never consults the alias
table cannot hold a second copy of it and cannot re-decide a rule of it. The
rule is not weakened and the classifier is not the reason for it.)*

**This is not a ban, and no demand may turn it into one.** `crate::cmdline`
stays a permitted import of `src/agent/classifier.rs` — LCV-148 moves the `use`
into that file's `#[cfg(test)] mod tests`, which still needs `parse` to record
what the grammar does with `z`, `ze` and `zoom in` — and `crate::cmdline` is
**not** added to the forbidden list of its `classifier_is_kernel_pure` scan.
`agent/ → cmdline/` is an allowed direction that the production path simply no
longer travels. Nothing in §A2a bars any file from *calling* `parse`; what is
barred is the second copy.

A UI file *reading* the table is the allowed direction
(`ui → cmdline`), but **reading it is not free**, and two constraints on
`src/ui/dialogs.rs` are load-bearing right now:

- it is at **299** of 300 implementation LOC, so [ADR 0004](0004-measuring-the-300-loc-cap.md)
  §"The `src/ui/dialogs.rs` seam" makes the next demand that adds an
  implementation line to it execute the `src/ui/shortcuts_dialog.rs` split
  first — and **LCV-134 already carries that split**;
- its body clears the clip by 8.00pt and one row costs 21.00pt (LCV-134), so
  *one* alias row overflows the dialog.

Therefore: **a demand that adds alias rows does not document them in the F1
dialog.** `SHORTCUT_GROUPS` is a table of **key bindings**; the command-line
vocabulary is a different reference with a different shape, and putting it
behind F1 is a scope change to that dialog, not a side effect of adding a row
to a parser. If the vocabulary should be discoverable in the UI, that is its own
demand, it lands **after** LCV-134 so it inherits the headroom, and it decides
its own presentation. Until then a new alias is documented in `CHANGELOG.md` and
in the rustdoc on `tool_alias`.

**A3. `ToolKind` → instance lives in `tools/`.** `src/tools/mod.rs` gains

```rust
pub fn make(kind: ToolKind) -> Box<dyn Tool>;
```

an exhaustive `match`, and `src/ui/shortcuts.rs::TOOL_KEYS` becomes
`&[(Key, ToolKind)]` calling it. This keeps **one** mapping from tool identity
to tool instance; the keyboard table and the alias table are two bindings into
the same enum. The LCV-104 test that cross-checks `TOOL_KEYS` against the
toolbar survives unchanged (`make(kind).name()`). Dependency direction stays
acyclic: `cmdline → geometry`; `tools → cmdline`; `app → {cmdline, tools, ui}`.

**A4. The recall ring belongs to the pure module, and one instance lives on
`App`.**

```rust
pub struct CommandHistory { /* VecDeque<String>, cursor */ }
impl CommandHistory {
    pub const CAPACITY: usize = 50;
    pub fn push(&mut self, entry: &str);     // ignores empty; evicts oldest past CAPACITY; resets the cursor
    pub fn older(&mut self) -> Option<String>;  // Up
    pub fn newer(&mut self) -> Option<String>;  // Down; None past the newest → caller clears the field
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

`App` gains `pub command_history: CommandHistory`. The widget owns **no** state
beyond the `String` it edits: egui is immediate-mode, so widget-held state is
state that cannot be asserted on in a headless test — which is the entire lesson
of ADR 0002. Returning `Option<String>` rather than `Option<&str>` is
deliberate: fifty short strings cost nothing and the caller never fights a
borrow of two `App` fields.

Duplicates are **not** deduplicated. Raw-mode content (D) is never pushed.

### B. `Tool::on_command_input` — exact signature and semantics

**B1. Signature** (replaces the LCV-068 default-only stub):

```rust
fn on_command_input(
    &mut self,
    input: ToolInput,
    doc: &mut Document,
    history: &mut History,
) -> bool { false }
```

with, in `src/cmdline/mod.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolInput {
    /// An absolute world point in mm. Already resolved: `X,Y` verbatim,
    /// `@dx,dy` added to the anchor, direct-distance entry projected.
    Point(Vec2),
    /// A bare number. `along` is the direct-distance point when a direction
    /// existed (anchor known, cursor known, distinct, ortho applied), else None.
    Distance { value_mm: f64, along: Option<Vec2> },
}

impl ToolInput {
    /// The world point this input designates, if any.
    pub fn as_point(self) -> Option<Vec2>;   // Point(p) => Some(p); Distance{along, ..} => along
}
```

**No `&mut App`.** LCV-041's narrowing holds: tools get `Document` + `History`
and nothing else. Everything app-shaped that the resolution needs — the tool's
`anchor()`, `App::last_cursor_world`, `App::ortho_enabled` — is consumed by the
app *before* the call. That is the whole answer to "must not force every tool to
duplicate the same X,Y parsing": **a tool never sees a string.**

**B2. Return value.** `true` = "I consumed this and advanced a phase (possibly
committing)"; `false` = "not for me". A `bool`, not an outcome enum: the only
extra information a richer type would carry is a per-tool error string, and the
tool's prompt (C) already tells the operator where it is. If a second tool ever
needs distinct rejection text, promote `bool` to an enum then — not now.

On `false` the app mutates nothing, sets the feedback message (B6), and leaves
the tool's phase untouched.

**B3. Canonical implementation.** A typed point is *exactly* a pointer-down at
that point, so the five point-driven tools are five lines each:

```rust
fn on_command_input(&mut self, input: ToolInput, doc: &mut Document, history: &mut History) -> bool {
    match input.as_point() {
        Some(p) => { self.on_pointer_down(p, false, doc, history); true }
        None => false,
    }
}
```

`CircleTool` is the one tool that overrides the `Distance` arm: while awaiting a
radius, `Distance { value_mm, .. }` **is** the radius, with no direction needed.
That is why `ToolInput::Distance` carries both fields — a keyboard-only session
(`c` ⏎ `50,50` ⏎ `25` ⏎) has no cursor at all, and `App::last_cursor_world` is
`None` until the pointer first enters the viewport.

`Tool::on_command_input` keeps its **`false` default**, so `SelectTool`,
`TrimTool`, `ExtendTool` and `DeleteTool` are unchanged and typed coordinates
never silently select or delete something.

**B4. Committing.** Unchanged and non-negotiable: the tool builds a
`Box<dyn Command>` and calls `history.commit(cmd, doc)` — the same call it
already makes from `on_pointer_down`. No new mutation path, so the AGENTS.md
contract and ADR 0002 §B's `History::revision()` dirty signal both hold for
free. After the call the app must poll succession exactly as the pointer path
does; `poll_successor` (today private in `src/app/viewport.rs`) becomes
`App::poll_successor(&mut self)` in `src/app/mod.rs` and both callers use it, so
`m` ⏎ `@10,0` ⏎ hands back to `SelectTool` like the mouse flow does.

**B5. Resolution — the app side, one place.** New file `src/app/cmdline.rs`
(a `mod.rs` phase file, re-exported per AGENTS.md), holding
`pub fn submit(app: &mut App, raw: &str)`. `src/ui/command_line.rs` calls this
and nothing else on Enter. Order of business:

1. If the active tool `wants_raw_input()` (D) → `on_raw_input`, return. Not
   pushed to the recall ring.
2. `cmdline::parse(raw)`; push the trimmed raw text to `command_history`
   (rejected input included — recall exists so the operator can fix a typo).
3. Dispatch:
   - `Tool(kind)` → `app.tool_manager.set_tool(tools::make(kind))`.
   - `Toggle(kind)` → flips `snap_enabled` / `grid_enabled` / `ortho_enabled`,
     same semantics as F3 / F7 / F8.
   - `Zoom(In|Out)` → `Camera::zoom_in`/`zoom_out` with one named constant in
     this file; `Zoom(Extents)` → `handle_zoom_extents(…, app.camera.viewport_size_px)`,
     which already no-ops on the zero-area first-frame sentinel.
   - `Point(p)` → `ToolInput::Point(p)`.
   - `Relative(d)` → `ToolInput::Point(anchor + d)` when
     `app.tool_manager.anchor()` is `Some`; with no anchor it is **not** sent to
     the tool — feedback message, no mutation.
   - `Distance(v)` → `ToolInput::Distance { value_mm: v, along }` where `along`
     is `Some(anchor + v * dir)` when an anchor exists, `last_cursor_world` is
     `Some` and distinct from the anchor, and `dir` is that direction
     **after `apply_ortho`** when `ortho_enabled`; otherwise `None`.
   - `Empty` → route `egui::Key::Enter` to the active tool (the existing
     `on_key` path). This is what keeps "Enter finishes the polyline" alive now
     that the command line usually holds keyboard focus, and it matches R14's
     "empty Enter accepts". Nothing is pushed to the ring.
   - `Unknown(text)` → feedback `Unknown command: "<text>"`, nothing else.

Typed coordinates **bypass snap entirely** and bypass ortho except as the
direction source for direct-distance entry: a number the operator typed is
exact. The app does **not** write `last_cursor_world` from a typed point; the
cursor is the mouse's, and faking it would corrupt the next direction.

**B6. Feedback.** `App` gains `pub command_feedback: String` (empty = none),
rendered by the command-line widget after the prompt and cleared at the top of
every `submit`. It is a display string only; no control flow reads it.

**B7. LCV-111 scope.** `LineTool`, `PolylineTool`, `RectTool`, `ArcTool`,
`MoveTool` take the B3 body verbatim; `CircleTool` takes it with the radius
override.

### C. Prompts — keep the pull model that already exists

`Tool::status_text(&self) -> &'static str` **is** the prompt mechanism, it is
already a pull, and `src/ui/command_line.rs:29` already renders it in the
command-line strip. No `prompt()` method is added, no prompt string is pushed
onto `App`, and the status bar is not involved — it keeps showing coordinates,
the tool name and the mode badges.

Consequences for LCV-111:

- `CircleTool` and `ArcTool` must gain `status_text()` overrides; they inherit
  `name()` today, which is why they show no phase prompt.
- The literals are rewritten to the R14 shape (`"LINE Specify first point:"`).
  The exact wording is `product-owner`'s, the mechanism is not.
- **Escape clears nothing by hand.** The gate routes Escape to
  `Tool::on_key` → `cancel()` → the state machine returns to `Idle`, and the
  next `status_text()` call *is* the idle prompt. A pull model has no stale
  prompt to clear. In the same frame the widget clears its own buffer and egui
  drops focus (verified, probe 5).
- `&'static str` is kept, so a prompt may not interpolate a runtime value.
  `"TEXT Specify height <5>:"` is a literal and fits. The day a prompt needs
  `"<30.5>"` from live state, the return type becomes `Cow<'static, str>` — a
  deferred, mechanical change (Marco 2).
- The name `status_text` is now a mild misnomer (only the command line consumes
  it). Renaming it touches ten files for zero behaviour: not in Marco 1. Do not
  wire it into the status bar.

### D. Raw-input mode — two pull/push methods on `Tool`

```rust
/// True while this tool wants the command line as a free-text field.
fn wants_raw_input(&self) -> bool { false }

/// The submitted text, unparsed. Returns true if consumed.
fn on_raw_input(&mut self, _raw: &str, _doc: &mut Document, _history: &mut History) -> bool { false }
```

When `wants_raw_input()` is true:

- `src/app/cmdline.rs::submit` skips `parse` entirely (B5 step 1). Typing the
  word "line" as text inserts the word; it does not switch tools. That is
  correct: raw means raw.
- The app **keeps the command line focused every frame** (it sets the one-shot
  focus request of E). This is the whole reason no keyboard-gate exception is
  needed for raw mode: with the field focused, `ctx.wants_keyboard_input()` is
  true and the existing gate already suppresses bare tool keys. The gate table
  is untouched.

`TextTool` (LCV-112) becomes:

| state | `wants_raw_input` | prompt | submit |
|---|---|---|---|
| `Idle` | false | `TEXT Specify start point:` | — (pointer click sets the anchor) |
| `WaitingText { anchor }` | true | `TEXT Enter text:` | non-empty → `WaitingHeight`; empty → cancel to `Idle`, nothing committed |
| `WaitingHeight { anchor, text }` | true | `TEXT Specify height <5>:` | empty → 5.0 mm; `parse_number` > 0 → that height; otherwise stay and re-prompt |

**Exactly one undo entry**: nothing is committed until the height is accepted,
and the acceptance commits a single `CreateEntities` carrying every glyph
stroke — which is already how `TextTool` commits. The acceptance test asserts
that one `Ctrl+Z` removes the whole string. `TextTool` commits through
`history.commit`, dropping its current `App::commit` call (LCV-041 alignment;
the dirty signal is revision-based, so nothing is lost).

Height parsing uses `cmdline::parse_number`, not an ad-hoc `str::parse` — one
number grammar in the product.

Because the `TextEdit` now does the editing, `TextTool::on_text_input` and its
per-character Backspace handling are **deleted**. `Tool::on_text_input` then has
no implementor: delete the trait method, the `ToolManager` forward, and the
"typed characters" row of the gate table — the gate's `Event::Text` handling is
repurposed by E. A net loss of a trait method.

### E. Focus and keyboard routing — ADR 0002 §A6 is extended, not superseded

**The single-gate invariant survives.** `dispatch_shortcuts` remains the sole
reader of key presses, `process_input` remains the sole owner of the focus gate,
no widget dispatches a key, and `consume_key` stays banned.

**E1. Who wins for a bare letter: the tool shortcut.** While the command line
does **not** have focus, the ten bound letters `L P R C A M E T X D` activate
their tool and the character does not reach the field. The decision is made in
exactly one place, the `TOOL_KEYS` loop in `dispatch_shortcuts`, and nowhere
else. LCV-111's acceptance criterion must therefore read "**any alphanumeric
that is not a bound tool shortcut** focuses the command line" — digits, `@`,
`-`, `.`, `,` and the unbound letters (`z`, `s`, `g`, …). `ze` works: `z` is
unbound, so it seeds and focuses, and `e` then types into the focused field
instead of activating ERASE.

**E2. How the character gets there: the gate seeds, the widget requests focus.**
In `src/app/input.rs`, the `Event::Text` branch (formerly the
`ToolManager::on_text_input` forward) becomes: when `!wants_kbd`, no shortcut
fired this frame, and the text is non-empty → append it to
`App::command_line_input` and set the one-shot `App::focus_command_line = true`.
`src/ui/command_line.rs` consumes that flag with
`if std::mem::take(&mut app.focus_command_line) { response.request_focus(); }`
**after** `ui.add(...)`. Verified (probe 1/3): the character lands exactly once —
the widget cannot double-consume the same frame's text because it had no focus
while it ran — and from the next frame egui types into the field normally.

`dispatch_shortcuts` returns `bool` ("this key was consumed by a shortcut") and
`process_shortcuts` folds it into a per-frame flag passed to `process_input`.
Both existing signatures stay source-compatible with `tests/lcv070.rs` and
`tests/lcv104.rs`, which call them as statements. This flag is what stops `l`
from both starting LINE and typing an `l` — real keyboards emit `Event::Key`
*and* `Event::Text` for the same keystroke.

**E3. Recall keys.** `ArrowUp` / `ArrowDown` are read in `process_input`,
**before** the `if wants_kbd { return; }` early-out, and act only when
`App::command_line_focused` is true — a plain mirror of `response.has_focus()`
that the widget writes each frame (writing state is allowed; reading keys is
not). Up replaces the buffer with `command_history.older()`; Down with
`newer()`, clearing the field when it runs past the newest entry. Verified
(probe 6): egui's single-line `TextEdit` leaves its buffer alone on arrows, so
no `consume_key` and no second reader is needed.

**E4. The gate table, as amended.** ADR 0002 §A6's table gains two rows and
loses one; every other row is unchanged:

| class | keys | fires while a text widget has focus |
|---|---|---|
| global commands | `Ctrl+Z/Y/N/O/S`, `Ctrl+Shift+S` | yes |
| view toggles | `F3`, `F7`, `F8` | yes |
| cancel | `Escape` | yes |
| view actions | `F`, `Ctrl+0` | no |
| tool activation | `L P R C A M E T X D` | no |
| tool key routing | `Enter`, `Delete`, `Backspace` | no |
| ~~typed characters → `ToolManager::on_text_input`~~ | — | **removed (D)** |
| typed characters → seed + focus the command line | `Event::Text` | no |
| command recall | `ArrowUp`, `ArrowDown` | **only while the command line has focus** |

Adding rows with an explicit gate column is what ADR 0002 reserved for a new
ADR; this is that ADR. **ADR 0002 is not superseded** — its Status stays
`Accepted` and its decisions A1–A6 and B stand as written.

**E5. The widget's one sanctioned key read.** `src/ui/command_line.rs` may read
`ui.input(|i| i.key_pressed(k))` **solely to disambiguate its own
`response.lost_focus()`** (Enter = submit, Escape = clear) — never to dispatch,
never with `consume_key`. That is the pattern already in the file, and this
sentence is the bright line a reviewer applies to any new occurrence.

**E6. What is not decided here.** Removing the bare tool shortcuts entirely so
that every printable key goes to the command line (true R14 fidelity: `L` ⏎
runs LINE, and there are no one-key tools) is a **product** question, not an
architectural one. The mechanism above supports it: delete the `TOOL_KEYS` loop
from `dispatch_shortcuts` and nothing else changes. Marco 1 keeps the shipped,
toolbar-advertised bindings (LCV-070 / LCV-104); `product-owner` may raise the
switch as its own demand.

### F. Testability

Every Marco 1 demand that touches input carries a headless regression test
driving the real `App::update_ui(ctx)` through `tests/harness/mod.rs`
(ADR 0002 §A3). One file per demand: `tests/lcv110.rs`, `tests/lcv111.rs`,
`tests/lcv112.rs`. Pure parser and ring tests are inline `#[cfg(test)]` modules
in `src/cmdline/*` — LCV-110's "exhaustively unit-tested" lives there and needs
no harness at all.

**F1. Harness additions** (`tests/harness/mod.rs`, additive):

```rust
pub fn text_events(s: &str) -> Vec<egui::Event>;           // one Event::Text
pub fn type_command(ctx: &egui::Context, app: &mut App, s: &str);  // one frame per char
pub fn submit_command(ctx: &egui::Context, app: &mut App, s: &str); // type_command + Enter tap
```

**F2. The recipe, verified against egui 0.29.1.**

- *Honest path* — `submit_command` sends one frame per character: the first
  character reaches the gate and seeds + focuses (E2), the rest land in the
  focused `TextEdit`, then one `tap(Enter)` frame submits. This exercises focus
  acquisition, typing and submit end to end. Commands are ≤ 10 characters, so
  the frame count is trivial.
- *Fast path* — for tests about the tool contract rather than focus: set
  `app.command_line_input` directly, set `app.focus_command_line = true`, run
  **one empty frame** (focus is granted at the end of it), then `tap(Enter)`.
- A test that only wants the parser or the ring calls `cmdline::parse` /
  `CommandHistory` directly — no `Context` at all.

**F3. The egui 0.29.1 traps. ADR 0002 §A4 documents four (the harness numbers
three and states the fourth on `key_events`); this ADR adds a fifth.**

1. **Never send `Ctrl+O`, `Ctrl+S` or `Ctrl+Shift+S`** — they reach
   `src/io/file_actions.rs`, open a blocking native `rfd` dialog and hang CI.
   `Ctrl+N`, `Ctrl+Z`, `Ctrl+Y` are safe.
2. **Build `App::default()`, never `App::new()`** — the full rule is
   [ADR 0002](0002-headless-input-tests-and-dirty-tracking.md) §A4 rule 2, as
   rewritten for LCV-119 / [ADR 0006](0006-real-user-paths-are-injected.md).
   Letting the 800 ms debounce elapse is harmless now: a test `App` carries no
   autosave path and writes nothing. What is not harmless is `App::new()`,
   which resolves the real per-user paths, or an injected path aimed anywhere
   but a tempdir the test owns. A per-character `submit_command` run is still
   one frame per character with no wall-clock sleep, so it never reaches the
   debounce either way.
3. **Pointer tests need a warm-up frame** carrying `PointerMoved` alone before
   the frame carrying `PointerButton`; otherwise the widget rect is not
   registered, `response.hovered()` is `false`, and the viewport handler never
   runs. This matters for LCV-112: the TEXT anchor click is a pointer test.
4. **Always emit press *and* release** — use `key_events` / `tap`. egui rewrites
   the second press of the same key on a `Context` as `repeat: true`, and
   `dispatch_shortcuts` filters repeats, so a hand-rolled press alone produces a
   phantom no-op.
5. **(New) Enter only submits if the field had focus in the *previous* frame.**
   The widget submits on `lost_focus() && key_pressed(Enter)`. A single-frame
   Enter tap against an unfocused command line does nothing — a silent
   false-negative that looks exactly like a broken parser. Every command-line
   test is therefore at least two frames. Add this to the harness module header.

**F4. Minimum coverage per demand.** LCV-110: every `CommandInput` variant,
including `Empty`, `Unknown`, whitespace and case variations, `NaN`/`inf`
rejection, and the ring at 50 entries with Up/Down past both ends. LCV-111: one
headless test per wired tool asserting the committed entity's coordinates, one
asserting ortho-constrained direct-distance entry, one asserting `l` activates
LINE while unfocused and types an `l` while focused. LCV-112: the full TEXT
flow ending in `history.len() == 1` and one `Ctrl+Z` clearing the document.

## Consequences

**Easier**

- Tools never parse text and never see a string. Six tools get a five-line
  method; adding a seventh is the same five lines.
- Typed input and pointer input converge on `on_pointer_down`, so a tool's state
  machine has one implementation and the command line inherits every fix to it.
- `ToolKind` gives the product one enum naming every tool, with the keyboard
  table and the alias table as two bindings into it — a new tool is registered
  in one place.
- The parser and the recall ring are unit-testable with no `egui::Context`,
  which is where LCV-110's exhaustive coverage goes.
- Raw mode needs no keyboard-gate exception: holding focus reuses the gate that
  already exists.

**Harder / costs**

- `Tool` gains `wants_raw_input` + `on_raw_input` and changes
  `on_command_input`'s signature; it loses `on_text_input`. Net +1 method, and
  every tool file is touched once by LCV-111.
- `App` gains four fields: `command_history`, `command_feedback`,
  `focus_command_line`, `command_line_focused`. Two of them are one-frame
  plumbing, and the file is near its structural budget — `src/app/cmdline.rs`
  must absorb the logic, not `mod.rs`.
- `dispatch_shortcuts` returning `bool` makes "a shortcut fired" a value that
  must be threaded through `process_shortcuts` into `process_input`. That is the
  price of not adding a second keyboard reader, and it is the right price.
- The bound-letter carve-out means the keyboard is not uniform: `l` starts LINE,
  `z` opens the command line. E6 names the switch if the product wants R14
  uniformity instead.

**Committed to**

- `src/cmdline/` is kernel-pure and joins the AGENTS.md purity list.
- `ToolInput` is the only thing a tool receives from the command line.
- `history.commit(cmd, doc)` stays the single mutation path, from typed input as
  much as from clicks.
- `Tool::status_text()` is the prompt, pulled per frame, never pushed.
- `dispatch_shortcuts` + `process_input` remain the only keyboard readers.

## Alternatives considered

- **Put the parser under `src/ui/`** — it would be untestable without an egui
  context and would violate the demand's own "pure kernel module" wording.
- **Name the module `src/command/`** — collides in the reader's head with
  `document::Command`, the undo/redo trait. Rejected on readability alone.
- **`Tool::on_command_input(&str, …)`, tools parse** — six copies of the same
  `X,Y` parser and six chances to disagree about `@`. Explicitly what the brief
  forbids.
- **Pass `&mut App` to `on_command_input`** so tools can read cursor and ortho —
  reverses LCV-041 and re-couples every tool file to `App`, the exact move
  ADR 0002 §B rejected for the dirty flag.
- **Resolve a bare distance into a `Point` always, with a `+X` fallback when no
  direction exists** — invents geometry the operator did not ask for.
  Deterministic refusal beats a convenient wrong line (AGENTS.md §Product
  Philosophy).
- **`enum CommandOutcome { Consumed, Ignored, Rejected(&'static str) }` instead
  of `bool`** — three variants where two suffice today; promote it when a second
  tool needs distinct rejection text.
- **A `prompt()` method, or an `App::prompt: String` that tools push to** — the
  pull model already ships (`status_text`), and a pushed prompt needs a clear
  rule for Escape, tool switch and undo. The pull model needs none.
- **First-refusal raw input (try `on_raw_input` first, always, with no
  `wants_raw_input`)** — one method fewer, but an over-eager tool would silently
  swallow toggles and aliases, and the app could not know to hold focus. The
  pull query is worth its four lines.
- **Let the widget own the recall ring** — immediate-mode widget state cannot be
  asserted on headlessly; ADR 0002 exists because of exactly that blind spot.
- **`ui.input_mut(|i| i.consume_key(…))` to stop the `TextEdit` seeing Up/Down** —
  banned by ADR 0002 §A6, and probe 6 shows it is not needed.
- **A second keyboard reader inside the command-line widget for focus-on-typing** —
  the defect class ADR 0002 was written to close (D4).
- **Upgrade egui to ≥ 0.30 for `egui_kittest`** — out of scope for Marco 1; the
  five behaviours this ADR leans on are all verified against 0.29.1 today.

## Revisit criteria

- A prompt must interpolate live state → `status_text` returns
  `Cow<'static, str>`; nothing else changes.
- A second tool needs distinct rejection text → `bool` becomes an outcome enum.
- Polar entry (`@50<30`), `SPACE` as Enter, empty-Enter "repeat last command",
  LINE's `c`/`u` options, `zoom window`/`previous`, or full-word aliases are
  wanted → each is a table row or one parser arm; none reopens this ADR.
- The product decides bare tool letters should go to the command line instead
  (E6) → delete the `TOOL_KEYS` loop; the gate table row disappears with it.
