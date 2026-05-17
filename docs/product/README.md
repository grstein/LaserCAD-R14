# LaserCAD v2 — Product Workspace

This directory holds product artifacts: principles, the demand format, the prioritized backlog, and the demand files themselves.

## Product principles

LaserCAD v2 is a focused CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL. It is not a general-purpose design tool.

Default answers, in priority order:

1. **Prefer command line and keyboard-first flows.** This is an AutoCAD R14 clone, not a graphics editor.
2. **Prefer SVG-native, plain, inspectable output.** Lines, circles, arcs, paths. No effects, no transforms, no live text.
3. **Prefer small tools that compose** over smart tools with hidden behavior.
4. **Prefer deterministic geometry** over visual convenience. mm canonical, radians in the kernel.
5. **Prefer rejecting a feature** over carrying accidental product complexity.

## Scope target — v0.1.0

Parity with v1.0.0 + the unreleased TEXT command + Agent Harness MVP:

- Drawing tools: Line, Polyline, Rect, Circle, Arc.
- Modify tools: Select (point + window + crossing), Move, Trim, Extend, Delete.
- Snaps: endpoint, midpoint, center, intersection.
- Undo/Redo (200-deep).
- Command line: absolute `X,Y`, relative `@X,Y`, distance, tool aliases, toggles, agent prefix.
- SVG export: cut / mark / engrave presets, LaserGRBL-compatible.
- SVG import: strict subset (what v2 emits, plus tolerant whitespace).
- TEXT command: ASCII strings as engravable line geometry via Hershey font.
- Autosave: debounced 800 ms, restore on boot.
- Native dialogs (Open / Save As).
- Recent files list.
- Agent harness: OpenAI-compatible LLM (default OpenRouter), opt-in, multi-turn loop with iteration cap.

## Explicit non-goals (v0.1.0)

- No fillet / chamfer / offset.
- No SVG transforms or live text on import.
- No mobile / touch.
- No layers, no blocks, no xref.
- No DXF (planned but not in v0.1.0 scope).
- No G-code emission.
- No multi-document MDI.

## Demand workflow

See [`product-owner-agent.md`](product-owner-agent.md) for the demand format, lifecycle, and the product-owner agent's job.

See [`backlog.md`](backlog.md) for the prioritized backlog by state.

Demand files live in [`demands/`](demands/) as `LCV-NNN-<kebab-title>.md`.
