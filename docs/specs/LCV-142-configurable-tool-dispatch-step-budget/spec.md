# LCV-142 - Larger tool budgets without losing turn undo

- **Status**: In Progress
- **Depends on**: LCV-123, LCV-125, LCV-129
- **Implementation**: -

## Problem

An agent turn may make at most 12 tool dispatches by default and 32 at most
(`u8`). That is too few to lay out a real cutting job — a panel with a
dozen holes and slots already runs out mid-drawing — and simply raising the
constant is unsafe: `History` evicts individual entries at depth 200, so a
turn of more than 200 commits would lose its oldest steps before the current
end-of-turn coalesce could fold them, and the operator's one-`Ctrl+Z` undo of
the agent's work would silently break. The turn therefore needs a larger
budget *and* a history shape that keeps one turn one undo entry at any size.

## Scope

- Budget constants, type and range per ADR 0007 §D13: `u32`, default 256,
  effective range 1..=4096, clamped at the read site.
- One flat history group per turn per ADR 0007 §D12, replacing §D6's
  end-of-turn coalesce.
- The fence's second witness and stop-after-first-refusal rule, ADR 0007 §D14.
- Malformed tool calls become refused `Act`s instead of failing the turn,
  ADR 0007 §D15.
- `TurnConfig` as the single owned value that crosses into the worker (§D13).
- Live `n of limit` progress in the agent panel while a turn is busy.
- The file splits ADR 0007 §D8 amendment (7) assigns to this demand.

## Out of scope

- Unlimited budgets, parallel tool execution, a history depth other than 200,
  nested groups, worker-held `Document`/`History` state, deferred application.
- Migrating stored budgets. A saved value is honoured verbatim (AC 2).
- Changing `io/settings.rs`' whole-file fallback for a value `u32` cannot
  represent (e.g. `-1`, `4294967296`); that existing behaviour stands.
- The system prompt (LCV-143), `create_drawing` (LCV-144), canvas capture
  (LCV-145). This demand only introduces the `TurnConfig` those extend.
- Any change to cancellation semantics: cancel still ends the turn at once on
  the UI side and never rolls back applied work; an in-flight blocking HTTP
  request still runs to its own timeout in the abandoned worker (LCV-129).

## Acceptance criteria

1. **Architecture gate — satisfied.** ADR 0007 amendment (7) (§D12–§D15, §D8
   rows) is the contract this demand implements. A constants-only change that
   raises the budget without §D12 must not ship.
