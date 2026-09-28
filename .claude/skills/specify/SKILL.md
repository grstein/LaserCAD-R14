---
name: specify
description: SDD step 1 for LaserCAD v2 — create or rewrite docs/specs/LCV-NNN-*/spec.md (problem, stories, EARS acceptance criteria) and get the user's approval to mark it Specified.
argument-hint: "[LCV-NNN] <idea or change request>"
disable-model-invocation: true
---

Arguments: $ARGUMENTS

1. Resolve the folder. For a new idea, take the next free number
   (`ls docs/specs | grep -o 'LCV-[0-9]*' | sort -V | tail -1`, and the git history's highest
   LCV id) and copy `docs/specs/_templates/spec.md` into `docs/specs/LCV-NNN-kebab-title/`.
   For an existing Draft, rewrite its `spec.md` into the template shape (keep facts, drop narrative).
2. Read only what you need: `docs/product/README.md` for principles, and code only to confirm
   the current behavior the problem describes.
3. Write the spec, ≤60 lines, what/why only: ACs numbered, EARS form, each testable. Put
   dependencies in `Depends on`.
4. Clarify: ask every open product question in one `AskUserQuestion` round (≤4 questions,
   recommended option first). Fold answers into the ACs; `Open questions` ends empty.
5. Show the ACs to the user. On approval set `Status: Specified`, run `scripts/backlog.sh`,
   commit `docs(LCV-NNN): specify <title>` (no AI trailer). If the user rejects the idea, set
   `Rejected` with a one-line reason instead.
