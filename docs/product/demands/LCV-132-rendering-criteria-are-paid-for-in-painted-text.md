# LCV-132 — A rendering criterion is paid for in painted text, not in a source scan

- **Status**: Done
- **Phase**: 12
- **Depends on**: LCV-125, LCV-126, LCV-129, LCV-133 — all `Done`. Each landed one of the copies this demand absorbs; there is nothing speculative left to extract against. **Ordering, not a dependency**: land this **after LCV-134**. See §Notes.
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 1b89f0e, 4069efc, 8495a2a, dbca0a3

## Problem

Every UI demand in this repo was written against a premise that is half wrong,
and the half that is wrong is the half that matters.

The premise: egui rendering cannot be asserted at the pinned 0.29.1, because
`egui_kittest` needs ≥ 0.30. Therefore a rendering acceptance criterion gets a
**bounded source scan** — "the panel's implementation section contains the
needle `TextStyle::Monospace`" — plus a headless frame proving the path does not
panic. That is what LCV-116, LCV-121, LCV-122 and LCV-125 all did.

The true half is that **pixels** cannot be asserted. The false half is the
conclusion. **Painted text runs can be asserted, today, at 0.29.1, with no new
dependency and no version bump.** `egui::Context::run` returns
`FullOutput { shapes, .. }`; each `ClippedShape` carries a `Shape`; and
`Shape::Text(TextShape { pos, galley, .. })` yields the exact string that was
laid out and where it was put.

### What the gap costs, measured

