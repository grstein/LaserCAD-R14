# LCV-143 - Editable harness-aware system prompt

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-125, LCV-141
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The built-in agent already has a hardcoded tool-aware prompt, but the operator
cannot edit it when a model describes work instead of using the LaserCAD
harness. The entire prompt should be editable and restorable.

## Scope

A pure default/effective-prompt module, persisted whole-prompt override,
multiline settings editor and explicit Restore default action.

## Out of scope

Changing the Copilot CLI prompt, templates, variables, skills loading, secret
substitution, new tools or permissions controlled only by natural language.

## Acceptance criteria

1. `src/agent/prompt.rs` owns the built-in text and pure resolution, re-exported at the agent root. `io/settings.rs` must not import agent.
2. Settings store an optional string override. Missing/null uses the built-in default; any present string replaces it completely, including empty/whitespace. Never silently repair a deliberate override.
3. The multiline editor shows the effective prompt and preserves exact content. Merely opening settings does not create an override.
4. Restore default clears the override explicitly. Done and X persist through the existing close path; no new Apply/Cancel transaction.
5. A turn snapshots the effective prompt once. Editing/resetting during a turn affects only the next turn.
6. Use the concrete default below. Tool advertisement, validation, budgets, revision fences and capture permissions remain code-enforced with any override.
7. No credentials are interpolated and no prompt content is added to diagnostic logs.
8. At 800x600 the editor scrolls within bounded settings content; Restore default, Done, existing fields and the plaintext-key warning remain reachable.

## Expected tests

- AC 1-2: resolution, old-settings loading, None/empty/whitespace/multiline/Unicode roundtrips and dependency boundaries.
- AC 3-4: actual editor/reset/close interactions and test-owned persistence.
- AC 5: edit settings between fake completion responses; active prompt stays fixed and next turn receives the new one.
- AC 6-7: exact default and request assembly; adversarial/blank overrides cannot bypass software permissions or interpolate secrets.
- AC 8: settled-frame bounds and scrolling with a long prompt, real Restore/Done clicks.

## Open questions

None at product level. Full replacement and explicit reset were confirmed by
the user; blank overrides are intentionally valid.

## Notes

Move the existing constant from `src/agent/loop_.rs` rather than keeping two
defaults. Related files: `src/io/settings.rs`, `src/agent/settings_ui.rs`,
`src/app/agent_turn.rs`, `src/agent/mod.rs`.

### Built-in system prompt

```text
You are the CAD assistant embedded in LaserCAD v2, a focused 2D CAD
application for preparing LaserGRBL-compatible laser drawings.

When asked to construct, modify, or inspect the open drawing, call the
advertised harness tools to do the work; do not only describe how to do it.
Ask a focused question when required dimensions or intent are missing.
Use the fewest tool calls that correctly satisfy the request.

All drawing coordinates, lengths, and radii are canonical millimeters (mm).
Follow each tool schema for angles: existing arc tools accept degrees;
the geometry kernel uses radians. Do not substitute pixels for geometry.

Entity indices are positional, not stable IDs. Deleting an entity shifts
every higher index down by one. Query the live drawing before targeting an
index you have not read in this turn, and query again after deletion before
reusing potentially stale indices.

If create_drawing is advertised, use it for suitable append-only batches
of lines, circles, and arcs, within its declared validation and size limits.
Request a canvas capture only if that tool is advertised and enabled.
Do not invent tools, skills, permissions, or capabilities.

Check every tool outcome. Report only changes and observations that actually
succeeded; never claim unperformed work. If the drawing-change fence refuses
an action or the turn is cancelled, stop rather than retrying. State any
partial completion or refusal honestly and summarize the outcome concisely.
```
