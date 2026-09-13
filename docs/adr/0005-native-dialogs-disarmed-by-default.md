# ADR 0005 — Native file dialogs are disarmed outside the app binary

- **Status**: Accepted
- **Date**: 2026-09-13
- **Deciders**: architect

## Context

ADR 0002 §A4 rule 1 and ADR 0003 §F3 trap 1 both say the same thing: a headless
test must never send `Ctrl+O`, `Ctrl+S` or `Ctrl+Shift+S`, because those reach
`src/io/file_actions.rs`, open a blocking native `rfd` dialog, and hang CI with
no output until the job times out.

That ban covers the **input** vector. It does not cover the **logic** vector,
and the LCV-113 review demonstrated the gap for real. While mutation-testing
`App::has_unsaved_changes`, the reviewer inverted it so that a `saved_revision`
of `None` reported the document **clean**. An existing, correct test then fell
through `App::request_open`'s guard into the real `App::action_open`, reached
`rfd::FileDialog::pick_file()`, and blocked. The test binary had to be killed by
hand. In the reviewer's words: "a regression here fails as a **hang**, not a
clean assertion, which is a nastier CI failure mode."

So today's guarantee is *"no test sends a key that reaches `rfd`"*, which holds
and was audited. The guarantee we do not have is *"no test can reach `rfd` at
all, whatever the logic does."* The gap is structural, not a test bug: any guard
predicate anywhere upstream of a dialog — `has_unsaved_changes` today, a
bed-size check or an export-preset check tomorrow — can silently convert a loud
assertion failure into a multi-minute CI timeout with no diagnostic output, on
all three platforms at once. The blast radius grows with every demand that adds
a guard, and LCV-114 / LCV-115 both add guards upstream of file actions.

`src/app/file_ops.rs` already documents the hazard in its module header and
routes around it by construction. That is discipline, and discipline is exactly
what the mutation test defeated.

Constraints that rule out the obvious fixes:

- `rfd` is real and the app genuinely needs native dialogs. Removing or
  feature-gating it out of the build is not on the table.
- The project does not depend on `tokio` and uses `reqwest::blocking`
  (`AGENTS.md` §Event flow). A timeout-on-a-future is not available and would
  not be welcome.
- `egui` is pinned at 0.29.1 by the lockfile; `egui_kittest` needs ≥ 0.30. Out
  of scope.

### The finding that decides the shape

**`#[cfg(test)]` cannot express this guard.** `cfg(test)` is set only for the
crate currently being compiled in test mode. When `tests/lcv113.rs` links
against `lasercad`, the library is compiled as an ordinary dependency with
`cfg(test)` **off**. A `#[cfg(test)]` arm inside `src/io/dialogs.rs` would
therefore be absent from precisely the binary that hung. The guard must be a
runtime value, not a compile-time one.

## Decision

**Native file dialogs are disarmed by default. The app binary arms them once at
boot. A disarmed call panics; it never blocks and never silently returns
`None`.**

The guard lives at the `rfd` boundary — `src/io/dialogs.rs`, whose own module
header already declares it "the sole consumer of `rfd` in the codebase" — and
nowhere else. That placement is load-bearing: it covers a test that reaches a
dialog through `App`, through `crate::io::action_open` directly, through the
agent, or through any path invented later. A guard on `App` would cover only
the first.

Shape for `implementer-rust` (this ADR does not ship code):

1. `src/io/dialogs.rs` gains a private
   `static NATIVE_DIALOGS_ARMED: AtomicBool = AtomicBool::new(false);` and one
   public arming function, e.g. `pub fn arm_native_dialogs()`, storing `true`
   with `Ordering::Relaxed`. Relaxed is correct: the flag publishes no other
   data, and a stale read can only produce an extra panic, never a hang.
2. Each of `open_file_dialog`, `save_file_dialog` and `pick_folder_dialog`
   begins with the same guard, **before** touching `rfd::FileDialog`. On a
   disarmed call it panics with a message that names the function, says the
   process is not the app binary, and cites this ADR — something a CI log
   reader can act on without a backtrace.
3. `crate::run()` (`src/lib.rs:40`) calls `arm_native_dialogs()` as its **first
   statement**, before `eframe::run_native`. `run()` is the single boot path and
   is called only from `src/main.rs`. Nothing else in the tree may call it.
4. There is **no disarm**, and tests never arm. A test that legitimately wants
   to exercise a dialog-returning path passes a `PathBuf` to
   `action_open_path` / `action_save` as it does today.

**Panic, not `None`.** Returning `None` would read as "the user cancelled",
which is a *valid* outcome of every one of these functions: the inverted-guard
regression would then have produced a green test against a wrong code path.
A panic is an ordinary test failure — libtest catches it, names the test, and
the run continues — which is the whole point of the change.

**Tests the demand must carry** (three, all cheap):

