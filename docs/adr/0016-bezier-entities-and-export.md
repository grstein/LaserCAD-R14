# ADR 0016 — One Bézier entity of degree 2 or 3; exported as `C`/`Q` paths for Béziers only

- **Status**: Accepted
- **Date**: 2026-09-30
- **Deciders**: architect (LCV-177 /design; self-approved per user goal)

## Context

SVG `C S Q T` segments carry most curved artwork, but `document/entity.rs::Entity` has no curve
beyond circles, arcs and (ADR 0015) ellipses, so LCV-172 reports and drops them. The export
contract in `AGENTS.md` says "never béziers". LCV-177 adds native Bézier entities that open,
edit, snap and export exactly. As with ADR 0015, a new variant crosses every exhaustive `match`
on `Entity`, and the export contract changes only with an explicit decision. Constraints:
millimetres in the kernel; documents without Béziers export byte for byte as before (AC 12);
intersections, TRIM/EXTEND and SPLINE drawing are out of scope.

## Decision

1. **One variant, two degrees, stored as given** (`src/geometry/bezier.rs`,
   `Entity::Bezier(Bezier)`):

   ```rust
   pub enum Bezier { Quadratic([Vec2; 3]), Cubic([Vec2; 4]) }
   ```

   A quadratic is never elevated to a cubic, so it reopens as the same `Q` bytes. One entity per
   path segment; there is no path/polyline entity. `kind_name()` is `"quadratic"` or `"cubic"`.
   One variant keeps one arm per consumer; only export and narration look at the degree.

2. **Import** (`io/svg/path_data.rs`, `io/svg/import/path.rs`): the resolver emits
   `Cubic { from, c1, c2, to }` and `Quad { from, c, to }` in absolute SVG coordinates. `S`
   reflects the previous segment's `c2` only when that segment was `C`/`S`; `T` reflects the
   previous control point only when it was `Q`/`T`; otherwise the implied control point is the
   current point (SVG 2 §9.3.6–9.3.7). The converter maps each point through the CTM (LCV-173)
   and then `flip_y`. This is exact for every affine CTM, skew and non-uniform scale included,
   because a Bézier's image is the Bézier of its mapped points. A segment whose points all lie
   within `EPSILON` of its first point creates nothing and is reported `path curve (degenerate)`.
   `PathData::Skipped` loses its last producers and is removed.

3. **Transforms**: `Transform::bezier` and `Entity::translate` map every control point through
   `Transform::point` / `+ delta`. Exact by affine invariance; no kind change is possible.

4. **Geometry** (`geometry/bezier.rs` + `geometry/bezier/` submodules):
   - `point(t)`, `start()`, `end()`, `points()`, `map(f)`.
   - Tight `bbox`: endpoints plus the curve at each root in (0, 1) of each axis's derivative
     (linear for a quadratic, quadratic for a cubic; a leading coefficient within `EPSILON` of
     zero falls back to the linear root). Never the control polygon's box.
   - `polyline(tol_mm)`: `n` even parameter steps, every vertex `point(i/n)` on the curve, with
     `n = ⌈√(M / (8·tol))⌉` and `M = d(d−1)·max‖Pᵢ − 2Pᵢ₊₁ + Pᵢ₊₂‖` (Wang's bound on chord
     deviation). `n` is clamped to [1, 4096]; the canvas passes `0.5 · mm_per_px`.
   - `nearest(p) -> (t, Vec2)`: 64 even samples, then golden-section search on the best sample's
     two neighbouring intervals; the result is on the curve. It can miss the global foot only when
     two local minima tie within one sample step, which ranks, never corrupts, a pick or snap.
   - Crossing test against an axis-aligned `Rect` edge: the derivative roots split [0, 1] into
     monotone pieces per axis; bisection finds the one crossing of the edge line in each piece,
     kept if the other coordinate lies within the edge. Same machinery as the bbox.

5. **Interaction scope.** Béziers are never cutters, boundaries or intersection participants:
   `trim::cut_points` returns nothing for a pair with a Bézier, `extend_reach` returns `None`.
   TRIM or EXTEND aimed at a Bézier commits nothing and posts "Cannot trim/extend a curve"
   through `Tool::take_message`. Snaps: Endpoint (`start`, `end`) and Nearest only.

6. **Export contract, additive for Bézier entities only** (`src/io/svg/export.rs`):
   - cubic: `<path d="M x0 y0 C x1 y1 x2 y2 x3 y3"/>`;
   - quadratic: `<path d="M x0 y0 Q x1 y1 x2 y2"/>`;
   - every `y` through `flip_y(y, bed_height)` of the exported document, `{:.4}`, single spaces.
     A Bézier has no orientation flag, so the mirror needs nothing else.
   - Lines, circles, circular arcs and ellipses are encoded byte for byte as before. **Arcs are
     never written as béziers.** The `AGENTS.md` line "Arcs as `<path … A …>`, never béziers"
     keeps its arc rule and gains: "Bézier entities, and only they, as `C`/`Q` paths".

7. **Agent: read-only.** `query_entities` and the narration give kind `cubic`/`quadratic` and the
   points in mm, in order. MOVE/ROTATE/MIRROR/SCALE by index work through `Entity::transformed`.
   `create_drawing` is unchanged.

8. **Autosave schema**: no bump; `document/schema.rs` lists the variant as backward-compatible
   (older builds refuse the file with an error), as ADR 0015 §10.

## Consequences

- Each exhaustive `match` on `Entity` or `SnapEntity` gains one arm; the compiler lists them.
- New geometry stays in the kernel under `geometry/bezier*`; the 300-LOC cap holds by submodule.
- The export contract grows for Bézier entities only; the LCV-170 `GOLDEN` bytes and the export
  audit pin every other kind. Changes to `export.rs` and `geometry/bezier*` get mutation testing.
- Reopening an exported Bézier holds within the file's four-decimal format (`FORMAT_TOL`), not
  `EPSILON`, as for arcs and ellipses; autosave (serde) is exact.
- Follow-ups, each its own spec: intersections, TRIM/EXTEND, Midpoint/Perpendicular/Tangent snaps
  on Béziers, a SPLINE command, agent creation of curves.
