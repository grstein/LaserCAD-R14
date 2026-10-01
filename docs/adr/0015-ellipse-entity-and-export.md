# ADR 0015 — One ellipse entity with a parametric span; exported as `<ellipse>` and `A rx ry φ`

- **Status**: Accepted
- **Date**: 2026-09-30
- **Deciders**: architect (LCV-176 /design; self-approved per user goal)

## Context

`document/entity.rs::Entity` is closed over `Line`, `Circle` and `Arc`. SVG `<ellipse>`, `A`
segments with rx ≠ ry, and circles under a non-uniform `transform` (LCV-173) cannot be held, so
import reports and drops them. LCV-176 adds native ellipses. A new variant crosses every exhaustive
`match` on `Entity` (render, pick, box select, snap, trim/extend, transforms, export, import, agent
narration), and adds SVG output to the export contract in `AGENTS.md`, which never changes without
an explicit decision. Constraints: millimetres and radians in the kernel; no béziers in exports;
bytes of documents without ellipses unchanged (LCV-176 AC 11); intersections with ellipses are out
of scope (quartic roots).

## Decision

1. **One variant, lossless storage** (`src/geometry/ellipse.rs`, `Entity::Ellipse(Ellipse)`):

   ```rust
   pub struct Ellipse { pub center: Vec2, pub rx: f64, pub ry: f64,
                        pub rotation: f64, pub span: Option<EllipseSpan> }
   pub struct EllipseSpan { pub start: f64, pub end: f64, pub ccw: bool }
   ```

   `point(t) = center + R(rotation)·(rx·cos t, ry·sin t)`. `rotation` is the angle of the rx axis,
   radians CCW from +X. `span` angles are **parametric** (eccentric anomaly), not polar; `None` is
   the full ellipse. Like `Arc`, nothing is normalized: rx may be smaller than ry and angles are not
   wrapped. Sweep and containment reuse `Arc::sweep_angle`/`Arc::contains_angle` on the unit
   circle, so there is one wrap-around rule in the kernel. `kind_name()` is `"ellipse"`.

   Parametric angles because SVG §F.6.5's θ1/Δθ are parametric, an affine map keeps them up to a
   shift and a sign (so every transform is exact), and the SVG large-arc flag is "parametric sweep
   > π".

2. **Circles stay circles.** Circular geometry is always `Circle`/`Arc`: import collapses a result
   whose principal radii differ by ≤ `EPSILON` to a circle or arc (polar angle = parametric angle
   + rotation). Edit transforms are similarities (LCV-158/181/182), so no edit turns one kind into
   the other.

3. **One affine entry point**: `Ellipse::from_conjugate(center, u, v, span)` builds the ellipse
   `center + u·cos t + v·sin t` in principal form. The principal axis chosen is the one nearest to
   `u` (shift `t0 = ½·atan(2u·v / (|u|² − |v|²))`, so `|t0| ≤ π/4`, and `t0 = 0` when u ⟂ v), so an
   axis-aligned or rotated ellipse keeps its own rx, ry and rotation through export and reopen.
   `u × v < 0` (a reflection) negates the span angles and flips `ccw`. Import uses it for
   `<ellipse>`, elliptical `A` segments and circles under a non-uniform CTM.

4. **Transforms** (`geometry/transform.rs::Transform::ellipse`): rotate adds the angle to
   `rotation`; uniform scale multiplies rx and ry; mirror across a line at angle θ sets
   `rotation' = 2θ − rotation`, negates span angles and flips `ccw` (endpoints keep their roles).
   Translation moves the center.

5. **Export contract, additive for the new kind only** (`src/io/svg/export.rs`; `AGENTS.md` §SVG
   export gains these bullets with the implementation):
   - full ellipse: `<ellipse cx cy rx ry/>` in SVG coordinates (`cy` through `flip_y`), plus
     `transform="rotate(a cx cy)"` only when `a` is not zero;
   - elliptical arc: `<path d="M sx sy A rx ry φ large sweep ex ey"/>`, never béziers;
   - `a = φ = −rotation` in degrees (the Y mirror negates angles), normalized into (−180, 180] and
     written `{:.6}`; "not zero" means it does not print as `0.000000`. Coordinates and radii keep
     `{:.4}`. `sweep = 0` for a CCW world span (inverted by the mirror, as for arcs); `large = 1`
     iff the parametric sweep > π.
   - Lines, circles and arcs are encoded byte for byte as before.

6. **Interaction scope.** Ellipses are never cutters, boundaries, or intersection participants:
   `trim::cut_points` returns nothing for any pair with an ellipse, `extend_reach` returns `None`.
   TRIM or EXTEND aimed at an ellipse commits nothing and posts "Cannot trim/extend an ellipse"
   through `Tool::take_message`. Snaps: Endpoint (span ends), Center, Quadrant (the four vertices
   `t ∈ {0, π/2, π, 3π/2}` inside the span) and Nearest only.

7. **Nearest point and picking** (`Ellipse::nearest`, `Ellipse::distance_to_point`): the robust
   bisection foot on the full ellipse (Eberly, "Distance from a point to an ellipse"); on a span,
   the foot if its parameter is inside the span, else the nearer endpoint. This is exact while the
   point is closer to the curve than its smallest radius of curvature (`min(rx,ry)²/max(rx,ry)`),
   which covers every pick and snap aperture at working zoom; farther away it only ranks candidates.
   Box selection: window = exact bbox (axis extremes inside the span plus endpoints) inside the
   rectangle; crossing = window or the curve meets an edge (segment mapped into the unit-circle
   frame, then the existing circle test and the span check).

8. **Painting**: `Ellipse::polyline(tol_mm)` samples evenly in the parameter, every vertex on the
   curve; the step satisfies `max(rx,ry)·(1 − cos(Δt/2)) ≤ tol` (a bound on chord deviation, since
   the ellipse is a linear image of the unit circle with norm `max(rx,ry)`). The canvas passes
   `0.5 · mm_per_px`; the count is clamped to [8, 4096], which keeps the bound up to
   `max(rx,ry) ≈ 1.7·10⁶ px`.

9. **Agent: read-only.** `query_entities` and the narration name the kind `ellipse` with its
   centre, radii, rotation and, for a span, start/end degrees and direction. MOVE/ROTATE/MIRROR/
   SCALE by index work through `Entity::transformed`. `create_drawing` is unchanged.

10. **Autosave schema**: no bump. `document/schema.rs` lists a new `Entity` variant as
    backward-compatible (older builds refuse it with an error).

## Consequences

- Each exhaustive `match` on `Entity` or `SnapEntity` gets one arm; the compiler finds them all.
- New geometry lives in `geometry/ellipse.rs` plus `geometry/ellipse/` submodules to respect
  the 300-LOC cap; the kernel stays pure.
- The export contract grows for the new kind only; the LCV-170 golden bytes and the export audit
  keep documents without ellipses unchanged. Changes to `export.rs` get mutation testing.
- Reopening an exported ellipse holds within the file's four-decimal format (`FORMAT_TOL`,
  `roundtrip_props.rs`), not `EPSILON`, as for circular arcs.
- Follow-ups: intersections, perpendicular/tangent snaps, TRIM/EXTEND on ellipses, and an
  ELLIPSE command each need their own spec. Bézier entities (LCV-177) follow the same pattern.
