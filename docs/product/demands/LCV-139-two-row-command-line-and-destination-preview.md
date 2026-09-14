# LCV-139 - Readable command input and destination preview

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-111, LCV-112, LCV-124, LCV-132
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

Prompt, feedback and editor currently compete in one horizontal row. The
operator cannot see the CAD/AI destination before submitting input.

## Scope

A bounded two-row command dock and a read-only destination indication using
the existing classifier, without changing input or routing semantics.

## Out of scope

LCV-131, new aliases, prefix-only routing, a CAD/AI mode selector, input
rewriting, network calls during preview or a command history viewer.

## Acceptance criteria

1. At 800x600 logical points, the default-font dock is at most 64pt high and its single-line editor at least 240pt wide. Context and editor have separate rows.
2. Prompt/result presentation is bounded, with full-text tooltips for elision. Stored input, feedback and recall data remain exact.
3. Raw tool input wins before classification. Otherwise reuse `agent::classify` and submission's availability definition, not another parser or prefix table.
4. Show CAD, AI, tool input, unavailable AI, empty AI prompt and busy AI as applicable. Blank CAD input still retains existing Enter behavior; valid CAD/raw input is never relabelled AI merely because the agent is busy.
5. Current routing remains authoritative: `LINE` is unknown grammar input, routes to AI with a configured key and remains a local error without one. Explicit prefixes and raw TEXT precedence are unchanged.
6. Editing/rendering the indicator never sends HTTP, arms a turn, opens the agent panel, changes focus, inserts history or mutates geometry.
7. Enter submits once; Escape, recall, raw TEXT focus and exact typed geometry retain their existing behavior. Labels cannot obscure input at supported sizes.

## Expected tests

- AC 1-2/7: settled real-App layout with long prompt/error content, actual editor bounds and tooltip content.
- AC 3-5: destination/submission table for raw text, blank, aliases, numeric input, prefixes, whitespace keys, `LINE` and busy state.
- AC 6: fake send counter and state comparisons over repeated edit/render frames.
- AC 7: preserve LCV-111/112/124 behavioral expectations and real Enter/Escape/recall events.

## Open questions

Before Ready, architect checks current ADR 0007 D9 wording against the user's
explicit retained-routing choice. If concurrent documentation specifies
prefix-only routing, reconcile that conflict explicitly; do not implement it
silently under this presentation demand.

## Notes

Primary files: `src/ui/command_line.rs`, a focused
`src/ui/command_destination.rs`, and root re-exports. Share the existing
availability predicate rather than duplicating it. Consume LCV-132 paint
helpers after the existing LCV-134-before-LCV-132 sequence.
