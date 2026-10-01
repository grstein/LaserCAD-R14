# ADR 0018 — The SVG export contract is frozen at 1.0; changing it needs a major version

- **Status**: Accepted
- **Date**: 2026-10-01
- **Deciders**: architect (LCV-202 /design; self-approved per user goal)

## Context

1.0 promises the operator that a file saved today exports the same way tomorrow and that every
1.0 mother SVG keeps opening (LCV-202). The export contract of `src/io/svg/export.rs` grew in
steps, each by its own decision: the header, Y mirror and arc rule (AGENTS.md §SVG export),
layers and per-layer files (ADR 0012), ellipses (ADR 0015 §5) and Béziers (ADR 0016 §6). Until
now the only guard was "changing it needs explicit user confirmation"; there was no freeze point
and no single byte reference covering every entity kind. LaserGRBL reads these files directly,
so a silent byte change can move a cut.

## Decision

1. **What is frozen.** The contract is the union, as worded at the 1.0 commit, of:
   - AGENTS.md §SVG export, every bullet (header, mirror, `fill="none"`, no live text or
     `filter`/`mask`/`clipPath`, layer `<g>` attributes, `File > Export layers` naming, arcs);
   - ADR 0012 §4 (mother SVG layer groups and what import reads back) and §5 (per-layer files,
     `<stem>-<file_key>.svg`, `file_key` rule);
   - ADR 0015 §5 (`<ellipse>` with optional `rotate`, elliptical `A rx ry φ`, `{:.6}` angles,
     `{:.4}` coordinates and radii, sweep and large-arc flags);
   - ADR 0016 §6 (`C`/`Q` paths for Bézier entities only; arcs never as béziers).
   Those texts are not restated here; this ADR fixes them in place.

2. **The byte reference.** `tests/fixtures/svg/contract-1.0.svg` is the exported mother of one
   contract document holding every 1.0 entity kind (line, Polyline and TEXT strokes as lines,
   circle, arc, full ellipse rotated and unrotated, elliptical arc, quadratic, cubic) on at least
   two layers, one of them empty with Output off and one with a name that needs XML escaping.
   `tests/it/io_svg/contract_1_0.rs` asserts the export is byte-identical. The fixture is never
   regenerated to make a test pass within 1.x.

3. **What counts as a change.** Within 1.x, for any document 1.0 can hold, `export_svg`,
   `export_layer_svg` and `layer_exports` produce the same bytes and file names as 1.0. Any
   change to those bytes, including a new element, attribute, number format or an additive
   encoding for a future entity kind, is a contract change.

4. **How it changes.** A contract change needs a major version (2.0), a new ADR that supersedes
   this one, an updated AGENTS.md §SVG export, a new `contract-<major>.svg` fixture, and the
   explicit confirmation of the user. A pure bug fix that restores the written contract is not a
   change, but its commit names this ADR and keeps the 1.0 fixture green.

5. **Import compatibility.** Import may grow in 1.x (tolerance, new SVG features), but it SHALL
   keep reading every 1.0 mother and per-layer file back to the same layers and geometry within
   `FORMAT_TOL`. `contract_1_0.rs` re-imports the fixture to pin it.

6. **Out of scope.** The autosave envelope (`SCHEMA_VERSION`), `settings.json`, the agent wire
   and the import report are not part of this contract; their compatibility is LCV-202 AC 4 and
   their own ADRs.

7. **Pointers.** AGENTS.md §SVG export replaces "changing it needs explicit user confirmation"
   with "frozen at 1.0 (ADR 0018); a change needs a major version", and the ADR list gains a
   0018 line. That edit lands with the LCV-202 implementation, after the svg branch merges.

## Consequences

- Future curve or shape kinds (splines, polylines as one entity, offsets) cannot change export
  bytes in 1.x; they either export through existing kinds or wait for 2.0.
- `src/io/svg/export.rs` keeps mutation testing on every change; the byte fixture makes any
  surviving mutant that alters output a failing test.
- Formatting refactors in `export.rs` are free as long as the fixture stays byte-identical.
- ADRs 0012, 0015 and 0016 stay Accepted; amending their export sections now requires
  superseding this ADR.
