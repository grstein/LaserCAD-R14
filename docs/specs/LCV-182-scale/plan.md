# LCV-182 — Plan

## Approach

This builds on LCV-158. Add `Transform::Scale { base, factor }`, which requires factor > 0.
Points map to `base + factor·(p − base)`, radii are multiplied by `factor`, and arc angles and
`ccw` stay the same. `SCALE` reuses `TransformEntities` in place, so undo restores the snapshot.
`ScaleTool` follows `RotateTool`: Idle → WaitingFactor { base, cursor, snapshots }. A typed
`ToolInput::Distance.value_mm` is the factor. A picked point uses its distance from the base
point in mm (AC4), and the preview uses the cursor's distance. A factor ≤ 0 or non-finite makes
`on_command_input` return `false`. The app then prints its existing
`"SCALE does not accept that input."` line and the phase is unchanged (AC5). A factor
within `EPSILON` of 1 commits nothing and the tool keeps prompting (AC6).

## Touches

- `src/geometry/transform.rs`: `Scale` variant.
- `src/cmdline/mod.rs::ToolKind::Scale`; `parse.rs` aliases `scale`/`sc`; `tools/mod.rs::make`.
- `src/tools/scale.rs` (new): `ScaleTool`.
- Agent: `AgentAction::Scale { index, x, y, factor }`, which rejects factor ≤ 0 at parse time,
  like `validate_r`. The schema goes in `agent/tools/schema.rs`, parsing in `agent/tools.rs`,
  the arm in `app/agent_apply/edit.rs`, and a line in `prompt.rs`.
- ADRs: ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Scale`.

## Risks

- A picked point at the base point gives factor 0. It is rejected (AC5), never committed as a
  collapse. The preview skips a zero factor.
- Tiny factors can leave radii near 0. Any factor > 0 is accepted, per the spec; the result
  stays a valid entity because r > 0.
- LOC: `agent/bridge.rs` ≈ 292 after 157/158/181/182. If it is past 285 before T6, first move
  `AgentAction` into `src/agent/bridge/action.rs` (no behavior change; kernel-pure, added to the
  AGENTS.md purity list).
- Mutation testing: no.
