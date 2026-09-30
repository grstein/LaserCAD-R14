# ADR 0014 — Entities carry a stable id beside their index; ids live for the app run and are never persisted

- **Status**: Accepted
- **Date**: 2026-09-30
- **Deciders**: architect (LCV-188). The gate was self-approved under the user's 2026-09-30 goal,
  as relayed by the team lead.
- **Amends**: [ADR 0007](0007-agent-turn-mutates-the-live-document.md) §D5 and §"Deferred to their
  own ADRs"; [ADR 0010](0010-declarative-drawing-batch-tool.md) §7 (outcome suffix, item 8 below).

## Context

Agent tools address entities by position (ADR 0007 §D5). A delete shifts every later index, and
the outcomes only narrate the shift (`agent_apply/edit.rs::shift_note`). In a 180-action session
the model had to query and count again after each delete. ADR 0007 §Deferred expected stable ids to
bump `SCHEMA_VERSION` and to touch the autosave envelope, SVG import, `Selection`, every command's
capture and every index-based test. Two things have changed since then. ADR 0012 §2 set the pattern
of a private per-entity vector kept in lockstep by four `Document` mutators. LCV-188 also scopes the
ids to the running app: they are never saved. With both, the change is much smaller.

Binding constraints: kernel purity; every mutation is a `Command` in `History`; `Document` stays
`!Clone`; `src/agent/` names no document type (ADR 0007 §D1); the SVG contract and ADR 0012's
"no `id`" rule do not change; `document/state.rs` (276) and `document/history.rs` (272) are
already in ADR 0004's band.

## Decision

**1. Type.** `EntityId(u64)` is a kernel type in `src/document/state/ids.rs`, re-exported by
`document/mod.rs`. It is `Copy`, `Eq`, `Hash` and `Ord`, and its `Display` is `e<N>`. N starts
at 1, so `e0` never exists.

**2. Storage.** Ids follow ADR 0012 §2. `Document` gains a private `entity_ids: Vec<EntityId>`,
kept the same length as `entities`, and a private `next_id: u64` that only ever goes up. The
lockstep mutators handle ids as follows:
- `push_entity` / `push_current` take a fresh id.
- `remove_entity(i) -> (Entity, LayerId, EntityId)` hands the id back.
- `insert_entity(i, e, layer, id)` puts a captured id back.
- `truncate_entities` drops the ids and leaves the counter alone.
Two reads: `entity_id(i) -> Option<EntityId>` and `index_of(id) -> Option<usize>` (a linear scan;
a resolve runs once per agent call, never per frame). Rejected: an id field on `Entity`, because
`Entity` is `Copy`, compared by geometry and matched in hundreds of places; and a
`Placed { id, entity }` wrapper, which ADR 0012 already turned down.

**3. In-place edits keep ids for free.** Writing `doc.entities[i] = …` keeps the slot, so it keeps
the id. Move, rotate, scale, mirror with erase, trim and extend all work this way today, so none of
them changes.

**4. Undo and redo restore ids, and `History` does not change.** Creating commands hold an
`IdLedger`. It records the ids handed out on the first `do_`, and on redo it gives the same ids
again through `Document::push_entity_as`. This covers `CreateLine`, `CreateCircle`, `CreateArc`,
`CreateEntities`, `CopyEntities` and `TransformEntities` with `keep_source`. `DeleteEntities`
captures `(index, entity, layer, id)` and puts the id back on undo. The counter is never rewound.
Rewinding it on undo would reuse an id after a new creation, and AC2 forbids that.
`debug_assert`s in `push_entity_as` and `insert_entity` check that a restored id is below
`next_id` and not in use.

