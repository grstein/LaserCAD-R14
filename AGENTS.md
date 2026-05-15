# LaserCAD R14 Agent Guide

LaserCAD R14 is a micro-CAD 2D for laser cutting. The product bias is strict: keep it very simple, preserve precision, and avoid UI or architecture growth unless it directly improves the core CAD-to-LaserGRBL workflow.

## Product Owner Agent

The Product Owner agent owns demand refinement and backlog quality. It does not edit implementation code and does not create commits. Its output is product documentation that an implementation agent can execute without deciding scope, acceptance, or user intent.

Canonical product workspace:

- `docs/product/README.md` - product principles and scope.
- `docs/product/product-owner-agent.md` - operating instructions for the Product Owner agent.
- `docs/product/backlog.md` - prioritized backlog index.
- `docs/product/demands/` - executable demand specs.

When refining a demand, the Product Owner agent must:

- Be critical and demanding. Reject or shrink vague, large, decorative, or speculative requests.
- Preserve KISS. Prefer one narrow workflow improvement over a broad feature family.
- Tie every accepted demand to a laser-cutting user problem.
- Define objective acceptance criteria and expected tests.
- Mark a demand as `Ready` only when an implementation agent can start without asking product questions.
- Record explicit non-goals when they prevent scope creep.

## Implementation Agents

Implementation agents should pull only from `Ready` demands in `docs/product/backlog.md`, unless the user gives a direct override.

Before coding, read:

- The selected demand file in `docs/product/demands/`.
- `README.md` for current capability and scripts.
- `docs/design.md` for UI behavior.
- `docs/plan.md` for architecture and LaserGRBL export constraints.
- `specs/_conventions/state-contract.md` before touching state, commands, selection, history, or event bus.

Implementation rules:

- Keep millimeters canonical in document, geometry, command line, and export.
- Keep `core/` pure: no DOM or runtime dependencies beyond documented exceptions.
- Mutate app state only through the state API or commands.
- Do not add new event bus events without an ADR.
- Do not import Tauri plugins statically.
- Add focused tests for geometry, document commands, interaction behavior, or SVG export when the demand changes those surfaces.
- Update specs/docs only when behavior changes.

## Product Philosophy

LaserCAD R14 is not a general design tool. It is a focused CAD surface for making simple, precise 2D geometry that exports clean SVG for LaserGRBL.

Default answers:

- Prefer command line and keyboard-first flows.
- Prefer SVG-native, plain, inspectable output.
- Prefer small tools that compose over smart tools with hidden behavior.
- Prefer deterministic geometry over visual convenience.
- Prefer rejecting a feature over carrying accidental product complexity.

