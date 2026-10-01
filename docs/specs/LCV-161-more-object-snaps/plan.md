# LCV-161 — Plan

## Approach

The kernel snap engine gains four `SnapKind` variants and a `SnapKinds` set (one `bool` per
kind, serde, `Default` = all on except Nearest). A new entry point `snap_query(world, tol,
entities, anchor, kinds)` collects candidates of the enabled kinds only. Quadrant is a
per-entity candidate. Perpendicular and Tangent come from the anchor, and Nearest is computed
only when no other candidate is within `tol`. The existing `snap(world, tol, entities)` stays
as a thin wrapper (`anchor = None`, `SnapKinds::default()`), so its callers and tests do not
change. `app/snap.rs::resolve_snap` passes `ToolManager::anchor()` and
`Settings::object_snaps`. The View menu gets an `Object snap` submenu with one checkbox per
kind; a toggle calls `App::set_object_snap`, which writes the setting and persists it. F3
(`snap_enabled`) stays the master switch, unchanged. The new geometry lives in a new
`snap/anchored.rs` so `candidates.rs` stays where LCV-160 may add arc intersections.

## Touches

- `src/geometry/snap/mod.rs` — `SnapKind::{Quadrant, Perpendicular, Tangent, Nearest}`;
  `SnapKinds` (+ `contains(kind)`, serde, `Default`); `snap_query`; `snap` wraps it;
  `priority` extended (Endpoint 0 … Tangent 6, Nearest 7); doc of the selection rule.
- `src/geometry/snap/anchored.rs` (new) — `collect_quadrants`, `collect_perpendicular`,
  `collect_tangent`, `nearest_candidate`. They reuse `Line::closest_point` and
  `Arc::contains_angle`; foot/tangent points only on the entity itself (t∈[0,1], inside sweep).
- `src/geometry/snap/candidates.rs` — `Candidate`/`make_candidate` become `pub(super)` for
  reuse; no change to the intersection code.
- `src/geometry/mod.rs` — re-export `SnapKinds`, `snap_query`.
- `src/io/settings.rs` — `object_snaps: SnapKinds` with `#[serde(default)]`.
- `src/app/snap.rs::resolve_snap` — new `anchor: Option<Vec2>, kinds: SnapKinds` params.
- `src/app/viewport.rs::handle_hover` — passes `app.tool_manager.anchor()` and
  `app.settings.object_snaps`.
- `src/render/snaps.rs` — `MarkerShape::{Diamond, RightAngle, Tangent, Hourglass}` and their
  pure shape helpers. Every glyph is painted in `marker_color()` (the `snap` token).
- `src/ui/menubar.rs` + `src/ui/menubar/object_snap.rs` (new) — `Object snap` submenu below
  the Snap checkbox; the `Object snap` item list is the `SNAP_KIND_LABELS` table.
- `src/app/persist.rs` — `App::set_object_snap(kind, on)`: writes `settings.object_snaps` and
  calls `persist_settings` (`app/mod.rs` is at 287, so it does not go there).
- ADRs: none. No new module boundary: the new files are private children of existing modules.

## Risks

- LOC cap: `src/ui/menubar.rs` is at 284; it gains only `mod object_snap;` and one call (≤287).
  The submenu body lives in `menubar/object_snap.rs`. `render/snaps.rs` (156) grows to ~230;
  `geometry/snap/mod.rs` (186) to ~230; `app/viewport.rs` (247) +2 lines.
- `snap` wrapper now includes Quadrant by default: an existing kernel test whose cursor sits
  within tolerance of a quadrant could change winner. T1 runs the old suite first; any such
  test is adjusted to state the new kind explicitly, and the change is noted in the commit.
- Perpendicular/Tangent from a tool without an anchor: `anchor = None` → no candidate (AC 4),
  pinned by a test.
- Nearest must not shadow real snaps: it is computed after the distance filter, only when the
  in-range set is empty (AC 5), pinned by a test with an endpoint and a line both in range.
- LCV-162 moves pick tolerances to screen points; the snap aperture (`SNAP_TOLERANCE_PX`) is
  already screen-space and is not touched here.
- Mutation testing: no (not a flagged high-risk area).