- Inline in `src/io/dialogs.rs`: the flag is `false` under `cargo test`, and
  `#[should_panic]` on one wrapper. These run in the unit-test binary, where
  `cfg(test)` *is* on.
- In `tests/`: a `#[should_panic]` calling `lasercad::io::open_file_dialog()`
  directly. This is the one that matters — it proves the backstop works in the
  integration-test compilation mode, the mode where `cfg(test)` is off and
  where the LCV-113 hang actually happened.
- A source-level assertion that `src/lib.rs` still contains the arming call,
  in the style of the existing `file_ops_does_not_import_eframe_or_rfd` test
  (`src/app/file_ops.rs:604`). This closes the one residual hazard — boot
  forgetting to arm — with a test instead of a convention.

**The existing bans stay.** ADR 0002 §A4 rule 1 and ADR 0003 §F3 trap 1 are
**not** superseded. They remain the first line of defence, and they are still
the correct thing to tell a test author. This ADR adds a backstop underneath
them for the case discipline cannot see: a test that does not send those keys
and reaches a dialog anyway.

**Scheduling.** This is **not** a Marco 1 exit criterion. It fixes no
user-visible behaviour and unblocks no queued demand, so gating Marco 1 on it
would be wrong. It should be its own S-sized demand, scheduled **before
LCV-114**: LCV-114 and LCV-115 both add state that guards upstream of the file
actions read, which is the exact predicate class that produced the hang, and
the change is ~40 lines including its three tests.

## Consequences

**Easier**

- A guard-predicate regression anywhere upstream of a file dialog now fails as
  a named test with a readable message in seconds, instead of a silent
  multi-minute timeout on three platforms.
- Mutation testing becomes safe to run against `src/app/file_ops.rs` and
  `src/io/file_actions.rs`, which is how this hazard was found and is how the
  next one will be.
- Adding a fourth dialog wrapper inherits the guard by being written in the
  same file; forgetting it is visible in a three-line diff.

**Harder / costs**

- One process-global `AtomicBool`. It is the minimum viable amount of global
  state: write-once, never read for control flow in the app, and it exists
  because the hazard is itself process-global.
- `src/io/dialogs.rs` grows an ordering dependency — `run()` must arm before
  any dialog opens. Mitigated by arming as the first statement of the single
  boot path and by the source-level test above.
- A library consumer outside this repo would have to arm explicitly. There is
  no such consumer; `lasercad` is a binary plus its own tests.

**Committed to**

- `src/io/dialogs.rs` remains the sole consumer of `rfd`. A new `rfd` call
  anywhere else is a review blocker — it would sit outside the guard.
- `crate::run()` is the only arming site.
- Any future blocking-and-native call (a folder picker, a message box, an OS
  print dialog) goes through `src/io/dialogs.rs` and inherits the guard.

## Alternatives considered

- **`#[cfg(test)]` arm inside the wrapper** — does not compile into integration
  test binaries at all, so it would be absent from the exact binary that hung.
  See §Context. This is the option that looks right and is not.
- **Inject the dialog as a closure / trait object on `App`** — the invasive
  option. It adds a field, a type and a substitution ceremony to every call
  site, and it only covers paths that go through `App`; a test calling
  `crate::io::action_open` directly still hangs. Over-engineered *and*
  incomplete. Rejected against `AGENTS.md` §Product Philosophy.
- **Return `None` when disarmed instead of panicking** — indistinguishable from
  "user cancelled", which turns a hang into a false green. Strictly worse than
  the status quo.
- **Keep the audit discipline and document why** — legitimate, and it is what
  ADR 0002 and ADR 0003 chose. Rejected now because the failure mode changed:
  the LCV-113 review showed the discipline can be correct at every test site and
  still hang, since the reaching path is decided by product logic, not by the
  test. The cost of the fix is ~40 lines; the cost of one missed case is a
  timed-out CI matrix with no diagnostic.
- **A watchdog thread that aborts the process after N seconds** — kills the run
  without naming the culprit, needs a timeout tuned per platform, and would fire
  on a slow machine. Worse diagnostics than a panic at the exact call site.
- **Spawn dialogs on a worker thread with a timeout** — the project has no async
  runtime by design, and a leaked native dialog thread in CI is a new problem,
  not a solved one.
- **`rfd`'s async API** — requires an executor. `AGENTS.md` §Event flow pins the
  crate to blocking I/O on plain `std::thread`.
- **Feature-flag `rfd` out of test builds** — `cargo test` builds the same
  feature set as the app by default; making them differ means the tested binary
  is not the shipped one.

## Revisit criteria

- `egui` reaches ≥ 0.30 and `egui_kittest` lands — unchanged; the guard is about
  `rfd`, not about the harness.
- A second boot path appears (a CLI mode, a headless export mode) — it arms only
  if it genuinely needs dialogs, and the source-level test grows a second
  witness.
- The panic proves noisy in a legitimate scenario — that scenario is a design
  bug in the caller. Fix the caller, not the guard.
