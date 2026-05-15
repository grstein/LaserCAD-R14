# LCD-002 - Document Command Line Behavior Gaps

Status: Ready
Priority: P1
Owner: Product Owner
Implementation: Unassigned

## Problem

The command line is the canonical LaserCAD R14 workflow, but future implementation agents need an explicit gap list before changing behavior. Current docs describe the intended interaction, while implementation may not cover every documented case equally.

The product needs a short audit document that separates implemented behavior, missing behavior, and intentionally deferred behavior.

## Smallest Useful Outcome

Create a concise command line behavior audit in docs. Do not change code in this demand. The output should let the Product Owner split future implementation tasks cleanly.

## Scope

In:

- Compare `docs/design.md`, `docs/atalhos.md`, and current command line implementation.
- Document implemented command submission behavior.
- Document missing or uncertain behavior as candidate demands.
- Identify any behavior that should stay deferred to preserve KISS.

Out:

- Implementing command line changes.
- Rewriting existing docs wholesale.
- Adding new commands.
- Changing shortcuts.

## Acceptance Criteria

- Add a new product doc under `docs/product/` describing command line behavior gaps.
- The doc lists at least: command aliases, coordinate input, relative input, direct distance input, Enter/Space behavior, Esc behavior, and history navigation.
- Each gap is labeled as `candidate demand`, `implemented`, or `deferred`.
- `docs/product/backlog.md` is updated only if the audit creates new demand files.

## Expected Tests

- No automated tests are required because this is a documentation audit.
- Run no formatter that rewrites unrelated files.

## Notes For Implementation Agents

- Relevant docs/specs/files: `docs/design.md`, `docs/atalhos.md`, `src/ui/command-line.ts`, `src/app/shortcuts.ts`, `src/tools/tool-manager.ts`.
- Risk: treating the audit as permission to implement broad command behavior. Keep this demand documentation-only.
- Product decision: preserve keyboard-first flow, but split actual behavior changes into narrow follow-up demands.

