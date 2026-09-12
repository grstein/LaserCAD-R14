# LCV-106 — OffsetTool: product decision and removal

- **Status**: Done
- **Phase**: 10
- **Depends on**: none (may land any time in Marco 0; independent of LCV-100..104)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: adbc878 — refactor(LCV-106): delete OffsetTool, an explicit product non-goal shipped by accident
- **Note**: this demand text originally stated `offset.rs` held 24 inline tests; a test-count bisection at removal time confirmed it held 23 (lib test count dropped 647 -> 624). Corrected in place per LCV-108.

## Problem

`src/tools/offset.rs` (207 lines, 23 inline tests) shipped in commit `3c99f68`
under the LCV-054 id. Two things are wrong with that:

1. The LCV-054 slot was planned as "snap integration into all drawing tools",
   a capability already delivered by LCV-041's global snap resolve. The id was
   silently reassigned to a different feature.
2. `docs/product/README.md:36` lists **"No fillet / chamfer / offset"** as an
   explicit v0.1.0 non-goal. So the repository currently ships a documented
   non-goal, and the product docs contradict the code.

This is exactly the failure mode AGENTS.md warns about: *"Prefer rejecting a
feature over carrying accidental product complexity."* A v0.1.0 that advertises
OFFSET has to keep it working, test it, put it in the toolbar and the menu, wire
its distance to the command line, and explain why it offsets lines and arcs but
not circles or polylines. None of that buys a laser-cutting operator anything
the existing tools do not already give them: kerf allowance is a LaserGRBL-side
concern, and a parallel line at a fixed distance is two clicks with LINE plus a
typed coordinate.

## Decision — delete the tool

**Verdict: remove `OffsetTool` from the codebase; `docs/product/README.md`
keeps its non-goal line verbatim.**

Reasoning, in product terms:

- The non-goal is deliberate and still correct. Amending the README to legalise
  the tool would mean adopting a feature that no user asked for, that no demand
  scoped honestly, and that arrived by an id mix-up — the weakest possible
  reason to grow the product surface.
- Nothing depends on it. `OffsetTool` is referenced by exactly two lines outside
  its own file (`src/tools/mod.rs:15` and `:31`), has no toolbar button, no
  keyboard binding, no menu item, no command-line alias, and no entry in the
  agent tool registry. It is unreachable today, so deletion changes **zero**
  user-visible behaviour.
- Deletion is reversible at near-zero cost: commit `3c99f68` carries the whole
  implementation plus its tests. If a future release accepts OFFSET as a real
  feature, it comes back through a real demand with a real problem statement.

The alternative — promoting it to a documented exception by amending
`docs/product/README.md` — is explicitly **rejected**.

## Scope

- `git rm src/tools/offset.rs` (the file and its 23 inline tests go together).
- `src/tools/mod.rs`: delete `pub mod offset;` (line 15) and
  `pub use offset::OffsetTool;` (line 31).
- `src/tools/select/hit.rs`: revert the visibility widening that commit
  `3c99f68` made **solely** for `offset.rs` — `PICK_THRESHOLD_MM` (line 18) and
  `entity_distance_to_point` (line 76) return to `pub(super)`.
  (`src/tools/trim.rs` declares its own independent `PICK_THRESHOLD_MM` and is
  unaffected.)
- The deletion commit message cites `3c99f68` so the implementation stays
  findable: `refactor(LCV-106): remove OffsetTool — OFFSET is a v0.1.0 non-goal
  (shipped by mistake in 3c99f68)`.

## Out of scope

- **Editing `docs/product/README.md`.** The non-goal list is already right; the
  point of this demand is that the code comes to the docs, not the reverse.
- **Editing `docs/product/demands/LCV-054-offset-tool.md`, `backlog.md`,
  `.claude/backlog.json`, `CHANGELOG.md` or any `Status:` line.** Those are
  `demand-manager` work — see Notes for the exact handoff.
- **Any other tool.** Trim, Extend, Move, Delete and the drawing tools are
  untouched.
- **Removing `CreateLine` / `CreateArc`**, which `offset.rs` used — they are
  core commands with many other consumers.
- **A replacement feature.** No "offset-lite", no kerf-compensation setting, no
  parallel-copy command lands in its place.

## Acceptance criteria

1. `git ls-files src/tools/offset.rs` returns nothing.

2. `grep -rn "OffsetTool\|offset::" src/ tests/` returns no match, and
   `grep -rn '"OFFSET"' src/ tests/` returns no match.

3. `src/tools/mod.rs` contains neither `pub mod offset;` nor
   `pub use offset::OffsetTool;`; every other `pub mod` / `pub use` line in that
   file is unchanged.

