# LCV-118 — Disarm native file dialogs outside the app binary

- **Status**: Done
- **Phase**: 11
- **Depends on**: none
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (commits fa158c0, 5e74557). The rework at
  5e74557 was verified by the implementer's own mutation testing — all three
  mandated mutations now fail with named assertion messages — with CI run
  34741796553 green. The re-review was requested twice and did not report
  back; this is not a review sign-off.

## Problem

A regression in any guard predicate upstream of a file action currently fails as
a **CI hang**, not as a test failure. During the LCV-113 review the reviewer
mutated `App::has_unsaved_changes` so that a `saved_revision` of `None` reported
the document *clean*. A correctly written test — one that broke none of the
rules in ADR 0002 §A4 — then fell through `App::request_open`'s guard into the
real `App::action_open`, reached `rfd::FileDialog::pick_file()` in
`src/io/dialogs.rs:22`, and blocked. The test binary had to be killed by hand.
In CI that is a twenty-minute timeout with no diagnostic output, on Linux,
Windows and macOS at once, for a defect that should have printed one assertion
line.

ADR 0002 §A4 rule 1 and ADR 0003 §F3 trap 1 already ban sending `Ctrl+O`,
`Ctrl+S` and `Ctrl+Shift+S` from a headless test. That ban holds, it was
audited, and it stays. But it is a property of every test **site**, and the path
that reaches a dialog is decided by product **logic**, not by the test. Any
predicate anywhere upstream — `has_unsaved_changes` today, a bed-size check or
an export-preset check tomorrow — can silently convert a loud assertion failure
into a silent timeout. LCV-114 and LCV-115 both add state that guards upstream
of the file actions read, which is exactly the predicate class that produced the
hang, so the exposure grows with the next two demands.

This demand makes the dialogs unreachable outside the app binary: the three
`rfd` wrappers are **disarmed** until `crate::run()` arms them, and a disarmed
call panics with a message a CI-log reader can act on. It changes nothing the
operator can see; it changes how the next guard regression fails.

The design is already decided — [ADR 0005](../../adr/0005-native-dialogs-disarmed-by-default.md),
commit `f44eeb3`. This demand does not reopen it; it turns it into acceptance
criteria and tests.

### Why `#[cfg(test)]` cannot express this guard (read this before writing code)

The obvious implementation is a `#[cfg(test)]` arm inside the wrapper. **It does
not work, and it fails silently.** `cfg(test)` is set only for the crate
currently being compiled in test mode. When `tests/lcv113.rs` links against
`lasercad`, the library is compiled as an ordinary dependency with `cfg(test)`
**off** — so a `#[cfg(test)]` guard in `src/io/dialogs.rs` would be absent from
precisely the binary that hung, while looking green in the unit-test binary
where it does compile. The same is true of doc tests. The guard must be a
**runtime** value — a `static AtomicBool` read on every call, present in every
build profile. This is the single decision the implementer is most likely to
"simplify" into a bug; AC 3 and the integration test in `tests/lcv118.rs` exist
to make that impossible.

### Why it panics instead of returning `None`

`None` already means *"the user cancelled"* and is a valid, documented return
from all three wrappers (`src/io/dialogs.rs:16-17, 29, 41`). If the disarmed
path returned `None`, the inverted-guard regression would have produced a
**green** test running down a wrong code path — strictly worse than the hang.
A panic is an ordinary libtest failure: it names the test, prints the message,
and the rest of the run continues.

### Why the guard sits at the `rfd` boundary and not on `App`

`src/io/dialogs.rs` is the sole consumer of `rfd` in the codebase (its own
module header says so; `AGENTS.md` §Implementation Rules repeats it). A guard
there covers a test that reaches a dialog through `App`, through
`crate::io::action_open` directly, through the agent, or through any path
invented later. A guard on `App` would cover only the first, and would cost a
field, a type and a substitution ceremony at every call site.

