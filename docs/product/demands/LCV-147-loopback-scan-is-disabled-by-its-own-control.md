# LCV-147 — The tree-wide loopback scan ships disabled, blocked by a control that breaks the rule it controls for

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-130 (shipped; this re-enables its AC 4b), LCV-124 (owns the file being corrected)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

LCV-130 exists because a **credential leak was reproduced**: a test pointed at
`http://127.0.0.1:1` believing a closed loopback port cannot be reached, and
under `HTTP_PROXY` a real `POST` carrying `authorization: Bearer
sk-test-DO-NOT-LEAK` and the full system prompt left the machine **while the
suite reported green**. LCV-130's AC 4b is the guard that stops that spelling
from re-landing anywhere in the tree: one scan over `src/` and `tests/` proving
no string literal on a code line parses as an `http`/`https` URL with a loopback
or unspecified host.

**That guard shipped `#[ignore]`d.** `tests/lcv130_no_loopback_url_literals.rs::no_loopback_http_literal_survives_in_the_tree`
is disabled in the suite, so the central invariant of the demand written in
response to a real leak is not running. An acceptance criterion that ships
disabled is a gap, not a nuance, and this demand closes it.

### Why it is disabled, and why the guard is right and the tree is wrong

`reqwest`/`url` normalise IPv4 hosts leniently, per WHATWG: fewer than four
dot-separated parts is legal and the remainder folds into the last one. Measured
against reqwest 0.12.28:

| literal | `Url::parse` | `host_str()` | loopback? |
|---|---|---|---|
| `http://127.0.0.` | `Ok` | `127.0.0.0` | **yes** |
| `http://127.1` | `Ok` | `127.0.0.1` | **yes** |
| `http://0x7f.1` | `Ok` | `127.0.0.1` | **yes** |
| `tp://127.0.0.1:1` | `Ok` | `127.0.0.1` | no — scheme is not `http`/`https` |
| `ht` | `Err` | — | no |

`tests/lcv124_command_line_routing.rs::the_test_endpoint_cannot_reach_a_proxy`
carries a positive control built as `concat!("http://127.0.0.", "1:1")` — the
sendable spelling `UNPARSEABLE_ENDPOINT` replaced, split so the file does not
leave the offending literal in its own source text. Under the table above the
split does not achieve that: **`"http://127.0.0."` is itself a complete,
flaggable loopback URL**, and the scan flags it. LCV-130's own §Test hygiene
requires that "neither its needles nor its control fixture may leave a whole
offending literal in its own source text" — so **the control is already in
breach of the rule it was written to serve**, and the scan finding it on its
first run is the guard working, not the guard misfiring. LCV-130's body asserted
"neither piece parses as a loopback URL"; that sentence is now corrected there.

The implementer who hit this refused two shortcuts — silently excluding the file
from the scan, and editing a file LCV-130 marked do-not-touch — and reported
instead. Both refusals were correct; this demand is the routing.

### Measured, so the size of the job is not a guess

With the `#[ignore]` lifted, over **142 files** in `src/` and `tests/`:
**exactly one literal is flagged, in exactly one file**, and it is the control
above. With the control respelled inside the scheme word — `concat!("ht",
"tp://127.0.0.1:1")`, which is the vocabulary `tests/lcv130_no_loopback_url_literals.rs`
already uses for its own bad needles — the scan is **green with zero flags**.
The whole fix is one line. `concat!` still yields the same string at compile
time, so the control's runtime claim, *"a closed port is still a sendable URL —
that was the leak"*, is unchanged.

### One stale doc comment rides along

`src/agent/transport.rs::owned_socket_closed_before_a_reply_is_a_request_error`
tells its reader that adding `if error.is_connect() { … }` to `request_error`
"turns this test red while (a) stays green". **Measured: both stay green.**
`is_connect()` answers `true` only for a failure during the connect step, and
this fixture `accept()`s the connection before dropping it, so its failure is
always post-connect. The claim is false in the tree, in a doc comment whose
whole job is to tell the next reader why the test is not a duplicate. It is
corrected here rather than in its own demand because it is the same defect as
the one above — a normative claim nothing checks, falsified by measurement — in
the same shipped demand, and it is two sentences.

## Scope

- Respell `tests/lcv124_command_line_routing.rs`'s positive control so no piece
  of its source text is a flaggable loopback URL, preserving what it controls.
- Lift `#[ignore]` from
  `tests/lcv130_no_loopback_url_literals.rs::no_loopback_http_literal_survives_in_the_tree`
  and remove its "Known blocker" section, in the same commit.
- Correct the falsified mutation claim in
  `src/agent/transport.rs::owned_socket_closed_before_a_reply_is_a_request_error`'s
  doc comment to the pair LCV-130 AC 1b now carries.

