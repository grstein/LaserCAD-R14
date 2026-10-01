# LCV-191 — Plan

## Approach

`set_layer` is a seventh set operation, not a new mechanism. The parser builds the LCV-186
`AgentAction::Set { indices, op: SetOp::Layer { layer: String } }`, reusing the `indices`
shape check of `tools/transform.rs` (AC4) and `drawing::layer_arg` for the name (string,
1..=64 chars; required here). The apply site `agent_apply/set.rs` already range-checks every
index against the live document, sorts, and commits one command into the turn's flat group
(ADR 0007 §D12), so AC1, AC4 and AC5 come for free. Its `Layer` arm resolves the name with
`agent_apply.rs::target_layer` (unknown → refused, nothing committed, AC3), drops the indices
already on that layer (AC2), and commits the existing `document::SetEntityLayers` — the command
behind Format > Move to Layer — for the rest. Only `String` names cross into `src/agent/`
(ADR 0012 §6).

Order: after LCV-186 and LCV-192 are Done (roadmap order 192 → 191). It lands on top of 192's
`tools/args.rs` split, `refusal(..)` shape and `AgentAction::tool_name`.

## Touches

- `src/agent/bridge/action.rs` — `SetOp::Layer { layer: String }`; `tool_name` arm → `set_layer`.
- `src/agent/tools/transform.rs` — `parse_set_layer`: `indices` required (no `index` form),
  `layer` required; reuses `indices(..)` and `layer_arg`.
- `src/agent/tools.rs::parse_tool_call` — one `"set_layer"` arm.
- `src/agent/tools/schema.rs::base_definitions` — `set_layer` after `scale_entity`
  (`indices` + `layer`, both required); `tools.rs::tool_definitions` order doc.
- `src/app/agent_apply/set.rs` — `Layer` arm: `target_layer`, filter, `SetEntityLayers`, narration.
- `src/agent/prompt.rs::DEFAULT_PROMPT` — a `set_layer` paragraph; the LAYERS section no longer
  says no tool moves entities between layers.
- ADRs: amend ADR 0012 §6 ("No layer-editing tools" → `set_layer` moves entities onto an
  existing layer; still no tool creates, renames, recolours or deletes a layer). No new ADR.

## Decisions (self-approved per user goal)

- `indices` only, no single `index` form: the spec asks for the set form, and `[i]` covers one.
- AC3 text: "today's" refusal is whatever `target_layer` emits when this lands — after LCV-192,
  `set_layer layer: unknown layer "X"; expected one of "0", "Cut"`. One formatter, not two.
- AC2: indices already on the target are dropped before the command. If none remain, the call
  succeeds with `all N entities are already on layer "Cut"` and commits nothing (no empty undo
  step). Otherwise: `moved M entities to layer "Cut"` plus `(K already there)` when K > 0.
- Layer lookup is by key, case-insensitive, like the create tools (`layer_by_name`).
- Moving entities does not change the current layer or the selection.

## Risks

- LOC cap: `tools/transform.rs` 147 (+~20), `bridge/action.rs` 237 (+~4), `tools.rs` 266 now but
  split by LCV-192 T1 (+1 arm). `agent_apply/set.rs` is new with LCV-186; if it passes 270 the
  layer arm moves to `agent_apply/set_layer.rs`.
- Mutation testing: yes — `src/agent/`; the already-there filter, the all-already-there branch,
  required `layer`, and the `tool_name` arm.
- Test churn: tool count 12 → 13 (`src/agent/tools/tests.rs`), `default_prompt.rs` whole-word
  check picks up the new tool automatically, `bridge.rs` variant-distinct test unchanged (no new
  variant). LCV-187 and LCV-190 edit `tools.rs`, `schema.rs`, `action.rs` again; they rebase.
- `SetEntityLayers::undo` restores in reverse; AC5 is pinned by a turn test mixing a create, a
  `set_layer` and a move, undone as one step with every entity back on its layer.
