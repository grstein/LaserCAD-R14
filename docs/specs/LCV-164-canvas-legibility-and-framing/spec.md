# LCV-164 — Canvas legibility and bed framing

- **Status**: Draft
- **Depends on**: none
- **Implementation**: -

## Problem

The canvas is hard to read, and it opens in the wrong place:

- **Grid.** The minor grid is gray 48 at 0.5 pt on the gray-40 bed, about 1.12:1
  (`render/grid.rs::draw_grid`). It disappears on most screens. The major grid is 2.34:1.
- **Snap glyph.** The 8 pt orange glyph (`render/snaps.rs`) has no dark edge, so it gets lost
  over bright geometry. Nothing names the snap kind.
- **Origin.** Nothing marks the machine origin (0,0), which is where LaserGRBL starts.
- **Arcs and overlays.** Arcs are always 64 chords, whatever the zoom
  (`render/entities.rs::PaintOptions`, LCV-035 AC 2). Big circles look faceted and tiny ones waste
  shapes. Translucent overlays are drawn one segment at a time, so the joints look darker.
- **Framing.** On startup the camera is at (0,0) at 1 mm/pt (`render/camera.rs::Camera`), so a
  400×400 mm bed opens off-centre. Open, autosave recovery and a bed-size change do not
  reframe either.

## Stories

- As an operator, I want to see the bed, the grid and the origin as soon as the drawing opens.
- As an operator, I want snap glyphs and curves that stay crisp at every zoom level.

## Direction

- Grid: minor ≈1.4:1 at 1 pt, major ≈2.3:1. The grid stays visible but never competes with
  geometry. Lines sit on pixel centres (`Painter::round_to_pixel_center`).
- Snap glyph: a 1 pt dark edge under the orange stroke, and a small kind label (`endpoint`,
  `midpoint`, …) next to it. The glyph table in DESIGN.md §6 also reserves the R14 glyphs for
  LCV-161.
- Origin marker: a small L-shaped X/Y indicator at world (0,0), drawn under the geometry.
- Arcs are tessellated to a chord tolerance in screen points, not a fixed segment count.
  Translucent overlays (selection halo, preview) are drawn as one path per entity.
- Frame the bed (Fit to Bed) on startup, after Open, after autosave recovery and after a
  bed-size change.
- DESIGN.md §3, §5 and §6 are updated in this spec's last task.

## Acceptance criteria

To be written by /specify.

## Out of scope

- A UCS icon that follows a user coordinate system (no UCS in LaserCAD).
- Grid settings dialog, and adaptive grid density beyond `render/grid.rs::pick_minor_spacing_mm`.
- Choosing layer colours (LCV-156). This spec only keeps the displayed colour readable.

## Open questions

- ✱ LCV-035 AC 2 pins `arc_segments == 64`. The chord-tolerance change replaces that AC.
- ✱ The LCV-137 paint-order test treats grid lines as `LineSegment` shapes. Drawing the grid or
  overlays as paths amends that test.
- LCV-156 paints the raw layer colour (ADR 0012 §9). Should a layer colour under 3:1 on the bed
  (e.g. `#0000ff`, 1.7:1) be lightened for display, or should layers offer a curated palette?
  DESIGN.md §3 sets the ≥3:1 target.
- Should the snap label be on by default, or only after the glyph has been still for a moment?
