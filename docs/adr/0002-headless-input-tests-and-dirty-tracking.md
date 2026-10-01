# ADR 0002 — Headless input regression tests and autosave dirty tracking

- **Status**: Accepted
- **Amended (1)**: 2026-09-13 — §A4 rule 2 rewritten. Its original text read
  *"Never let the autosave debounce elapse. A fired autosave writes to the real
  `~/.local/share/lasercad/autosave.json`."* That premise died at `2d81a14`
  (LCV-119): under [ADR 0006](0006-real-user-paths-are-injected.md) a test
  `App` carries no path and a fired autosave writes nothing. The rule now
  guards the hazard that survived injection. Nothing else in this ADR changes.
- **Amended (2)**: 2026-09-14 — §A3 rewritten. Its original text read
  *"Shared plumbing lives in `tests/harness/mod.rs` … `tests/harness/mod.rs`
  exposes exactly five items"*, followed by the signatures of `SCREEN`,
  `key_events`, `raw_input`, `frame` and `tap` (all five still exist; the block
  is deleted, not the helpers). That enumeration was stale before LCV-132 —
  eight items shipped against it — and LCV-132 made the harness three files with
  twenty-four exports, so §A3 now states the harness's *shape and rules* and
  records a dated measurement instead of an inventory. The same rewrite records
  the `cfg(test)` compilation fact that decides the harness's home, which until
  now lived only in the `1b89f0e` commit message, and rules on two review
  questions (uniform `pub`; an explicit `pixels_per_point`). §A4, §A5 and §B are
  unchanged.
- **Amended (3)**: 2026-09-27 — LCV-152 note on §A3. New headless tests are
  modules `tests/it/lcvNNN.rs` of the single integration binary
  `tests/it/main.rs`, which declares the harness once (`#[path]` to
  `tests/harness/mod.rs`); a module brings it in with `use crate::harness;`.
  The `tests/lcvNNN.rs` paths below are historical.
- **Amended (4)**: 2026-09-30 — §A6: `Ctrl+A` joins the table as
  `select all | Ctrl+A | no` (LCV-166). A focused text field keeps `Ctrl+A`
  for its own text; every other row is unchanged.
- **Amended (5)**: 2026-09-30 — §A6: a `dialog` row joins the table (LCV-169).
  While any dialog is open, `src/app/input.rs::take_dialog_key` runs first in
  the frame and consumes `Enter` and `Escape` for the topmost dialog only, which
  treats the key as a click on its primary or Cancel/Close button. No shortcut,
  tool, command line or recall sees them; the focused command line keeps its
  text and focus. That `consume_key` is the routing itself, not a cover for a
  double dispatch, so the rule below does not apply to it. The one exception is
  `Enter` in the AI Settings system prompt (`agent::SYSTEM_PROMPT_ID`), which
  stays a newline. Every other row is unchanged.
- **Date**: 2026-09-12
- **Deciders**: architect (Marco 0 / LCV-103)

## Context

The May-2026 drive shipped 72 demands with a green `cargo test --all` gate and
still landed ~10 integration defects. Every one of them lives in the same blind
spot: **nothing in the test suite ever drives the real frame body.** Unit tests
call `dispatch_shortcuts(key, mods, wants_kbd, app)` with synthetic values
(`tests/lcv070.rs`), so they prove the dispatch table is correct and prove
nothing about whether the dispatch table is *reachable*.

Four confirmed defects, all downstream of that blind spot:

- **D4 — double dispatch.** F8 and Ctrl+Z/Ctrl+Y are handled twice in the same
  frame: once in `src/ui/shortcuts.rs` (~76-81, ~109-112) and again inside the
  `CentralPanel` closure in `src/app.rs` (~376-387). F8 toggles twice and is a
  no-op; Ctrl+Z undoes two commands.
- **D5 — `Key::Enter` is unrouted.** `src/app.rs:358` routes only `Escape` to
  the active tool, so `TextTool::on_key(Key::Enter, …)` (`src/tools/text.rs:98`)
  and Polyline's "Enter to finish" are dead code reachable only from unit tests.
