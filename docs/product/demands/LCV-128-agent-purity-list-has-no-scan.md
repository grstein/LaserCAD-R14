# LCV-128 — `AGENTS.md`'s normative per-file and per-symbol enumerations have no scan

- **Status**: Done
- **Phase**: 12
- **Depends on**: none. **Sequencing note, not a dependency**: AC 3 overlaps LCV-129 AC 5 by one needle — whichever lands first owns the file and the other extends it in place. See AC 3.
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (8e2b7fe, 3b0e4a8). Reviewed and approved by reviewer-rust. CI: not run (billing hold).

## Problem

`AGENTS.md` carries a handful of enumerations that are **normative per entry**:
each name in the list carries a rule *for that name*. They are not orientation
and not prose — they are the whole statement of an invariant, and in several
cases they are the only statement of it. A new file, or a new assignment site,
silently escapes the list, and the omission is silent in exactly the direction
that matters: a name missing from the list is a name with no stated rule, so the
next person writing it has nothing to violate.

Two instances motivated this demand, found a day apart, and they are why it is
scoped to the *class* rather than to one list.

### Instance 1 — the `src/agent/` purity buckets

`AGENTS.md` §"Purity rule" classifies `src/agent/` file by file into two
buckets: the files that may import `egui` (`panel.rs`, `settings_ui.rs`) and the
files that are kernel-pure (`classifier.rs`, `wire.rs`, `transport.rs`,
`tools.rs`, `bridge.rs`, `loop_.rs`). A new file can be added under
`src/agent/` without appearing in either bucket, and nothing fails.

It has already happened twice. `src/agent/bridge.rs` landed with LCV-122 on
2026-09-13 and sat unclassified until a human noticed by hand. And it is **live
right now**: the directory holds nine files and the purity section names eight —
`mod.rs` is in neither bucket today.

### Instance 2 — the `agent_busy` / `agent_rx` single-writer claim

`AGENTS.md` §"Event flow" → Repaint policy states that `agent_busy` is set in
exactly one place (`src/app/agent_turn.rs::arm_turn`) and cleared in exactly one
(`src/app/agent_poll.rs::end_turn`). That sentence is load-bearing: it is what
makes the `if self.agent_busy` repaint guard a guard rather than a latch, and it
is the whole argument that LCV-120 stays closed. During the LCV-122 review the
reviewer planted `app.agent_busy = false;` and `app.agent_rx = None;` in
`src/agent/panel.rs` and **both survived the entire suite**.

The neighbouring claim in that same paragraph — the three repaint call sites and
their guards — *is* scanned, by `every_repaint_request_in_src_is_conditional` in
`src/app/viewport.rs`.

### The coverage census (measured 2026-09-13, at `562a2b7`)

Most of this demand's original target was closed by other demands while it sat
in `Draft`. Identifying the remaining gap precisely is most of the work, so the
census is recorded here rather than left for the implementer:

| normative enumeration in `AGENTS.md` | scanned by | status |
|---|---|---|
| kernel purity: `geometry/`, `document/`, `io/svg/`, `text/`, `cmdline/` must not import `egui`/`eframe`/`rfd` | — | **unscanned** |
| every file under `src/agent/` sits in one of the two buckets | — | **unscanned** |
| `agent_rx` is cleared in exactly one place | — | **unscanned** |
| `agent_busy` is set in exactly one place | — | **unscanned** |
| `agent_busy` is cleared in exactly one place | LCV-129 AC 5 (Ready, not yet implemented) | **claimed** |
| only `panel.rs` / `settings_ui.rs` import `egui` under `src/agent/` | `tests/lcv125_…::ac14_only_panel_and_settings_ui_import_egui_source_scan` | covered |
| only `transport.rs` imports `reqwest` | `tests/lcv121_source_scans.rs::only_the_transport_imports_reqwest` | covered |
| `panel.rs` renders and reports — no thread, no `Document`, no `History` | `tests/lcv122_source_scans.rs` (three tests) | covered |
| the three `request_repaint` call sites and their guards | `src/app/viewport.rs::every_repaint_request_in_src_is_conditional` | covered |
| `bridge.rs` / `classifier.rs` / `wire.rs` are kernel-pure | their own inline `*_is_kernel_pure` tests | covered per file |