## Out of scope

- **Weakening the scan in any form.** No per-file exclusion list, no allowlist,
  no carve-out for short IPv4 spellings, and no narrowing of
  `is_loopback_or_unspecified_url`'s judgment. Tolerating `http://127.0.0.`
  means tolerating `http://127.1` and `http://0x7f.1`, which are real, sendable
  loopback endpoints — a guard with that hole is worse than no guard, because it
  reports green. If the scan is red on anything other than the one control named
  in §Problem, **stop and report**; do not tune it until it passes.
- **`UNPARSEABLE_ENDPOINT`, its doc comment, and everything else in
  `tests/lcv124_command_line_routing.rs`.** One line changes in that file. The
  `http://127.0.0.1:1` spellings in its `///` doc comments stay: the scan reads
  code lines only, deliberately, so a rule spelled out in prose cannot flag
  itself.
- **Changing `request_error`'s mapping**, or adding any branch to it. LCV-147
  corrects a doc comment; the mutations it names are applied and reverted.
- **`.no_proxy()`, anywhere, production or test-scoped.** Rejected in LCV-130
  §Out of scope and still rejected.
- **Broadening AC 4b's claim.** The scan is about loopback URL literals, not
  about every sendable endpoint, and its doc comment says so. Leave that limit
  stated as it is — a doc comment implying more than the code checks is the
  failure mode this repository keeps paying for.
- **New scans, new harness code, new dependencies, new files under `scripts/`.**
- **Anything that depends on a CI run.** CI is on a billing hold; the local gate
  is the acceptance gate.

## Acceptance criteria

1. **The control no longer leaves a loopback URL in its own source text, and
   still controls.** `tests/lcv124_command_line_routing.rs::the_test_endpoint_cannot_reach_a_proxy`'s
   third assertion is rebuilt so that **no double-quoted run on its line parses
   as an `http`/`https` URL with a loopback host** — split inside the scheme
   word, `concat!("ht", "tp://127.0.0.1:1")`, matching the spelling
   `tests/lcv130_no_loopback_url_literals.rs::the_extractor_flags_the_known_bad_spellings`
   already uses. The assertion still reads `reqwest::Url::parse(…).is_ok()`, the
   concatenated value is still exactly `http://127.0.0.1:1`, and the failure
   message still names the leak. `git diff` over that file touches that one
   assertion and nothing else.

2. **The tree scan runs by default and is green.**
   `no_loopback_http_literal_survives_in_the_tree` carries no `#[ignore]`, and
   `cargo test --all --no-fail-fast` runs it and passes. The ignored-test count
   in the suite's summary drops by exactly one against its parent commit, and
   the handover quotes both numbers.

3. **The blocker record is removed, not left to rot.** The "Known blocker"
   section of `tests/lcv130_no_loopback_url_literals.rs`'s module header is
   deleted — it describes a state that no longer exists. The paragraph that
   explains *why* every needle splits inside the scheme word, including the
   measured `Url::parse("http://127.0.0.") -> 127.0.0.0` leniency, **stays**:
   that is the reason the spelling is what it is, and the next person to add a
   needle needs it.

4. **The scan still discriminates, proven after the lift, not before.** With the
   scan enabled, re-run LCV-130 §Expected tests' four mutations through it, one
   at a time and reverted: planting `http://127.0.0.1:1` on a code line goes
   red; planting `http://localhost:3000` goes red; and the scan stays green on
   `https://openrouter.ai/api/v1`, `http://www.w3.org/2000/svg`, `"127.0.0.1:0"`
   and `"http://{}"`. A guard that is green because it was just switched on and
   a guard that is green because the tree is clean look identical from the
   summary line; these mutations are what tells them apart.

5. **The stale doc comment tells the truth.**
   `src/agent/transport.rs::owned_socket_closed_before_a_reply_is_a_request_error`'s
   doc comment no longer claims an `is_connect()` mutation discriminates. It
   names the measured pair instead — `is_request()` turns this test red while
   the unparseable test stays green, `is_builder()` the reverse — and states in
   one clause *why* `is_connect()` cannot work here: this fixture accepts the
   connection before dropping it, so its failure is never a connect-step
   failure, and the only fixture shape that would answer `is_connect()` is the
   bound-then-dropped port LCV-130 AC 1b rejects as a flake. It cites symbols,
   never line numbers (§Documentation Hygiene).

6. **Nothing under `src/` changes except that doc comment.** `git diff` over
   `src/` touches only the `#[cfg(test)] mod tests` block of
   `src/agent/transport.rs`, and within it only that doc comment —
   `request_error`, `chat_completion` and `chat_completion_with_timeout` are
   byte-identical to their parent-commit state, and no test body changes.
   `Cargo.toml` and `Cargo.lock` are unchanged.

