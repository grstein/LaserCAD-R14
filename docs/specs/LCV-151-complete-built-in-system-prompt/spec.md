# LCV-151 - Built-in system prompt describes every tool

- **Status**: In Progress
- **Depends on**: LCV-143
- **Implementation**: -

## Problem

The built-in prompt (LCV-143's `src/agent/prompt.rs::DEFAULT_PROMPT`) says little beyond
units and indices, so the model guesses at tools and their arguments, routing, fence
refusals and the step budget. It must be English, complete and true to the code.

## Scope

A static, sectioned English `DEFAULT_PROMPT` covering: what LaserCAD is (2D laser-cutting
CAD, SVG for LaserGRBL); millimeters, degrees in the arc tools; Y-up world with the
origin bottom-left; every tool with its real argument names (e.g. `create_line` takes
`x1`, `y1`, `x2`, `y2` in mm); zero-based positional entity indices; routing; the fence;
the step budget; reply style.

## Out of scope

- Templating or interpolation; skills (LCV-146); capabilities granted through prompt text.
- Changes to `resolve`, `TurnConfig`, `start_turn` or the prompt editor (LCV-143).

## Acceptance criteria

1. WHEN no operator prompt is set THE SYSTEM SHALL send `DEFAULT_PROMPT` as the turn's
   system message, and it SHALL be ASCII-only English with the sections in Scope, in order.
2. WHEN any tool is in `tool_definitions()` THE SYSTEM SHALL name it in `DEFAULT_PROMPT`
   in its own paragraph that states its purpose and lists, as whole words, every
   `parameters.properties` key of its schema.
3. THE SYSTEM SHALL describe entity indices as zero-based and positional: entity 0 is the
   first, a delete renumbers every higher index down by one, and `query_entities` reads
   the current indices.
4. THE SYSTEM SHALL state that only `:`- and `/ai`-prefixed input reaches the model and
   everything else is a CAD command.
5. THE SYSTEM SHALL quote verbatim the fence strings the model can receive as tool
   results, `AGENT_FENCE_REFUSAL` and `FENCE_STOP_PLACEHOLDER`, and tell the model to stop
   and report rather than retry.
6. THE SYSTEM SHALL state the step budget (default `AGENT_STEP_BUDGET_DEFAULT`, range
   1..=`AGENT_STEP_BUDGET_MAX`, one tool call is one step) and that a batch which would
   exceed it is refused whole and ends the turn with no message to the model; only the
   operator sees `step budget exceeded (N tool calls per turn)`.
7. THE SYSTEM SHALL tell the model to reply in brief prose saying what it did, and not to
   ask for confirmation of parameters it can choose itself.
8. The implementer writes the exact prompt text; the user reviews it word for word, and
   the spec is not marked Done until the user approves that text.

## Expected tests

- AC 1: golden test; `drive_turn` wire test pins `messages[0]` as the system `DEFAULT_PROMPT`.
- AC 1, 3, 4, 6, 7: ASCII-only, section needles in order, budget numbers.
- AC 2: per-tool paragraph coverage over `tool_definitions()`, with a positive control.
- AC 5, 6: the fence constants and `step budget exceeded` are substrings of the prompt.
- AC 8: the user's approval is recorded in `tasks.md`.

## Open questions

- None: at plan approval the user chose the code as the contract.