**5. Lifetime: one app run, not persisted.** Save, export, autosave and `SCHEMA_VERSION` do not
change, and none of them writes an id. `from_parts` (open, import, autosave restore) numbers the
entities `e1..=en`. When the app replaces its document (File > New, Open, Open Recent;
`io/file_actions.rs`), the new document continues the old counter through
`Document::ids_after(self, prev: &Document) -> Document`. So an id kept in the agent's
conversation memory (ADR 0007 §D16) can never resolve to an entity in another drawing: it is
refused as unknown. On app start, `Document::default()` and autosave restore begin at `e1`.

**6. Index-based subsystems stay as they are.** `Selection`, snapping, rendering, the tools and the
command line keep using indices. Ids are for the agent only and are not shown in the operator UI.

**7. Agent vocabulary (ADR 0007 §D1, §D2a).** The wire spells an id as the string `"e<N>"`, the
text `query_entities` prints. No bare integer is accepted, so an id can never be mistaken for an
index. The thread checks only the shape: `e` followed by a decimal `u64` ≥ 1; `ids` holds 1..=1000
entries with no duplicate. It builds `AgentAction::ById { ids: Vec<u64>, op: SetOp }` (plain
`u64`, so `src/agent/` still names no document type). `id` is short for a one-entry `ids`. Each
tool that takes `index` or `indices` (the six edit tools and `set_layer`) takes exactly one of
`index`, `indices`, `id`, `ids`. The apply site resolves every id against the live document. One
unknown id refuses the whole call and commits nothing. Otherwise the resolved indices go through
the LCV-186 set path unchanged, so they behave exactly like the matching `indices` call.

**8. Outcomes.** `query_entities` prints `<i> e<N>: <kind> …`. Any commit that appends entities
ends its outcome with ` New id: e<N>.` or ` New ids: e<A>..=e<B>.` An appended range is contiguous
by construction (one command takes its fresh ids in sequence); a non-contiguous set is listed
comma-separated. This adds a suffix to ADR 0010 §7's batch sentence; its words stay as they are.

**9. What stays in ADR 0007.** Indices still exist and still shift. So §D5's prompt statement,
the entity count and the delete shift note all stay. The prompt now prefers ids for multi-step
edits. §D4/§D14's fence stays as it is, and LCV-188 does not relax it. §D5 is extended here, not
superseded: ADR 0007's revisit line "D5 is superseded" is answered by this item.

## Consequences

- The model can delete, then act on `e12`, without querying again. A stale id is refused and
  never lands on the wrong entity. A stale index can still land on the wrong entity, as before.
- **Every future creating command must use an `IdLedger`**, or its redo gets new ids. The test for
  each creating command checks redo ids. The v0.4 edit commands (LCV-160..163, another worktree)
  must follow this rule when `agent-harness` rebases onto them.
- `document/state.rs` needs a seam first (ADR 0004): the layer-editing `impl Document` block moves to
  `document/state/layers.rs`, and the id code goes in `document/state/ids.rs`.
- Ids cost 8 bytes per entity plus one counter. Resolving an id is O(n) per id, and only on agent
  calls.
- Mutation testing applies: `src/agent/`, the ledger, and the delete capture next to `History`.

## Alternatives considered

- **A process-wide `static AtomicU64`** (the `History::id` precedent). It needs the fewest call
  sites, but the numbers then depend on test order, and the many exact-string `query_entities`
  tests would flake. Rejected.
- **Restarting the ids for each document.** Then `e3` from memory would silently name a different
  entity after File > Open. Rejected in favour of item 5's continuation, which takes three lines.
- **Persisting the ids** (SVG `data-id`, autosave field). That breaks the SVG contract and AC7, and
  needs a schema bump. Rejected. Revisit only if ids must survive a restart.
- **Ids inside `History`** (a renumbering map). That moves document state into history and grows a
  file already in the band. Rejected.

## Revisit criteria

- Ids must survive save/load or a restart, e.g. a persisted agent memory: that means a schema bump
  and a new ADR.
- The operator UI needs ids, e.g. picking by id on the command line: that is a new ADR 0003 surface.
- `index_of` shows up in a profile: add a map, kept in lockstep like the vector.
