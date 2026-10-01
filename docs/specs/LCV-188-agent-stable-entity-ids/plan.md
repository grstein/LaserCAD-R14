# LCV-188 — Plan

## Approach

Ids are one more private per-entity vector in `Document`, kept in lockstep like layers (ADR 0012 §2),
plus a counter that only ever goes up (ADR 0014). A push takes a fresh id; an in-place write keeps
it. Creating commands replay their ids on redo through an `IdLedger`, and a delete captures the id
and puts it back on undo. `History` does not change. Nothing is persisted: SVG, autosave and
`SCHEMA_VERSION` stay as they are. File > New and Open continue the counter, so a stale id is
refused and never lands on the wrong entity. On the agent side, `id`/`ids` (strings `"e<N>"`) parse
into `AgentAction::ById { ids: Vec<u64>, op: SetOp }`. The apply site resolves them to indices and
hands them to LCV-186's set path unchanged, so AC4 holds by construction. Lands after 186, 187,
192, 191, 190 and 193.

## Touches

- `src/document/state.rs` — fields `entity_ids`, `next_id`; the four mutators; `from_parts` numbers
  `e1..=en`. The layer-edit `impl` block moves to `src/document/state/layers.rs` (seam, T1).
- `src/document/state/ids.rs` (new) — `EntityId` (`Display` `e<N>`), `IdLedger`, `entity_id`,
  `index_of`, `push_entity_as`, `ids_after`. `src/document/mod.rs` — `pub use`.
- `src/document/commands/create.rs` — ledger in `CreateLine/Circle/Arc`, `CreateEntities`.
  `commands/edit.rs` — `DeleteEntities` captures the id; ledger in `CopyEntities`.
  `commands/transform.rs` — ledger for `keep_source`.
- `src/io/file_actions.rs::{action_new, action_open, action_open_path}` — `ids_after(&app.document)`.
- `src/agent/bridge/action.rs` — `ById` variant; `tool_name` arm (LCV-192).
- `src/agent/tools/transform.rs` — parse `id`/`ids`. `src/agent/tools.rs::parse_tool_call` — route
  it, and allow only one of `index|indices|id|ids`. `tools/schema.rs` — `id`/`ids` on the 7 tools.
  `tools/args.rs::expected_form` (LCV-192) — `id`, `ids` rows.
- `src/app/agent_apply.rs::{plan, apply}` — `ById` arm; the new-ids suffix after an appending commit.
  `agent_apply/set.rs` — `resolve(ids, doc)`. `agent_narrate.rs` — `list_entities` id column, `new_ids`.
- `src/agent/prompt.rs::DEFAULT_PROMPT` — ids paragraph.
- ADRs: **ADR 0014** (new; amends ADR 0007 §D5/§Deferred and ADR 0010 §7). The header pointer lines
  in 0007 and 0010 are added at landing (T19), with the next free amendment numbers.

## Decisions (self-approved per user goal)

- A new ADR 0014, not a numbered 0007 amendment. The decision is mostly about the `document/` model,
  0007 is 1080 lines, and LCV-192/193 have also planned 0007 amendments. The same pattern as ADR
  0010/0011 extending 0007.
- "Session" means one app run. `Document::default()` and autosave restore start at `e1`; New/Open
  continue the counter.
- On the wire an id is only the string `"e<N>"`, N ≥ 1. A bare integer is refused, so an id can
  never be read as an index.
- `id` is a one-entry `ids`, so it uses the set path and its narration (AC4: "as with the matching
  indices"). `set_layer` takes `indices` or `ids`, exactly one.
- `ids` has the same shape rules as `indices`: 1..=1000, no duplicates, naming the offending entry.
  One unknown id refuses the whole call (AC5).
- AC6 covers every commit that appends entities: create_*, create_drawing, copy, mirror/rotate/scale
  keeping the source. Suffix: ` New id: e7.` / ` New ids: e7..=e9.`
- Ids are agent-only: `query_selection`, the operator UI and the command line don't change.

## Risks

- LOC cap: `document/state.rs` 276 → T1 seam (−~75) before anything is added. `bridge/action.rs`
  ~259 after 193 (+~7): if it passes 270, move `SetOp` to `bridge/set_op.rs` (kernel-pure; add to the
  AGENTS.md list). `tools/transform.rs` ~167 after 191 (+~40): if it passes 270, `tools/ids.rs`.
  `agent_apply.rs` ~235 (+~10). `history.rs` 272: not touched.
- Mutation testing: **yes**. `src/agent/` (the `e<N>` shape, bounds, mixing), the ledger
  (first `do_` vs redo), the delete capture order, and `ids_after`.
- Creating commands that exist now or are added later (v0.4 LCV-160..163 in another worktree) must
  use `IdLedger`, or redo hands out new ids. After the rebase, T7's redo test gets a case for each of
  them.
- Signature change: `remove_entity`/`insert_entity` have 5 call sites. It is done in one task, so
  every commit compiles.
- Test churn: `list_entities` exact strings (`tests/it/agent/*`, `agent_apply/tests.rs`) gain `eN`.
  They are updated in T11. Outcomes of appending commits gain the suffix; they are updated in T17.
- ADR amendment numbering: LCV-192/193 plan to amend ADR 0007 as "(11)". T19 takes whatever number
  is free when 188 lands.