## Scope

- **`src/io/dialogs.rs`**: a private `static NATIVE_DIALOGS_ARMED: AtomicBool`,
  initialised `false`; a private `fn require_armed(fn_name: &str)` holding the
  single copy of the panic message; a public `fn arm_native_dialogs()`.
- **The three wrappers** `open_file_dialog`, `save_file_dialog`,
  `pick_folder_dialog` each call `require_armed` as their first statement,
  before touching `rfd::FileDialog`. Signatures and behaviour when armed are
  untouched.
- **`src/io/mod.rs`**: `arm_native_dialogs` added to the existing
  `pub use dialogs::{…}` re-export (one line — `AGENTS.md` §Module tree forbids
  deep-path imports from outside the module).
- **`src/lib.rs`**: `crate::run()` calls `arm_native_dialogs()` as its **first
  statement**, plus an inline source-scan test that it still does.
- **`tests/lcv118.rs`**: the decisive `#[should_panic]` tests, run in the
  compilation mode where `cfg(test)` is off.

## Out of scope

- **No disarm function.** The flag is write-once. A `disarm_native_dialogs()`
  would be an escape hatch whose only user is a test that should not exist.
- **No arming from tests, ever** — not from an inline `#[cfg(test)] mod tests`,
  not from `tests/`, not from a doc example. A test that legitimately wants to
  exercise a file action passes an explicit `PathBuf` to `action_open_path` /
  `action_save`, exactly as today. AC 8 gates this with a source scan, not a
  convention.
- **No change to ADR 0002 §A4 rule 1 or ADR 0003 §F3 trap 1.** They are **not**
  superseded. They remain the first line of defence and are still the correct
  thing to tell a test author. This demand adds a backstop *underneath* them; it
  does not license sending `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` from a headless
  test, and the module headers of `tests/harness/mod.rs`, `tests/lcv103.rs`,
  `tests/lcv113.rs` and `src/app/file_ops.rs` keep saying so, unedited.
- **No `None` return when disarmed** — see §Problem. Rejected in ADR 0005.
- **No dialog abstraction on `App`** (closure, trait object, injected provider).
  Rejected in ADR 0005 §Alternatives as over-engineered *and* incomplete.
- **No watchdog thread, no timeout, no async dialog API, no feature flag.** All
  four rejected in ADR 0005 §Alternatives; the project has no async runtime by
  design and `cargo test` must build the same feature set as the app.
- **No behaviour change anywhere the operator can see.** `src/io/file_actions.rs`,
  `src/app/file_ops.rs`, `src/ui/menubar.rs` and `src/ui/shortcuts.rs` are not
  edited by this demand.
- **No fourth dialog wrapper**, no native message box, no print dialog. Future
  ones inherit the guard by living in the same file; that is a later demand.
- **Not a fix to the LCV-113 mutation finding itself** — the `saved_revision` /
  autosave source-scan tests shipped in `1cb4ae8`.

## Acceptance criteria

1. **The flag.** `src/io/dialogs.rs` declares
   `static NATIVE_DIALOGS_ARMED: AtomicBool = AtomicBool::new(false);`. It is
   **private** (no `pub`, not re-exported), and
   `grep -rn "NATIVE_DIALOGS_ARMED" src/ tests/` shows it only inside
   `src/io/dialogs.rs`. It is read with `Ordering::Relaxed` and written with
   `Ordering::Relaxed`; a doc comment on the static states why Relaxed is
   correct — the flag publishes no other data, and a stale read can only produce
   an extra panic, never a hang.

2. **The arming function.** `pub fn arm_native_dialogs()` stores `true` with
   `Ordering::Relaxed` and does nothing else. Its doc comment states that it is
   called exactly once, by `crate::run()`, that no test may call it, and cites
   ADR 0005. **There is no disarm**: `grep -rn "fn disarm" src/` returns no
   matches.