**The largest gap is the one the demand did not name.** `AGENTS.md` opens its
architecture rules with the kernel purity list — five directories, three
forbidden crates, "this is the kernel… it must remain testable as a pure Rust
library and runnable in a future headless / CLI / WASM context. Reviewers check
this on every demand." It is the founding invariant of the project (ADR 0001)
and **nothing checks it**. Every kernel file carries a module-header comment
restating the rule, which is how it has survived; a comment is a reminder, not a
gate. `transport.rs`, `tools.rs` and `loop_.rs` are in the same position inside
`src/agent/` — named kernel-pure, with no inline test of their own, unlike their
three siblings.

So this demand keeps its title and its thesis, drops the parts other demands
closed, and adds the one the census turned up.

### What this demand does not reopen

The module tree next door in the same document is **not** in scope. The
architect declared it explicitly non-exhaustive on 2026-09-13: the tree is
orientation, so a gap there is cosmetic, and making it scannable would first
mean making it exhaustive — roughly seventy names duplicating what `ls` already
answers, churning on every new file. That decision is right and stands. The
distinction that admits a list into this demand is that the list is normative
*per entry*.

## Scope

- One test file, `tests/lcv128_normative_enumerations.rs`, carrying three scans
  over the tree and the two lists that are normative per entry.
- One sentence added to `AGENTS.md` §"Purity rule" placing `src/agent/mod.rs`,
  so AC 2 has something true to check.
- A named mutation per scan, run and reported.

## Out of scope

- **Making `AGENTS.md` prose an input to correctness beyond a section-bounded
  substring check.** AC 2 asks one question of the document — "does this file
  name appear anywhere in the §Purity rule section" — and nothing else. It does
  not parse the bullets, does not care about wording, punctuation, order, or
  which bucket a name sits in, and does not compare sentences. Anything richer
  makes a reviewer's clarity edit a red build, which punishes the exact
  behaviour the repo wants. The cost of the cheap version is stated in AC 2.
- **Fixing the comment-line blind spot.** `files_containing` skips `//` lines,
  which is why a stale claim inside `src/agent/mod.rs`'s header survived every
  gate in LCV-123. That skip is not a bug to repair here: it is what lets a
  kernel file *document* "MUST NOT import `egui`" without the scan reading its
  own documentation as a violation — and every one of the ~15 kernel files does
  exactly that. Its own doc comment in `tests/lcv122_source_scans.rs:85-96`
  already records the trade honestly. A demand to catch stale prose is a demand
  to parse prose; see the bullet above.
- **A scan over the `AGENTS.md` module tree.** Non-exhaustive by decision.
- **Re-scanning what the census marks covered.** Duplicate coverage is not free:
  it is a second thing to update when a rule changes, and the second one is the
  one that gets forgotten. If a covered scan looks weak, that is a finding to
  report, not a scan to rewrite here.
- **Extending the scans to `tests/`.** These enumerations are about what the
  library is. A test binary that imports `egui` to drive a headless frame is
  doing its job.
- **Moving the shared walker into `tests/harness/`.** Tempting — this makes a
  third copy of `rs_files` / `sections` / `files_containing` — but
  `tests/harness/mod.rs` is the headless-frame harness, is being edited by
  LCV-125 and is the subject of LCV-132. Do not contend for that file here. See
  §Notes for when to open the consolidation demand.
- **Any change to the code the scans police.** If a scan goes red on `main` for
  a reason this body did not predict, **stop and report** — do not "fix" the
  kernel to suit a test written a minute ago.

## Acceptance criteria

1. **The kernel purity rule is enforced.** A test asserts that no `.rs` file
   under `src/geometry/`, `src/document/`, `src/io/svg/`, `src/text/` or
   `src/cmdline/` names `egui`, `eframe` or `rfd` **on a code line**. Comment
   lines are skipped — every kernel file's module header states the rule in
   prose and a naive scan would read those as violations (verified: on `main`
   the only occurrences in all five directories are in `//` headers).
   All six needles are built with `concat!`. Two controls, both mandatory:
   - **the walk is not empty** — each of the five directories contributes at
     least one file, asserted per directory, so a typo in a directory name
     fails rather than silently scanning nothing;
   - **the needles work** — each of the three is first run through the *same*
     helper against a witness string that contains it, so a misspelt needle
     fails before the real haystack is consulted (the technique
     `no_agent_file_holds_document_state` uses).