`reviewer-rust` wrote the probe during the LCV-125 review, ran it against the
shipped code, and used it to kill **five mutants that survived the full gate** —
`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
all 1208 tests green:

| mutation applied to the shipped code | breaks | caught by the gate |
|---|---|---|
| wrap the transcript loop in `ui.horizontal_wrapped` — every row runs into one paragraph | LCV-125 AC 2, AC 5 | **no** |
| iterate the transcript `.rev()` — newest row first | LCV-125 AC 5 | **no** |
| `if role == "note" { continue; }` — the undo-shape note never reaches the screen | LCV-125 AC 4 | **no** |
| guard the plaintext-key warning on a non-empty key — invisible on exactly the frame an operator is about to paste a key in | LCV-125 AC 10 | **no** |
| the same guard on the step-budget explanation | LCV-125 AC 9 | **no** |

Five stated acceptance criteria, five silent breakages. The reason is one
sentence and it belongs in every future UI demand: **a source scan asserts that
a call is written; nothing asserted that the paint happened.** Those are
different claims. The gap between them is where all five live, and it is also
where the fourth one lives — a warning that vanishes at the moment it is needed
is a security-relevant regression that the scan for its sentence cannot see,
because the sentence is still right there in the source.

That argument won. Four demands since have paid for their rendering criteria in
painted text: LCV-125, LCV-126, LCV-129 and LCV-133. **Each of them paid by
copy-pasting the collector**, because this demand was still `Draft` when they
ran, and each said so in its own module header. That is the debt now due.

### The census, measured at `2860edd`

**Four private copies of the paint collector**, in
`tests/lcv125_agent_panel_and_settings.rs` (the original),
`tests/lcv126_command_line_group.rs`, `tests/lcv129_agent_timeout_and_cancel.rs`
and `tests/lcv133_shortcuts_dialog_fits.rs`. The blocks run 89 to 130 lines
each — roughly **440 lines of test infrastructure with no home**.

Three of the divergences between them are **load-bearing and were disclosed and
reviewed**, not drift, and a consolidation that erases them breaks the suite:

- **LCV-133's `Run` carries untruncated `pos: egui::Pos2` and `height: f32`**
  where the other three carry rounded `y: i32, x: i32` and **no height at all**.
  LCV-133 AC 2's containment arithmetic is
  `pos.y >= clip.top() && pos.y + galley.size().y <= clip.bottom()`, which
  cannot be computed from the rounded shape — the `height` term is simply not
  there — and LCV-126's line grouping never wanted it. `reviewer-rust` ruled
  that divergence compliant and properly disclosed. Measured, the margin it
  turns on is thin: at LCV-133's 1280×460 pin the `F1` run straddles the clip
  bottom by **0.682pt** (`pos.y = 446.682`, `height = 14.0`, clip bottom
  `460.000`) — **less than one whole-point rounding step**, so whether a
  rounded collector still sees that straddle is decided by which way the round
  falls rather than by the geometry. And the containment term cannot be
  reconstructed at all from a shape that never stored a height. **A shared
  collector that quietly rounds would silently break the one assertion class in
  this repo that catches a sliced row.**
- **LCV-126 buckets by column before it groups by `y`** (`column_split_x`),
  because `ui.columns` clones the parent painter without narrowing its clip, so
  both of LCV-133's columns share one clip rect and grouping by `y` alone braids
  them together.
- **LCV-133 has no line grouping and no `SAME_LINE` at all.** It has
  `dialog_body`, which scopes by the `Command line` marker and hands back the
  clip rect as well as the runs.

One divergence is **drift**: LCV-125 drops runs on `text.is_empty()`, where the
other three drop on `text.trim().is_empty()`. Same intent, different reach — a
whitespace-only run survives in one file and dies in three.

**Five private copies of the source-scan walker**, in
`tests/lcv121_source_scans.rs`, `tests/lcv122_source_scans.rs`,
`tests/lcv125_agent_panel_and_settings.rs`,
`tests/lcv128_normative_enumerations.rs` and
`tests/lcv129_agent_timeout_and_cancel.rs`. LCV-128's §Notes named a fourth copy
as the trigger for this consolidation; the fifth is in LCV-125, and it is the
same ten lines. `reviewer-rust` gave an independent read of the first four that
this demand adopts rather than re-derives, and it holds for the fifth:

- **`rs_files(dir, &mut out)` is byte-identical in all five** apart from module
  docs — confirmed by hashing the extracted function body: one digest, five
  files. Pure duplication, trivially shareable.
- **The walk-and-slice wrappers differ in ways that matter.** LCV-121's
  `implementation_sections` is single-root, bounded, tuple-returning; LCV-122's
  `sections(dir, bound, minimum)` is parametrised over bounded-versus-whole-file;
  LCV-125's `agent_sections` is `src/agent/`-only, bounded, and strips comment
  lines while it *builds* each section; LCV-128's `kernel_sections` is
  unbounded-only with **five separate per-directory positive controls** rather
  than one, so a typo'd directory name fails naming that directory; LCV-129's
  `implementation_sections` is single-root, bounded, vec-returning.
- **The matcher differs too.** `files_containing` returns presence as
  `Vec<String>`; LCV-129's `occurrences` returns counts as `Vec<(String, usize)>`
  and is a **strict superset** of it; LCV-125's `files_naming` is a third shape
  that matches with a plain `contains` because its sections arrive
  comment-stripped.

### The trap this demand is named after

Every criterion below is at risk of being written as a source scan asserting
that a helper is shared — `tests/harness/` contains `fn collect_text`, no test
file contains a second one. **That scan would pass on a helper that is shared
and broken**, which is the exact failure mode the title names. So the
acceptance here is behavioural: the mutations those seven files catch **today**
must still be caught after the move, run and reverted, with the verbatim
failure messages in the handover.

## Scope

- Moving the duplicated plumbing into `tests/harness/`, where ADR 0002 §A3 says
  shared plumbing lives:
  - `rs_files` — one definition, five callers.
  - the count-based matcher, with the presence-returning one **derived** from it
    — one comment-skipping rule instead of the four copies of it in the tree.
  - the paint collector — one `Run`, one `collect_text`, one frame driver, one
    `SAME_LINE`, one `lines_on_surface_of` and `texts`.
- Collecting at **full precision** and rounding only at the point of use, so
  both the rounded line-grouping callers and LCV-133's exact containment
  callers are served by one collector and neither can be broken by the other.
- Rewriting every call site in the seven affected files
  (`lcv121`, `lcv122`, `lcv125`, `lcv126`, `lcv128`, `lcv129`, `lcv133`) and
  deleting the private originals — **one of each in the tree**.
- The traps and hygiene rules, as module docs, merged from the four headers that
  carry them today. They are as much the deliverable as the code.
- The documented assertion idiom — a literal table of visual lines — and one
  named alternative for when a test needs to prove two lines are distinct.
- One rule in `AGENTS.md` §Implementation Rules: a rendering acceptance
  criterion is not satisfied by a source scan alone.
- One guard against a `mod harness;` that nobody uses — the cost the blanket
  `#![allow(dead_code)]` imposes, paid for instead of left unstated (§Notes).
- Behavioural proof, by mutation, that nothing got weaker.

## Out of scope

- **Collapsing the five walk-and-slice wrappers into one function.**
  `reviewer-rust` recommended against it and this demand endorses that: each
  wrapper's root, bound, return shape and positive-control shape is a decision
  its own demand made and its own tests depend on. A
  `sections(dir, bound, minimum, per_directory_controls, return_root)` would be
  a worse artefact than the duplication it deletes, and LCV-128's five
  per-directory controls would become a boolean that nobody can read. **Leave
  the per-caller bound and positive-control shape exactly alone.** An
  implementer who "finishes the job" here has damaged the demand.
- **Collapsing LCV-126's column-aware `lines_on_surface_of` into the shared one
  behind a flag**, for the same reason. It may sit beside the shared helper, or
  stay private to its file; it may not become a parameter on it.
