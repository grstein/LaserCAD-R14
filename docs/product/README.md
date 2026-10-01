# LaserCAD — Product Workspace

This directory holds product principles and the generated backlog. Demand specs live in `docs/specs/`.

## Product principles

LaserCAD is a focused CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL. It is not a general-purpose design tool.

Default answers, in priority order:

1. **Prefer command line and keyboard-first flows.** This is an AutoCAD R14 clone, not a graphics editor.
2. **Prefer SVG-native, plain, inspectable output.** Lines, circles, arcs, paths. No effects, no transforms, no live text.
3. **Prefer small tools that compose** over smart tools with hidden behavior.
4. **Prefer deterministic geometry** over visual convenience. mm canonical, radians in the kernel.
5. **Prefer rejecting a feature** over carrying accidental product complexity.

## Scope target — v0.1.0

Parity with the original LaserCAD R14 1.0 + the TEXT command + Agent Harness MVP:

- Drawing tools: Line, Polyline, Rect, Circle, Arc.
- Modify tools: Select (point + window + crossing), Move, Trim, Extend, Delete.
- Snaps: endpoint, midpoint, center, intersection.
- Undo/Redo (200-deep).
- Command line: absolute `X,Y`, relative `@X,Y`, distance, tool aliases, toggles, agent prefix.
- SVG export: cut / mark / engrave presets, LaserGRBL-compatible.
- SVG import: strict subset (what LaserCAD emits, plus tolerant whitespace).
- TEXT command: ASCII strings as engravable line geometry via Hershey font.
- Autosave: debounced 800 ms, restore on boot.
- Native dialogs (Open / Save As).
- Recent files list.
- Agent harness: OpenAI-compatible LLM (default OpenRouter), opt-in, multi-turn loop with iteration cap.

## Explicit non-goals (v0.1.0)

- No fillet / chamfer / offset.
- No SVG transforms or live text on import.
- No mobile / touch.
- No blocks, no xref. (Layers were lifted from this list by LCV-156.)
- No DXF (planned but not in v0.1.0 scope).
- No G-code emission.
- No multi-document MDI.

## Demand workflow

Demands follow lean Spec-Driven Development (`AGENTS.md` §Workflow): one folder per demand under
[`docs/specs/`](../specs/) with `spec.md`, `plan.md` and `tasks.md`, created from
[`docs/specs/_templates/`](../specs/_templates/). [`backlog.md`](backlog.md) is generated from the
`Status` lines by `scripts/backlog.sh`.
