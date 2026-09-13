# LCV-128 — `AGENTS.md`'s normative per-file and per-symbol enumerations have no scan

- **Status**: Draft
- **Phase**: 12
- **Depends on**: —
- **Suggested agent**: product-owner (refinement) → implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`AGENTS.md` carries a handful of enumerations that are **normative per entry**:
each name in the list carries a rule *for that name*. They are not orientation
and not prose — they are the whole statement of an invariant, and in several
cases they are the only statement of it. Nothing enforces any of them. A new
file, or a new assignment site, silently escapes the list, and the omission is
silent in exactly the direction that matters: a name missing from the list is a
name with no stated rule, so the next person writing it has nothing to violate.

Two instances are known, found a day apart, and they are the reason this demand
is scoped to the *class* rather than to one list.

### Instance 1 — the `src/agent/` purity buckets

`AGENTS.md` §"Purity rule" classifies `src/agent/` file by file into two
buckets: the files that may import `egui` (`panel.rs`, `settings_ui.rs`) and the
files that are kernel-pure (`classifier.rs`, `wire.rs`, `transport.rs`,
`tools.rs`, `bridge.rs`, `loop_.rs`). A new file can be added under
`src/agent/` without appearing in either bucket, and nothing fails.

It has already happened once. `src/agent/bridge.rs` landed with LCV-122 on
2026-09-13 and sat unclassified until a human noticed by hand; the purity rule
names it only because someone went looking.

### Instance 2 — the `agent_busy` / `agent_rx` single-writer claim

`AGENTS.md` §"Event flow" → Repaint policy now states that `agent_busy` is set
in exactly one place (`src/agent/panel.rs::submit`) and cleared in exactly one
(`src/app/agent_poll.rs::end_turn`). That sentence is load-bearing: it is what
makes the `if self.agent_busy` repaint guard a guard rather than a latch, and it
is the whole argument that LCV-120 stays closed. **Nothing enforces it.** During
the LCV-122 review the reviewer planted `app.agent_busy = false;` and
`app.agent_rx = None;` in `src/agent/panel.rs` and **both survived the entire
suite**.

The neighbouring claim in that same paragraph — the three repaint call sites and
their guards — *is* scanned, by `every_repaint_request_in_src_is_conditional` in
`src/app/viewport.rs`. The busy-flag half is the only unscanned sentence left in
it.

### Why one demand and not two

Both scans read the same source tree with the same machinery, and one of them
(instance 2) would otherwise be a near-duplicate file opened a day later. They
share a home and most of a mechanism, so they are cheaper together. Splitting
them into LCV-128 and LCV-129 was considered and rejected on that ground.

### What this demand does not reopen

The module tree next door in the same document is **not** in scope. The
architect declared it explicitly non-exhaustive on 2026-09-13: the tree is
orientation, so a gap there is cosmetic, and making it scannable would first
mean making it exhaustive — roughly seventy names duplicating what `ls` already
answers, churning on every new file. That decision is right and stands. The
distinction that admits a list into this demand is that the list is normative
*per entry*.

## Proposed shape

Two tests in one file, sharing the tree walk. They belong alongside the existing
tree-wide scans in `tests/lcv121_source_scans.rs` and
`tests/lcv122_source_scans.rs` — the latter already has every piece of machinery
either one needs: `rs_files`, `implementation_or_all`, `files_containing`, the
comment-line skip, and the `concat!` discipline. The architect estimates a dozen
lines for the first; the reviewer's sketch for the second is about the same.

**Scan A — every file under `src/agent/` is named in a bucket.** Fail when a
file exists under `src/agent/` and is named in neither the egui-allowed bucket
nor the kernel-pure bucket of `AGENTS.md` §"Purity rule".

**Design constraint that must survive refinement — the non-obvious part.** Scan
A is **one-directional only**. It fails when a file exists and is unlisted. It
must **not** fail when the list names a file that does not exist yet: the list
names `classifier.rs`, which LCV-124 has not created. That forward commitment is
deliberate and is a feature, not drift. A symmetric "list == directory" equality
assertion is the wrong test and would have to be deleted the moment it was
written.

**Scan B — the busy-flag single-writer claim.** The reviewer sketched it
concretely and it is worth keeping verbatim:

- `concat!("agent_busy", " = false")` appears in **exactly one** file under
  `src/`, and that file is `agent_poll.rs`;
- `concat!("agent_rx", " = None")` appears in exactly one file, allowing
  `init.rs` as the one startup exception (it seeds the field on construction);
- positive control, so the scan cannot pass vacuously: `agent_busy = true`
  appears in exactly one file and that file is `panel.rs`.

One detail the implementer should not have to rediscover: the needle is the
`= false` / `= None` **clear**, not any assignment. `agent_rx = Some(…)` legally
has two writers today — `panel.rs::submit` arms it, and `agent_poll.rs` puts the
receiver back on the `TryRecvError::Empty` path — so a scan written against
"assignments to `agent_rx`" fails on `main` on its first run for the wrong
reason.

## Traps

Two failure modes this repository has already paid for. Both must be addressed
in the acceptance criteria, not left for the implementer to rediscover. They
apply to both scans.

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
   in LCV-116 and again in LCV-120; `AGENTS.md` §Implementation Rules now
   carries the rule. Sort on the rendered string too.

Scan B carries both traps in full, because it names two files by name
(`agent_poll.rs`, `panel.rs`) and asserts on literals that appear in its own
source.

## Open questions for refinement

- **`src/agent/mod.rs` is today named in neither bucket.** The eight files
  present are `bridge.rs`, `loop_.rs`, `mod.rs`, `panel.rs`, `settings_ui.rs`,
  `tools.rs`, `transport.rs`, `wire.rs`; the purity rule names all of them
  except `mod.rs`. So scan A as described above fails on `main` on its first
  run. That is a real question, not a typo: `mod.rs` imports no `egui` but does
  re-export `panel::draw_agent_panel`. Refinement must decide whether `mod.rs`
  is exempt by name or joins the kernel-pure list — and, if it is exempt, say
  so in `AGENTS.md` so the exemption is stated rather than implied.
- Whether scan A reads `AGENTS.md` at runtime or carries its own copy of the
  two lists, and which of the two existing scan binaries the pair joins.
- Whether the same argument applies to the remaining per-file rules in the
  purity section (only `transport.rs` may import `reqwest`; `panel.rs` never
  spawns a thread and never constructs a `Document` or a `History`) — the last
  of those is partly covered by
  `tests/lcv122_source_scans.rs::no_agent_file_but_panel_holds_document_state`,
  and refinement should say which of the rest are already pinned and which are
  not, rather than assume.
- Whether scan B should also pin the claim one clause earlier in the same
  sentence — that `end_turn` is what *every* terminal path of a turn tail-calls
  (ADR 0007 §D11 enumerates four exits). That is a stronger property than
  counting assignment sites and may not be reachable by a source scan at all.

## Notes

- Origin, instance 1: `architect`, 2026-09-13, while fixing documentation drift
  in the `AGENTS.md` module tree. Asked whether the tree deserved a scan, they
  argued no and declared it non-exhaustive instead, then pointed at the purity
  enumeration as the one that genuinely earns one.
- Origin, instance 2: `reviewer-rust`, 2026-09-13, during the LCV-122 review.
  They found the second instance of exactly this problem and recommended
  widening this demand rather than opening LCV-129; `project-manager` agreed and
  directed the widening.
- **Not part of Marco 2.** Must not be driven ahead of LCV-123, LCV-124 or
  LCV-125. No dependencies — it can be done at any time after them.
- The file name still reads `agent-purity-list-has-no-scan`; it records the
  original, narrower framing and was deliberately left alone so existing links
  keep resolving. The title is the scope.
- Body is deliberately thin on acceptance: opened and widened by
  `demand-manager` for registration. `product-owner` writes Scope / Out of scope
  / Acceptance criteria / Expected tests.
- Related: `AGENTS.md` §Purity rule and §Event flow → Repaint policy,
  [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md) §D8 (the
  file table the purity rule mirrors) and §D11 (the four turn exits),
  `tests/lcv121_source_scans.rs`, `tests/lcv122_source_scans.rs`,
  `src/app/viewport.rs::every_repaint_request_in_src_is_conditional`,
  `src/io/dialogs.rs:179`.