2. **Every file under `src/agent/` is named in the purity section, and the
   section is made true first.** `AGENTS.md` §"Purity rule" gains one sentence
   placing `mod.rs` — it declares the modules and re-exports their symbols, it
   imports nothing itself, and it is bound by the same no-`egui`/`eframe`/`rfd`
   rule. Then the scan: for every `.rs` file directly under `src/agent/`, its
   **file name** appears somewhere in the text of `AGENTS.md` between the
   `### Purity rule` heading and the next `###` heading. Section-bounded,
   substring only.

   **One-directional, deliberately.** The scan fails when a file exists and is
   unnamed. It must **not** fail when the section names a file that does not
   exist: the section named `classifier.rs` for days before LCV-124 created it,
   and that forward commitment is a feature. The asymmetry is the point — a
   missing name means *no rule for a real file*, a lingering name means *a rule
   for nothing*. Only the first can be violated.

   Controls: the positive control asserts the extracted section is non-empty and
   contains a name known to be in it (`transport.rs`); a length assertion proves
   the section is shorter than the whole document, so a failed extraction cannot
   pass by scanning all of `AGENTS.md` — which contains every one of these file
   names in the module tree.

3. **The single-writer claims are enforced, without duplicating LCV-129.** Over
   every `.rs` file under `src/`, each haystack bounded at the offset of its bare
   `#[cfg(test)]` at column 0, needles built with `concat!`:
   - `agent_rx = None` appears in **exactly one** file, `src/app/agent_poll.rs`;
   - `agent_busy = true` appears in **exactly one** file,
     `src/app/agent_turn.rs`, which is what `AGENTS.md` §Event flow claims.

   **LCV-129 AC 5 covers the third needle** (`agent_busy = false`, exactly once,
   in `agent_poll.rs`) with the same hygiene. Whichever demand lands first owns
   the file; the second **extends the existing test with its needles rather than
   adding a second scan**, and says so in the handover. Two tests asserting
   adjacent halves of one invariant in two files is how one of them gets deleted
   by someone who thinks it is a duplicate.

   Two details the implementer should not have to rediscover: the needle is the
   **clear** (`= None`), not any assignment — `agent_rx = Some(…)` legally has
   two writers today (`agent_turn.rs::arm_turn` arms it, `agent_poll.rs` puts
   the receiver back on the `TryRecvError::Empty` path) — and `src/app/init.rs`
   needs no exception, because it writes `agent_rx: None` in a struct literal,
   which the `=` needle does not match.

4. **Every scan is shown to fail.** For each of the four assertions above the
   implementer applies the mutation named in §Expected tests, records the test
   name and the assertion message, and reverts. **A scan added without a
   demonstrated failure is the defect this repo already has six of, not the fix
   for it.**

5. **Hygiene, caps, gates.** The scans live in `tests/`, never under `src/`, so
   they cannot match themselves. Every path compared or reported is rebuilt from
   `components()` joined with `/`, never `Path::display()` (this has broken CI
   twice), and sorted on the rendered string. The new file is at or under 300
   implementation LOC by ADR 0004's `awk` recipe, never `wc -l`, and the number
   is reported. `cargo fmt --all -- --check`, `cargo clippy --all-targets -D warnings`
   and `cargo test --all --no-fail-fast` all green locally; **CI: not run
   (billing hold)** — the local gate is the acceptance gate for this demand.

## Expected tests

All three scans live in `tests/lcv128_normative_enumerations.rs`.

- **Unit / AC 1 — the kernel scan.** Mutation: add `use egui;` to
  `src/geometry/vec2.rs` (or any kernel file), run, confirm the test names the
  file and the needle; revert. Second mutation, for the control: misspell one
  needle and confirm the witness control fails *before* the haystack assertion.
- **Unit / AC 2 — the bucket scan.** Mutation: `touch src/agent/zz_new.rs` with
  a one-line module doc, run, confirm the test fails naming `zz_new.rs` and
  pointing at `AGENTS.md` §"Purity rule"; delete it. Second mutation, for the
  section bound: point the extraction at a heading that does not exist and
  confirm the non-empty control fails rather than the scan passing.
- **Unit / AC 3 — the writer scans.** Mutation: plant `app.agent_rx = None;` in
  `src/app/agent_turn.rs` and `app.agent_busy = true;` in `src/agent/panel.rs`,
  run, confirm each fails by name listing both files; revert. This is the same
  plant the LCV-122 reviewer made, which survived the whole suite — the report
  must state that it now does not.