- **D8 — no keyboard-focus gate.** `F`, `Ctrl+0`, `Delete`, `Backspace`
  (`src/app.rs:347,365,370`) and the `Event::Text` → `on_text_input` forward
  (~`src/app.rs:390`) read raw input with no `ctx.wants_keyboard_input()` guard.
  Typing `f` in a focused text field triggers zoom-extents.
- **D3 — autosave never fires.** `dirty_since` is written in exactly one place,
  `App::commit` (`src/app.rs:154`), but every tool except `TextTool` calls
  `history.commit(cmd, doc)` directly — a deliberate LCV-041 design so tools
  need no `&mut App` (see `src/tools/tool.rs:1-8`). ~20 commit sites bypass the
  dirty flag. The debounce is also 5 s; the original app used 800 ms.

Constraints: `egui`/`eframe` pinned at **0.29.1**, Rust 1.88. `egui_kittest`
requires egui >= 0.30 and is **out of scope** — the harness must be built from
what 0.29.1 already exposes.

### What was verified before deciding

Against the vendored `egui-0.29.1` source and a throwaway probe crate:

- `Context::run(&self, new_input: RawInput, run_ui: impl FnMut(&Self)) -> FullOutput`
  takes `&self`, so `ctx.run(input, |ctx| app.update_ui(ctx))` borrows cleanly.
- `Event::Key { key, physical_key, pressed, repeat, modifiers }` — five fields;
  `physical_key: Option<Key>` may be `None`.
- A key event supplied in `RawInput::events` is visible both to a top-of-frame
  `ctx.input(|i| i.events)` scan (the `process_shortcuts` path) and to
  `ctx.input(|i| i.key_pressed(k))` inside a `CentralPanel` closure.
- `ctx.wants_keyboard_input()` read at the **top** of frame N correctly reports
  focus established by a `TextEdit` in frame N-1.
- `RawInput::screen_rect: None` yields a ~9984 pt wide central panel. Usable,
  but tests must pass an explicit rect to get realistic layout.
- **egui rewrites `repeat` across frames.** Sending the same key as
  `pressed: true, repeat: false` in two consecutive frames on one `Context`
  *without an intervening release* makes egui mark the second event
  `repeat: true`. `src/ui/shortcuts.rs:29` filters on `repeat: false`, so the
  second press is silently dropped and the test sees a phantom no-op. Emitting
  press **and** release in the same frame fixes it — both frames then report
  `repeat: false`. (Note also that `i.key_pressed(k)` returns `true` even for a
  repeat event, so the two dispatch paths in `App::update` today do not even
  agree on what counts as a press — more fuel for A6.)
- **Pointer interaction needs two frames.** With `PointerMoved` + `PointerButton`
  in a single frame, `ctx.input(|i| i.pointer.primary_pressed())` is `true` but
  `response.hovered()` is `false` and `response.hover_pos()` is `None` — the
  widget rect is not yet registered for hit-testing. The viewport handler is
  behind `if response.hovered()`, so a pointer test **must** run a warm-up frame
  first. Keyboard-only tests are single-frame.
- `App::update` already takes `_frame: &mut eframe::Frame` **unused**
  (`src/app.rs:184`). The only viewport control in the tree,
  `src/ui/menubar.rs:40`, goes through `egui::Context::send_viewport_cmd`. The
  extraction below is therefore lossless, not approximate.
- `App::new()` takes **no** `&eframe::CreationContext` (`src/app.rs:135`); it
  takes no arguments and calls `crate::io::load_autosave()`.

## Decision

### A. Headless input regression tests

**A1. Extract the frame body as `App::update_ui`.**

```rust
impl App {
    /// The whole frame body. `eframe::App::update` delegates here; headless
    /// regression tests drive it directly through `egui::Context::run`.
    pub fn update_ui(&mut self, ctx: &egui::Context) { /* former update body */ }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }
}
```

The `eframe::App` impl keeps **only** the delegation. Nothing else may be added
to it: if a future demand needs `&mut eframe::Frame`, it takes a narrowed value
as a parameter of `update_ui`, never the `Frame` itself. `eframe` stays a
one-line dependency of `src/app/mod.rs`.