7. **The local gate is green.** `cargo fmt --all -- --check`,
   `cargo clippy --all-targets -- -D warnings` and
   `cargo test --all --no-fail-fast`, all three clean, with the per-binary
   `test result` lines summed rather than a truncated tail read.

## Expected tests

- **Unit / AC 1 — the control still controls.** `the_test_endpoint_cannot_reach_a_proxy`
  passes. *Mutation:* change the concatenated value to something that does not
  parse (`concat!("ht", "tp://")` is enough) and confirm the third assertion goes
  red — a control that passes for any input is not a control. Revert.
- **Unit / AC 1, AC 2 — the respelling is what unblocks the scan.** Run the tree
  scan against the parent commit's spelling of the control (temporarily restore
  `concat!("http://127.0.0.", "1:1")`) and confirm it goes red naming
  `tests/lcv124_command_line_routing.rs` and the literal `http://127.0.0.`;
  restore the respelling and confirm green. This is the one test that proves the
  fix addresses the actual blocker rather than coinciding with it.
- **Unit / AC 2 — the scan is enabled.** `cargo test --all --no-fail-fast`
  includes `no_loopback_http_literal_survives_in_the_tree` in its passing set.
  Report the ignored-test count before and after; it drops by exactly one.
- **Unit / AC 4 — the four mutations**, one at a time, each reverted: two plants
  that must go red, four false-positive controls that must stay green. Run the
  controls through the **same** extractor the real scan uses; a control that
  takes a different path proves nothing.
- **Unit / AC 5 — the corrected claim is true, not merely rewritten.** Apply
  `if error.is_request() { return TransportError::Timeout { secs: request.as_secs() }; }`
  to `request_error` and confirm `owned_socket_closed_before_a_reply_is_a_request_error`
  goes red while `unparseable_endpoint_is_a_request_error` stays green; then
  apply `if error.is_builder() { … }` alone and confirm the reverse. Revert both.
  **If either direction does not discriminate, stop and report** — do not write a
  second false claim over the first one.
- **Unit / AC 6 — the diff bound.** `git diff` over `src/` shows one doc comment.
- **[manual] smoke — none.** This demand changes no user-visible behaviour: no
  production code path, no UI, no SVG output. `cargo run` is not part of its
  gate.

## Test hygiene (mandatory)

- Build every needle with `concat!`, split **inside the scheme word**. Any new
  bad spelling added to this tree must be verified against the leniency table in
  §Problem before it is committed.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- No test mutates the process environment. No test binds a fixed port. No test
  reaches a real endpoint.

## Open questions

None.

## Notes

- Origin: `implementer-rust`'s LCV-130 handover, 2026-09-14, which reported both
  findings rather than tuning around them — exactly what LCV-130's own text asked
  for. Routed by `product-owner`; LCV-130's body now carries the corrected AC 1b,
  AC 4b, AC 5 and AC 7 text and points here for re-enablement.
- **Two alternatives were considered and rejected on measurement.**
  *Redefining AC 4b's contract* so the tree can satisfy it as written would mean
  tolerating short IPv4 forms, and `http://127.1` / `http://0x7f.1` both resolve
  to `127.0.0.1` — a guard blind to those would report green on a re-landed leak.
  *A documented single-file exception* carves out the one file this demand's own
  witness names, which is the shape LCV-130 §Test hygiene exists to forbid, and
  it rots the first time someone adds a second exception. Neither is a cheaper
  fix than one line.
- **This is not re-fixing what `a1b37aa` fixed.** LCV-130 §Out of scope closed
  `tests/lcv124_command_line_routing.rs` against re-opening the *leak*;
  `UNPARSEABLE_ENDPOINT` and its doc comment are untouched here. What changes is
  the file's own hygiene defect, found by a guard that did not exist when
  `a1b37aa` landed.
- Reproduction for the leniency table, outside the repository:
  `reqwest::Url::parse("http://127.0.0.")` returns `Ok` with
  `host_str() == Some("127.0.0.0")`, and `Ipv4Addr::is_loopback()` answers `true`
  for `127.0.0.0`.
- Related: LCV-130 (AC 4b, AC 1b and their post-ship corrections), LCV-124
  (`the_test_endpoint_cannot_reach_a_proxy`, `UNPARSEABLE_ENDPOINT`), LCV-132
  (`tests/harness/scan.rs::rs_files`, the walker this scan uses), `AGENTS.md`
  §Implementation Rules (the isolation rule LCV-130 AC 6 added),
  [ADR 0008](../../adr/0008-test-gates-run-with-no-fail-fast.md).
