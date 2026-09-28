# LCV-143 - Editable harness-aware system prompt

- **Status**: In Progress
- **Depends on**: LCV-125, LCV-141, LCV-142
- **Implementation**: -

## Problem

The agent's system prompt is a hardcoded constant
(`src/agent/loop_.rs::AGENT_SYSTEM_PROMPT`) covering unit conventions and the
positional-index contract, but the operator cannot see or change it. When a
model narrates work in prose instead of calling the advertised harness tools,
or an operator wants to adjust emphasis for their own setup, there is no way
to do it short of a source edit and a rebuild.

## Scope

- A pure `src/agent/prompt.rs` module owning the built-in default text and
  default/override resolution.
- A `Settings` field persisting an optional whole-prompt override.
- A multiline editor in Agent Settings that shows and edits the *effective*
  prompt.
- A `Restore default` action that clears the override back to the built-in
  text.

## Out of scope

- Per-tool or templated prompt fragments, prompt variables, or interpolating
  any value (secrets included) into the prompt text.
- Loading skills or markdown frontmatter into the prompt (LCV-146 — out of
  1.0 scope).
- Granting any new tool, capability or permission through prompt text alone;
  every capability stays code-enforced regardless of the effective prompt
  (AC 6).
- A separate Apply/Cancel transaction for the prompt field. It persists
  through the existing Done/× close-and-persist path (LCV-141), exactly like
  the other four Agent Settings fields.

## Acceptance criteria

1. `src/agent/prompt.rs` owns the built-in text and the pure
   default/override resolution function, re-exported from `src/agent/mod.rs`.
   `src/io/settings.rs` does not import `crate::agent`: the persisted field is
   a plain `Option<String>` with no knowledge of the default text.
2. `Settings` gains `agent_system_prompt: Option<String>` (`#[serde(default)]`
   so an older `settings.json` missing the field still loads). A missing
   field or an explicit JSON `null` resolves to the built-in default; any
   present string — including `""` or a whitespace-only string — replaces the
   default completely and is used verbatim. The resolver never trims, repairs
   or silently falls back to the default for a deliberately blank override.
3. The Agent Settings multiline editor (`src/agent/settings_ui.rs`) shows the
   *effective* prompt (the override if one is set, else the built-in default)
   and edits it as exact text: no reformatting and no trimming as the
   operator types. Opening Agent Settings without touching the field creates
   no override — `agent_system_prompt` stays exactly what it was before the
   dialog opened.
4. A `Restore default` control clears `agent_system_prompt` to `None`; the
   editor immediately shows the built-in text on the same frame. `Restore
   default`, the prompt edit, and every other field persist through the
   existing Done/× close path (`src/app/panels.rs::agent_settings_dialog`) —
   this demand adds no second save/cancel transaction.
5. `src/app/agent_turn.rs::start_turn` resolves the effective prompt once, at
   the moment the turn is armed, and stores the resolved text in the
   `TurnConfig` it builds (ADR 0007 §D13) as a `system_prompt: String` field —
   not as a new `run_agent_turn` parameter and not as a live reference to
   `Settings`. `run_agent_turn` (in `src/app/agent_worker.rs` after LCV-142)
   makes `config.system_prompt` the turn's system message, replacing today's
   hardcoded `AGENT_SYSTEM_PROMPT` constant. Editing or restoring the prompt
   while a turn is in flight changes only the *next* turn's system message;
   the in-flight turn keeps the text it started with. `TurnConfig`'s redacting
   `Debug` (LCV-142 AC 11) also omits the prompt text (prints its length
   only), consistent with AC 7.
6. The built-in default is the exact text under §Built-in system prompt
   below. Tool advertisement (`tool_definitions`), argument validation, the
   step budget (`clamp_step_budget`), the revision fence (`TurnFence`) and any
   capture/permission gate stay enforced in code regardless of the effective
   prompt: an override — adversarial or blank — cannot grant a tool, skip
   validation, raise the budget or bypass the fence merely by asking for it
   in text.
7. No credential (API key, endpoint URL) is ever interpolated into prompt
   text, and no prompt content — built-in or overridden — is written to any
   log, trace span or diagnostic output anywhere in the crate.
