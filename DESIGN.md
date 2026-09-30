# LaserCAD v2 — UI Design Directives

These are the UI design directives. They are not the `/design` SDD step, which writes `plan.md`.

This file names the visual and interaction rules that the chrome and canvas follow.
Precedence: `AGENTS.md` > ADRs (`docs/adr/`) > this file > code comments. It does two things:

- It records decisions already made. Their source is a Done spec, an ADR or shipped code.
- It sets rules for new work.

A target that would contradict a Done AC or an ADR is never written as a rule here. It appears
as **gap → LCV-NNN** (a Draft spec) or **gap → F#** (a fast-lane defect, §11). The gap closes in
that change, and that change's last task updates this file.

## 1. Principles

1. **Precision is visible before the click.** Whatever a click or Enter will do (the point, the
   entity, the mode) is shown while the cursor is still moving.
2. **Keyboard and command line first.** Every action has a command word or a key. The mouse is a
   shortcut, never the only way.
3. **R14 where it helps precision; modern feedback only where it removes ambiguity.** Keep the
   AutoCAD R14 layout, prompts, glyphs and keys. Add a modern cue (hover highlight, dashed
   crossing box, severity colour) only when R14 leaves the operator guessing.
4. **Quiet chrome, loud geometry.** The chrome is neutral dark gray. Colour belongs to the
   canvas and to state.
5. **State is shown by form first, hue second.** Halo, dash, thickness and glyph carry the
   meaning. Entity hue belongs to layers (LCV-156), so no state may rely on hue alone.
6. **One name per concept, one pattern per interaction** (§9).
7. **Fits 800×600.** Every surface passes the budgets in §2.
8. **No decoration.** No emoji, gradients, shadows or animation (§12). Icons are functional,
   not decoration: flat line icons with R14 metaphors on the tool rail and in the menus
   (user decision 2026-09-30; rail: LCV-183; menus: gap → LCV-166). Modern means flat and consistent, never
   ornamental (LCV-184): windows and menus carry no shadow, one 1 pt `border`, one radius per
   kind (§5), and every chrome colour is a §3 token in `ui/theme.rs::apply_theme`.

## 2. Layout

```
┌─ menubar: File  Edit  View  Tools  Help ───────────────────────┐
├──────────┬─────────────────────────────────┬───────────────────┤
│ tool     │ canvas                          │ AI Assistant      │
│ rail     │ bed · grid · geometry ·         │ panel             │
│ ≤80 pt   │ feedback                        │ ≤ ⅓ app width,    │
│          │                                 │ closed by default │
├──────────┴─────────────────────────────────┴───────────────────┤
│ command dock: prompt row + editor row                  ≤64 pt  │
├────────────────────────────────────────────────────────────────┤
│ status bar                                             ≤56 pt  │
└────────────────────────────────────────────────────────────────┘
```

- Panels: `app/panels.rs` (`menubar`, `statusbar`, `command_line`, `toolbar` left,
  `agent_panel` right); canvas `app/viewport.rs::draw`.
- Budgets, proven at 800×600, 1024×600 and 1280×800 at the default font: rail ≤80 pt, never
  scrolling at these sizes (LCV-183), status rows ≤56 pt (LCV-140); dock ≤64 pt, editor ≥240 pt wide (LCV-139); agent panel ≤ ⅓
  of the app width (LCV-141); canvas ≥320×300 pt with every panel open (LCV-140 AC 6).
- New UI goes into an existing region (LCV-156 puts its layer control in the status bar). A new
  docked region must justify itself against these budgets in its spec.
- Dialogs are the only overlays. Their body is capped at 426 pt (ADR 0009).

## 3. Colour tokens

Contrast is WCAG, measured on the surface named. Home: where the value lives today. The goal is
one named constant per token in `ui/theme.rs` (chrome, done by LCV-184) or a new
`render/palette.rs` (canvas) (gap → F4).

