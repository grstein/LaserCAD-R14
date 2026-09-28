---
paths:
  - "docs/specs/**"
---
# Spec rules

- Copy from `docs/specs/_templates/`. Folder name `LCV-NNN-kebab-title`, next free number.
- `spec.md` ≤ 60 lines: what and why, never how. ACs numbered and in EARS form:
  `WHEN <trigger> THE SYSTEM SHALL <response>` / `IF <bad case> THEN THE SYSTEM SHALL …` /
  `WHILE <state> …`. Each AC objectively testable.
- `plan.md` ≤ 80 lines: files/symbols touched, ADRs, risks, LOC-cap seams, mutation yes/no.
- `tasks.md`: `- [ ] T<n> [AC<k>] <action> (files: a.rs, b.rs)`; 1–3 files each; test before code;
  `[P]` marks tasks with no shared files.
- Only the `Status` line changes state: Draft · Specified · Planned · In Progress · Done · Blocked · Rejected.
  Run `scripts/backlog.sh` after changing it.