**A2. Tests construct `App` with `App::default()`. No new constructor.**

`App::default()` is already `pub`, already used by `tests/lcv070.rs`,
`tests/skeleton.rs` and `tests/app_tool.rs`, and is side-effect free. No
`new_for_test`, no `new_with_settings`. The smallest change is no change.

The matching rule, which must be added as a doc comment on both functions:

- **`App::new()` is boot-only.** It reads the user's real platform data
  directory via `crate::io::load_autosave()`. It MUST NOT be called from any
  test.
- **`App::default()` is the test constructor.** It touches no filesystem.

**A3. Headless tests live in `tests/lcvNNN.rs`, one file per demand**, matching
the existing `tests/lcv070.rs` convention. Shared plumbing lives under
**`tests/harness/`**, declared `mod harness;` in each consumer. *(Rewritten
2026-09-14 — see **Amended (2)** in the header for the text this replaces.)*
*(Location superseded by LCV-152 — see **Amended (3)**: new tests go in
`tests/it/`.)*

**Why `tests/harness/` and not a `#[cfg(test)]` module under `src/`.** This is a
compilation fact, not taste, and it has been load-bearing since the first
harness landed. `cfg(test)` is set only for the crate *being compiled in test
mode*; an integration-test binary under `tests/` links this library as an
ordinary dependency, with `cfg(test)` **off**. A `#[cfg(test)] mod` under `src/`
is therefore invisible to every file in `tests/`, and an ungated one would ship
test plumbing — a paint collector, a source-scan walker — in the library's own
public surface, where nothing the product does would ever call it. `tests/harness/` has neither problem: Cargo
compiles it into each consumer binary through its `mod harness;`, and it is not
auto-discovered as a test target, because only `tests/*.rs` and
`tests/*/main.rs` are.

**The shape: three modules, divided by kind.**

- `tests/harness/mod.rs` — **input**: the screen rect, one frame, a key tap, the
  command-line gestures, and §A4's rules restated in the voice of the code they
  bind.
- `tests/harness/paint.rs` — **what reached the screen**: the `Shape::Text`
  collector, the frame driver and its positive control, the line grouping, and
  the traps that decide whether a painted-text assertion means anything.
- `tests/harness/scan.rs` — **source scans**: the `.rs` walker and the
  comment-skipping matcher, and the rules that keep a scan able to fail.

A fourth module needs a *kind*, not a line count. Size pressure is answered by
[ADR 0004](0004-measuring-the-300-loc-cap.md) rule 4 — a seam for `paint.rs` is
already recorded in its Consequences — never by inventing a module whose name
is "the rest of it".

**The list of exported items is deliberately not written here.** The text this
paragraph replaces enumerated the harness as "exactly five items"; eight
shipped, and twenty-four are exported today across three files. Nothing in the
build could catch either drift, and nothing was going to. `ls tests/harness/`
and `grep -n '^pub ' tests/harness/*.rs` are the live inventory and they never
rot. `AGENTS.md` §Module tree makes the same call for the `src/` tree: a gloss
is orientation, and an enumeration earns its place only when it carries a rule
*per item*. This one carries none — a harness is meant to grow, and there is no
per-helper consequence for a reviewer to check.

*Rejected: keep the list and pin it with an assertion in
`tests/lcv132_harness_is_shared.rs`.* That is the right instinct aimed at the
wrong list. The enumerations this repository does pin — the three conditional
repaint sites, `src/agent/`'s purity buckets, LCV-128's normative sets — each
carry a consequence per item, so the assertion is how the rule is enforced.
Adding a harness helper is not an architectural event; a pin there would turn
every helper into an ADR edit plus a red test, and a rule that costs that much
gets obeyed rather than read. What is worth pinning about the harness is
behavioural, and LCV-132 pinned it: AC 8's orphan-include guard and AC 4's
precision test, both in `tests/lcv132_harness_is_shared.rs`.

