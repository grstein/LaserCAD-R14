# LCV-158 — Plan

## Approach

Add one kernel transform layer that ROTATE, MIRROR (LCV-181) and SCALE (LCV-182) all share.
`geometry/transform.rs` holds `Transform`, a closed enum with one variant per edit. It starts
with `Rotate { base, angle }` in radians; 181 adds `Mirror` and 182 adds `Scale`. Its methods
`point`, `line`, `circle` and `arc` return new values. `Entity::transformed(&Transform)` dispatches
to them. One command, `TransformEntities { indices, transform }`, replaces entities in place and
snapshots the originals, so undo restores them bit-exact. A rotation by a non-trivial angle is
not exactly invertible in `f64`, which is why undo cannot just apply the inverse as MOVE does.
In-place replacement keeps each entity's layer and the selection for free. `RotateTool` copies
`tools/move_.rs`: select first, pick a base point, then an angle. Degrees exist only at the input
edge: a typed number and the agent's `degrees`. The agent's `rotate_entity` commits the same command.

## Touches

- `src/geometry/transform.rs` (new): `Transform::Rotate`, `point/line/circle/arc`; `geometry/mod.rs` re-export.
- `src/document/entity.rs::Entity::transformed`: per-variant dispatch.
- `src/document/commands/transform.rs` (new): `TransformEntities` (snapshot undo); `commands/mod.rs`, `document/mod.rs` re-exports.
- `src/cmdline/mod.rs::ToolKind::Rotate`; `cmdline/parse.rs` aliases `rotate`/`ro`; `tools/mod.rs::make`.
- `src/tools/rotate.rs` (new): `RotateTool` (Idle → WaitingAngle { base, cursor, snapshots }).
  A typed `ToolInput::Distance.value_mm` is the angle in degrees; a point gives the angle of
  base→point. The preview and `anchor()` work like MOVE.
- Agent: `agent/bridge.rs::AgentAction::Rotate`; `agent/tools.rs` parse; `agent/prompt.rs` line.
- The agent schema goes in `src/agent/tools/schema.rs` and the apply arm in
  `src/app/agent_apply/edit.rs`, both split out by LCV-157 (T5/T6).
- ADRs: ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Rotate` and records that
  `geometry::Transform` is an additive kernel type needing no ADR of its own.

## Risks

- LOC cap: depends on LCV-157, whose T5/T6 split `agent/tools.rs` and `app/agent_apply.rs`
  before any new agent tool lands. `agent/bridge.rs` is at 246: ≈ 255 after COPY, ≈ 266 after
  this; LCV-181/182 carry the seam if it passes 285.
- Float drift: tests compare with `EPSILON`, and undo is compared with `==` (snapshot).
  A property test checks that a rotation keeps lengths, radii and arc sweeps, and that arc
  start and end points land on the rotated points.
- Zero angle: `angle.rem_euclid(TAU)` within `EPSILON` of 0 or TAU commits nothing and the tool
  keeps prompting (AC8).
- Agent units: `create_arc` takes radians, but `rotate_entity` takes degrees (spec AC9). The
  prompt line says "degrees" explicitly.
- Mutation testing: no. This is not high risk under the AGENTS.md list, and export is untouched.