3. **The guard, at the `rfd` boundary, at runtime.** Each of
   `open_file_dialog()`, `save_file_dialog(default_name: &str)` and
   `pick_folder_dialog()` calls `require_armed("<its own name>")` as its **first
   statement**, before the first `rfd::FileDialog::new()`. The guard is a
   runtime flag read, not a `cfg`: `src/io/dialogs.rs` contains exactly one
   `#[cfg(...)]` attribute, the bare `#[cfg(test)]` at column 0 on the bottom
   `mod tests` block, and no `#[cfg(test)]`, `#[cfg(not(test))]` or
   `cfg!(test)` anywhere in the implementation section. The three public
   signatures are unchanged — the existing
   `src/io/dialogs.rs::dialog_fn_signatures` and
   `src/io/mod.rs::dialogs_reexported_from_io` tests still compile and pass with
   **no edit**.

4. **The panic message.** `require_armed` holds the single copy of the message.
   With `fn_name = "open_file_dialog"` it reads exactly:

   > ``native file dialog `open_file_dialog` was called while dialogs are disarmed — this process is not the LaserCAD app binary. Only `crate::run()` arms them (`arm_native_dialogs`, src/io/dialogs.rs); tests never do. A code path under test reached a real OS dialog, which would block the process forever with no output; the usual cause is an inverted or missing guard predicate upstream of a file action. Fix that caller — do not arm dialogs from a test. See docs/adr/0005-native-dialogs-disarmed-by-default.md``

   It must contain, verbatim and testably: the wrapper's own name, the word
   `disarmed`, the phrase `not the LaserCAD app binary`, the phrase
   `do not arm dialogs from a test`, and the path
   `docs/adr/0005-native-dialogs-disarmed-by-default.md`. No backtrace is
   required to act on it.

5. **Panic, never `None`.** The disarmed path panics; it does not return. A doc
   comment on `require_armed` records why (`None` means "the user cancelled" and
   is a valid return from all three wrappers, so returning it would turn a hang
   into a false green — ADR 0005 §Decision).

6. **`crate::run()` arms, as its first statement.** In `src/lib.rs`, `pub fn run()`
   begins with `crate::io::arm_native_dialogs();`, before the `NativeOptions`
   value is built and before `eframe::run_native`. `run()` remains the single
   boot path, called only from `src/main.rs:4`. The only occurrences of
   `arm_native_dialogs` in the tree are: the definition and its inline tests in
   `src/io/dialogs.rs`, the re-export in `src/io/mod.rs`, and the call plus its
   source-scan test in `src/lib.rs`.

7. **The re-export.** `src/io/mod.rs:15` becomes
   `pub use dialogs::{arm_native_dialogs, open_file_dialog, pick_folder_dialog, save_file_dialog};`
   and that line is the file's only change. `src/lib.rs` calls it through
   `crate::io::`, not through `crate::io::dialogs::`.

8. **No test arms, and nothing under `tests/` can.** No `#[cfg(test)] mod tests`
   block in `src/` calls `arm_native_dialogs`, and no file under `tests/`
   contains the call. This is enforced by a test (AC 8's scan in §Expected
   tests), not by a grep in a PR description — LCV-113's review proved a
   hand-run grep is not a gate.

9. **No behaviour change in the shipped application.** Once armed, all three
   wrappers behave byte-for-byte as they do today: the same two filters
   (`"SVG files"` / `["svg"]`, then `"All files"` / `["*"]`) in the same order on
   the open and save dialogs, the same `set_file_name(default_name)` prefill, the
   same `pick_file()` / `save_file()` / `pick_folder()` calls, the same
   `Option<PathBuf>` returns including `None` on cancel. No call site changes:
   `git diff --stat` for this demand touches only `src/io/dialogs.rs`,
   `src/io/mod.rs`, `src/lib.rs` and `tests/lcv118.rs`.