2. **Budget type, default, range.** `AGENT_STEP_BUDGET_DEFAULT: u32 = 256`,
   `AGENT_STEP_BUDGET_MIN: u32 = 1`, `AGENT_STEP_BUDGET_MAX: u32 = 4096` in
   `src/agent/loop_.rs`; `Settings::agent_step_budget: u32` with a serde
   default of 256 written as a literal in `io/settings.rs`, pinned to the
   constant by a test. A settings file missing the field loads 256. **A stored
   value is honoured verbatim: an existing user whose file says `12` keeps 12**
   (product call: the file cannot distinguish a chosen 12 from a default that
   was written back, so we never overwrite an operator's number). The clamp to
   1..=4096 happens in `agent_turn` at the read site (`0 → 1`, `4097 → 4096`,
   `u32::MAX → 4096`). The Agent Settings `Steps per turn` control spans
   1..=4096; a stored in-range value (e.g. 4096) survives drawing the form
   unchanged, and a stored out-of-range value is written back in range and
   reported as a change (the existing `changed` rule). `CHANGELOG.md`'s
   `[Unreleased]` section states: new default 256, range 1–4096, existing
   saved values are kept and can be raised in Agent Settings.
3. **What a step is.** A step is one `Act` carrying a tool action — a query, a
   mutation, a refusal, a `Fenced` answer and a malformed call (AC 10) each
   count as one. An HTTP response, an entity and a success are not steps. The
   whole-batch preflight stays: a response whose tool calls would push the
   count past the limit is rejected with `IterationLimitExceeded` before any
   call of it is dispatched.
4. **Exhaustion and progress.** After a batch that lands exactly on the limit,
   exactly one more completion request is sent; text ends the turn `Done`,
   tool calls end it `Failed` with `step budget exceeded (N tool calls per
   turn)` where N is the effective limit, and none of them is dispatched.
   While `agent.busy`, the panel's thinking row reads exactly
   `Thinking… {n} of {limit} steps` where `n` is the count of step `Act`s the
   UI has received this turn and `limit` is the turn's snapshotted effective
   limit; the count is kept UI-side in `TurnState`, with no progress event.
5. **Flat group.** `History` gains `begin_group(label)`, `commit_grouped`,
   `end_group()` and `group_open()` with the semantics of ADR 0007 §D12.
   `arm_turn` begins the group labelled with the existing `Agent: <prompt>`
   label; every agent mutation is applied immediately through
   `commit_grouped` (document changes and `revision` bumps by one at once);
   `finish_turn` calls `end_group()` exactly once per turn. `coalesce_last`,
   `TurnFence::may_coalesce` and the fence's `start` field are deleted.
   `can_undo` / `len` count a non-empty open group as one entry. No
   `Document` clone, no composite grown per step, no nesting.
6. **More than 200 steps undo as one.** With 200 prior human entries on the
   stack, a turn committing 300 agent mutations leaves `len() == 200`: one
   `Ctrl+Z` removes all 300 and restores the pre-turn drawing, one `Ctrl+Y`
   re-applies all 300, and of the 200 prior entries exactly the oldest one was
   evicted.
7. **Sealing and chronology.** A foreign create/delete, a selection command,
   an Undo and a Redo during a turn each seal the group before touching the
   stack, and the fence trips (sticky). The turn's work so far is exactly one
   entry directly beneath the foreign event; no foreign work is absorbed into
   it, reordered or replayed. Document replacement (`File > New`,
   `File > Open`) drops the group with the old `History`, and the next agent
   `Act` is `Fenced` — **including a turn armed at revision 0 on a freshly
   opened file, where the revision alone would still read `0 == 0`**.
8. **One finalization per exit.** `Done`, `Failed` (including limit and the
   §D14 stop), channel disconnect, a failed `reply.send` and `cancel_turn` each
   finalize the turn exactly once through `end_turn` → `finish_turn`;
   `agent.busy` clears on every one. Applied work stays applied and undoable
   as one entry; cancel is not rollback.
9. **Fence stop (§D14).** `TurnFence::check(revision, group_open)` trips when
   the revision moved or no group is open. The first refused action is
   answered `AgentOutcome::Fenced(AGENT_FENCE_REFUSAL)` (`is_refused()` is
   true; transcribed with role `refused`). On `Fenced` the worker dispatches
   nothing further: every remaining call of that batch receives the tool
   result `not run: the turn stopped after the drawing changed outside it`,
   so every `tool_call_id` is answered; then exactly one more completion is
   sent. Text → `Done(text)`. Tool calls → not dispatched, the turn ends
   `Failed` with a new `AgentError` variant whose message is exactly
   `the turn stopped after the drawing changed outside it`.
10. **Malformed calls (§D15).** A JSON syntax error in the arguments, an
    unknown tool name or any `ToolCallError` becomes
    `AgentAction::Malformed { tool, reason }`, sent as an ordinary `Act`. The
    UI answers `Refused(reason)` without reading or mutating the document,
    transcribes it (role `refused`, same text the model reads), counts it as
    one step, and the turn continues. It passes the fence like any action, so
    after a trip it is `Fenced`. `reason` names the tool and the field and
    never contains the raw argument string. A syntax error reads
    `tool \`{tool}\` arguments are not valid JSON: {serde error}`; the other
    reasons are the existing `ToolCallError` messages.
11. **`TurnConfig`.** Everything that crosses into the worker is one owned
    `TurnConfig` (endpoint, API key, model, effective step limit), built once in
    `start_turn` from `Settings`. `run_agent_turn` takes it instead of separate
    endpoint/key/model/budget parameters. Settings edited mid-turn affect the
    next turn only. `TurnConfig`'s `Debug` output never contains the API key:
    it is a manual impl printing `api_key: "<redacted>"`.
12. **End-of-turn note.** Derived from `end_group()`'s report, never from the
    fence alone. When `finish_turn`'s own `end_group()` sealed all `applied`
    commands: `Applied 1 action — Ctrl+Z undoes it.` /
    `Applied {n} actions — Ctrl+Z undoes the whole turn.` Otherwise (the group
    was sealed earlier by a foreign event or dropped by document replacement):
    `Applied 1 action before the drawing changed outside this turn.` /
    `Applied {n} actions before the drawing changed outside this turn.` —
    no undo claim. A turn that applied nothing writes no note. The old
    "they stay {n} separate undo steps" sentence is deleted.
13. **File splits (ADR 0007 §D8, amendment 7; field paths per amendment 8).**
    `run_agent_turn`, `ask_ui` and `TurnConfig` move to a new
    `src/app/agent_worker.rs`; `src/app/agent_turn.rs` keeps `TurnFence`,
    `TurnState`, `arm_turn`, `start_turn`. `AgentState`'s (`src/app/agent_state.rs`)
    `fence`, `applied` and `turn_label` fields are replaced by one field
    `pub turn: TurnState`, reached as `app.agent.turn` (fence, applied count,
    dispatched-step count, snapshotted limit); `busy` and `rx` stay direct
    fields of `AgentState` (`app.agent.busy`, `app.agent.rx`); this demand
    does not touch `src/app/mod.rs`. If this demand takes `src/io/settings.rs`
    above 270 implementation LOC, `platform_path` / `load_from` / `save_to`
    and the `.bak` logic move to `src/io/settings_store.rs`. Every touched
    file stays ≤ 300 implementation LOC by the ADR 0004 recipe.

## Expected tests

All tests are network-isolated: none calls `start_turn` against a parseable
endpoint; worker-side tests use the fake `send_fn`/`ask` seams, UI-side tests
drive `arm_turn` and push `AgentEvent`s by hand.

- AC 1: review — the diff implements §D12–§D15; no test.
- AC 2: `loop_.rs` unit — constants are 256/1/4096; `clamp_step_budget` over
  0/1/12/32/33/256/4096/4097/`u32::MAX`. `settings.rs` unit — missing field
  → 256 and the literal default equals `AGENT_STEP_BUDGET_DEFAULT`; a file
  holding `12` and one holding `4096` load unchanged. `settings_ui.rs` unit —
  `draw_agent_settings` on a stored 4096 leaves it 4096 and returns
  `changed == false` (breaks if the control's max stays 32); on a stored 5000
  it writes 4096 and returns `true`. CHANGELOG line: review.
- AC 3: `loop_.rs` unit with a fake `send_fn` — budget 300: batches of 100,
  100, 100 all dispatch; a fourth batch of 1 is `IterationLimitExceeded` with
  zero dispatches from it. Budget 5, one batch of 6 → zero dispatches.
  A turn mixing queries, refusals and one malformed call counts each as one.
- AC 4: `loop_.rs` unit — exact exhaustion followed by text → `Ok`; followed by
  tool calls → `IterationLimitExceeded(limit)`, `send_fn` called exactly once
  after exhaustion, dispatch count unchanged. Harness paint test
  (`tests/harness/paint.rs`): arm a turn with limit 256, push 3 step `Act`s,
  run a frame, assert `Thinking… 3 of 256 steps` is painted.
- AC 5: `history.rs` unit — `commit_grouped` bumps revision per call and
  applies immediately; `end_group` seals 0 / 1 / n into nothing / bare /
  one composite and is idempotent; `len`/`can_undo` see an open non-empty
  group as one entry; `commit`, `undo`, `redo` seal first.
- AC 6: integration (`tests/lcv142_*.rs`) — 200 human commits, arm, 300
  `CreateLine` `Act`s, end; assert `len() == 200`, one undo restores the
  pre-turn entity list, one redo restores 300 lines, and 199 further undos
  succeed while the 200th fails (oldest evicted).
- AC 7: integration — mid-turn foreign create, delete, selection command,
  Undo and Redo, one test each: the next `Act` is `Fenced`, entity list and
  undo order prove the turn's work is one entry beneath the foreign one.
  **Revision-0 replacement:** on a fresh `App` (revision 0) arm a turn, then
  `action_open_path` a tempdir-owned file (ADR 0006) holding ≥ 1 entity; the
  next `DeleteEntity { index: 0 }` `Act` is `Fenced`, the opened file's
  entities are untouched and revision is still 0. Deleting the `group_open`
  witness must make this test fail.
- AC 8: integration — after 250 applied `Act`s, each of `Done`, `Failed`,
  dropped sender, dropped reply receiver and `cancel_turn` leaves
  `agent.busy == false`, `agent.rx == None`, exactly one note row, and one
  undo entry covering all 250. Existing `tests/lcv129_agent_timeout_and_cancel.rs`
  and `tests/lcv123_agent_turn.rs` stay green.
- AC 9: `agent_worker.rs` unit with fake `send_fn`/`ask` — a batch of 3 whose
  first `ask` returns `Fenced`: `ask` is called once; the message list holds 3
  tool results, the last two exactly the placeholder; `send_fn` is called
  exactly once more; final text → `Ok`, final tool calls → the new variant
  with its exact message and no further `ask`. `TurnFence` unit — trips on
  revision move, on `group_open == false`, and stays tripped.
- AC 10: `agent_worker.rs` unit — invalid JSON, unknown tool and a missing
  field each reach `ask` as `Malformed` and the turn continues to `Ok`; the
  reason never contains the raw argument string (test with a sentinel
  string inside the arguments). Integration — a `Malformed` `Act` answers
  `Refused`, leaves revision and entities unchanged, adds one `refused` row,
  increments the step count; after a trip it answers `Fenced`.
- AC 11: `agent_worker.rs` unit — `format!("{config:?}")` with key
  `sk-test-DO-NOT-LEAK` does not contain the key and contains `<redacted>`.
  Unit on the `TurnConfig` builder — built from settings with budget 7, then
  settings changed to 9: the config still says 7, and the armed `TurnState`
  limit is 7 (harness paint test from AC 4 variant shows `of 7 steps`).
- AC 12: `agent_poll.rs` unit — the four sentences pinned character for
  character; integration — a clean 3-action turn gets the "whole turn" note, a
  turn sealed by a foreign commit and a turn whose document was replaced each
  get the neutral note (fails if the note is derived from the fence).
- AC 13: source scan in `tests/lcv142_*.rs` — `fn run_agent_turn`,
  `fn ask_ui` and `struct TurnConfig` are defined in `src/app/agent_worker.rs`
  and not in `agent_turn.rs`; `src/app/agent_state.rs` declares
  `pub turn: TurnState` and none of `pub fence:`, `pub applied:`,
  `pub turn_label:`, with a positive control on `pub busy:`. LOC: review with
  the ADR 0004 recipe.

## Open questions

None. The architecture gates are closed by ADR 0007 amendment (7); the stored
budget call is made in AC 2.

## Notes

Primary files: `src/agent/loop_.rs`, `src/agent/bridge.rs`,
`src/agent/panel.rs`, `src/agent/settings_ui.rs`, `src/io/settings.rs`,
`src/document/history.rs`, `src/app/agent_turn.rs`, new
`src/app/agent_worker.rs`, `src/app/agent_poll.rs`, `src/app/agent_apply.rs`,
`src/app/agent_state.rs`. Implementation order: LCV-142 first, then LCV-143,
LCV-144, LCV-145 — all three extend `TurnConfig`, §D12's group or §D15's
`Malformed` path. LCV-144's batch is one step and one command in the group,
never one per entity.

## Architecture decision

Recorded 2026-09-27 in ADR 0007 amendment (7) (satisfies AC 1's gate):
§D12 flat history group (supersedes §D6), §D13 `u32` budget 256 / 1..=4096 and
`TurnConfig` (supersedes §D7's constants), §D14 second fence witness
(`History::group_open()`) and stop-after-first-fence-refusal (amends §D4),
§D15 malformed tool calls become `Refused` `Act`s (amends §D2a's routing),
§D8 seams (`src/app/agent_worker.rs` split, `app.agent.turn: TurnState` on
`AgentState`).

> **Pointer (architect, 2026-09-27) — resolved by product-owner, 2026-09-27:**
> ADR 0007 amendment (8) restates the `TurnState` field path against
> LCV-136's `AgentState`: it is `app.agent.turn` (a field of `AgentState` in
> `src/app/agent_state.rs`), not `App::agent_turn`; `busy` / `rx` stay direct
> `AgentState` fields; this demand does not touch `src/app/mod.rs`. Item 13,
> AC 13's scan and the primary-file list have been reconciled to those paths
> above.
