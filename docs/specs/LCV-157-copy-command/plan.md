# LCV-157 — Plan

## Approach

COPY is MOVE that appends instead of translating and stays armed after a placement. The
simplest version is:

- A new kernel command `CopyEntities { indices, delta }` in `document/commands/edit.rs`.
  `do_` appends a translated clone of each source through `Document::push_entity`, on the
  source's layer from `Document::entity_layer`. `undo` calls `Document::truncate_entities`
  back to the length before the command. Each placement commits one command (AC6).
- A new `tools/copy.rs::CopyTool`, cloned from `MoveTool`'s state machine. After a commit it
  stays in `WaitingSecond` with the same base point and snapshots. The sources never move and
  copies are appended, so the source indices stay valid.
- A new `ToolKind::Copy` with the word rows `copy`, `co` and `cp`. The letter axis is not
  touched.
- For the agent, `copy_entity {index, dx, dy}` mirrors `move_entity` end to end.

AC8 "end COPY": Enter or Escape drops the tool back to the base-point prompt and the tool
stays COPY. This matches `MoveTool::on_key`, and PLINE also cancels on Enter/Escape. Successors
are only polled after a pointer press or release or a consumed command-line input, never after a
key. So handing over to SELECT on Enter would need wiring outside this spec.

## Touches

- `src/document/commands/edit.rs::CopyEntities` (new), re-exported from
  `document/commands/mod.rs` and `document/mod.rs`.
- `src/tools/copy.rs::CopyTool` (new) and `tools/mod.rs` (`pub mod`, `pub use`, the `make` arm).
- `src/cmdline/mod.rs::ToolKind::Copy` and `src/cmdline/parse.rs::tool_alias` (the rows and the doc table).
- `src/agent/bridge.rs::AgentAction::Copy`, `src/agent/tools.rs` (`parse_tool_call` arm,
  schema-order doc), `src/agent/prompt.rs` (one tool line), `src/app/agent_apply/edit.rs`
  (the `Copy` arm, reusing `in_range`, `describe` and `pt`).
- Seams for the sprint (no behavior change, before any agent tool lands):
  `src/agent/tools/schema.rs` (new, kernel-pure, added to the AGENTS.md purity list) takes
  `base_definitions`; `src/app/agent_apply/edit.rs` (new) takes the Delete/Move arms of `plan`.
  LCV-158/181/182 add their schema and apply arms there.
- Not touched: the toolbar, the Tools menu and `ui/shortcuts.rs::TOOL_KEYS`. The spec asks
  for no button and no letter, and the letter axis is closed.
- ADRs: ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Copy`, and also
  `Rotate`, `Mirror`, `Scale` and `Dist` for LCV-158/181/182/159.

## Risks

- LOC cap: `src/agent/tools.rs` is at 268 and `src/app/agent_apply.rs` at 258, with four
  agent tools coming (157/158/181/182). T5/T6 split them here, before `copy_entity`, so
  LCV-158 onward start from the split files. `edit.rs` (112), `parse.rs` (187) and
  `cmdline/mod.rs` (163) have room.
- Mutation testing: no. Nothing touches `src/agent/` loop or transport, the SVG export or `History`.
- An undo in the middle of a COPY run removes the latest copies. The source indices are still
  valid, because copies are only ever appended. T1 pins this with a unit test.
- The LCV-151 test checks that the built-in prompt describes every tool. It fails until
  `prompt.rs` gets the `copy_entity` line, so T9 keeps them in step.
