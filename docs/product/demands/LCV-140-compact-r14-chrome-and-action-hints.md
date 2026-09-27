# LCV-140 - Compact R14 chrome and useful action hints

- **Status**: Ready
- **Phase**: 11
- **Depends on**: LCV-115, LCV-116, LCV-132, LCV-139, LCV-141
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The left toolbar (`src/ui/toolbar.rs::draw_toolbar`) has no width ceiling, so
it can take more horizontal space than its eleven text labels need. Toolbar
buttons and the status bar's `SNAP`/`GRID`/`ORTHO` indicators carry no hover
guidance beyond their own label, the agent toggle is an icon-only `"🤖"` with
no visible text, the coordinate readout (`format_coords`) carries no unit,
and the read-only preset badge (`format_preset`) shows `CUT`/`MARK`/`ENGRAVE`
with no explanation that the preset applies to the whole document at export.

## Scope

- Cap the left tool rail's width to what its eleven text labels need, with no
  change to which tools exist, their order or their shortcuts.
- Add hover-text hints for every toolbar button and status-bar mode
  indicator, derived from the existing `TOOLS` table and mode set — no new
  keyboard shortcut is invented.
- Replace the agent toggle's icon-only label with a short visible text label.
- Add an explicit `mm` unit to the status-bar coordinate readout.
- Explain, in the preset badge's hover text, that the selected preset
  receives every entity on export and the other two groups are written empty
  (LCV-115).
- Verify, by rendering measurement, that toolbar + status bar (+ the agent
  panel when open) leave a contained, non-overlapping canvas at 800x600,
  1024x600 and 1280x800.

## Out of scope

- A ribbon, an icon package, new keyboard bindings, layers, a unit switcher,
  an editable preset badge, or a visual theme overhaul.
- F1 shortcuts-dialog headroom — that is LCV-134's (Done); this demand does
  not touch `src/ui/shortcuts_dialog.rs`.

## Acceptance criteria

1. All eleven `TOOLS` entries (`src/ui/toolbar.rs::TOOLS`) stay reachable, in
   the same order; every hover hint is derived from that entry's own `label`
   and `shortcut` field, not a second hand-typed table. The active tool's
   highlighted state and every existing `TOOL_KEYS` mnemonic are unchanged.
2. At the default egui font, the tool rail's outer width is at most 120pt
   while every label renders in full (no truncation, no icon-only
   fallback); the existing draw/modify separator (`MODIFY_GROUP_START`) is
   preserved. Vertical scrolling inside the rail activates only when the
   panel's available height is smaller than the eleven-entry content.
3. Every toolbar button carries a hover tooltip naming its keyboard shortcut
   where one exists (`Select` has none and gets a hint with no invented key);
   the three status-bar mode indicators (`SNAP`/`GRID`/`ORTHO`) carry a
   tooltip naming their existing key (`F3`/`F7`/`F8`). The agent toggle shows
   a short visible text label instead of the icon-only `"🤖"`, keeping its
   existing click behavior and hover text.
4. The coordinate readout shows an explicit `mm` unit for both axes. The
   preset badge gains hover text stating that the current preset
   (`CUT`/`MARK`/`ENGRAVE`) receives every exported entity and the other two
   groups are written empty; the badge's visible label, read-only nature and
   position in the status-bar order are unchanged.
5. At 800x600, 1024x600 and 1280x800, every status segment — active tool,
   entity count, preset badge, the three mode indicators and the autosave
   indicator — paints fully inside its row, with no clipped or culled run. An
   optional second status row may hold entity-count/autosave detail; the two
   status rows together are at most 56pt tall at the default font.
6. With the agent panel open at LCV-141's one-third ceiling (≤ 266.67pt at an
   800pt application width), the toolbar, status bar, command dock (LCV-139)
   and agent panel together still leave at least 320x300pt of canvas. A
   six-digit signed coordinate pair (e.g. `X: -1234.56mm  Y: -1234.56mm`) and
   a four-digit entity count do not clip or hide any mode indicator, tool
   button or the preset badge.
7. Hovering any new hint mutates no document/history state and sends no
   command; each toolbar/mode click still fires its existing action exactly
   once. Preset semantics (LCV-115) and exported SVG bytes are unchanged.

## Expected tests

- AC 1-3: a table-derived inventory test (every hint traces back to a
  `TOOLS`/mode-table entry, not a hand-typed literal), real painted hover-text
  assertions, an active-tool-highlight test, and one real click test per mode
  plus the agent toggle.
- AC 4-6: painted-text assertions (`tests/harness/paint.rs`) for the `mm`
  suffix and the preset tooltip text, full-precision layout-bounds
  measurements at all three sizes with the agent panel both open and closed,
  and large signed-coordinate / four-digit-count fixtures.
- AC 7: hover-only frames leave document entity count, history revision and
  `dirty_since` unchanged (extending the existing
  `toggle_click_flips_only_its_own_flag` pattern), and the LCV-115 export
  byte-identity regression stays green.

## Open questions

None at product level. Any rail/status allocation claim must be demonstrated
by a rendering measurement (a painted-text or layout assertion), never a
source scan alone (AGENTS.md, "A rendering acceptance criterion is not
satisfied by a source scan alone").

## Notes

Primary files: `src/ui/toolbar.rs`, `src/ui/statusbar.rs`,
`src/ui/menubar.rs`, `src/app/panels.rs`. Consume `tests/harness/paint.rs`
(LCV-132, Done) for the painted-text assertions AC 3-6 need. LCV-134 (Done)
already owns F1 dialog headroom; do not re-open `shortcuts_dialog.rs` here.
AC 6's figure is LCV-141's own ceiling (`min(300pt, application_width/3)`,
≈266.67pt at 800pt) — land after LCV-141 so the number is real rather than
assumed.