- **Unit / AC 5** — the LOC number from the `awk` recipe, in the handover.
- **[manual] smoke**: none, and deliberately. This demand adds three tests and
  one sentence to `AGENTS.md`; it changes no code the application runs, so there
  is nothing an operator could observe. The failure mode it guards is caught by
  `cargo test`, which is the point.

## Test hygiene (mandatory)

- Bound every source-scan haystack at the offset of a **bare `#[cfg(test)]` at
  column 0** and build every needle with `concat!`. Canonical correct example:
  `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`. Six self-matching
  scans have shipped here.
- **AC 1 is the exception that proves the rule and must be written knowingly**:
  it scans whole files rather than implementation sections, because a kernel
  unit test that imports `egui` breaks the same headless promise the rule is
  about. It is safe from self-matching for a different reason — the scan lives
  in `tests/` and the haystack is `src/geometry/` and friends, which can never
  contain it. Say that in a comment; do not leave the next reader to work out
  why one scan is bounded differently.
- Rebuild compared paths from `components()` joined with `/`, never
  `Path::display()` or `to_string_lossy()` on a whole path. Sort on the rendered
  string.
- Every absence assertion carries a positive control. An absence assertion over
  an empty haystack passes for the wrong reason, and that is how five of the six
  bad scans passed review.
- No test reaches a real endpoint.

## Open questions

None. The four questions this demand carried into refinement are decided in the
body: `mod.rs` joins the purity section with its own sentence rather than
becoming a by-name exemption (AC 2); the scan reads `AGENTS.md` as a
section-bounded substring rather than carrying a Rust copy of the list, with the
cost stated (AC 2, §Out of scope); the remaining per-file purity rules are
inventoried in the census and only the unscanned ones are in scope; and the
stronger "`end_turn` is what every terminal path tail-calls" claim is left to
ADR 0007 §D11 and LCV-129, because it is a control-flow property and a source
scan cannot honestly assert it. This demand is Ready when `demand-manager`
flips it.

## Notes

- Origin, instance 1: `architect`, 2026-09-13, while fixing documentation drift
  in the `AGENTS.md` module tree. Asked whether the tree deserved a scan, they
  argued no and declared it non-exhaustive instead, then pointed at the purity
  enumeration as the one that genuinely earns one.
- Origin, instance 2: `reviewer-rust`, 2026-09-13, during the LCV-122 review.
- **Size, honestly.** The architect's "about a dozen lines" estimate was for the
  bucket scan alone, and the bucket scan turned out to be the smallest of the
  three. With the walker helpers (a third copy — see below), three scans, the
  controls each one needs and six mutations to run and report, this is a
  ~150-line test file plus one `AGENTS.md` sentence plus a mutation pass. That
  is a medium demand, comparable to LCV-127, **not** a filler task. Taking it in
  a drive alongside LCV-129 is a real cost; taking it *instead of* half of
  LCV-129 is not a trade, because LCV-129 fixes a hang an operator can hit and
  this one fixes nothing an operator can see.
- **When to open the consolidation demand.** This makes a third private copy of
  `rs_files` / `sections` / `files_containing` (`lcv121`, `lcv122`, `lcv128`).
  Three copies of forty lines is tolerable; a fourth is not. If a fourth demand
  needs them, open a demand to move them into a scan-side module under
  `tests/harness/`, sequenced **after** LCV-132 so the two are not editing that
  file at once.
- **Do not "fix" the comment-line blind spot as a side quest.** It is load-
  bearing: ~15 kernel files document the rule they obey in a `//!` header, and a
  scan that read comments would fail on all of them. See §Out of scope.
- Related: `AGENTS.md` §Purity rule and §Event flow → Repaint policy,
  [ADR 0001](../../adr/0001-pure-rust-egui.md) (why the kernel is pure),
  [ADR 0004](../../adr/0004-measuring-the-300-loc-cap.md) (the `awk` recipe),
  [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md) §D8 (the
  file table the purity rule mirrors) and §D11 (the four turn exits),
  `tests/lcv121_source_scans.rs`, `tests/lcv122_source_scans.rs:85-96` (the
  comment-skip trade, written down), LCV-129 AC 5 (the third needle),
  LCV-132 (`tests/harness/`), `src/io/dialogs.rs:179-194`.
