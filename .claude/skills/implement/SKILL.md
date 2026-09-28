---
name: implement
description: SDD step 3 for LaserCAD v2 — execute a Planned spec with implementer-rust (opus), confirm the gate, one review round by reviewer-rust (fable), then close the demand as Done and push.
argument-hint: "LCV-NNN"
disable-model-invocation: true
---

Spec: `docs/specs/$ARGUMENTS-*/`. Status must be `Planned` or `In Progress` (resume).

1. Set `Status: In Progress` (if not already), `scripts/backlog.sh`, note `base=$(git rev-parse HEAD)`.
2. Spawn `implementer-rust` (model: opus) with: the folder path, "execute unchecked tasks in
   tasks.md, one commit per task, finish with scripts/gate.sh; never add Co-Authored-By or any
   AI trailer; stop and report on any deviation; report ≤10 lines".
3. If it stopped on a deviation: product question → ask the user; design question → fix
   `plan.md`/`tasks.md` with the user's approval; then resume step 2.
4. Run `scripts/gate.sh` yourself; do not trust a reported pass. Red → back to the implementer once.
5. Spawn `reviewer-rust` (model: fable) with the folder and range `$base..HEAD`.
   - `APPROVE` → step 6.
   - `BLOCKING` → one fix round by the implementer with the findings verbatim, gate again, then
     re-review only those findings. Still blocking → stop and ask the user.
6. Close in one commit: `Status: Done`, `Implementation: <first>..<last commit>`, CHANGELOG
   `[Unreleased]` line if user-visible, `scripts/backlog.sh`; commit
   `docs(LCV-NNN): mark Done` (no AI trailer); `git push origin main`.
7. Report to the user in ≤3 lines: what shipped, commit range, what `scripts/backlog.sh --next` offers.
