# LCV-161 — Tasks

- [x] T1 [AC1, AC5, AC6, AC9] Test: kernel `snap_query` tests. Quadrant on a circle and only
  inside an arc's sweep; Nearest only when nothing else is in range and only when enabled; the
  tie order Endpoint > Intersection > Midpoint > Center > Quadrant > Perpendicular > Tangent; a
  disabled kind gives no candidate. Plus the `SnapKinds::default()` values (files:
  src/geometry/snap/tests.rs)
- [ ] T2 [AC1, AC5, AC6, AC9] Add `SnapKind` variants, `SnapKinds`, `snap_query`, the `snap`
  wrapper, the extended `priority`, Quadrant and Nearest candidates; old kernel tests stay
  green or are restated (files: src/geometry/snap/mod.rs, src/geometry/snap/anchored.rs,
  src/geometry/snap/candidates.rs)
- [ ] T3 [AC2, AC3, AC4] Test: Perpendicular foot on a segment (and none when the foot falls
  on the extension), up to two feet on a circle, feet only inside an arc's sweep; Tangent
  points from an outside anchor on a circle and inside an arc's sweep; no candidate without an
  anchor, with the anchor on or inside the circle, or at the centre; only feet/tangent points
  within the aperture count (files: src/geometry/snap/tests.rs)
- [ ] T4 [AC2, AC3, AC4] Implement `collect_perpendicular` and `collect_tangent` in the
  anchored helpers and wire them into `snap_query` (files: src/geometry/snap/anchored.rs,
  src/geometry/snap/mod.rs, src/geometry/mod.rs)
- [ ] T5 [P] [AC8, AC10] Test: `Settings` without `object_snaps` in JSON loads
  `SnapKinds::default()`; a round-trip keeps a toggled kind (files: src/io/settings/tests.rs)
- [ ] T6 [AC8, AC10] Add `Settings::object_snaps` with `#[serde(default)]` (files:
  src/io/settings.rs)
- [ ] T7 [AC2, AC3, AC9] Test (app level): LINE with a first point, hovering near a circle's
  tangent point sets `active_snap.kind == Tangent`, and near a segment's foot sets
  `Perpendicular`; with that kind off in `settings.object_snaps` it does not; F3 off gives no
  snap at all (files: tests/it/app/object_snaps.rs, tests/it/app/mod.rs)
- [ ] T8 [AC2, AC3, AC9] `resolve_snap` takes `anchor` and `kinds`; `handle_hover` passes
  `tool_manager.anchor()` and `settings.object_snaps`; update the in-file `resolve_snap`
  tests (files: src/app/snap.rs, src/app/viewport.rs)
- [ ] T9 [P] [AC7] Test: painting each new kind with `draw_snap_marker` in a test
  `egui::Context` emits shapes in `marker_color()` of the expected form: a 4-vertex diamond, a
  right-angle mark, a circle plus a tangent bar, and an hourglass. Existing kinds keep their
  shapes (files: src/render/snaps.rs)
- [ ] T10 [AC7] Implement the four `MarkerShape` variants and their pure shape helpers
  (files: src/render/snaps.rs)
- [ ] T11 [AC8] Test: with the View menu and its `Object snap` submenu opened through the
  harness, the painted text shows one checkbox label per kind (8). Toggling a kind through
  `App::set_object_snap` flips `settings.object_snaps` and writes the settings file at an
  injected temp `settings_path` (files: tests/it/ui/object_snap_menu.rs, tests/it/ui/mod.rs)
- [ ] T12 [AC8] Add the `Object snap` submenu below the `Snap\tF3` checkbox. Its
  `SNAP_KIND_LABELS` table drives one checkbox per kind; a change calls
  `App::set_object_snap`, which writes the setting and persists it. The existing menubar
  source-scan tests stay green (files: src/ui/menubar.rs, src/ui/menubar/object_snap.rs,
  src/app/persist.rs)
- [ ] T13 CHANGELOG `[Unreleased]` line: Quadrant, Perpendicular, Tangent and Nearest snaps;
  View > Object snap per-kind toggles (files: CHANGELOG.md)
