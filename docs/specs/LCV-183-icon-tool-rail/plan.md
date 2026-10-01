# LCV-183 — Plan

## Approach

Icons are plain functions `fn(&egui::Painter, egui::Rect, egui::Stroke)` that draw line
primitives into a 20 pt square. `ToolEntry` gains `icon: IconFn`, so `TOOLS` stays the single
source for the rail, the Tools menu and the shortcuts dialog (menus keep `label`). The rail
becomes a two-column `Grid`-free layout: `ui.horizontal` holding two `ui.vertical` columns of
`icon_button`s (draw group left, modify group right), then a separator and the `AI` toggle, all
inside the existing `ScrollArea`. `icon_button` allocates a 32×32 clickable rect, paints the
selectable background from `ui.style().interact_selectable(&resp, selected)` and then the icon;
it returns the `Response` so `on_hover_text` works as today. The rail width becomes fixed
geometry (2 × 32 + 4 gap + 2 × 4 frame margin = 76 pt) instead of measured label text.

## Touches

- `src/ui/icons.rs` (new) — `IconFn` alias, `icon_button`, shared helpers (`marker` filled
  2.5 pt square, `arrow_head`, `dashed`), the inner-square/stroke constants.
- `src/ui/icons/draw.rs` (new) — `select`, `line`, `polyline`, `rect`, `circle`, `arc`, `text`.
- `src/ui/icons/modify.rs` (new) — `move_`, `copy`, `rotate`, `mirror`, `scale`, `trim`,
  `extend`, `delete`, `dist`.
- `src/ui/mod.rs` — `mod icons;` (crate-private).
- `src/ui/toolbar.rs` — `draw_toolbar` two columns + `AI` toggle; `tool_hover_text` new format
  `<Label> — <key> · <WORD>` / `<Label> — <WORD>` (WORD = `tool_name.to_uppercase()`);
  `AGENT_TOGGLE_LABEL = "AI"`, tooltip `AI Assistant`.
- `src/ui/toolbar/table.rs` (new) — `ToolEntry` (+ `icon`) and `TOOLS`, re-exported by
  `toolbar.rs` so every `crate::ui::toolbar::TOOLS` path keeps working.
- `src/app/panels.rs` — `toolbar_width` → constant `RAIL_WIDTH` (76 pt) and the rail's
  `SidePanel` frame inner margin 4 pt; drop `toolbar_width_for` and its clamp test.
- Tests: `tests/it/ui/icon_tool_rail.rs` (new); `tests/it/ui/compact_chrome_and_action_hints.rs`
  AC 1/2 rail tests rewritten from labels to tooltips/buttons; `src/ui/toolbar.rs` unit tests
  for the new tooltip format.
- `DESIGN.md` §2 (rail ≤80 pt), §7 (tool rail line); `CHANGELOG.md`.
- ADRs: none (no new dependency, module stays inside `ui/`, no trait change).

## Test approach

- Icons (AC 1, 2): unit tests in `ui/icons.rs` run each `IconFn` on a `Painter` of a test
  `Context` layer and collect the shapes: every shape is a vector shape (`LineSegment`, `Path`,
  `Circle`, `Rect`), none is `Text`/`Mesh`; all lie inside the 20 pt square; strokes are 1.5 pt in
  the given colour; the 16 shape "signatures" (count and kind of shapes plus rounded
  coordinates) are pairwise distinct.
- Layout, tooltip, active fill, click, AI toggle, width, no-scroll (AC 3–9): integration test on
  painted output. Button centres are computed from the toolbar `PanelState` rect and the fixed
  geometry (`RAIL_*` constants are mirrored as test constants and pinned by the width assert);
  hovering a centre must paint exactly the expected tooltip text (proves order and columns);
  clicking it must change `active_tool_name`; the selected fill `Rect` sits under the active
  button only; at 800×600 / 1024×600 / 1280×800 every one of the 17 centres lies inside the
  panel rect with no scroll offset; at 220 pt height a wheel scroll reaches the `AI` text.

## Risks

- LOC cap: `ui/toolbar.rs` is at 257; the table moves to `ui/toolbar/table.rs` first (T3) so
  adding `icon` fields never crosses 270. Icons split by group keep each file < 200.
- Tests that found the rail by the text `Select`/`Agent` (LCV-140) must move to tooltips; T8
  rewrites them in the same commit that changes the rail.
- `AGENT_TOGGLE_LABEL` "Agent" → "AI" overlaps LCV-167 (which renames the rest); only the rail
  toggle changes here.
- Mutation testing: no.
