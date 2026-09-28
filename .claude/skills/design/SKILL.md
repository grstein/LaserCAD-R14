---
name: design
description: SDD step 2 for LaserCAD v2 — turn a Specified spec into plan.md + tasks.md, calling the architect only for boundary changes, and get the user's approval to mark it Planned.
argument-hint: "LCV-NNN"
disable-model-invocation: true
---

Spec: `docs/specs/$ARGUMENTS-*/`.

1. Read `spec.md` (must be `Specified`; if Draft, run /specify first). Read the code the ACs
   touch; check near-cap files with `scripts/loc-cap.sh`-style awk.
2. Boundary check: does it add a module, a cross-module trait/type, a dependency, a new thread or
   channel, or contradict an ADR? Only then spawn `architect` (model: opus) with the folder and the
   exact question, and use its ≤25-line result.
3. Write `plan.md` (≤80 lines) and `tasks.md` from `docs/specs/_templates/`: every AC maps to
   ≥1 task; test before code; 1–3 files per task; `[P]` where tasks share no file; last task is
   the CHANGELOG line if user-visible. State mutation testing yes/no in Risks.
4. Consistency pass: every AC has a task, every task cites an AC or is CHANGELOG, nothing in the
   plan exceeds the spec's scope.
5. Present approach + task list briefly. On approval set `Status: Planned`, run
   `scripts/backlog.sh`, commit `docs(LCV-NNN): plan <title>` (no AI trailer).
