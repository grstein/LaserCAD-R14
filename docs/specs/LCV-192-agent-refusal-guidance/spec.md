# LCV-192 — Agent refusal guidance

- **Status**: Planned
- **Depends on**: LCV-185
- **Implementation**: -

## Problem

Tool refusals are plain sentences: `create_drawing` names a field path, scalar tools name the
argument without the expected form, and document refusals (index out of range, unknown layer) use
free text. In an agent session (2026-09-30) the model sent the same invalid call several times,
and nothing told it that it was repeating itself.

## Stories

- As an operator, I want every refusal to say which tool, which field, why and what form is
  expected so that the agent corrects itself on the next call.
- As an operator, I want the agent stopped from looping on a call already refused.

## Acceptance criteria

1. WHEN any tool call is refused for its arguments, THE SYSTEM SHALL answer
   `<tool> <path>: <reason>; expected <form>`, where `<path>` names the field (`r`,
   `entities[3].r`, `indices[2]`) and `<form>` the accepted type and range.
2. WHEN a call is refused because of the document (index out of range, unknown layer), THE SYSTEM
   SHALL use the same shape, with `<form>` naming the valid range or the existing names.
3. WHEN a malformed-JSON argument is refused, THE SYSTEM SHALL use the same shape with path `(root)`.
4. IF a call repeats, byte for byte, the tool and arguments of a call already refused in this turn
   THEN THE SYSTEM SHALL not run it and SHALL answer `repeated call, refused before: <first
   refusal>; change the arguments`. It still counts one step.
5. WHEN a refusal text is built, THE SYSTEM SHALL keep today's limits: no payload echo, key names
   cut to 64 characters.

## Out of scope

- Session metrics and evaluation (LCV-193).
- Changing the tool set.

## Open questions

- None.
