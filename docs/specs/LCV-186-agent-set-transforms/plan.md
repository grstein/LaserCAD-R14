# LCV-186 — Plan

## Approach

Additive, so the single-index paths stay byte-for-byte (AC8). The six schemas gain an optional
`indices` array and `index` leaves `required` (the parser enforces exactly one of the two).
`tools.rs::parse_tool_call` routes a call carrying `indices` to a new parser in
`tools/transform.rs` that shape-checks the list (1..=1000 non-negative integers, no duplicate,
not together with `index`, naming the offending entry — AC5) and builds one new variant
`AgentAction::Set { indices, op: SetOp }`. The op's own arguments reuse today's validators
(e.g. `factor > 0`), so a bad transform refuses before anything runs (AC6). The app side,
`agent_apply/set.rs`, range-checks every index against the live document (AC5), sorts
ascending, and commits ONE existing document command — `DeleteEntities`, `MoveEntities`,
`CopyEntities` or `TransformEntities(.with_keep_source)` already take `Vec<usize>` — so a set
is one action, one step (AC1), one base point/axis (AC2), copies appended in ascending source
order on the source's layer (AC3), and part of the turn's flat undo group (AC7, ADR 0007 §D12).
Identity rotate/scale on a set commits nothing, like the single path.

## Touches

- `src/agent/bridge/action.rs` — `AgentAction::Set { indices: Vec<usize>, op: SetOp }`, `enum SetOp`.
- `src/agent/tools/transform.rs` (new, kernel-pure) — `indices` parsing + `SetOp` building.
- `src/agent/tools.rs::parse_tool_call` — route `indices` calls; refuse `index` + `indices`.
- `src/agent/tools/schema.rs::base_definitions` — optional `indices` on the six tools.
- `src/app/agent_apply/set.rs` (new) — range check, sort, one command, narration (count +
  "later indices shifted" for delete, AC4); `src/app/agent_apply.rs::plan` — one arm.
- `src/agent/prompt.rs::DEFAULT_PROMPT` — the six paragraphs name `indices`.
- `AGENTS.md` — add `tools/transform.rs` to the kernel-pure list.
- ADRs: none (additive action variant; ADR 0007 §D2a/§D5 rules apply per index).

## Decisions (self-approved per user goal)

- A new `Set` variant rather than changing `index: usize` → keeps AC8 trivially true.
- Out-of-range is checked app-side (live count), duplicates/empty/shape parser-side.
- Refusal names the first offending entry, e.g. `delete_entity indices[3]: duplicate of indices[1]`.

## Risks

- LOC cap: `tools.rs` 263 → only a routing line there; logic in `tools/transform.rs`.
  `agent_apply.rs` 226 + 1 arm; `edit.rs` untouched.
- Mutation testing: yes — `src/agent/`; bounds (0, 1000, 1001), duplicate and ordering branches.
- `CopyEntities`/keep-source `TransformEntities` append in the order given → the ascending sort
  in `set.rs` is pinned by an AC3 test with `indices` given descending.
- Cross-spec overlap: `tools.rs`, `bridge/action.rs`, `prompt.rs` are edited again by LCV-187.
