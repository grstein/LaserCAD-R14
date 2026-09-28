---
name: next
description: Show the LaserCAD v2 SDD board — which specs are ready to implement, which need design or specification — and suggest what can run in parallel.
disable-model-invocation: true
---

1. Run `scripts/backlog.sh --next` (Planned / In Progress with deps Done) and read
   `docs/product/backlog.md` for Specified and Draft rows.
2. Answer in ≤8 lines: what to `/implement` now; what needs `/design` or `/specify`; which
   `/implement` candidates touch disjoint files (from their `plan.md`) and may run in parallel
   git worktrees, each with its own `CARGO_TARGET_DIR`.