- **Converting LCV-125's `agent_sections` / `files_naming` to the shared
  matcher.** Only its `rs_files` moves. The two are equivalent for
  single-line needles, but they strip comments at different points, and
  proving that equivalence across four landed scans buys nothing this demand
  needs.
- **Rounding at collection.** See §Problem: the containment term cannot be
  rebuilt from a shape that stored no height, and the margin the one honest
  straddle turns on is 0.682pt — under a rounding step. Any shared `Run` that
  stores a rounded coordinate instead of deriving one is a rejected design, not
  an implementation detail.
- **Changing what any converted test asserts.** No expectation, literal table or
  message is edited to accommodate the move. If unifying the empty-run filter on
  `trim()` changes any literal table, **stop and report** — that is a finding
  about the shipped UI, not a line to update.
- **Upgrading egui, or adding `egui_kittest`.** The entire point is that this
  works at the pinned **0.29.1**. "Better rendering tests" is not a licence to
  bump a version — a bump re-validates every headless test in the repo and every
  trap recorded in ADR 0002, and that is a separate demand with a separate risk
  budget. AC 11 pins `Cargo.toml` and `Cargo.lock` as unchanged so this cannot
  arrive by the side door.
- **Any change under `src/`.** This demand ships no user-visible change and must
  not. AC 7's mutations are applied and reverted, so `git diff` over `src/` is
  empty when this demand ends.
- **Pixels.** No snapshot images, no image diffing, no golden files, no
  rasterising. The true half of the old premise stays true.
- **Asserting colour, font, text style, size, or any rect.** LCV-125 AC 2 says a
  `tool` row "is not the same colour as assistant prose"; that stays a source
  scan, because a colour assertion is a brittle claim about a theme. This demand
  raises the floor for *text and its order*, not for appearance.
- **Absolute coordinates in an assertion.** No test asserts that anything is at
  `y = 421`. Every positional claim is relative — strictly-increasing, distinct,
  or contained in a rect that came out of the same frame.
- **Retrofitting UI that has no collector today.** The status bar, the toolbar,
  the viewport overlay and the command line keep the tests they have.
  Retrofitting is unbounded; the rule that earns its keep is that the *next* UI
  demand reaches for this. Rewriting the existing call sites is
  de-duplication of the code this demand extracts, not a retrofit.
- **Re-opening LCV-125 AC 6.** A paint-shape test could now assert that the
  dummy key appears in no painted run anywhere on screen, which is stronger than
  what AC 6 asserts. Still out of scope: LCV-125 is closed, and this demand does
  not reopen a criterion that passed.
- **A UI-testing framework.** No widget queries, no click-by-label, no
  accessibility tree, no `find_button("Send")`. Functions that hand back text,
  positions and visual lines; every judgement stays at the call site.
- **Narrowing `#![allow(dead_code)]` to per-item allows.** Rejected — see
  §Notes for the reasoning and for what is done instead.
- **Making the helper reachable from `#[cfg(test)]` modules under `src/`.**
  `tests/harness/` is not compiled into the library. Paint-shape assertions live
  in `tests/`, one file per demand, per ADR 0002 §A3 — a second copy under
  `src/` would be the thing this demand exists to prevent.
- **LCV-134's work.** It adds assertions to the shortcuts dialog and it lands
  first. This demand moves code; it does not add a dialog criterion.

## Acceptance criteria

1. **`rs_files` has exactly one definition, and five callers.** It moves to
   `tests/harness/` unchanged and `tests/lcv121_source_scans.rs`,
   `tests/lcv122_source_scans.rs`, `tests/lcv125_agent_panel_and_settings.rs`,
   `tests/lcv128_normative_enumerations.rs` and
   `tests/lcv129_agent_timeout_and_cancel.rs` call the shared one. No private
   copy survives in any of the five.

2. **The matcher has one definition, it counts, and presence is derived from
   it.** `occurrences(sections, needle) -> Vec<(String, usize)>` moves to the
   harness with its comment-skipping rule and its doc comment intact;
   `files_containing(sections, needle) -> Vec<String>` is **defined in terms of
   `occurrences`**, not beside it, so the rule about which lines count exists
   once. The four callers that use the comment-skipping semantics —
   `tests/lcv121_source_scans.rs`, `tests/lcv122_source_scans.rs`,
   `tests/lcv128_normative_enumerations.rs`,
   `tests/lcv129_agent_timeout_and_cancel.rs` — use the shared pair, and
   `tests/lcv129_agent_timeout_and_cancel.rs`'s count-based assertions keep
   reading counts.