10. **No existing test is edited.** Every test that exists at the demand's base
    commit still exists, unmodified, and passes. The library and integration
    test counts only grow.

11. **Caps, purity, dependencies.** `src/io/dialogs.rs` stays ≤ 300
    *implementation* lines, measured with the ADR 0004 `awk` recipe
    (`awk '/^#\[cfg\(test\)\]/{print NR-1; f=1; exit} END{if(!f) print NR}'`);
    it is **45** today and this demand adds roughly 20, so no split is needed
    and none is invited. `src/lib.rs` (54 today) likewise. `src/io/dialogs.rs`
    keeps importing only `rfd` and `std` (now also `std::sync::atomic`) — it is
    the `rfd` wrapper and is one of the few files *allowed* to import `rfd`; the
    kernel purity rule (`AGENTS.md` §Purity rule) is unaffected and no kernel
    module is touched. **No new dependency**: `AtomicBool` is `std`.

12. **The deliberate `panic!` is not an `unwrap`/`expect` violation.** This
    demand adds no `unwrap()` and no `expect()` to library code. The single
    `panic!` inside `require_armed` **is** the mechanism of the demand; it
    carries an inline comment naming LCV-118 and ADR 0005. A reviewer must not
    flag it under `AGENTS.md` §Implementation Rules — the rule bans silent
    unwraps, and this is a documented, tested, deliberate abort with a
    diagnostic message.

13. **Build gate.** `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and `cargo test --all` all
    exit 0.

## Expected tests

Three test sites, per ADR 0005. **The middle one is the decisive one**: it is
the only one that runs in the compilation mode where `cfg(test)` is off, which
is the mode in which the LCV-113 hang actually happened. Without it this demand
does not do its job.

- **Unit, inline in `src/io/dialogs.rs`** (AC 1, 2, 3, 4, 5). These run in the
  library unit-test binary, where `cfg(test)` *is* on:
  - `dialogs_are_disarmed_by_default` (AC 1) — asserts
    `!NATIVE_DIALOGS_ARMED.load(Ordering::Relaxed)`. Doc comment: no test in
    this crate may arm, so this must hold no matter which tests ran first.
  - `open_file_dialog_panics_when_disarmed` — `#[should_panic(expected = "open_file_dialog")]`.
  - `save_file_dialog_panics_when_disarmed` — `#[should_panic(expected = "save_file_dialog")]`,
    called with `""`.
  - `pick_folder_dialog_panics_when_disarmed` — `#[should_panic(expected = "pick_folder_dialog")]`.
  - `disarmed_panic_cites_adr_0005` (AC 4) —
    `#[should_panic(expected = "docs/adr/0005-native-dialogs-disarmed-by-default.md")]`
    on `open_file_dialog()`. A second `should_panic` on the same wrapper is the
    cheapest way to assert a second substring of one message.
  - `no_disarm_function_exists` (AC 2) — source scan over
    `include_str!("dialogs.rs")` asserting it contains no `fn disarm`.
  - `guard_is_runtime_not_cfg` (AC 3) — source scan over
    `include_str!("dialogs.rs")`: the implementation section (everything before
    the bare `#[cfg(test)]` at column 0) contains no `cfg(` at all. This is the
    test that stops the `#[cfg(test)]` "simplification" from being reintroduced
    six months from now.
  - The existing `dialog_fn_signatures` stays byte-identical (AC 3, AC 10).

