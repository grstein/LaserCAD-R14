# LCV-192 — Agent refusal guidance

- **Status**: Draft
- **Depends on**: LCV-185
- **Implementation**: -

## Problem

Tool refusals are plain sentences: `create_drawing` names a field path, scalar tools name the
argument without the expected form, and document refusals (index out of range, unknown layer) use
free text. In an agent session (2026-09-30) the model repeated the same invalid call several times.
Nothing tells the model that it just sent an identical refused call, and no harness measures
repeated errors or calls per task across real model runs.

## Stories

- As an operator, I want every refusal to say which tool, which field, why and what form is
  expected so that the agent corrects itself on the next call.
- As a developer, I want an offline evaluation of agent sessions (calls per task, repeated
  refusals) so that harness changes are judged on evidence.

## Acceptance criteria

To be written by /specify.

## Out of scope

- Scoring the visual quality of the result.
- Changing the tool set.

## Open questions

- Structured refusal as JSON text, or a fixed sentence template?
- Refuse an identical repeat of a refused call without dispatching, or only annotate it?
- Evaluation harness here or as its own demand; recorded transcripts or live providers?