3. **Every walk-and-slice wrapper keeps its own shape.** After the move,
   LCV-121's `implementation_sections` still returns its tuple, LCV-122's
   `sections` still takes `(dir, bound, minimum)`, LCV-128's `kernel_sections`
   still runs **five** per-directory positive controls and still scans whole
   files rather than the bounded slice, LCV-129's `implementation_sections`
   still returns a bare `Vec`, and LCV-125's `agent_sections` still strips
   comments as it builds. None of them gains a parameter. The diff over these
   five functions touches only the line that used to call the private
   `rs_files` and the lines that used to call a private matcher.

4. **The paint collector has one definition and it collects at full
   precision.** `Run` carries `clip: egui::Rect`, `pos: egui::Pos2`,
   `height: f32` and `text: String`. The rounded `y` / `x` that the
   line-grouping callers read are **derived from `pos` at the point of use** —
   an accessor, a local, anything except a stored rounded field. A unit test
   hands `collect_text` a hand-built `Shape::Text` laid out at a deliberately
   fractional position and asserts the `Run` comes back with `pos` **exactly
   equal** to the input, and `height` equal to the galley's `size().y`. This is
   the criterion that protects LCV-133's containment checks, which are today the
   only assertions in the repo that can see a sliced row (§Problem).

5. **Nesting and clip propagation are proven, not assumed.** A test hands
   `collect_text` a hand-built `Shape::Vec` containing a `Shape::Text`, nested
   two deep, and asserts the inner string comes back **and carries the clip rect
   of the `ClippedShape` it was nested inside**. `Shape::Vec` is the only shape
   variant at 0.29.1 that contains other shapes; the doc comment says so, so an
   egui bump has one named place to revisit.

6. **One frame driver, one positive control, one empty-run rule.** The shared
   driver is parameterised on screen size and on the events for the frame, so
   `tests/lcv133_shortcuts_dialog_fits.rs` stops carrying its own `raw_input_at`
   and `tests/lcv126_command_line_group.rs` stops carrying its own
   events-taking variant. It keeps the `runs.len() > 10` positive control: a
   harness helper that asserts is unusual and deliberate — it is what stops a
   test that renders nothing at all from reading as "the string is absent, as
   expected". It drops runs on `text.trim().is_empty()`, which is what three of
   the four files already do; the fourth (`lcv125`) changes to match, **and if
   that changes any literal table in it, stop and report** (§Out of scope).