- **Integration, `tests/lcv118.rs`** (AC 3, 4, 8 — **decisive**). No
  `mod harness;` — these tests need no `egui::Context` and no frame. The module
  header must state, in plain words, that this file exists because `cfg(test)`
  is **off** when an integration binary links `lasercad`, and that if these
  tests ever *hang* instead of failing, the guard has been removed:
  - `open_file_dialog_panics_in_an_integration_binary` —
    `#[should_panic(expected = "disarmed")]` on
    `lasercad::io::open_file_dialog()`. This is the test the demand is for.
  - `save_file_dialog_panics_in_an_integration_binary` —
    `#[should_panic(expected = "disarmed")]` on
    `lasercad::io::save_file_dialog("untitled.svg")`. `Ctrl+S` /
    `Ctrl+Shift+S` are half of ADR 0002 §A4 rule 1, so the save vector gets its
    own witness.
  - `no_integration_test_arms_native_dialogs` (AC 8) — reads every `*.rs` under
    `concat!(env!("CARGO_MANIFEST_DIR"), "/tests")` and its `harness/`
    subdirectory and asserts none contains the arming call. **Build the needle
    with `concat!("arm_native_", "dialogs")`** so this file does not match
    itself; without that split the test fails on its own source and the
    implementer loses twenty minutes to it.

- **Unit, inline in `src/lib.rs`** (AC 6) —
  `run_arms_native_dialogs_as_its_first_statement`, in the style of
  `file_ops_does_not_import_eframe_or_rfd` (`src/app/file_ops.rs:604`). Over
  `include_str!("lib.rs")`: find the byte offset of `"pub fn run()"`, then of
  `"arm_native_dialogs()"` **after** it, then of `"eframe::run_native"` after
  that, and assert both orderings. Searching from the `pub fn run()` offset is
  what keeps a doc-comment mention from passing the test. This turns "boot
  forgets to arm" from a convention into a test, and it is the only automated
  cover for the one residual hazard of the design.

- **Static checks (AC 7, 9, 11)** — `src/io/mod.rs` diff is the single
  `pub use` line; `git diff --stat` names exactly four files; the ADR 0004
  `awk` count on `src/io/dialogs.rs` and `src/lib.rs`;
  `grep -nE '^use ' src/io/dialogs.rs` shows only `rfd`/`std`; `Cargo.toml` is
  unchanged.

- **Build gate (AC 13)** — the three cargo commands.

- **[manual] smoke (AC 9, and the one thing no headless test can prove).**
  `cargo run`; **File > Open…** → the native dialog opens (it does not panic),
  shows the SVG filter, and Cancel leaves the document untouched. Draw one line;
  **Ctrl+S** → the native save dialog opens with the filename prefilled; save to
  `/tmp/lcv118.svg`; **File > Open…** → reopen it and confirm the line is there.
  If `run()` failed to arm, the very first of these steps would abort the app
  instead of opening a dialog — which is exactly the diagnostic we want, and
  exactly why AC 6 is tested at the source level too.

## Risks

- **A missing guard makes `tests/lcv118.rs` hang, not fail.** Unavoidable and
  acceptable: a hang in `lcv118` *is* the diagnosis, the file is two tests long,
  and it is the only place in the suite where that is true. The module header
  says so.
- **`[profile.release] panic = "abort"`.** Verified on 2026-09-13 in a scratch
  crate: cargo forces unwinding for test harness targets, so `#[should_panic]`
  passes under `cargo test --release` too. The shipped release binary does abort
  on panic — but it arms at boot, so the guard cannot fire there.
- **Doc tests are a third compilation mode.** A doc test links `lasercad` as a
  dependency with `cfg(test)` off, so a runnable `///` example that calls a
  wrapper would panic, and one that calls `arm_native_dialogs()` and then a
  wrapper would hang the doc-test run. Write the new doc comments as prose; add
  no runnable example to `src/io/dialogs.rs`.
- **`arm_native_dialogs` is public API.** `lasercad` is a binary plus its own
  tests and has no external consumer, so the cost is nil today. If a second boot
  path ever appears (a CLI mode, a headless export mode) it arms only if it
  genuinely needs dialogs, and the AC 6 scan grows a second witness — ADR 0005
  §Revisit criteria.
- **One process-global `AtomicBool`.** Accepted in ADR 0005 §Consequences: it is
  write-once, never read for control flow in the app, and it exists because the
  hazard is itself process-global.
