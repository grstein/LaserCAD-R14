# LCV-142 - Larger tool budgets without losing turn undo

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-123, LCV-125, LCV-129
- **Suggested agent**: architect
- **Suggested model**: opus
- **Implementation**: -

## Problem

The current default is 12 tool dispatches, capped at 32 with `u8` storage.
Raising the limit alone loses undo commands: history evicts individual entries
at depth 200 before the agent's end-of-turn coalescing.

## Scope

Default 256, configurable 1..4096 actual tool dispatches, with safe whole-turn
undo, live progress, cancellation and existing revision fencing.

## Out of scope

Unlimited execution, parallel tools, larger normal history depth, worker-owned
Document/History snapshots, deferred application or recursively nested groups.

## Acceptance criteria

1. Architect approves and records the revised ADR 0007 D4/D6/D7 contract before implementation. A constants-only increase must not ship.
2. Persisted/runtime budget types use `u32`; default is 256 and effective range 1..=4096. Missing legacy fields use the new default; stored valid old budgets remain unchanged. Clamp representable out-of-range values at the existing enforcement boundaries.
3. A step is one admitted tool dispatch, including queries and refusals, not an HTTP response, entity or successful mutation. Preserve whole-response preflight: reject an oversized tool-call batch before dispatching any of it.
4. After exact budget exhaustion, allow the existing final completion request but no further tool dispatch. Show dispatched/effective-limit progress while busy.
5. Each mutation still applies immediately through commands/history and advances revision. Accumulate the turn's owned commands in one flat history group before individual steps can be evicted; do not clone the drawing or repeatedly nest composites.
6. More than 200 agent operations remain undoable as one group. Earlier user groups obey only the normal 200-group cap: a new turn displaces at most one oldest group when already full.
7. Foreign edits, selection commands, Undo/Redo and document replacement seal the group and trip the sticky fence. Never absorb, reorder or replay foreign work during turn cleanup.
8. Success, limit, cancel, transport/channel failure and fence finalize exactly once through the existing terminal cleanup. Already-applied work remains undoable; cancellation is not rollback.
9. Stop dispatch after a known sticky-fence refusal rather than knowingly spend 4096 retries. This explicit safety change requires the D4 amendment; preserve honest partial-result reporting.
10. Snapshot budget at turn start; settings edits affect the next turn. Cancel stays responsive, with existing bounded blocking-HTTP limitations described honestly.

## Expected tests

- AC 1-2: architecture review, old/missing settings and values 0/1/32/33/256/4096/4097/u32::MAX.
- AC 3-4: deterministic fake loops above 32 and 255, exact/oversized multi-call batches, queries/refusals and final-text completion.
- AC 5-6: over 200 live commits, one Undo/Redo of the entire result, prior history retention and flat grouping.
- AC 7: foreign create/delete/selection, Undo/Redo and document replacement during a turn; prove chronology and no mixed groups.
- AC 8-9: cancel/error/disconnect/limit/fence after over 200 mutations; no busy latch, lost undo or repeated refused dispatches.
- AC 10: turn-start snapshot and actual progress/Cancel UI interactions using fakes, not external providers.

## Open questions

Architect must approve the flat group lifecycle and sealing under foreign
edits/Undo, plus bounded terminal reporting after a fence. These are explicit
Ready gates, not permission to discard undo safety.

## Notes

Primary files: `src/agent/loop_.rs`, `src/io/settings.rs`,
`src/app/agent_turn.rs`, `src/app/agent_poll.rs`, `src/document/history.rs`,
the existing composite command, and agent panel/settings UI. ADR 0007's
end-only grouping must be amended; its old 32-step reasoning no longer holds.
LCV-144's atomic batch is one command and dispatch, not one per entity.

## Architecture decision

Recorded 2026-09-27 in ADR 0007 amendment (7) (satisfies AC 1's gate):
§D12 flat history group (supersedes §D6), §D13 `u32` budget 256 / 1..=4096 and
`TurnConfig` (supersedes §D7's constants), §D14 second fence witness
(`History::group_open()`) and stop-after-first-fence-refusal (amends §D4),
§D15 malformed tool calls become `Refused` `Act`s (amends §D2a's routing),
§D8 seams (`src/app/agent_worker.rs` split, `App::agent_turn: TurnState`).
`product-owner` reconciles the ACs against those sections.