7. **Every converted file asserts exactly what it asserted before, and the
   mutations prove it.** This is the criterion that decides whether the demand
   worked, and it is behavioural because a scan for a shared helper would pass
   on a shared broken one. Each mutation is applied to the shipped code, the
   suite is run with `cargo test --all --no-fail-fast`, the **failing test name
   and its verbatim assertion message** are recorded, and the mutation is
   reverted:

   | # | mutation | must go red in |
   |---|---|---|
   | a | `ui.horizontal_wrapped` around the transcript loop | LCV-125's ordered line table |
   | b | `.rev()` on the transcript iteration | LCV-125's ordered line table |
   | c | `if role == "note" { continue; }` | LCV-125 AC 4's row |
   | d | the plaintext-key warning guarded on a non-empty key | LCV-125 AC 10 |
   | e | the step-budget explanation guarded the same way | LCV-125 AC 9 |
   | f | delete LCV-126's column bucketing (the `retain` on the column side of `split_x`) | LCV-126 AC 4's ordered table |
   | g | invert the guard that shows the `Cancel` button only during a turn | LCV-129 AC 8 |
   | h | swap the order of the cancel note and the undo-shape note | LCV-129 AC 7 |
   | i | revert the shortcuts dialog to one column (LCV-133's own documented mutation) | LCV-133 AC 1 / AC 2, at all three sizes |
   | j | round `pos` inside the shared `collect_text` | AC 4's precision test — **and nothing else in the suite**, which is the point |
   | k | make the shared `rs_files` non-recursive | AC 1's `ac1_rs_files_returns_every_rs_file_in_the_tree_and_nothing_else`, **and** the whole-`src/` positive controls in `lcv121::implementation_sections`, `lcv129::implementation_sections` and `lcv122::sections` at its `("src", 30)` call site. **Not** `lcv125::agent_sections` and **not** `lcv128::kernel_sections` — measured, see §Notes "Row (k)" |
   | l | paste the old hardcoded model id back into `loop_.rs::send_fn` | LCV-121's scan, by name |
   | m | name `Document` on a code line in a file under `src/agent/` | LCV-122's scan, by name |
   | n | `use eframe::egui;` in a kernel file | LCV-128 AC 1, naming that directory |
   | o | clear the busy flag in the panel's click handler instead of `end_turn` | LCV-129 AC 5, with the **count** in the message |

   Fifteen mutations, fifteen named failures. **A mutation that survives is a
   finding to report, not a criterion to quietly relax** — and (j) and (k) are
   the two that say the consolidation itself did not weaken anything, so a
   survivor there blocks the demand. For (k) the named places are the four
   above and only those: a wrapper outside that list staying green is the
   **measured expectation**, not a finding, and adjusting a wrapper's positive
   control so that it fires is forbidden. §Notes "Row (k)" records why, and
   the same rule holds generally — when a criterion does not fire, report it;
   never tune the test until it does.

8. **A `mod harness;` that nobody uses fails.** A scan over the files directly
   in `tests/`: every file declaring the module on a code line must also name
   `harness::` on a code line. Needles by `concat!`, comment lines skipped
   (a module header naming the harness is not a use), and a positive control
   over a synthetic two-file fixture — one that uses it, one that does not — run
   through the same helper, because an absence assertion over an empty haystack
   passes for the wrong reason. This exists because the blanket
   `#![allow(dead_code)]` already let an unused `mod harness;` sit undetected in
   `tests/lcv133_shortcuts_dialog_fits.rs` until a reviewer read for it, and
   this demand makes that blanket cover more code, not less (§Notes).

9. **The traps and the idiom are written down, in the harness's existing
   voice.** `tests/harness/mod.rs` already opens with numbered hard rules, each
   carrying its *reason* rather than just its rule, and that is why they work.
   The paint module's header carries, in the same shape, the traps the four
   copied headers carry today, merged and de-duplicated:
   - paint order is not reading order;
   - surfaces interleave vertically and membership is clip-rect containment, so
     a `Window` title can **never** be the marker;
   - two columns share one clip rect, so a two-column surface must bucket by
     column before grouping by `y`;
   - runs on one visual row do not share a `y` — the `402`/`401`
     label-and-field case, and why `SAME_LINE` is 6 and not a round number;
   - a binding's monospace column is padded, so runs are trimmed before
     comparison;
   - empty and whitespace-only runs exist and are dropped;
   - the first frame is not settled, so an **absence** assertion must be made on
     a later one;
   - positions are collected untruncated and rounded only at the point of use,
     with the 0.682pt straddle named as the reason.

   The scan module's header carries the two rules that keep a scan able to fail
   (bound at the bare `#[cfg(test)]` at column 0, needles by `concat!`), the
   `components()`-joined path rule, and the comment-skipping trade-off with the
   cost LCV-123's review found. The assertion idiom is recorded too:
   - **a literal table of visual lines** —
     `assert_eq!(texts(&lines_on_surface_of(&runs, marker)), vec![vec!["…"], …])`.
     One assertion covering presence, verbatim text, left-to-right order within
     a line, and top-to-bottom order of lines. `texts` drops the `y`, which is
     what keeps absolute coordinates out of the assertion;
   - **the y-carrying variant**, `lines_on_surface_of` without `texts`, only
     when a test must prove two lines are *distinct* rather than merely present
     and ordered — and then as a strict inequality, never as a value.

   The marker is a string the test injected into its own fixture content
   (LCV-125 uses `LCV125ROW-{role}`) or a shipped label the test has pinned
   elsewhere, and `lines_on_surface_of` asserts it is painted **exactly once**.
   Never a substring: a test controls its fixture text, it does not control the
   product's copy.

10. **The rule is recorded where the next UI demand will hit it.**
    `AGENTS.md` §Implementation Rules gains one entry, in the existing voice: a
    rendering acceptance criterion is not satisfied by a source scan alone — a
    scan asserts that a call is written, not that the paint happened — and
    painted text is assertable at the pinned egui via `tests/harness/`. It cites
    this demand and the five surviving mutants as the witness, the way the
    path-comparison rule cites its two CI breakages. It sits next to the
    existing derived-expected-set rule rather than repeating it.

    Two hand-offs the implementer **opens as tasks and does not action**, both
    `architect`'s file and `architect`'s call: ADR 0002 §A3 enumerates
    `tests/harness/mod.rs` as one file holding "exactly five items", which is
    already stale (eight ship today) and is more so after this demand; and the
    rule in this criterion may belong in ADR 0002 rather than `AGENTS.md`.

11. **No dependency moved, no `src/` diff, and the caps hold.** `git diff` over
    `Cargo.toml` and `Cargo.lock` is **empty** — no new crate, no egui bump, no
    feature flag — and so is `git diff` over `src/`, every mutation in AC 7
    having been reverted. Every file under `tests/harness/` is measured with
    ADR 0004's `awk` recipe, never `wc -l`, and every number is reported:
    `tests/harness/mod.rs` is **124** today and roughly 200 lines are moving in,
    so the shared code lands as more than one module under `tests/harness/`
    (§Notes names the expected division). **No module exceeds 270**; if one
    does, flag `architect` and do not split further on your own initiative
    (ADR 0004 rule 4). The seven converted files each shrink — report the before
    and after for all seven, and the net line delta, which must be strongly
    negative. All three local gates green with the pass count reported:
    `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    `cargo test --all --no-fail-fast`. **CI: not run (billing hold)** — the
    local gate is the acceptance gate.

## Expected tests

New tests live in `tests/lcv132_harness_is_shared.rs`, which is also the one
consumer that exists to prove the harness is reachable from a test binary.

- **Unit / AC 1, AC 2 — the shared scan helpers do what the private ones did.**
  `rs_files` over a synthetic directory tree the test creates and owns (a
  nested `.rs`, a nested non-`.rs`, an empty subdirectory) returns exactly the
  `.rs` files. `occurrences` over a synthetic two-file fixture returns the
  counts, and `files_containing` over the same fixture returns exactly the paths
  `occurrences` reported. *Mutation:* make `files_containing` its own
  independent filter again and change the comment rule in only one of the two;
  the derivation test goes red. Revert.
- **Unit / AC 4 — precision, and why it needs its own test.** The hand-built
  `Shape::Text` at a fractional position. *Mutation:* round inside
  `collect_text` — `pos: egui::pos2(text.pos.x.round(), text.pos.y.round())` —
  and confirm the equality goes red. Then run the **whole suite** under that
  same mutation and report what else fails. Expect: **nothing**. Worked through
  on the measured numbers, LCV-133's control reads `pos.y = 446.682` and rounds
  to `447`; `447 + 14.0 = 461.0` is still past the `460.000` clip bottom, so the
  straddle is still seen and the control still passes. A collector that rounds
  is therefore **invisible to every test in the repo today** — which is exactly
  why this criterion is a dedicated unit test rather than a line in AC 7's
  table, and why "the suite is green" is not evidence that the consolidation
  preserved precision. Revert.
- **Unit / AC 5 — nesting and clip propagation.** Prove it discriminates twice:
  flatten `collect_text` to a non-recursive `match` and show it goes red; hand
  the nested run a different clip rect and show the propagation assertion goes
  red. Revert both.
- **Unit / AC 6 — the empty-run rule, and what the positive control buys.**
  A hand-built run list containing an empty galley and a whitespace-only galley
  comes back with neither. *Mutation:* delete the `runs.len() > 10` assertion
  from the shared driver, then make `App::update_ui` paint nothing for one
  frame, and confirm a converted absence assertion reads as a clean pass — that
  is the false green the control exists to prevent. Report the message, restore
  both.
- **Integration / AC 3 — the wrappers are untouched.** The five wrappers' own
  tests pass unchanged, and the diff over those five functions is the two kinds
  of line named in AC 3 and nothing else. The diff is the evidence here; a scan
  would only prove a name still exists.
- **Mutation / AC 7 — the fifteen.** Each applied, run, recorded by test name
  **and verbatim message**, reverted. (d) is the one that matters most — it is
  the only one of the five original mutants with a security consequence, and it
  was invisible to every test form the repo had before this seam. (j) and (k)
  are the two that certify the consolidation itself. (k) is expected to take
  down **nine** tests — AC 1's unit test plus eight scan tests across `lcv121`,
  `lcv122` and `lcv129` — and to leave `lcv125` and `lcv128` green; report the
  count and the names, and if either of those two goes red as well, say so
  rather than assuming the list in AC 7 was right.
- **Integration / AC 8 — the orphan-include guard.** *Mutation:* add
  `mod harness;` to a converted test file and delete its uses, and confirm the
  guard names that file. *Second mutation:* run the guard's helper over a
  synthetic fixture whose only `harness::` sits in a `//!` line, confirm it is
  reported, then disable the comment-skipping and confirm the same fixture
  passes wrongly. Revert both; report both messages.
- **Manual / AC 9, AC 10** — the module headers read in the harness's voice and
  carry every trap listed, each with its reason; the `AGENTS.md` entry exists;
  the two `architect` tasks exist and cite the 5-vs-8 drift and the
  now-multi-file harness.
- **Manual / AC 11** — `git diff --stat` over `Cargo.toml`, `Cargo.lock` and
  `src/` is empty; every `awk` number is in the handover; the seven before/after
  line counts and the net delta are in the handover.
- **[manual] smoke** — none for product behaviour; this demand ships no
  user-visible change and must not. One timing check instead: report the wall
  time of `cargo test --all --no-fail-fast` before and after. Two full-app
  frames cost 0.01 s, so the expected delta is noise — a measurable regression
  means the helper is doing something it was not asked to do.

## Test hygiene (mandatory)

- The source scans this demand touches are not deleted where they still assert
  something a paint test cannot see — colour, style, the absence of a `Document`
  import. Adding a paint test is not a licence to drop a scan; it is a licence
  to stop pretending a scan covers rendering.
- Any scan this demand adds is bounded to the implementation section (bare
  `#[cfg(test)]` at column 0) with needles built by `concat!`, and carries a
  positive control run through the same helper. Canonical example:
  `guard_is_runtime_not_cfg` in `src/io/dialogs.rs`.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- No test asserts an absolute coordinate. See §Out of scope.
- `pixels_per_point = 1.0`, set explicitly, anywhere a position is read.
- At least two frames before any position is measured: egui sizes a layout on
  the frame after a widget is first seen.
- No test reaches a real endpoint.

## Open questions

None.

## Notes

- **Ordering: LCV-134 first.** LCV-134 is queued, it edits
  `tests/lcv133_shortcuts_dialog_fits.rs` — its AC 8 rewrites that file's
  negative-control message and its AC 7 runs two growth mutations through the
  same collector — and it carries the `src/ui/shortcuts_dialog.rs` file split as
  its first commit. Changing a helper while it is private to one file is a local
  edit; doing the same after this demand has promoted it to a shared `pub`
  helper with four call sites turns it into a cross-file change with innocent
  bystanders. That is exactly the reasoning LCV-133 §Notes used against this
  demand, and it applies again. It is a scheduling preference, not a dependency:
  LCV-134 §Expected tests already carries both shapes, one per this demand's
  state. **If this demand is picked up first anyway, stop and report** rather
  than absorbing `tests/lcv133_shortcuts_dialog_fits.rs` while LCV-134 is mid-flight.
- **Row (k): which controls actually bind, measured — and the decision.**
  AC 7 row (k) first read "the positive control in **every** scan wrapper".
  That was over-broad. The implementer ran the mutation, measured that two of
  the five wrappers survive it, and **routed the wording here instead of
  adjusting either wrapper to make the sentence come true**. That was the right
  call and it is the standing rule: when a criterion does not fire, report it;
  never quietly tune the test until it does. Measured under a non-recursive
  `rs_files` — nine tests go red, and:
  - **binds** — `lcv121::implementation_sections` and
    `lcv129::implementation_sections` (root `src/`, control `files.len() > 30`)
    and `lcv122::sections` at its `("src", 30)` call site. `src/` holds **2**
    `.rs` files at its top level and **106** in the tree, so a flat walk misses
    the control by two orders of magnitude. Eight scan tests, plus AC 1's unit
    test, is the nine.
  - **cannot bind** — `lcv125::agent_sections`. Its control is
    `files.len() >= 9` and `src/agent/` holds **9** `.rs` files and **no
    subdirectory at all**, so a recursive and a flat walk return the *identical*
    set. No threshold fixes that: there is nothing deeper to count. A follow-up
    demand against this wrapper would be asking for a test that cannot exist
    while `src/agent/` stays flat.
  - **does not bind** — `lcv128::kernel_sections`. Its five controls are each
    `!files.is_empty()`, and every kernel directory has top-level `.rs` files.
    The haystack does shrink silently (`geometry` 7 of 13, `document` 6 of 15;
    `io/svg`, `text` and `cmdline` are flat and lose nothing) while all five
    controls pass.

  **Decision: narrow the row — option (a). No follow-up demand is filed, and
  neither weak control is strengthened.** Three reasons, in order of weight:

  1. `rs_files` now has **one definition**, and AC 1's unit test asserts
     recursion directly against a synthetic tree the test builds, with a file
     two levels down. The mutant dies there **unconditionally and by name** —
     it cannot depend on how the real source tree happens to be shaped today,
     which is precisely the property a wrapper's positive control lacks.
     `lcv125`'s control is green on this mutant only because `src/agent/` is
     flat *this week*; that is an accident of layout, not coverage.
  2. `lcv128`'s control was never given this job. Its doc comment says it exists
     so "a typo'd directory name fails naming that directory", and
     `!files.is_empty()` does exactly that. Re-aiming it at a walker defect
     needs either a hardcoded per-directory minimum — which loosens **silently**
     the day a nested file moves up to the top level, and is a count that goes
     stale on every legitimate refactor — or a structural "at least one scanned
     path contains a `/`" claim, which goes stale the day a directory
     legitimately flattens. Both are worse artefacts than the one unit test they
     would duplicate.
  3. Requiring five wrappers to each re-catch one shared function's defect is
     n-fold coverage of a property that now has exactly one owner. That is the
     duplication this demand exists to delete; re-imposing it in the acceptance
     criteria would undo the point.

  **Residual risk, stated so it is not rediscovered as a surprise.** Neither
  `lcv125::agent_sections` nor `lcv128::kernel_sections` can see its haystack
  shrink from a walker change; both would scan less and still pass. What closes
  that today is that the only walker is shared and is pinned by AC 1's test, so
  the shrink cannot reach `main` without that test going red. Considered and
  accepted — not missed. If `src/agent/` ever grows a subdirectory, `lcv125`'s
  `>= 9` still will not bind until its top-level count drops below 9; that is a
  note for whoever adds the subdirectory, not work for this demand.

- **The `#![allow(dead_code)]` blanket stays, and is paid for instead.** It is
  not laziness: Cargo compiles `tests/harness/` into *every* consumer binary, so
  a file that uses only `tap` would trip `-D warnings` on every other item in it
  — eight today, roughly seventeen after this demand. Per-item
  `#[allow(dead_code)]` is the same blanket with more noise and no new signal,
  and splitting the harness finer does not help — each consumer still compiles
  whole modules. The blanket also propagates from
  `tests/harness/mod.rs` down into any submodule declared there, so the division
  in AC 11 needs no second attribute. What the blanket **does** cost is real and
  measured: an unused `mod harness;` sat undetected in
  `tests/lcv133_shortcuts_dialog_fits.rs` until a reviewer read for it, and this
  demand widens the blanket's reach. So the cost is paid directly by AC 8's
  guard, which catches precisely the case the blanket hides — a declared and
  unused include — in about twenty lines, and asserts nothing about whether a
  helper works, which is not a scan's job.
- **The expected division under `tests/harness/`**, for AC 11's cap: the input
  plumbing and its four hard rules stay in `mod.rs`, the paint collector and its
  traps go in one submodule, the scan walker and matcher and their hygiene rules
  in another. That is a division by kind, it keeps every file well under 270, and
  it puts each set of traps in the header of the code it is about. ADR 0002 §A3
  describes the harness as a single file with five items; extending it this way
  is the obvious reading, and AC 10's second hand-off asks `architect` to record
  it.
- **This demand absorbs the scan side too, and LCV-128 expected otherwise.**
  LCV-128 §Notes anticipated a *separate* consolidation demand for
  `rs_files` / the matcher, "sequenced **after** LCV-132 so the two are not
  editing that file at once". That split is dropped: both consolidations land in
  the same directory, and running two demands through `tests/harness/`
  back-to-back costs more review and more conflict surface than doing the whole
  thing once. The size is real, so the commit order is named instead of a
  demand split — **scan side first**, which is mechanical and touches no
  rendering claim, then the paint side, which carries the precision risk AC 4
  exists for. Those are two natural checkpoints for `project-manager` if the
  drive has to stop in the middle; they are not two demands.
- **Provenance.** The premise argument and the five mutants: `reviewer-rust`,
  2026-09-13, during the LCV-125 review — it did not report the gap in the
  abstract, it wrote the collector, ran it against the shipped panel, and killed
  five mutants with it. The four-file scan-walker read and the recommendation to
  extract `rs_files` and the count-based matcher while leaving the wrappers
  alone: `reviewer-rust`, adopted here rather than re-derived. The collector
  divergence ruling: `reviewer-rust` on LCV-133. Re-measured by `product-owner`
  at `2860edd`: the five `rs_files` copies (one digest across five files — the
  review named four, LCV-125's is the fifth), the four paint blocks at 89–130
  lines, the `is_empty()` / `trim().is_empty()` drift, the 0.682pt straddle
  (`pos.y = 446.682`, `height = 14.0`, clip bottom `460.000` at 1280×460), and
  `tests/harness/mod.rs` at **124** implementation LOC by ADR 0004's recipe.
  Two figures in the previous draft of this demand did not survive that
  re-measurement and are corrected here: the harness was recorded as **117**,
  and the census as three collectors and one extraction source.
- **The scratch probe** `tests/zz_probe_lcv125.rs` is **not** in the tree at
  `2860edd`. Nothing to delete; do not add one.
- Related: `tests/harness/mod.rs`,
  [ADR 0002 §A3/§A4](../../adr/0002-headless-input-tests-and-dirty-tracking.md)
  (where shared plumbing lives, and the stale five-item enumeration),
  [ADR 0004](../../adr/0004-measuring-the-300-loc-cap.md) (the `awk` recipe and
  rule 4), [ADR 0009](../../adr/0009-dialog-content-is-capped-at-420pt.md)
  (why a derived expected set and a containment assertion are the shapes to
  copy), `AGENTS.md` §Implementation Rules, LCV-125 (AC 2, 4, 5, 9, 10 — the
  five criteria the mutants broke, and the original collector), LCV-126 (the
  second copy and the column trap), LCV-128 (§Notes, which named this
  consolidation's trigger), LCV-129 (the third copy and the count-based
  matcher), LCV-133 (the fourth copy and the precision that must survive),
  LCV-134 (which goes first).
