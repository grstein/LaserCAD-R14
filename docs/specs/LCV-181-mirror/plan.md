# LCV-181 — Plan

## Approach

This builds on LCV-158. Add `Transform::Mirror { a, b }`, a reflection across line ab. Points
reflect. For an arc, the center reflects, the angles map to `2θ − angle` (θ is the angle of ab),
the start stays the start, and `ccw` flips. The result is still one `Arc`, so the export stays
one `A` command (AC8). `TransformEntities` gains `keep_source: bool`. `false` is the existing
in-place replacement. `true` appends the transformed copies, each on its source's layer via
`Document::push_entity`, and undo truncates to the captured length, as `CreateEntities` does.
`MirrorTool` has phases FirstPoint → SecondPoint → Confirm. At Confirm it sets
`wants_raw_input() = true`, so `y`/`yes`/`n`/`no`/blank reach `on_raw_input` unparsed. This is
the TEXT precedent (ADR 0003 §D), and it keeps `n` from being read as a command alias.
Other text is refused and the tool keeps prompting.

## Touches

- `src/geometry/transform.rs`: `Mirror` variant (point/line/circle/arc).
- `src/document/commands/transform.rs`: `keep_source` mode (append on source layers, truncate undo).
- `src/cmdline/mod.rs::ToolKind::Mirror`; `parse.rs` aliases `mirror`/`mi`; `tools/mod.rs::make`.
- `src/tools/mirror.rs` (new): `MirrorTool`. It ignores a coincident second point (AC4) and
  shows the preview after the first point.
- Agent: `AgentAction::Mirror { index, x1, y1, x2, y2, erase_source }`; the schema in
  `agent/tools/schema.rs`, parsing in `agent/tools.rs`, the arm in `app/agent_apply/edit.rs`,
  and a line in `prompt.rs`.
- ADRs: ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Mirror`. The SVG export contract is unchanged: the mirrored arc is exported through the existing path.

## Risks

- Sweep inversion is the bug-prone part. A test checks that `export_svg` of a mirrored CCW arc
  has the same `large` flag and the inverted `sweep` flag compared with a CW arc drawn directly.
  A property test checks that mirroring twice is the identity within `EPSILON`.
- Raw mode: Escape and tool switching still have to cancel. A test drives Escape at the prompt.
- Selection after a keep-source mirror stays on the sources (R14). After an erase-source mirror
  it stays on the same indices, which now hold the mirrored geometry.
- LOC: `agent/bridge.rs` ≈ 281 after 157 + 158 + this (over the 270 mark). Seam if needed:
  LCV-182's Risks.
- Mutation testing: no (export untouched; the round-trip test pins the arc path).
