# LCV-193 — Agent turn metrics

- **Status**: Draft
- **Depends on**: LCV-192
- **Implementation**: -

## Problem

No harness measures how an agent turn went. In an agent session (2026-09-30) a simple drawing
cost 180 actions, several identical refusals and captures whose use nobody could verify, and the
final report could not be checked against what happened. Developers tuning the harness, and
operators comparing models, have no numbers to judge by.

## Stories

- As an operator, I want a one-line summary at the end of each agent turn so that I can see what
  it cost and how clean it was.
- As a developer, I want the same numbers in tests so that harness changes are judged on evidence.

## Acceptance criteria

1. WHEN an agent turn ends (done, cancelled, fenced or failed), THE SYSTEM SHALL add one transcript
   `note`: `Turn: <s> steps, <a> actions applied, <r> refused (<p> repeated), <c> captures sent,
   <m> model replies.`
2. WHEN the counts are computed, THE SYSTEM SHALL take them from the loop's own records, not from
   the model's text, and expose them to tests as a plain struct.
3. WHEN a scripted test turn (stub send) runs a known call sequence, THE SYSTEM SHALL report exactly
   the counts of that sequence, including repeated refusals from LCV-192.
4. WHEN a turn applies nothing, THE SYSTEM SHALL still add the note with zero counts.

## Out of scope

- Live model benchmarks, stored run history, token or cost accounting.
- Scoring visual quality or the accuracy of the model's final report.

## Open questions

- None.
