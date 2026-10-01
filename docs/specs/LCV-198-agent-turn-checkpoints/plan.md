# LCV-198 — Plan

## Approach

A checkpoint is a count: how many commands the turn's open flat group (ADR 0007 §D12) holds when
the checkpoint is set. `History` gains `group_len()` and `rewind_group(mark, doc)`. The rewind
undoes the group's commands after `mark` in reverse, drops them, and bumps `revision` once if it
undid any. The turn keeps a small `Checkpoints` list (name → mark, `start` = 0 built in) in
`TurnState`. That state is rebuilt by every `arm_turn`, which gives AC9 for free. `checkpoint` and
`rollback` are two new tools, answered in `agent_apply::apply` beside `CaptureCanvas` because they
need `&mut App`. They go through `answer_act` → `apply_fenced` like every call. So a fenced turn
refuses them (AC8), and a rollback, which moves the revision, re-anchors the fence and counts as
applied (spec decision). `end_group` already seals zero, one or a composite (AC6). Ids come back
through the LCV-188 command semantics: a create's undo retires its id, and a delete's undo
restores it (AC3). Lands after LCV-188 and LCV-190.

## Touches

- `src/document/history.rs` — the group `impl` block moves out (seam, T1).
- `src/document/history/group.rs` (new, kernel) — `begin_group`, `commit_grouped`, `end_group`,
  `group_open` (moved); new `group_len() -> usize` and `rewind_group(mark, doc) -> usize`.
- `src/agent/bridge/action.rs` — `Checkpoint { name: String }`, `Rollback { name: String }`;
  `tool_name` arms.
- `src/agent/tools.rs::parse_tool_call` — `"checkpoint"`, `"rollback"`. `name` is a required
  string; its shape is checked at apply time so the refusal can list the known checkpoints
  (AC7). `src/agent/tools/schema.rs` — the two schemas. `tools/args.rs::expected_form` — rows.
- `src/app/agent_checkpoint.rs` (new) — `Checkpoints` (`set`, `target`, `truncate_after`,
  `known`), `valid_name`, `checkpoint(app, name)` and `rollback(app, name)` → `AgentOutcome`.
- `src/app/agent_turn.rs::TurnState` — `checkpoints: Checkpoints` field (pub(crate)).
- `src/app/agent_apply.rs::apply` — two arms delegating to `agent_checkpoint`.
- `src/agent/prompt.rs::DEFAULT_PROMPT` — one sentence: set a checkpoint before a risky step and
  roll back instead of deleting by hand.
- ADRs: ADR 0007 §D12/§D14, next free "Amended (n)" (text below, appended in T8).

## Decisions (self-approved per user goal)

- Replies. Set: `Checkpoint <name> set at <k> changes.` Rollback: `Rolled back to <name>: <k>
  changes undone, <n> entities.` (AC2 verbatim). A rollback with k = 0 succeeds and does not move
  the revision.
- Refusals (`AgentOutcome::Refused`, LCV-192 wording) end with
  ` Known checkpoints: start, a, b.` The three refusals:
  - bad shape: `checkpoint name "<x>" must be 1–32 characters of A–Z a–z 0–9 _ -.`
  - unknown: `no checkpoint named "<x>".`
  - `checkpoint start`: `"start" is built in and cannot be set.`
- Outside a turn (no group open), both tools are refused with `No agent turn is open.`, which
  covers `apply` called directly. Names are case-sensitive.

## ADR amendment

> **Amended (n), <date> (LCV-198).** §D12: `History` gains two group operations.
> `group_len()` is the number of commands in the open group, 0 when none is open.
> `rewind_group(mark, doc)` undoes, in reverse order, every command of the open group past
> `mark`, drops them, leaves redo empty and bumps `revision` once when it undid at least one.
> It returns how many it undid, and is a no-op when no group is open or `mark ≥ group_len()`.
> The turn uses it for `rollback` to a checkpoint (a group mark it keeps in `TurnState`). A
> rewind is the turn's own change, like `commit_grouped`. It does not seal the group, and §D14's
> fence advances to the new revision as after any applied call, so it re-arms nothing. A
> rolled-back command is gone: there is no redo of a rollback. `end_group` then seals only the
> survivors, so one turn is still at most one undo entry, and no entry when nothing survived.
> Rollback never reaches past the group's start, so it never touches the undo stack below.

## Risks

- LOC cap: `history.rs` 272 → T1 moves the group block (~70 LOC) to `history/group.rs` before
  anything is added. `bridge/action.rs` ~259 after 188 (+~8 here): if it passes 270, apply
  LCV-188's planned seam (`SetOp` → `bridge/set_op.rs`). `agent_apply.rs` ~255 after 188 (+~6),
  with all logic in `agent_checkpoint.rs`. `agent_turn.rs` 250 (+~3).
- Mutation testing: **yes**. Run `scripts/mutants.sh` on `History` (`rewind_group` order, the
  single revision bump, the `mark ≥ len` guard) and the `src/agent/` parse arms.
- The reverse order is load-bearing: a move undone before its create panics or leaves
  garbage. A rewind test with create → move → delete pins it by comparing entities and ids
  with the checkpoint.
- The fence and the group: a human Undo seals the group (§D12), so a later rollback must answer
  `Fenced`, never rewind a sealed group. The AC8 test does an operator Undo and then a rollback.
