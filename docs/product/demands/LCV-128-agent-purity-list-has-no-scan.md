# LCV-128 — Every file in `src/agent/` needs a stated purity status

- **Status**: Draft
- **Phase**: 12
- **Depends on**: —
- **Suggested agent**: product-owner (refinement) → implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`AGENTS.md` §"Purity rule" (lines 81–97) classifies `src/agent/` file by file
into two buckets: the files that may import `egui` (`panel.rs`,
`settings_ui.rs`) and the files that are kernel-pure (`classifier.rs`,
`wire.rs`, `transport.rs`, `tools.rs`, `bridge.rs`, `loop_.rs`). A new file can
be added under `src/agent/` without appearing in either bucket, and nothing
fails.

It has already happened once. `src/agent/bridge.rs` landed with LCV-122 on
2026-09-13 and sat unclassified until a human noticed by hand; `AGENTS.md:95`
names it only because someone went looking.

The reason this enumeration deserves a guard is that it is **normative per
entry**: each name carries a rule *for that file*. A file missing from it is a
file with no stated purity status, so the next person writing it has nothing to
violate — the omission is silent in exactly the direction that matters. This is
the distinction that separates it from the module tree next door in the same
document, which the architect declared explicitly non-exhaustive on the same
day: the tree is orientation, so a gap there is cosmetic, and making it
scannable would first mean making it exhaustive — roughly seventy names
duplicating what `ls` already answers, churning on every new file. That
decision is right and this demand does not reopen it.

## Proposed shape

A test that fails when a file under `src/agent/` is named in neither the
egui-allowed bucket nor the kernel-pure bucket of `AGENTS.md` §"Purity rule".
It belongs alongside the existing tree-wide scans in
`tests/lcv121_source_scans.rs` and `tests/lcv122_source_scans.rs`. The
architect estimates a dozen lines.

**Design constraint that must survive refinement — the non-obvious part.** The
scan is **one-directional only**. It fails when a file exists and is unlisted.
It must **not** fail when the list names a file that does not exist yet: the
list names `classifier.rs`, which LCV-124 has not created. That forward
commitment is deliberate and is a feature, not drift. A symmetric
"list == directory" equality assertion is the wrong test and would have to be
deleted the moment it was written.

## Traps

Two failure modes this repository has already paid for. Both must be addressed
in the acceptance criteria, not left for the implementer to rediscover.

1. **The self-matching scan — wrong six separate times here.** A test that
   scans source text with `include_str!` matches its own source and passes
   vacuously. The fix is to bound the haystack at the offset of a bare
   `#[cfg(test)]` at column 0 and to build every needle with `concat!`. The
   canonical correct example is `guard_is_runtime_not_cfg` at
   `src/io/dialogs.rs:179`. A scan whose haystack is `AGENTS.md` rather than
   `src/` may sidestep this entirely — if so, the demand should *say* it
   sidesteps it and why, rather than leave the next person to work it out.
   Either way, a positive control is required: an absence assertion over an
   empty haystack passes for the wrong reason.

2. **The path separator.** Any scan comparing a path against a literal must
   rebuild the path from `components()` joined with `/` — never
   `Path::display()` or `to_string_lossy()` on a whole path, which emit `\` on
   Windows and pass on Linux and macOS while failing only in CI. That broke CI
   in LCV-116 and again in LCV-120; `AGENTS.md` §Implementation Rules (line
   185) now carries the rule. Sort on the rendered string too.

## Open questions for refinement

- **`src/agent/mod.rs` is today named in neither bucket.** The eight files
  present are `bridge.rs`, `loop_.rs`, `mod.rs`, `panel.rs`, `settings_ui.rs`,
  `tools.rs`, `transport.rs`, `wire.rs`; the purity rule names all of them
  except `mod.rs`. So the scan as described above fails on `main` on its first
  run. That is a real question, not a typo: `mod.rs` imports no `egui` but does
  re-export `panel::draw_agent_panel`. Refinement must decide whether `mod.rs`
  is exempt by name or joins the kernel-pure list — and, if it is exempt, say
  so in `AGENTS.md` so the exemption is stated rather than implied.
- Whether the scan reads `AGENTS.md` at runtime or carries its own copy of the
  two lists, and which of the two existing scan binaries it joins.
- Whether the same argument applies to the two neighbouring per-file rules in
  the same section (only `transport.rs` may import `reqwest`; `panel.rs` never
  spawns a thread), or whether those are already covered.

## Notes

- Origin: `architect`, 2026-09-13, while fixing documentation drift in the
  `AGENTS.md` module tree. Asked whether the tree deserved a scan, they argued
  no and declared it non-exhaustive instead, then pointed at this enumeration
  as the one that genuinely earns one.
- **Not part of Marco 2.** Must not be driven ahead of LCV-123, LCV-124 or
  LCV-125. No dependencies — it can be done at any time after them.
- Body is deliberately thin: opened by `demand-manager` for registration.
  `product-owner` writes Scope / Out of scope / Acceptance criteria / Expected
  tests.
- Related: `AGENTS.md` §Purity rule, [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D8 (the file table the purity rule mirrors), `tests/lcv121_source_scans.rs`,
  `tests/lcv122_source_scans.rs`, `src/io/dialogs.rs:179`.