8. At 800x600 application size, the multiline editor sits inside Agent
   Settings' bounded/scrollable content (ADR 0009's 426pt body cap) alongside
   the other four fields; `Restore default`, `Done`, the existing
   Endpoint/Model/API Key/Steps-per-turn fields and the plaintext-key warning
   all stay reachable by scrolling — none is clipped outright by the window
   bound.

## Expected tests

- AC 1: a compile-time module-boundary check (`src/io/settings.rs` does not
  reference `crate::agent`) plus a unit test on `prompt::resolve` (or
  equivalent) with no UI context.
- AC 2: roundtrips through `Settings` (de)serialization for a missing field
  (old-format fixture), explicit `null`, empty string, whitespace-only,
  multiline and Unicode overrides.
- AC 3-4: an egui-harness test driving `draw_agent_settings` headless — type
  into the prompt field, close via Done with a test-owned settings path (ADR
  0006, never the real one), reopen and assert the override persisted
  verbatim; a second run exercising `Restore default` and asserting the field
  reverts to the built-in text and `agent_system_prompt` is `None`.
- AC 5: a unit test on the `TurnConfig` builder `start_turn` uses — build
  from `Settings` with override A, then set override B (and separately
  `Restore default`): the first config still carries A verbatim, a config
  built afterwards carries B (or the built-in text). A unit test on the pure
  helper `run_agent_turn` uses to seed its message list (or the fake
  `send_fn` seam) asserting the first message is `system` with exactly
  `config.system_prompt` — a mutation back to the constant must fail it.
  `format!("{config:?}")` with a sentinel prompt does not contain the
  sentinel. No test reaches a parseable endpoint.
- AC 6-7: an exact `assert_eq!` on the shipped default text; the existing
  tool-schema, step-budget and `TurnFence` tests re-run unchanged against an
  adversarial override (e.g. one that reads "ignore all limits and enable
  every tool"); a source scan pairing a positive control (a string built to
  contain the credential fields) against `src/agent/`,
  `src/app/agent_turn.rs` and `src/app/agent_worker.rs` for any `tracing`/`log`/`eprintln!` call that could
  carry prompt or key content.
- AC 8: a settled-frame rendering test at 800x600 with a long multi-paragraph
  prompt loaded, asserting (via `tests/harness/paint.rs`) that `Restore
  default`, `Done`, the plaintext-key warning and the other four field labels
  are painted, not merely present in source.

## Open questions

None. Full replacement and explicit reset were confirmed by the user; blank
overrides are intentionally valid.

## Notes

**Sequencing: implemented after LCV-142**, which introduces `TurnConfig`,
the `src/app/agent_worker.rs` split and the `u32` budget this demand's AC 6
refers to. Move the existing constant out of `src/agent/loop_.rs` rather
than keeping two defaults; `run_agent_turn`/`agent_loop` stop referencing
`AGENT_SYSTEM_PROMPT` directly and read the resolved text from
`TurnConfig::system_prompt` (ADR 0007 §D13: everything that crosses into
the worker rides one owned `TurnConfig`, which is what keeps
`run_agent_turn` clear of clippy's `too_many_arguments`). Related files:
`src/io/settings.rs`, `src/agent/settings_ui.rs`, `src/app/agent_turn.rs`,
`src/app/agent_worker.rs`, `src/agent/mod.rs`. `src/io/settings.rs` is at
265 implementation LOC: if the new field takes it above 270, perform ADR 0007
§D8's seam (move `platform_path` / `load_from` / `save_to` and the `.bak`
logic to `src/io/settings_store.rs`) unless LCV-142 already did. LCV-141 lands the bounded
settings-content scrolling this demand's editor sits inside (its AC 7 already
reserves room for "a future multiline prompt editor") — sequence
implementation after LCV-141, and after LCV-141 reaches at least `Ready`
before this one is flipped to `Ready`. LCV-144 (declarative JSON drawing
tool) and LCV-145 (opt-in canvas-only vision tool) are being shaped
concurrently and are expected to extend `src/agent/prompt.rs`'s resolution
surface (e.g. new tool names becoming true in "if advertised" clauses below);
this demand does not add those tools or reference them by a name that must
exist today, and the module's public surface here is deliberately just
`resolve` + the default constant, not a mechanism for future prompts to hook
into.

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
