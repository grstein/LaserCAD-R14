# LCV-151 - Built-in system prompt describes every tool

- **Status**: Specified
- **Depends on**: LCV-143
- **Implementation**: -

## Problem

The built-in system prompt (LCV-143's `src/agent/prompt.rs::DEFAULT_PROMPT`) is
incomplete. It describes units and the positional-index contract for
multi-line input, but the language model cannot know what tools are available,
what each tool does, its argument names and types, the distinction between
CAD and AI routing, the fence-refusal behaviour when the operator draws
during a turn, the step budget and its reset semantics, or the expected
reply style. The prompt must be English, comprehensive, and give the model
everything it needs to work effectively.

## Scope

Update the built-in system prompt text to include:

- **What LaserCAD is**: a 2D CAD micro-tool for laser cutting, precision-
  focused, exporting to SVG for LaserGRBL
- **Units and conventions**: all coordinates, distances and dimensions are
  in millimeters; angles are in degrees
- **Coordinate system**: the world is Y-up; the viewport origin is
  bottom-left (in world coordinates)
- **Every available tool**: read and render the complete list from
  `src/agent/tools.rs::tool_definitions()` into the prompt, including tool
  names, argument names, argument types, and what each tool does (e.g.
  `create_line { p1, p2 }` creates a line from point p1 to p2 in mm)
- **Positional index contract**: explain that multi-line input begins with a
  line number (1-based, shown in the UI) and the rest is the input
  (e.g. `2 100,150`)
- **CAD vs AI routing**: the command line accepts both CAD commands (e.g.
  `line`) and AI commands (prefixed `/ai` or `:`) — the model should
  recognize which are which
- **Fence-refusal behaviour**: if the operator draws (adds, deletes, moves
  entities) during a turn, the turn stops; any pending action is refused
  with a standard `Fenced: revision mismatch` message
- **Step budget**: the model has a limited number of steps per turn (default
  256, configurable via LCV-142); when the budget is exhausted, any further
  action is refused with a standard message; the model should use steps
  wisely and stop early if the work is complete
- **Reply style**: the model should respond in prose, explain what it did,
  ask clarifying questions, and use tool calls to make changes to the
  drawing — never ask for confirmation or user input for parameters it can
  choose itself

## Out of scope

- Templating, variable interpolation, or dynamic prompt construction
- Per-tool override or prompt fragments
- Markdown frontmatter or external skill loading (LCV-146)
- Granting capabilities through prompt text

## Acceptance criteria

1. `src/agent/prompt.rs::DEFAULT_PROMPT` (or the function that returns it)
   contains all the content listed in the Scope section above
2. The prompt is in English, readable, and logically organized
3. Every tool from `tool_definitions(true)` and `tool_definitions(false)`
   is mentioned by name with its argument names and purpose clearly stated
4. The prompt explicitly names the fence-refusal and step-budget standard
   messages so the model recognizes them when they occur
5. The prompt includes the positional-index contract for multi-line input
6. The prompt is updated when LCV-142 or LCV-143 ships if they add new
   fields to `TurnConfig` that affect the model's behaviour
7. At runtime, `src/app/agent_turn.rs::start_turn` resolves and stores the
   effective prompt in `TurnConfig.system_prompt` (LCV-143 AC 5), ensuring
   the model sees the prompt that was configured at turn start

## Expected tests

- Unit: a test that enumerates all tools from `tool_definitions(true)` and
  scans the default prompt to confirm every tool name appears
- Unit: a test that scans for the exact fence-refusal and step-budget
  messages that the code produces, confirming they are mentioned in the
  prompt so the model recognizes them
- Integration: a turn with a mocked model that receives the system prompt
  and confirms the message is complete and well-formed JSON

## Notes

The current hardcoded text in `src/agent/loop_.rs::AGENT_SYSTEM_PROMPT` is
the starting point. LCV-143 moves this into `src/agent/prompt.rs` and makes
it editable by the operator; this demand enriches the built-in text itself
with all the detail the model needs. The prompt must remain non-interpolated
(never substituting credentials or secrets) and must stay read-only by the
model (no prompt injection risk from adversarial tool results).