**Chrome** (surface `bg.panel` #252525)

| Token | Value | Role | Contrast | Home |
|---|---|---|---|---|
| `bg.canvas` | #1a1a1a | canvas outside the bed | — | `ui/theme.rs::BG_CANVAS` (alias `CANVAS_BG`) |
| `bg.panel` | #252525 | panels, windows | — | `ui/theme.rs::BG_PANEL` |
| `text.primary` | #d0d0d0 | all chrome text | 9.9:1 | `ui/theme.rs::TEXT_PRIMARY` |
| `text.muted` | #8c8c8c | secondary text, only on `bg.panel` | 4.6:1 (3.3:1 on buttons, forbidden) | `ui/theme.rs::TEXT_MUTED` |
| `fill.selected` | #005c80 | selected widget fill | — | `ui/theme.rs::FILL_SELECTED` |
| `fill.widget` | #3c3c3c | idle button or field fill | — | `ui/theme.rs::FILL_WIDGET` |
| `fill.hover` | #464646 | hovered widget fill | — | `ui/theme.rs::FILL_HOVER` |
| `fill.active` | #373737 | pressed or open widget fill | — | `ui/theme.rs::FILL_ACTIVE` |
| `border` | #404040 | the one 1 pt border: windows, menus, separators, off pills | — | `ui/theme.rs::BORDER` |
| `status.warning` | #ff8f00 | warnings, command feedback | 6.7:1 | `ui/theme.rs::STATUS_WARNING` |
| `agent.tool` | #78beff | agent tool rows | 7.7:1 | `agent/panel.rs::TOOL_COLOR` |
| `status.error` | #ff6b6b | errors: AI panel error rows, `! ` dock lines, failed autosave badge (egui `error_fg_color`) | 5.5:1 | `ui/theme.rs::STATUS_ERROR` |
| `accent` | #4fa3e0 | foreground-only highlight, never a fill: focus rings, active pill text, active rail icon, prompt verb | 5.5:1 | `ui/theme.rs::ACCENT` |

**Canvas** (surface: bed, gray 40)

| Token | Value | Role | Contrast | Home |
|---|---|---|---|---|
| `canvas.frame` | gray 64, 1 pt | canvas edge | — | `app/viewport/paint.rs::paint` |
| `bed.fill` | gray 40 | work area | — | `render/bed.rs::draw_bed_fill` |
| `bed.border` | gray 160, 1.5 pt | bed edge | 5.6:1 | `render/bed.rs::draw_bed` |
| `bed.outside` | black α 96 | dims off-bed area | — | `render/bed.rs::draw_bed` |
| `grid.minor` | gray 62, 1 pt on a pixel centre | minor grid | 1.38:1 | `render/palette.rs::GRID_MINOR` |
| `grid.major` | gray 96, 1 pt on a pixel centre | major grid, every tenth line | 2.34:1 | `render/palette.rs::GRID_MAJOR` |
| `origin` | gray 220, 2 pt, two 12 pt arms | machine origin (0,0) | 10.8:1 | `render/palette.rs::ORIGIN` |
| `entity` | its layer's colour, 1 pt | geometry | per layer (§3 rules) | `render/entities.rs::draw_entities` |
| `layer.new` | #0000ff, #00aa00, #ff00ff, #00aaaa, #ff8000, #8000ff | colours offered to new layers, in order | per layer | `app/layers.rs::NEW_LAYER_COLORS` |
| `selection` | rgba(64,160,255,180), 3 pt | selection halo | 3.4:1 blended (floor) | `render/selection.rs` |
| `preview` | rgba(255,220,100,160) | rubber-band geometry, window/crossing box | 5.3:1 blended | `render/palette.rs::preview` |
| `snap` | #ffa000, 8 pt glyph and its kind label | object snap marker | 7.2:1 | `render/snaps.rs::marker_color` |
| `snap.edge` | `bg.canvas`, glyph stroke + 2 pt | dark edge under each snap glyph shape | — | `render/palette.rs::SNAP_EDGE` |
| `cursor` | gray 220, 1 pt | crosshair and pickbox | 10.8:1 | `render/cursor.rs::cursor_color` |
| `hover` | its layer's colour, 2.5 pt | entity a click would pick | per layer (§3 rules) | `render/palette.rs::HOVER_WIDTH_PT` |
| `danger` | #ff4d6a, 1 pt dashed | what TRIM or ERASE will remove | 4.6:1 | `render/palette.rs::DANGER` |

Rules:

- Text ≥4.5:1 on its surface. State graphics (halo, glyph, box, preview) ≥3:1 on the bed.
- Every layer colour shown on the canvas should reach ≥3:1 on the bed. LCV-156 paints the
  layer colour as it is (ADR 0012 §9): the default Cut `#ff0000` gives 3.7:1, but `#0000ff`
  gives 1.7:1. LCV-164 keeps them raw: no display lightening, no curated palette.
- The grid stays visible but below the geometry.
- Colour is never the only cue (§1.5).
- **No new colour literal outside the token homes.** A new colour is a new token, in this table,
  in the same commit.
- The observation raster (ADR 0011, `render/raster.rs`) is theme-independent. It does not use
  these tokens.

## 4. Typography

- egui 0.29 defaults: proportional Ubuntu-Light, monospace Hack. Sizes: body/button 12.5,
  small 9, heading 18, monospace 12. No custom fonts.
- Monospace is for key bindings (`ui/shortcuts_dialog.rs`), agent tool rows (`agent/panel.rs`),
  and any text aligned in columns.
- Live numbers keep a fixed width: pad with figure spaces (U+2007), not ASCII spaces
  (`ui/statusbar.rs::format_coords` today; gap → LCV-167).
- `heading` is for a surface title (the AI Assistant panel) or a dialog section. Everything else
  is body or small.
- Any change to a font size re-measures the LCV-139/140 budgets and the ADR 0009 cap.

## 5. Spacing and sizing

- Units are egui points everywhere in the chrome. Names ending in `_PX` are points
  (e.g. `render/snaps.rs::MARKER_SIZE_PX`). Geometry is millimetres (AGENTS.md invariants).
- egui's default spacing. Any other size is a named constant next to its use.
- Chrome (LCV-184, `ui/theme.rs`): one border weight, 1 pt `border`; corners 3 pt on widgets
  (`WIDGET_ROUNDING`) and 4 pt on windows and menus; no shadows. Mode pill padding 6×1 pt
  (`ui/statusbar/pill.rs::PILL_PADDING`); editor-row frame margin 4×2 pt
  (`ui/command_line.rs::EDITOR_FRAME_MARGIN`).
- Canvas strokes: hairline 1 pt · grid 1 pt on pixel centres · bed border 1.5 pt · origin 2 pt with
  12 pt arms · selection halo 3 pt · hover 2.5 pt · snap glyph 8 pt, its edge 2 pt wider ·
  snap aperture 12 pt (`app/snap.rs::SNAP_TOLERANCE_PX`) · crosshair and pickbox 1 pt.
- **Pointer tolerances are in screen points**, turned into mm with the live zoom: entity pick
  aperture 5 pt for Select, TRIM and EXTEND (`tools/tool.rs::PICK_APERTURE_PT`; the pickbox
  side is twice it) and box-drag threshold 2 pt (`tools/tool.rs::DRAG_THRESHOLD_PT`). New
  tolerances are in points.
- Curves are tessellated so each chord's sagitta is ≤0.25 pt on screen:
  `n = ⌈2π / (2·acos(1 − 0.25/r_pt))⌉` chords per full turn, clamped to [8, 1024] (8 when
  r ≤ 0.25 pt); an arc gets `⌈n·sweep/2π⌉`, at least 2 (`render/tessellate.rs`, LCV-164). Each
  entity, and each overlay of it (halo, hover, preview), is one shape: a segment or one path.

## 6. Canvas visual language

Paint order (LCV-137 AC 1, `app/viewport/paint.rs::paint`): canvas background → bed fill → grid →
bed border and outside overlay → origin marker (LCV-164) → entities → selection halo → hover →
preview and danger marks in the tool's order (ADR 0013, LCV-163) → snap edge, glyph and label →
pickbox → crosshair (LCV-162).

| Element | Form | Status |
|---|---|---|
| Entity | 1 pt, `entity` (its layer's colour, LCV-156) | shipped |
| Selected | 3 pt `selection` halo over the entity | shipped |
| Hover / pick target | 2.5 pt `hover` stroke in the layer colour (Select, TRIM, EXTEND) | shipped |
| Preview | `preview` stroke | shipped |
| Window box / crossing box | solid / dashed `preview` outline | shipped |
| Trim / Delete preview | dashed, `danger` | shipped |
| Crosshair + pickbox | full canvas, `cursor`; pickbox only while an entity pick is pending; OS cursor hidden | shipped |
| Origin (0,0) | one `origin` path: 12 pt along +X → (0,0) → 12 pt along +Y, on the bed's lower-left border, under geometry (`render/bed.rs::draw_origin`) | shipped |

Snap glyphs are R14 shapes at a fixed 8 pt in `snap` (`render/snaps.rs::marker_shape_for`). Each
glyph is painted twice from one shape list (`render/snaps.rs::glyph_shapes`): first 2 pt wider in
`snap.edge`, then in `snap`. The kind's lower-case name (`endpoint`, `midpoint`, …) follows in the
body font and `snap`, no backing, shown at once, its left-top 8 × 6 pt below-right of the point,
clear of the point and the crosshair (`render/snaps/label.rs`, LCV-164).

| Kind | Glyph | Status |
|---|---|---|
| Endpoint | ■ filled square | shipped |
| Midpoint | ▲ filled triangle | shipped |
| Center | ○ circle (stroke) | shipped |
| Intersection | ✕ crossed diagonals | shipped |
| Quadrant | ◇ diamond | shipped |
| Perpendicular | ⊥ | shipped |
| Tangent | ○ with a tangent bar | shipped |
| Nearest | ⧗ hourglass | shipped |

- Feedback reflects this frame's input: `app/viewport.rs::draw` handles the pointer before it
  paints (LCV-162). While an entity pick is pending there is no running snap and no glyph (R14
  has no osnap at "Select objects"). The snap glyph stays at the pre-ortho point and after the
  pointer leaves (gap → F2).
- Navigation: the wheel zooms about the cursor (gap → F3), middle-drag pans, `F` / `Ctrl+0`
  zoom to extents, `View > Fit to Bed` frames the bed. Zoom All and Zoom Extents in the menu:
  gap → LCV-166. Startup (autosave recovery included), Open and a Bed dialog OK that changes the
  size frame the bed as `View > Fit to Bed` does, on the first frame whose viewport has a size
  (`App::frame_bed_pending`, LCV-164). Undo of a bed change and `File > New` do not reframe.

## 7. Chrome components

- **Menubar**: R14 order — File Edit View Format Tools Help (`Format > Layers…`, LCV-156).
  Title Case labels (§9); `…` only when a dialog
  follows. Shortcuts in an aligned column: gap → LCV-166 (today `"\t"` in `ui/menubar.rs`).
- **Tool rail** (`ui/toolbar.rs::TOOLS`, LCV-183): 32 pt square buttons with no text, each
  holding a flat line icon painted with egui shapes (`ui/icons.rs`: 1.5 pt stroke in the text
  colour, `accent` on the active tool, centred 20 pt square, R14 metaphors). Buttons take the
  theme's 3 pt corners and state fills: flat when idle, `fill.hover` hovered, `fill.selected`
  for the active tool (LCV-184). Two columns: draw (Select … Text) left, modify
  (Move, Copy, Rotate, Mirror, Scale, Trim, Extend, Delete, Dist) right; then a separator and
  the `AI` toggle (32 pt square, text `AI`, tooltip `AI Assistant`). No group captions. Width
  fixed at 76 pt (`app/panels.rs::RAIL_WIDTH`); the rail scrolls only when the window is too
  short. Tooltip `<Label> — <key> · <WORD>` or `<Label> — <WORD>` (`tool_hover_text`), e.g.
  `Line — L · LINE`. One table drives the rail, the Tools menu (text labels) and the shortcuts
  dialog. The v0.3 tools have no bare-letter key (ADR 0003 amendment 5).
- **Command dock** (`ui/command_line.rs`, ADR 0003)
  - Prompt row, then editor row with its destination label (`ui/command_destination.rs`).
  - The prompt is coloured by part (`ui/command_line/prompt.rs::prompt_spans`, LCV-184): the
    leading verb (≥2 capitals ending at a space or `:`) in `accent`, the request in
    `text.primary`, `[Options]` and `<default>` in `text.muted`. A prompt without a verb
    (`Command:`) is all `text.primary`.
  - The editor row sits in a 1 pt frame, `border` idle and `accent` while the editor holds
    keyboard focus; the field itself is frameless.
  - New prompts follow `VERB  Specify <thing> [Opt/Opt]:`. A `<default>` needs a runtime
    value, which ADR 0003 §C rules out (`&'static str`). Defaults and existing prompts:
    gap → LCV-165.
  - Messages state a fact and the next step. Severity is error / warning / info (gap → LCV-165;
    today everything is `status.warning`). Query results such as DIST are info.
- **Status bar** (`ui/statusbar.rs::draw_statusbar`): coords · tool · `Entities: n` · current
  layer dropdown (`ui/layer_combo.rs`, LCV-156) · SNAP GRID ORTHO · autosave. A 1 pt `border`
  rule separates adjacent segments. Coordinates use egui's built-in monospace font. Mode
  toggles are always-visible clickable pills (LCV-116, `ui/statusbar/pill.rs::mode_pill`,
  LCV-184): on = `fill.selected` fill and `accent` text; off = no fill, 1 pt `border` outline
  and `text.muted` text (`fill.hover` while hovered); the label is the second cue.
  New segments are appended.
- **Dialogs** (`ui/dialogs.rs`, `app/bed_dialog.rs`, `app/discard.rs`, `ui/layers_dialog.rs`): Title Case titles,
  buttons ordered primary → Cancel. Non-modal, no keyboard handling (LCV-069, LCV-113, ADR 0002
  §A6). Enter/Esc, destructive styling and one close pattern: gap → LCV-169.
- **AI Assistant panel** (`agent/panel.rs`): transcript roles per LCV-125 — user, assistant,
  tool (monospace, `agent.tool`), refused (`status.warning`), note (small), error (gap →
  LCV-167). No avatars, bubbles or decoration; one heading.

## 8. Interaction and keyboard

Canonical bindings live in ADR 0002 §A6 (gate table), ADR 0003 (command line) and
`ui/shortcuts_dialog.rs::SHORTCUT_GROUPS`. They are linked here, not copied.

- Every action has a command word or key. No action is mouse-only.
- Keys: a tool key is a bare letter. A mode is an F-key (F3 Snap, F7 Grid, F8 Ortho). A file or
  edit action is Ctrl+letter. A new key needs its gate class in ADR 0002 §A6.
- Escape follows ADR 0003: one press clears the command line, releases focus and cancels the
  tool.
- Enter on an empty line goes to the active tool (ADR 0003 §B5). Repeating the last command:
  gap → LCV-165.
- The right button is reserved (LCV-041 AC 4). Right-click = Enter: gap → LCV-165.
- Ortho overrides snap (LCV-053).
- The agent is addressed only by prefix, `:` or `/ai` (LCV-148).

## 9. Copy

- **Title Case**: menus, menu items, buttons, window titles, toolbar labels
  (`Save As…`, `Fit to Bed`, `Select All`).
- **Sentence case**: prompts, messages, hints, tooltips, field labels.
- **UPPER**: command words (`LINE`, `PLINE`, `ERASE`), mode toggles (`SNAP`, `GRID`,
  `ORTHO`).
- `…` only when more input follows (a dialog or a file picker).
- Units are always shown (`mm`, `°`). Keys are written `Ctrl+Shift+S`, `F3`.
- A message states the fact, then the next step: `Could not write 'x.svg': … — choose another
  folder.`

| Concept | Say | Not |
|---|---|---|
| Multi-segment line | Polyline (`PLINE`) | path, polygon |
| Remove entities | Delete (`ERASE`) | remove, erase in labels |
| Machine work area | Bed | table, sheet, canvas |
| Frame the bed / frame the geometry | Fit to Bed / Zoom Extents | zoom fit |
| Cut group | Layer (`Format > Layers…`, `LAYER`, LCV-156) | pen, colour, preset |
| The LLM feature | AI Assistant; short form AI; settings AI Settings | Agent, bot |

Existing labels that break these rules, including LCV-156's `File > Export layers`: gap →
LCV-167. The rules already apply to every new label, including the v0.3 commands.

## 10. Accessibility and scaling

- Contrast per §3; colour never alone (§1.5); every action reachable from the keyboard (§8).
- HiDPI works through points. Budgets and paint tests run at `pixels_per_point` 1.0 and zoom 1.0
  (ADR 0002); dialogs fit the ADR 0009 cap at 800×600.

## 11. Changing the design

- Code owns values. This file owns names, roles and rules. Change both in the same commit.
- A user-visible UI change is a spec that cites the section it follows. Its last task updates
  this file and removes the gap tag. A refactor that keeps values (e.g. moving a literal into a
  token) takes the fast lane.
- Prove UI ACs on painted output: paint-harness text (`tests/harness/paint.rs`), colour/shape
  tests on `FullOutput.shapes`, and source scans only for structure.
- Fast-lane defects (≤3 files, regression test, no spec; candidates for v0.3 step 0):
  - **F2**: hide the snap glyph when Ortho moved the point, and clear `active_snap` when the
    pointer leaves the canvas.
  - **F3**: make the wheel zoom factor proportional to the scroll delta, and clamp zoom in
    `render/camera.rs::Camera`.
  - **F4**: move colour literals into token homes, values unchanged (`refactor:`). `render/`
    never imports `ui/`, except `render/palette.rs::SNAP_EDGE` = `CANVAS_BG` (LCV-164). Keep the `TOOL_COLOR`/`warn_fg_color` names that LCV-125 tests scan.
  - **F5**: fix stale comments: the frame order in `.claude/rules/repaint-ui.md`, the claim in
    `render/preview.rs` that egui 0.29 has no `Shape::dashed_line`, and the icon notes in
    `ui/toolbar.rs` and `app/agent_state.rs`.

## 12. Rejected

Not without new evidence from the CAD → LaserGRBL flow: ribbon, docking or floating
toolbars (LCV-065/066/140), icon files or icon fonts · light theme or theme switcher · grips · on-canvas dynamic input ·
F2 text-history window (LCV-139) · chat decoration in the AI Assistant (LCV-125) · animations
and transitions · custom fonts · multiple documents or tabs.