- **Merge overlap: none expected.** LCV-114 and LCV-115 edit
  `src/io/file_actions.rs`, `src/ui/menubar.rs` and the save actions; this
  demand edits none of them. `src/lib.rs` and `src/io/dialogs.rs` are otherwise
  quiet files.

## Open questions

*(none — demand is Ready)*

## Notes

- **Decided by [ADR 0005](../../adr/0005-native-dialogs-disarmed-by-default.md)**
  (`f44eeb3`). The implementer does not choose the shape here; ADR 0005
  §Decision is the specification and §Alternatives records the six options
  already rejected. If something in it looks wrong, route to `architect` —
  do not improvise.
- **Origin**: the LCV-113 review (`.claude/backlog.json`, LCV-113 note). The
  mutation that produced the hang was inverting `App::has_unsaved_changes` for
  `saved_revision == None`. LCV-113 shipped in `475d666` + `a7df030`; the
  source-scan tests its review demanded shipped in `1cb4ae8`.
- **Scheduling**: not a Marco 1 exit criterion — it fixes no user-visible
  behaviour and unblocks no queued demand. It is scheduled **before LCV-114**
  because LCV-114 and LCV-115 both add state that guards upstream of the file
  actions read (ADR 0005 §Scheduling).
- **Verified against the tree at `0139bd4`** (do not re-derive these; they were
  checked, not copied):
  - `src/io/dialogs.rs` exists, is 45 implementation lines, and holds exactly
    the three wrappers: `open_file_dialog() -> Option<PathBuf>` (line 18),
    `save_file_dialog(default_name: &str) -> Option<PathBuf>` (line 30),
    `pick_folder_dialog() -> Option<PathBuf>` (line 42).
  - `pub fn run() -> eframe::Result<()>` is at `src/lib.rs:40`; its only caller
    in the tree is `src/main.rs:4`.
  - The style reference for the source-scan test,
    `file_ops_does_not_import_eframe_or_rfd`, is at `src/app/file_ops.rs:604`.
  - The wrappers' only callers are `src/io/file_actions.rs:54`
    (`open_file_dialog`) and `:168` (`save_file_dialog`); `pick_folder_dialog`
    has **no** caller outside its own tests and is guarded anyway, because the
    guard belongs to the boundary rather than to the current call graph.
  - `Cargo.toml` declares one binary target (auto-discovered `src/main.rs`), no
    `[[example]]`, no `[[bench]]`, no second `[[bin]]`. There is no `examples/`
    or `benches/` directory. The agent tool registry (`src/agent/tools.rs`)
    exposes only `create_line`, `create_circle`, `create_arc`, `delete_entity`
    and `move_entity` — no file I/O, no dialog. `scripts/` holds packaging shell
    scripts that never run the binary headless. **`crate::run()` is therefore
    the only site that needs to arm.**
  - No test anywhere calls `action_open` / `action_save` / `action_save_as` or
    any wrapper, so no existing test changes behaviour when the guard lands.
- **CHANGELOG**: `product-owner` recommends **no `[Unreleased]` entry**. Nothing
  the operator can do or see changes (`AGENTS.md` §Documentation Hygiene ties
  the CHANGELOG to user-visible behaviour). `demand-manager` decides; if an
  entry is written anyway, it belongs under `### Changed` as a developer-facing
  line, e.g. *"Native file dialogs are now disarmed outside the app binary: a
  test that reaches one panics with a named message instead of hanging CI. No
  change to the application's behaviour. See LCV-118, ADR 0005."*
- `AGENTS.md` §Implementation Rules already carries the forward reference:
  *"Per ADR 0005 the three wrappers are to be disarmed until `crate::run()` arms
  them … that guard is decided but not yet implemented (own demand, ahead of
  LCV-114)."* When this ships, that sentence needs its tense fixed — a
  documentation edit for `demand-manager`, not for the implementer.
