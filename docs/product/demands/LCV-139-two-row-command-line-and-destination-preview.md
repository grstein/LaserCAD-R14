# LCV-139 - Readable command input and destination preview

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-111, LCV-112, LCV-124, LCV-132
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

Prompt, feedback and the editable field currently share one horizontal row
(`src/ui/command_line.rs::draw_command_line`). A long tool prompt, a long
feedback message and the input field compete for the same strip, and the
operator cannot see whether the line they are about to submit will be read as
CAD grammar or sent to the agent before pressing Enter.

## Scope

- A bounded two-row command dock: a context row (prompt + feedback + a
  read-only destination label) above an editable input row.
- The destination label reflects, live and without side effects, where the
  current field text would go if submitted right now.

## Out of scope

- Any change to routing behavior. LCV-131 (full CAD command words, e.g.
  `line`/`circle`) and LCV-148 (prefix-only agent routing — only `:` / `/ai`
  reach the model, every other line is CAD grammar, recognised or not) are
  already shipped (ADR 0007 §D9, amendment (5)). This demand **presents**
  that routing; it does not add a prefix, an alias or a word of its own.
- A CAD/AI mode selector, input rewriting, network calls made while previewing
  (before Enter), and a command-history viewer.

## Acceptance criteria

1. At 800x600 logical points, the default-font two-row dock is at most 64pt
   tall in total, and its single-line editor is at least 240pt wide. The
   context row (prompt, feedback, destination label) and the editable input
   sit on visually separate rows.
2. Prompt and feedback text are bounded (no strip that grows unbounded with a
   long message); elided text carries a full-text tooltip. The stored
   `command_line_input`, `command_feedback` and recall-ring contents remain
   byte-exact — only the *display* elides.
3. Raw tool input (`app.tool_manager.wants_raw_input()`) wins before
   classification and shows a `tool input` destination without calling
   `classify`. Otherwise the destination is computed by calling
   `crate::agent::classify(raw, agent_available)` with the same
   `agent_available` definition `src/app/cmdline.rs::agent_available` uses (a
   configured API key with non-whitespace content) — never a second parser,
   parallel prefix table or independent availability check.
4. The label shows exactly one of: `CAD` (`Route::Cad`, including a blank
   line and any unrecognised word), `AI` (`Route::Agent` with a non-empty
   prompt and a configured key), `tool input` (raw-input mode), `AI
   unavailable` (`Route::Agent` with no configured key), `AI prompt empty` (a
   bare `:` or `/ai` prefix with nothing after it), and `AI busy` (a
   `:`/`/ai`-prefixed line while `app.agent.busy` is true). A blank line
   always reads `CAD` and Enter's existing empty-input behavior
   (`CommandInput::Empty`) is unchanged; a valid CAD or raw-tool line is never
   relabelled `AI` merely because the agent happens to be busy.
5. Routing precedence itself is unchanged (ADR 0007 §D9, amendment (5)): a
   `:` or `/ai` prefix is the only way to reach the agent; every unprefixed
   line stays local whether or not the grammar recognises it. An unrecognised
   unprefixed word (e.g. `lien`) always shows `CAD` and, on submit, still
   answers the grammar's own `Unknown command: "…"` — never a network call.
   `line`, `circle` and the rest of LCV-131's tool words are ordinary CAD
   grammar under this label, not an agent destination.
6. Editing the field or re-rendering the label never sends HTTP, arms a turn,
   opens the agent panel, changes egui focus, writes to the recall ring, or
   mutates the document/history.
7. Enter submits exactly once from the editor row; Escape, recall
   (`ArrowUp`/`ArrowDown`), raw-TEXT focus-holding and exact typed geometry
   (LCV-111 / LCV-112) keep their existing behavior unchanged. The
   destination label never overlaps or truncates the editor's visible text at
   any supported size.

## Expected tests

- AC 1-2 / AC 7: a settled real-`App` frame with long prompt/feedback content,
  measuring actual editor bounds, row separation and tooltip content — not a
  source scan.
- AC 3-5: a destination/submission table covering raw-tool-input mode, a
  blank line, CAD tool aliases (`line`, `l`, `circle`), numeric/coordinate
  input, `:`/`/ai` prefixes, a whitespace-only key, an unrecognised word
  (`lien`) and the busy-agent state, each asserted with and without a
  configured key.
- AC 6: a fake send counter plus before/after state comparisons (focus,
  history revision, recall ring, `agent.busy`) across repeated edit/render
  frames with no Enter.
- AC 7: the existing LCV-111 / LCV-112 / LCV-124 regression suites still
  pass unchanged, plus a real Enter/Escape/recall-driven frame test on the
  two-row layout.

## Open questions

None. ADR 0007 §D9 / amendment (5) and `src/agent/classifier.rs`'s shipped
precedence table (`the_precedence_table_holds_for_both_availabilities`)
already settle prefix-only routing; this demand presents that routing and
defines no new one.

## Notes

Primary files: `src/ui/command_line.rs`, a focused
`src/ui/command_destination.rs`, and the `src/ui/mod.rs` re-export. Reuse
`crate::agent::classify` and `src/app/cmdline.rs::agent_available` rather than
duplicating either. Consume `tests/harness/paint.rs` (LCV-132, Done) for the
rendering assertions in AC 1, AC 2 and AC 7.
