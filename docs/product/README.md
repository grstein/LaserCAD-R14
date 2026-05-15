# LaserCAD R14 Product Workspace

This directory is the product workspace for LaserCAD R14. It exists to turn user requests into small, executable demands for AI implementation agents.

## Product Intent

LaserCAD R14 is a micro-CAD 2D for laser cutting, compatible with LaserGRBL through plain SVG export. It should feel close to a compact AutoCAD R14 workflow: drawing area first, keyboard-first command line, precise millimeter geometry, minimal chrome, and no decorative product surface.

The product is successful when a user can quickly draw simple laser-cut parts: plates, holes, slots, straight cuts, arcs, and small assemblies, then export a clean SVG without post-processing.

## Non-Negotiables

- Keep it very simple.
- Millimeters are the canonical unit.
- SVG is both the rendering surface and the export target.
- The command line and keyboard shortcuts are first-class workflows.
- LaserGRBL compatibility is more important than rich SVG features.
- Existing tools must remain predictable before new tools are added.

## Product Owner Workflow

1. Capture the raw request.
2. Identify the actual user problem.
3. Shrink the scope to the smallest useful change.
4. Define clear acceptance criteria and expected tests.
5. Add or update a demand file in `docs/product/demands/`.
6. Link it from `docs/product/backlog.md`.
7. Mark it `Ready` only when implementation requires no product decisions.

## Sources Of Truth

- `README.md` describes current capabilities and scripts.
- `docs/design.md` defines the UI and interaction philosophy.
- `docs/plan.md` records technical constraints and LaserGRBL assumptions.
- `docs/adr/` records architectural decisions.
- `specs/` describes module-level contracts.
- `AGENTS.md` defines how agents should collaborate in this repo.