**Measured on the date of this amendment** (`dbca0a3`, 2026-09-14, with
ADR 0004 rule 2's `awk` recipe — never `wc -l`): `mod.rs` **152**, `paint.rs`
**267**, `scan.rs` **106**; **525** implementation lines, **24** exported items.
That is a snapshot for whoever reads this decision later, not a list anyone
maintains. Measure before you depend on it.

**Four rules that *are* load-bearing.**

1. **`key_events` returns a press *and* a release**, and there is deliberately
   no single-event `key_event` helper, so the trap cannot be stepped into by
   accident. See the repeat-rewrite finding above; this is the D4 fix's
   companion.
2. **The blanket `#![allow(dead_code)]` in `mod.rs` is paid for, not
   tolerated.** Every consumer binary compiles the whole directory, so a test
   using only `tap` would otherwise trip `-D warnings` on everything else; the
   attribute propagates into the submodules, which carry no second copy. Its
   cost is that it also hides a declared-and-unused `mod harness;` — one sat in
   `tests/lcv133_shortcuts_dialog_fits.rs` until a reviewer read for it — so
   `tests/lcv132_harness_is_shared.rs` guards exactly that case.
3. **Every item in the harness is `pub` and stays `pub`.** Reachability is not a
   signal here: the blanket allow means an unused item is invisible either way,
   and an item with one caller today and three tomorrow would cost a visibility
   diff each time. `raw_input_at` currently has only an internal caller
   (`paint::painted_runs_at`); that is not a finding and narrowing it is not an
   improvement. What the harness offers is what its doc comments say.
4. **Positions are only meaningful at `pixels_per_point = 1.0`, and the driver
   checks it rather than each test remembering.** `InputState::default()` is
   `1.0` (`egui-0.29.1/src/input_state/mod.rs:248`), so every position-reading
   test is correct today — but `tests/lcv126_command_line_group.rs` is correct
   by inheritance, having never set it. The dependency is real: epaint rebuilds
   the font atlas when the value changes
   (`epaint-0.29.1/src/text/fonts.rs::Fonts::begin_pass`), so galley metrics,
   `SAME_LINE`'s four-point intra-row spread and LCV-133's 0.682pt straddle are
   all measured at 1.0 and none of them is reproducible at another value. So
   `paint::painted_runs_at` carries that check beside its `runs.len() > 10`
   control, **after** `ctx.run` returns and never before: `set_pixels_per_point`
   becomes active "at the start of the next pass"
   (`egui-0.29.1/src/context.rs:1893-1901`), so a pre-run read on a fresh
   context returns the default and would pass on exactly the frame that broke
   the assumption. Existing `ctx.set_pixels_per_point(1.0)` calls stay — a test
   that lays out its own galley must set the value before building one — but a
   test driven through the harness stops having to remember. **Not shipped
   yet**: it is roughly five implementation lines and belongs to the next demand
   that touches `tests/harness/paint.rs`, landing the file near 272 — inside
   ADR 0004's 270..300 band, with the seam already recorded, and not a reason to
   split.

**The demand-facing rule lives in `AGENTS.md`, not here.** "A rendering
acceptance criterion is not satisfied by a source scan alone" is
`AGENTS.md` §Implementation Rules' entry, added by LCV-132 AC 10, and it stays
there: it is read at the moment a test is written, which an ADR is not.
ADR 0004's own Alternatives record what happens when a rule is left in an ADR
and expected to be found — it failed once already. This section owns the
harness's *shape and home*; that entry owns the *obligation*, and neither
restates the other.

**A4. Four hard rules for headless tests.**

1. **Never send `Ctrl+O`, `Ctrl+S`, or `Ctrl+Shift+S`.** Those reach
   `src/io/file_actions.rs`, which opens a blocking native `rfd` dialog and will
   hang CI. (`Ctrl+S` with `current_file: None` falls through to
   `action_save_as` — also a dialog.) `Ctrl+N` is safe.
2. **Build `App::default()`, never `App::new()`, and point an injected path
   only at a directory the test owns.** *(Rewritten 2026-09-13 — see the
   **Amended** note in the header for the text this replaces.)*

   Since LCV-119 ([ADR 0006](0006-real-user-paths-are-injected.md)) the
   settings and autosave locations are resolved once in `App::new()` and
   carried as data on `App`. `App::default()` — the test constructor — leaves
   both `None`, and `App::write_autosave` returns `false` without touching the
   filesystem. **Letting the autosave debounce elapse is therefore harmless**,
   and `src/app/autosave.rs`'s
   `a_due_flush_with_no_injected_path_writes_nothing_and_still_settles` does
   exactly that on purpose, to prove it.

   Two hazards survived injection, and this rule is now about them:

   - **`App::new()` resolves the real per-user paths.** It loads the
     developer's `~/.config/lasercad/settings.json`, adopts their
     `~/.local/share/lasercad/autosave.json` as the document if one exists, and
     leaves both paths armed for every subsequent write and every
     `clear_autosave` in that test. It makes the test non-hermetic on the way
     in and destructive on the way out. **No test calls `App::new()`.** Build
     `App::default()` and override the fields you need — the
     `App { settings_path: Some(…), ..App::default() }` form used throughout
     `src/app/persist.rs`'s tests.
   - **An injected path is a loaded gun pointed wherever you aim it.** A test
     that sets `settings_path` or `autosave_path` aims it inside a temporary
     directory it created and owns. Never `directories::ProjectDirs`, never a
     literal under `~`, never a path derived from the environment of the
     machine running the test.

   So: a test that wants to assert on bytes injects a tempdir path and reads
   them back; a test that wants no write at all leaves the fields `None` and
   asserts on `app.dirty_since` / `autosave_due(...)`. Asserting against the
   real per-user location is not correct in either case — the one thing that
   has not changed.
3. **Pointer tests run a warm-up frame** with `PointerMoved` alone before the
   frame carrying `PointerButton`. Keyboard tests do not.
4. **Always emit a key release with the press** — use `key_events` / `tap`,
   never a hand-rolled `Event::Key`. Otherwise egui marks the next press on the
   same key as a repeat and `dispatch_shortcuts` drops it.

**A5. Reference test — the F8 case (D4, demand LCV-103). Verified to compile
and pass
against egui 0.29.1** (validated in a scratch crate with a stand-in `App` whose
`update_ui` mirrors the `src/ui/shortcuts.rs` event filter):

```rust
//! tests/lcv103.rs — regression: F8 must toggle ortho exactly once per press.

mod harness;

use harness::{key_events, raw_input, tap};
use lasercad::app::App;

#[test]
fn f8_toggles_ortho_exactly_once_per_press() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    assert!(!app.ortho_enabled, "ortho starts off");

    ctx.run(
        raw_input(key_events(egui::Key::F8, egui::Modifiers::NONE)),
        |ctx| app.update_ui(ctx),
    );
    assert!(
        app.ortho_enabled,
        "one F8 press must toggle ortho on (D4: double dispatch made this a no-op)"
    );

    tap(&ctx, &mut app, egui::Key::F8, egui::Modifiers::NONE);
    assert!(!app.ortho_enabled, "a second F8 press must toggle it back off");

    tap(&ctx, &mut app, egui::Key::F8, egui::Modifiers::NONE);
    assert!(app.ortho_enabled, "and a third turns it on again");
}
```

with the harness **as it stood on 2026-09-12** — the block below is the
proof that the pattern compiles at egui 0.29.1, not a copy of the current file;
§A3 (as amended) describes what `tests/harness/` is today:

```rust
//! tests/harness/mod.rs — shared headless-frame plumbing for LCV-1xx tests.
//!
//! Each integration-test binary compiles its own copy, hence `allow(dead_code)`.
#![allow(dead_code)]

use lasercad::app::App;

/// Screen size handed to egui, in points. Matches `DEFAULT_WINDOW_SIZE`.
pub const SCREEN: [f32; 2] = [1280.0, 800.0];

/// A full key *tap*: press followed by release, both `repeat: false`.
///
/// The release is mandatory. egui auto-detects key repeats: pressing the same
/// key in two consecutive frames without a release makes the second event
/// `repeat: true`, which `src/ui/shortcuts.rs` filters out.
pub fn key_events(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    vec![
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers },
        egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers },
    ]
}

/// One frame of raw input with a realistic screen rect. `modifiers` is taken
/// from the first key event so `ctx.input(|i| i.modifiers)` agrees with it.
pub fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
    let modifiers = events
        .iter()
        .find_map(|e| match e {
            egui::Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(egui::Modifiers::NONE);
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(SCREEN[0], SCREEN[1]),
        )),
        modifiers,
        events,
        ..Default::default()
    }
}

/// Drive exactly one `App::update_ui` tick with `events`.
pub fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
    ctx.run(raw_input(events), |ctx| app.update_ui(ctx));
}

/// Drive one frame containing a single complete key tap.
pub fn tap(ctx: &egui::Context, app: &mut App, key: egui::Key, modifiers: egui::Modifiers) {
    frame(ctx, app, key_events(key, modifiers));
}
```

Note that `egui` must be a `[dev-dependencies]` entry (or reachable via
`lasercad`'s own dependency) for `tests/` to name `egui::Key` directly;
`tests/lcv070.rs` already does `use egui::{Key, Modifiers};`, so this already
works today.

**A6. One keyboard gate.** `src/ui/shortcuts.rs::dispatch_shortcuts` becomes the
**sole** reader of key presses. The duplicate blocks inside the `CentralPanel`
closure are deleted, not guarded — that is the D4 fix. Escape / Enter / Delete /
Backspace routing to the active tool, `F` / `Ctrl+0` zoom-extents, and the
`Event::Text` → `ToolManager::on_text_input` forward all move into
`src/app/input.rs` behind the same gate. `dispatch_shortcuts` keeps its current
signature, so every test in `tests/lcv070.rs` keeps passing.

There is a **third** dispatch site to remove that the first draft of this ADR
did not name: `src/ui/command_line.rs:37-46` reads `Key::Escape`, calls
`ToolManager::handle_key` itself, and then papers over the collision with
`ui.input_mut(|i| i.consume_key(…))`. The widget keeps "clear my own buffer" and
nothing else; cancelling the tool is the gate's job. A `consume_key` call that
exists to hide a double dispatch is the smell this decision removes — treat a
new one as a review blocker.

The gate table is the contract:

| class | keys | fires while a text widget has focus |
|---|---|---|
| dialog (any open; topmost only) | `Enter`, `Escape` | yes, consumed first |
| global commands | `Ctrl+Z/Y/N/O/S`, `Ctrl+Shift+S` | yes |
| view toggles | `F3`, `F7`, `F8` | yes |
| cancel | `Escape` | yes |
| view actions | `F`, `Ctrl+0` | **no** |
| select all | `Ctrl+A` | **no** |
| tool activation | `L P R C A M E T X` | **no** |
| tool key routing | `Enter`, `Delete`, `Backspace` | **no** |
| typed characters | `Event::Text` | **no** |

The *rows* are the contract; the tool-activation *set* is not frozen. It lists
the bindings that exist today, and LCV-104 adds `D` for `TextTool`. A new tool
key joins that row without amending this ADR — what may not change without one
is a key's gate column.

`Enter` must be gated: `src/ui/command_line.rs:50` already consumes Enter to
submit the command line, and an ungated route would both submit the command line
and commit the `TextTool` on one press.

Zoom-extents reads `app.camera.viewport_size_px`, which the previous frame's
`CentralPanel` synced. On frame 0 that is `[0.0, 0.0]`
(`src/render/camera.rs:56`), so `handle_zoom_extents` must no-op on a
zero-area viewport. A user cannot press a key before the first frame is painted,
so this costs nothing.

### B. Autosave dirty tracking

**Decision: a monotonic revision counter on `History`, sampled once per frame.**

- `src/document/history.rs` gains a private `revision: u64` and
  `pub fn revision(&self) -> u64`. It is incremented in `commit`, and in `undo`
  and `redo` on the `true` (work-was-done) branch. Kernel-pure — no egui.
- `src/app` gains `last_synced_revision: u64` (defaults to `0`) and two methods:
  - `fn sync_dirty(&mut self)` — if `history.revision() != last_synced_revision`,
    `dirty_since.get_or_insert_with(Instant::now())` and store the new revision.
  - `pub fn mark_clean(&mut self)` — `dirty_since = None` **and**
    `last_synced_revision = history.revision()`.
- `sync_dirty()` is called **once**, at the end of `App::update_ui`, immediately
  before the autosave-flush check. It is the **only** writer of
  `dirty_since = Some(_)`.
- The `dirty_since.get_or_insert_with(...)` line in `App::commit`
  (`src/app.rs:154`) is **removed**. One writer, one rule.
- The five `app.dirty_since = None;` sites in `src/io/file_actions.rs`
  (lines 40, 80, 108, 142, 182) become `app.mark_clean();`. This is mandatory,
  not cosmetic: `action_new` / `action_open` / `action_open_path` replace
  `app.history` with a fresh `History` whose revision is `0`, so clearing
  `dirty_since` without resyncing the revision would re-dirty the document on
  the very next frame.
- `AUTOSAVE_DEBOUNCE` becomes `Duration::from_millis(800)` (parity with the original app).
- Extract `pub fn autosave_due(dirty_since: Option<Instant>, now: Instant) -> bool`
  so the debounce is unit-testable without sleeping and without touching disk.

**Why this and not "route every tool commit through `App::commit`".**

Option 2 does not weaken the AGENTS.md "all entity mutation goes through
`Command` + the history stack" contract — it is the first thing that *cashes it
in*. Because `History` is provably the sole mutation gateway, a counter on
`History` is by construction a complete and sound dirty signal. There is nowhere
else for a mutation to come from.

Option 1 fails on four counts:

1. It would have to hand `&mut App` back to `Tool::on_pointer_down` /
   `on_pointer_up`, reversing LCV-041's deliberate narrowing (documented in
   `src/tools/tool.rs:1-8`) and re-coupling every tool file to `App`.
2. ~20 commit sites across `src/tools/*` change, versus 1 new call site.
3. It does not cover `src/agent/tools.rs` (5 commit sites), which holds only
   `&mut Document` + `&mut History` and no `App`.
4. It does not cover **undo and redo at all** — both change the document away
   from what is on disk while committing nothing. A revision counter catches
   them for free.

The statusbar dirty indicator is explicitly **out of scope** for Marco 0.

## Consequences

**Easier:**

- Every input defect in Marco 0 (D4, D5, D8) becomes expressible as a test that
  fails today and passes after the fix — the regression gate the May drive
  lacked. No new dependency, no egui upgrade.
- The keyboard contract is one table in one file. A reviewer can diff a demand
  against the gate table above instead of grepping two files.
- Autosave becomes correct for all mutation sources at once, including undo/redo
  and the agent, with a single touch point.
- A future `egui_kittest` adoption (egui >= 0.30) reuses `update_ui` unchanged;
  only `tests/harness/mod.rs` is rewritten.

**Harder / costs:**

- Headless tests assert on `App` state, not pixels. Painting regressions stay
  uncovered. Accepted — visual verification remains manual for Marco 0.
- Pointer-driven tests carry a warm-up frame that looks like boilerplate but is
  load-bearing. `tests/harness/mod.rs` must document it.
- `App` grows one field (`last_synced_revision`) whose invariant is invisible
  from its type. Mitigated by making `mark_clean()` the only reset path.

**Committed to:**

- `App::update_ui(&mut self, ctx: &egui::Context)` is public API. `eframe::App::update`
  contains nothing but the delegation.
- `App::default()` is the test constructor; `App::new()` is boot-only.
- `src/ui/shortcuts.rs` is the single keyboard dispatch site.
- `History::revision()` is the document-dirty signal.

## The 300-LOC cap and `src/app.rs`

`AGENTS.md` §"Module tree" caps files at 300 LOC. Twelve files already exceed
300 *total* lines and passed review, so the cap is being applied — correctly —
to **implementation LOC**: an inline `#[cfg(test)] mod tests` block does not
count against it. This ADR states that reading explicitly so it stops drifting.

By that measure `src/app.rs` is 482 implementation lines (tests start at 483)
and is over the cap on its own. **LCV-105 splits it.** The `update_ui`
extraction in decision A must not make that split harder, so the two are
designed together: `update_ui` is a short orchestrator that calls one function
per phase, and each phase moves to its own file.

Target shape — `src/app.rs` becomes `src/app/mod.rs`, matching the repo's
existing directory-module convention (`src/document/mod.rs`,
`src/tools/select/mod.rs`):

| file | contents |
|---|---|
| `src/app/mod.rs` | module docs, `struct App`, `impl Default`, `impl App` (`new`, `default`, `commit`, `action_*`, `mark_clean`, `sync_dirty`), `update_ui` as a ~20-line orchestrator, `impl eframe::App`, and the re-exports |
| `src/app/input.rs` | the `wants_keyboard_input` gate and all key/text routing into `ToolManager` — owner of the D5 and D8 fixes |
| `src/app/viewport.rs` | the `CentralPanel` body: camera sync, painter setup, render calls, pointer events, wheel zoom, middle-drag pan, plus `handle_wheel_zoom` / `handle_pan` / `handle_zoom_extents` |
| `src/app/panels.rs` | menubar, statusbar, command line, toolbar, agent panel, and the three dialogs |
| `src/app/autosave.rs` | `AUTOSAVE_DEBOUNCE`, `autosave_due`, the flush check |
| `src/app/ortho.rs`, `snap.rs`, `agent_poll.rs` | unchanged |

Two constraints on that split:

1. `src/app/mod.rs` must re-export `suppress_snap_if_disabled`,
   `handle_wheel_zoom`, `handle_pan`, `handle_zoom_extents`, `apply_ortho`,
   `resolve_snap` and `poll_agent_rx` so the paths `lasercad::app::*` used by
   `tests/lcv070.rs` keep resolving. Per AGENTS.md, `mod.rs` re-exports the
   module's public surface; deep-path imports from outside stay forbidden.
2. `src/app/tests.rs` (133 lines, added by the LCV-053 merge `d8d7a01`) is an
   **orphan** — no `mod tests;` declares it, so it has never compiled. It
   duplicates the inline test module in `src/app.rs`. LCV-105 either wires it up
   as the extracted test module (the `src/geometry/snap/mod.rs:188` pattern) or
   deletes it. It must not survive the split in its current state.

## Alternatives considered

- **`egui_kittest`** — the purpose-built harness for exactly this. Requires
  egui >= 0.30; upgrading is out of scope for Marco 0. Revisit after v0.1.0.
- **Drive `eframe::run_native` in a test** — needs a display server, is not
  deterministic, and cannot be gated in CI.
- **Move all input handling into a pure `fn handle_input(&mut self, input: &InputSnapshot)`
  and test that instead of `update_ui`** — tests the dispatch table again
  without proving it is reachable. That is precisely the blind spot that shipped
  the ten defects.
- **Sample `history.len()` instead of a revision counter** — `len()` is not
  monotonic and is ambiguous across an undo-then-commit in the same frame
  (3 → 2 → 3). A monotonic counter has no such case.
- **Put the revision counter on `Document`** — wrong owner: `action_open`
  replaces the whole `Document`, so the counter would reset exactly when the
  dirty state matters most.
- **Dirty-flag via `Document` hashing / `PartialEq` against a snapshot** —
  O(entities) per frame and allocates a full clone. Anti-KISS for a `u64`
  comparison.
- **Keep the 5 s debounce** — rejected: the original app used 800 ms and the demand
  requires it.

## Revisit criteria

- egui reaches >= 0.30 in this repo — replace `tests/harness/mod.rs` with
  `egui_kittest`, keep `update_ui` and every assertion.
- A demand needs `&mut eframe::Frame` inside the frame body — narrow it to a
  value parameter of `update_ui`; do not move logic back into `eframe::App::update`.
- A mutation path appears that does not go through `History` — that breaks the
  AGENTS.md contract first and the dirty signal second. Fix the mutation path,
  not the counter.
