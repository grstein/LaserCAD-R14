# LCV-164 — Canvas legibility and bed framing

- **Status**: In Progress
- **Depends on**: none
- **Implementation**: -

## Problem

The minor grid is gray 48 at 0.5 pt on the gray-40 bed, about 1.12:1
(`render/grid.rs::draw_grid`), and vanishes on most screens. The snap glyph
(`render/snaps.rs::draw_snap_marker`) has no dark edge, so it gets lost over bright geometry,
and nothing names the snap kind. Nothing marks the machine origin (0,0), where LaserGRBL starts.
Arcs are always 64 chords (`render/entities.rs::PaintOptions`, LCV-035 AC 2): big circles look
faceted, tiny ones waste shapes. Translucent overlays are drawn one segment at a time, so the
joints look darker. The camera starts at (0,0) at 1 mm/pt (`render/camera.rs::Camera`), so a
400×400 mm bed opens off-centre, and Open, autosave recovery and a bed-size change do not
reframe.

## Stories

- As an operator, I want to see the bed, the grid and the origin as soon as the drawing opens.
- As an operator, I want snap glyphs and curves that stay crisp at every zoom level.

## Acceptance criteria

1. THE SYSTEM SHALL paint minor grid lines at 1 pt with ≥1.35:1 contrast on the bed fill and
   major lines with ≥2.2:1, each on a pixel centre.
2. WHEN a snap glyph is painted THE SYSTEM SHALL paint a dark edge (≥1 pt wider, `CANVAS_BG`)
   under the `snap` stroke of the same shape.
3. WHEN a snap glyph is painted THE SYSTEM SHALL paint the kind's lower-case name (`endpoint`,
   `midpoint`, `center`, `intersection`, `quadrant`, `perpendicular`, `tangent`, `nearest`) as
   text beside the glyph, offset so it does not cover the snap point.
4. THE SYSTEM SHALL paint an origin marker at world (0,0): an X arm along +X and a Y arm along
   +Y, each 12 pt long, after the grid and before the entities.
5. WHEN an arc or circle is painted THE SYSTEM SHALL use enough chords that the sagitta of each
   chord is ≤0.25 pt on screen, with at least 8 and at most 1024 chords per full turn (replaces
   LCV-035 AC 2's fixed 64).
6. WHEN a translucent overlay (selection halo, preview, hover) strokes an entity THE SYSTEM SHALL
   paint it as one path shape per entity, not one shape per segment.
7. WHEN the app starts, a file is opened, an autosave is recovered or the bed size changes THE
   SYSTEM SHALL frame the bed as `View > Fit to Bed` does, once the viewport size is known.
8. THE SYSTEM SHALL record the grid, origin, snap label and tessellation rules in DESIGN.md §3,
   §5 and §6 in this spec's last task (amends the LCV-137 paint-order test for path shapes).

## Out of scope

- A UCS icon (no UCS in LaserCAD); a grid settings dialog or new grid density rules.
- Adjusting a layer's displayed colour: layers paint their raw colour (ADR 0012 §9).
- Snap tracking; the snap glyph shapes themselves (LCV-161).

## Open questions

- None. Decided (self-approved per user goal): LCV-035 AC 2 is replaced by AC 5; the LCV-137
  paint-order test is amended for path shapes; low-contrast layer colours are left raw (no
  display lightening, no curated palette); the snap label shows at once, with no delay.