4. `src/tools/select/hit.rs` declares `PICK_THRESHOLD_MM` and
   `entity_distance_to_point` as `pub(super)`;
   `grep -n "pub(crate)" src/tools/select/hit.rs` returns no match.

5. `SelectTool` behaviour is unchanged: its existing unit tests (point pick,
   window box, crossing box, Escape, Delete) all still pass.

6. `src/tools/trim.rs` still compiles against its own `PICK_THRESHOLD_MM`;
   `TrimTool` and `ExtendTool` tests still pass.

7. No toolbar entry, menu item, keyboard binding, command-line alias or agent
   tool definition references OFFSET:
   `grep -rni "offset" src/ui/ src/agent/` returns no match.

8. The test count drops by exactly the 23 tests that lived inside
   `src/tools/offset.rs`, and no other test is removed or modified.

9. `docs/product/README.md:36` still reads `- No fillet / chamfer / offset.`
   (unchanged).

10. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0 — in particular, no `unused_imports` or
    dead-code warning appears in `src/tools/select/hit.rs` after the visibility
    revert.

## Expected tests

- **(AC 1, 2, 3)**: static checks — `git ls-files`, two `grep` runs, and a read
  of `src/tools/mod.rs`.
- **(AC 4)**: static `grep` on `src/tools/select/hit.rs` plus a successful
  `cargo build`.
- **(AC 5)**: `cargo test --all` — the `src/tools/select/` unit tests and
  `tests/app_tool.rs` pass unchanged.
- **(AC 6)**: `cargo test --all` — `src/tools/trim.rs` and `src/tools/extend.rs`
  unit tests pass unchanged.
- **(AC 7)**: static `grep` across `src/ui/` and `src/agent/`.
- **(AC 8)**: compare the `cargo test --all` summary lines before and after; the
  only delta is the 23 offset tests.
- **(AC 9)**: `grep -n "fillet" docs/product/README.md`.
- **(AC 10)**: the three build gates.
- **Manual smoke**: `cargo run`. Every toolbar button, every Tools-menu entry
  and every keyboard tool shortcut still activates its tool; the SelectTool
  pick threshold still feels identical (click within ~5 mm of a line selects
  it). Nothing in the UI ever offered OFFSET, so there is nothing for a user to
  miss.

## Risks

- **Someone re-adds it later without a demand.** Mitigation: the non-goal line
  in `docs/product/README.md` stays, and this file is the written record of the
  call.
- **Hidden consumer.** Verified at authoring time that `OffsetTool` is
  referenced only by `src/tools/mod.rs:15` and `:31`; AC 2 re-verifies at
  implementation time. If a consumer has appeared since, stop and route back to
  `product-owner` rather than working around it.
- **Visibility revert causes a compile error** in some path not found by grep.
  Then the correct move is to keep `pub(crate)` on the offending item only and
  note it in the commit message — the deletion of the tool is the load-bearing
  part, not the visibility tidy.
- **LCV-054 stays `Done` in the backlog while its code is gone**, which would
  re-create exactly the docs/code contradiction this demand closes. The
  `demand-manager` handoff in Notes is mandatory, not optional.

## Open questions

*(none — demand is Ready)*

## Notes

### Handoff to `demand-manager` (LCV-108 closes it)

Once this demand ships, the following paperwork must follow — none of it is
`implementer-rust` scope:

- `docs/product/demands/LCV-054-offset-tool.md`: `Status: Done` → `Rejected`,
  with the reason "OFFSET is a v0.1.0 non-goal; tool removed by LCV-106" and the
  removal commit recorded. The body stays as the historical record.
- That file's Notes currently instruct a future agent to *remove* the
  `- No fillet / chamfer / offset.` line from `docs/product/README.md`. That
  instruction is **void**; the line stays.
- `docs/product/backlog.md` and `.claude/backlog.json`: move LCV-054 out of
  `Done` into `Rejected`, and fix its title, which still reads "Snap
  integration into all drawing tools" (a capability LCV-041 delivered).
- `CHANGELOG.md` must not advertise an OFFSET command in `[Unreleased]`.
- The snap-integration capability that LCV-054 was *supposed* to cover is
  already shipped (LCV-041, global snap resolve in the viewport); no new id is
  needed for it. LCV-041's out-of-scope note referring to "LCV-054 snap
  integration" should be read as already satisfied.

### Reference

- Shipping commit of the tool: `3c99f68` — *feat(LCV-054): OffsetTool — offset
  line/arc at fixed distance*. It also widened the two `hit.rs` items this
  demand reverts.
- AGENTS.md §"Product Philosophy": *"Prefer rejecting a feature over carrying
  accidental product complexity."*
