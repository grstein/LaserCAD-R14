---
paths:
  - "src/ui/**"
  - "src/render/**"
  - "src/app/**"
  - "src/tools/**"
  - "src/agent/panel.rs"
  - "src/agent/settings_ui.rs"
---
# UI design

Read `DESIGN.md` before changing anything the operator sees. A value marked **gap → LCV-NNN**
there is not yet a rule: follow the code until that spec lands. The spec's last task updates
`DESIGN.md`.

Review blockers:

- A new colour literal outside the token homes (`DESIGN.md` §3). A new colour is a new token,
  added to that table in the same commit.
- A new pointer tolerance in world millimetres. Tolerances are screen points (§5).
- A label or message that breaks the casing or terminology rules (§9), or a second name for an
  existing concept.
- A new panel, icon, emoji or decoration (§1, §12), or a change that breaks the §2 layout budgets.
- State shown by hue alone on the canvas (§1.5).
