# LaserCAD R14 design

## Design principles

LaserCAD R14 is a 2D micro-CAD in the browser. The interface follows three non-negotiable principles, inherited from AutoCAD R14 and adapted to the modern laser-cutting context:

1. **Sovereign drawing area.** The viewport occupies almost the entire screen. Toolbar, command line, and status bar together take less than 12% of the usable height. No floating panels by default; no decorative overlays.
2. **High density, low noise.** Monochrome icons, no shadows, no gradients on controls. Color is reserved for operational information (active snap, selected tool, selection, preview).
3. **Keyboard first.** Each tool has a single-letter shortcut. The command line is the canonical path; the mouse accelerates but does not replace it. Edge-to-edge crosshair cursor, R14-style.

There are no light themes: the dark background is part of the identity and of laser-work legibility. There is no onboarding tutorial covering the screen. There are no transition animations between tool states — only functional micro-feedback (snap highlight, preview dash).

## 450nm laser palette

450nm is the wavelength of the blue-violet laser diodes used in laser engravers. The palette derives directly from that spectrum, applied with a strict hierarchy: purple only appears on active elements and on cut geometry.

| Token                 | Hex       | Role                                                                   | Usage                          |
| --------------------- | --------- | ---------------------------------------------------------------------- | ------------------------------ |
| `--bg-canvas`         | `#0A0612` | Viewport background                                                    | Base color of the infinite "paper" |
| `--bg-chrome`         | `#120A1F` | Background of toolbar, command line, status bar                        | Slightly above the canvas      |
| `--bg-elevated`       | `#1A1030` | Menus, popovers, dialogs                                               | Single elevation level         |
| `--border-subtle`     | `#241638` | Dividers between regions                                               | 1px line, no shadow            |
| `--border-strong`     | `#3D2466` | Border of active input, focus                                          | 1px line                       |
| `--grid-minor`        | `#1E1232` | Auxiliary grid (1mm)                                                   | Fine dots                      |
| `--grid-major`        | `#2E1A4D` | Main grid (10mm)                                                       | Solid line                     |
| `--grid-axis`         | `#5B2DD1` | X/Y axes (0,0)                                                         | 1px line                       |
| `--text-primary`      | `#E8DDFF` | Main text                                                              | Off-white with a purple tint   |
| `--text-secondary`    | `#8E7CB8` | Labels, hints, coordinates                                             | Violet-gray                    |
| `--text-disabled`     | `#4A3E66` | Unavailable tools                                                      |                                |
| `--laser-450`         | `#6E00FF` | **Primary color: cut stroke, active tool, committed preview**          | Perceptual equivalent of 450nm |
| `--laser-glow`        | `#9D4DFF` | Hover, selection highlight                                             | Lighter version                |
| `--laser-dim`         | `#3D0099` | Pressed state, stroke behind another                                   |                                |
| `--snap-endpoint`     | `#FFD400` | Endpoint snap marker (□)                                               | Chrome yellow, high contrast   |
| `--snap-midpoint`     | `#FF8A00` | Midpoint snap marker (△)                                               | Orange                         |
| `--snap-center`       | `#00E5FF` | Center snap marker (○)                                                 | Cyan                           |
| `--snap-intersection` | `#FF2D7A` | Intersection snap marker (×)                                           | Magenta                        |
| `--status-ok`         | `#3DDC97` | Confirmations, autosave saved                                          | Discreet green                 |
| `--status-warn`       | `#FFB020` | Non-blocking warnings                                                  | Amber                          |
| `--status-error`      | `#FF4D6D` | Command errors, validation                                             | Pink-red                       |

**Deliberate decision:** the classical R14 red is replaced by `--laser-450` (450nm purple) as the geometry color. The cultural reference of "colored line on a black background" is kept, but aligned to the product domain. The snap colors (yellow/orange/cyan/magenta) keep the AutoCAD vocabulary of distinct snap colors to avoid confusion between snap types.

```text
┌─────────────────────────────────────────────────────────────────┐
│  #0A0612   #120A1F   #1A1030   ── chrome levels                 │
│                                                                 │
│      #6E00FF  ──  laser 450nm (geometry, active)                │
│      #9D4DFF  ──  hover/selection                               │
│      #5B2DD1  ──  X/Y axes                                      │
│                                                                 │
│   ▣ #FFD400   △ #FF8A00   ○ #00E5FF   × #FF2D7A                 │
│   endpoint    midpoint    center      intersection              │
└─────────────────────────────────────────────────────────────────┘
```

## Typography

| Token            | Family                               | Usage                                            |
| ---------------- | ------------------------------------ | ------------------------------------------------ |
| `--font-ui`      | `Inter`, `system-ui`, sans-serif     | Toolbar tooltips, menus, dialogs                 |
| `--font-mono`    | `JetBrains Mono`, `Menlo`, monospace | Command line, coordinates, dimensions, status bar |
| `--font-display` | `Inter`, sans-serif, weight 600      | Dialog titles only                               |

Fixed sizes, no fluid scaling:

| Scale  |  px | Usage                                |
| ------ | --: | ------------------------------------ |
| `xs`   |  11 | Status bar, coordinates at the cursor |
| `sm`   |  12 | Command line, tooltips, labels       |
| `base` |  13 | Inputs, dialog body                  |
| `md`   |  14 | Dropdown menus                       |
| `lg`   |  16 | Dialog title                         |

Line-height fixed at 1.4 for text and 1.0 for command line / coordinates. No italics in the UI. Bold only on active command-line prompts ("`Specify next point:`").

## General layout

The screen is divided into four fixed regions. No drag-reordering, no dockable panels in the MVP.

```text
┌─────────────────────────────────────────────────────────────────┐
│ FILE  EDIT  VIEW  DRAW  MODIFY  HELP             ▢ LaserCAD R14 │ ← 28px menubar
├──┬──────────────────────────────────────────────────────────────┤
│  │                                                              │
│ L│                                                              │
│ I│                                                              │
│ N│                                                              │
│ E│                  ╋  (infinite crosshair)                     │
│  │                                                              │ ← Viewport
│ R│                                                              │
│ E│                                                              │
│ C│                                                              │
│  │                                                              │
│ C│                                                              │
│  │                                                              │
├──┴──────────────────────────────────────────────────────────────┤
│ Command: line                                                   │ ← 22px × 3 lines
│ LINE  Specify first point: _                                    │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ 124.500, 87.300  mm  │ SNAP │ GRID │ ORTHO │       ● autosaved  │ ← 24px statusbar
└─────────────────────────────────────────────────────────────────┘
   40px
```

| Region            | Height/Width    | Content                                                                                                     |
| ----------------- | --------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------- | ---------------------- |
| Menubar           | 28px            | File / Edit / View / Draw / Modify / Help. Text only, no icons.                                             |
| Vertical toolbar  | 40px            | Tool icons (line, polyline, rect, circle, arc, select, trim, extend, move, delete). Single column.         |
| Viewport          | remainder       | Native SVG, fills the whole space between toolbar and command line.                                         |
| Command line      | 66px (3 lines)  | 2-line history + active input line.                                                                         |
| Status bar        | 24px            | mm coordinates                                                                                              | toggles (SNAP/GRID/ORTHO) | autosave indicator.    |

**Non-negotiable layout rules:**

- The viewport always occupies ≥ 88% of the height. The toolbar never exceeds 40px in width.
- Menubar and toolbar carry no decorative app icons — no logo, no branding in the chrome.
- The status bar is fixed at the bottom, always visible. Coordinates update on every cursor move.
- No side rulers in the MVP. Grid + status bar cover that role.

## Vertical toolbar

Inspired directly by R14: a narrow column of monochrome icons with no visible labels. Each icon is 24×24px inside a 40×32px button. A tooltip appears after 400ms of hover with the tool name and shortcut (`Line (L)`).

| State                          | Visual                                                                                                                 |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| Inactive                       | Icon `--text-secondary` (#8E7CB8) on `--bg-chrome`                                                                     |
| Hover                          | Icon `--laser-glow` (#9D4DFF), background `--bg-elevated`                                                              |
| Active (selected tool)         | Icon `--text-primary` (#E8DDFF), background `--laser-dim` (#3D0099), 2px vertical bar `--laser-450` on the left edge   |
| Disabled                       | Icon `--text-disabled`, no hover                                                                                       |

```text
┌──────┐
│ ─    │ Line       (L)
├──────┤
│ ◢    │ Polyline   (P)
├──────┤
│ ▢    │ Rectangle  (R)
├──────┤
│ ○    │ Circle     (C)
├──────┤
│ ⌒    │ Arc        (A)
├──────┤
│ ▭    │ Select     (S)   ← separator before edit
├──────┤
│ ⊣    │ Trim       (T)
├──────┤
│ ⊢    │ Extend     (E)
├──────┤
│ ✥    │ Move       (M)
├──────┤
│ ✕    │ Delete   (Del)
└──────┘
```

Icons are drawn as inline SVG with 1.5px stroke, no fill, square corners. Single style throughout the app — no mixing of outline and filled icons.

## Viewport

The viewport is an `<svg>` that covers all available space, with `viewBox` in mm. The default cursor inside the viewport is an **edge-to-edge crosshair** (horizontal + vertical 1px lines crossing the entire area), in `--text-secondary` at 60% opacity. When a tool is armed, the crosshair switches to `--laser-glow`.

| Viewport element                         | Style                                                                               |
| ---------------------------------------- | ----------------------------------------------------------------------------------- |
| Background                               | `--bg-canvas` (#0A0612), solid                                                      |
| Minor grid (1mm)                         | Dots `--grid-minor` (#1E1232), 1px                                                  |
| Major grid (10mm)                        | Lines `--grid-major` (#2E1A4D), 0.5px                                               |
| X/Y axes                                 | 1px continuous line `--grid-axis` (#5B2DD1)                                         |
| Committed geometry                       | Stroke `--laser-450` (#6E00FF), 0.1mm converted to px per zoom, min 1px             |
| Geometry in preview                      | Stroke `--laser-glow` (#9D4DFF), `stroke-dasharray="4 2"`                           |
| Selected geometry                        | Stroke `--laser-glow` + 6px square control points `--laser-450` at endpoints        |
| Hovered geometry (pre-selection)         | Thick stroke `--laser-glow`, no dash                                                |
| Selection box (drag → right)             | Continuous border `--laser-450`, fill `--laser-450` at 8%                           |
| Selection box (drag → left, crossing)    | Dashed border `--laser-450` `4 2`, fill `--laser-450` at 8%                         |

**Snap markers** appear floating above the cursor when near a candidate. Each type has a distinct shape and color, always 10px:

| Type         | Shape             | Color                           |
| ------------ | ----------------- | ------------------------------- |
| Endpoint     | Hollow square     | `--snap-endpoint` (#FFD400)     |
| Midpoint     | Hollow triangle   | `--snap-midpoint` (#FF8A00)     |
| Center       | Hollow circle     | `--snap-center` (#00E5FF)       |
| Intersection | ×                 | `--snap-intersection` (#FF2D7A) |

Next to the marker, an 11px mono tooltip with the name (`endpoint`, `midpoint`, `center`, `intersection`), in `--text-primary` on a translucent `--bg-elevated` background at 90%.

**Dynamic coordinates:** when a tool is in `preview` state, show a floating label 16px from the cursor displaying the current dimension in 11px mono: `42.500 mm` for line, `35.0 × 22.5 mm` for rectangle, `R 18.000 mm` for circle. Color `--text-primary` on `--bg-elevated` at 85%, no border.

## Command line

The heart of the R14 experience. Always visible, always focusable. Three lines tall, mono typography.

```text
─────────────────────────────────────────────────────────────────
 Command: line
 LINE  Specify first point: 10,20
 LINE  Specify next point or [Undo]: _
─────────────────────────────────────────────────────────────────
```

| Line     | Content                                |
| -------- | -------------------------------------- |
| 1 (top)  | Previous command or last result        |
| 2 (mid)  | Prompt for the active tool             |
| 3 (base) | Active input with blinking cursor `_`  |

**Behavior:**

- Any alphanumeric key outside an input automatically focuses the command line (replicating R14).
- `Enter` confirms the command or repeats the last one.
- `Space` also confirms (R14 compatibility).
- `Esc` cancels the active tool, clears the input, returns to `idle`.
- History is navigable with `↑`/`↓` when the input is empty.
- Active prompts in weight 600, `--text-primary`. History in weight 400, `--text-secondary`.
- Errors in `--status-error` (#FF4D6D) with the `! ` prefix: `! Invalid point: '10x20'`.

The input accepts:

- Absolute coordinates: `124.5,87.3`
- Relative coordinates: `@50,0`
- Direct distance (after first point + Shift for ortho): `50` + Enter
- Command: `line`, `l`, `rect`, `r`, etc.

## Status bar

Slim footer, read-only except for the toggles. 11px mono typography.

```text
 124.500, 87.300  mm  │ ◉ SNAP │ ◉ GRID │ ○ ORTHO │       ● autosaved 2s ago
```

| Slot        | Content                                            | State                                                             |
| ----------- | -------------------------------------------------- | ----------------------------------------------------------------- |
| Coordinates | `X.XXX, Y.YYY  mm`, updates on every move          | `--text-primary`                                                  |
| SNAP        | Toggle (F3)                                        | `◉` on in `--laser-450` / `○` off in `--text-secondary`           |
| GRID        | Toggle (F7)                                        | idem                                                              |
| ORTHO       | Toggle (F8)                                        | idem                                                              |
| Autosave    | `● saved Xs ago` or `● saving…` or `! save failed` | `--status-ok` / `--text-secondary` / `--status-error`             |

Dividers `│` in `--border-subtle`. Hover on toggles switches the color to `--laser-glow`. Click toggles the state and is reflected in the corresponding keyboard shortcut.

## Menubar

Plain text, no icons. 13px Inter items, 14px horizontal padding, 28px height. Dropdown opens on click (not hover), closes on `Esc` or outside click.

| Menu       | Items                                                                                                                                           |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **File**   | New `(Ctrl+N)` · Open… `(Ctrl+O)` · Save SVG… `(Ctrl+S)` · Recent files › · — · Exit                                                            |
| **Edit**   | Undo `(Ctrl+Z)` · Redo `(Ctrl+Y)` · — · Cut `(Ctrl+X)` · Copy `(Ctrl+C)` · Paste `(Ctrl+V)` · — · Delete `(Del)` · Select all `(Ctrl+A)`        |
| **View**   | Zoom in `(+)` · Zoom out `(−)` · Zoom extents `(Z E)` · Zoom window `(Z W)` · — · Toggle grid `(F7)` · Toggle snap `(F3)` · Toggle ortho `(F8)` |
| **Draw**   | Line `(L)` · Polyline `(P)` · Rectangle `(R)` · Circle `(C)` · Arc `(A)`                                                                        |
| **Modify** | Select `(S)` · Move `(M)` · Trim `(T)` · Extend `(E)` · Delete `(Del)`                                                                          |
| **Help**   | Keyboard shortcuts `(F1)` · About LaserCAD R14                                                                                                  |

Dropdowns have a minimum width of 220px, `--bg-elevated` background, 1px `--border-subtle` border, no shadow. The active item is highlighted with `--laser-dim` and `--text-primary` text. Shortcuts are right-aligned in `--text-secondary` 11px mono.

## Dialogs

Dialogs are rare and centered. No modal dimming the background. Window 360–480px wide, `--bg-elevated` background, 1px `--border-strong` border, 36px header with 16px Inter 600 title.

```text
┌─────────────────────────────────────────────────────┐
│ Export SVG                                       ✕  │
├─────────────────────────────────────────────────────┤
│                                                     │
│   Filename       drawing-001.svg                    │
│                                                     │
│   Preset         ◉ Cut    ○ Mark    ○ Engrave       │
│                                                     │
│   Stroke width   0.1 mm                             │
│                                                     │
│   ☑ Flatten transforms                              │
│   ☑ Force fill="none"                               │
│   ☐ Include grid as comment                         │
│                                                     │
├─────────────────────────────────────────────────────┤
│                              [ Cancel ]  [ Export ] │
└─────────────────────────────────────────────────────┘
```

| Control          | Style                                                                                                                |
| ---------------- | -------------------------------------------------------------------------------------------------------------------- |
| Text input       | 28px height, 10px horizontal padding, 1px `--border-subtle` border, `--bg-chrome` background, focus border `--laser-450` |
| Radio / checkbox | 14×14px, no animation, marker `--laser-450`                                                                          |
| Primary button   | `--laser-450` background + `#FFFFFF` text, 8×16px padding, no extreme border-radius (4px)                            |
| Secondary button | Transparent, 1px `--border-strong` border, `--text-primary` text                                                     |
| Button hover     | Primary switches to `--laser-glow`; secondary gains a `--laser-450` border                                           |

## Cursor and feedback

| State                                    | Cursor                                                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Over viewport, no tool                   | Full-bleed crosshair `--text-secondary` 60% + 4px center dot                                                            |
| Over viewport, tool armed                | Full-bleed crosshair `--laser-glow` + 11px floating tooltip with tool name at the lower-right corner of the cursor      |
| Over hovered entity (select mode)        | Crosshair + entity highlight in `--laser-glow`                                                                          |
| Snap candidate detected                  | Crosshair "snaps" visually to the point: the colored marker appears and the crosshair shifts to the exact snap position |
| Active pan (middle button or Space+drag) | System "grab" cursor                                                                                                    |
| Zoom box                                 | System magnifier cursor                                                                                                 |
| Over chrome (toolbar/menu)               | System default cursor                                                                                                   |

No custom cursor outside the viewport — use native OS cursors to reduce uncanny valley.

## Iconography

Single set, drawn as inline SVG. Specifications:

- **Size:** 24×24px viewBox, visible stroke within an effective 18×18.
- **Stroke:** 1.5px, `stroke-linecap="round"`, `stroke-linejoin="round"`.
- **Fill:** `none` always. No filled icons.
- **Style:** geometric, no perspective, no decorative details. Each icon should suggest the result, not the action (e.g. the Line icon is a diagonal line, not a hand drawing).

| Tool       | Conceptual glyph                                                   |
| ---------- | ------------------------------------------------------------------ |
| Line       | Diagonal line `\` corner to corner, 3px square endpoints           |
| Polyline   | Three connected zigzag lines                                       |
| Rectangle  | Hollow square                                                      |
| Circle     | Hollow circle + center dot                                         |
| Arc        | 180° arc with marked endpoints                                     |
| Select     | Diagonal cursor arrow                                              |
| Trim       | Stylized scissors (two crossing lines) with a cut indicator        |
| Extend     | Short line + arrow indicating extension                            |
| Move       | Cross with arrows on the four ends                                 |
| Delete     | Diagonal ×                                                         |

## Tool states and visual feedback

Each tool has 5 states (mirroring the state machine from the plan: `idle`, `armed`, `preview`, `commit`, `cancel`). The visual for each state:

| State     | Toolbar                       | Command line                                                              | Cursor                              | Viewport                                                     |
| --------- | ----------------------------- | ------------------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------------ |
| `idle`    | no button active              | `Command: _`                                                              | gray crosshair                      | normal grid                                                  |
| `armed`   | tool button highlighted       | `LINE  Specify first point:`                                              | purple crosshair + tooltip          | normal grid                                                  |
| `preview` | idem                          | `LINE  Specify next point or [Undo]:`                                     | purple crosshair + dimension label  | dashed line `--laser-glow` from last point to the cursor     |
| `commit`  | idem                          | `LINE  Specify next point or [Undo]:` (returns to `armed` for next point) | idem                                | new entity appears in solid `--laser-450`                    |
| `cancel`  | button deactivates            | `*Cancel*` for 1 frame, returns to `Command: _`                           | gray crosshair                      | preview disappears                                           |

## Accessibility

| Requirement        | Application                                                                                                                                       |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contrast           | Main text over background: `#E8DDFF` on `#0A0612` = 13.8:1 (AAA). Secondary text 5.9:1 (AA).                                                      |
| Visible focus      | 2px `--laser-glow` outline on any focusable element outside the viewport. Inside the viewport, focus is implicit via the crosshair cursor.        |
| Keyboard           | Every function reachable by keyboard. `Tab` walks toolbar → command line → status bar toggles → menubar.                                          |
| Reduced motion     | No decorative animations; the only motion is the command-line cursor blink, which respects `prefers-reduced-motion: reduce` (static caret).      |
| Minimum size       | Toolbar and toggles have a 32×32px minimum hit area.                                                                                              |

Color blindness: the four snap colors (yellow/orange/cyan/magenta) are also distinguishable by shape (□/△/○/×). Never rely on color alone to distinguish snap types.

## Inspirations and cultural references

| Element                          | Origin      | What we keep                       | What we modernize                                              |
| -------------------------------- | ----------- | ---------------------------------- | -------------------------------------------------------------- |
| Full-bleed crosshair             | AutoCAD R14 | Yes, central characteristic        | Native SVG antialiasing, tuned opacity                         |
| Mandatory command line           | AutoCAD R14 | Yes, identical prompts and flow    | JetBrains Mono typography instead of a bitmap font             |
| Narrow vertical toolbar          | AutoCAD R14 | Yes, single column                 | Redrawn SVG icons, larger hit areas                            |
| Status bar with toggles          | AutoCAD R14 | Yes, SNAP/GRID/ORTHO + F3/F7/F8    | Modern autosave indicator                                      |
| Black background + colored lines | AutoCAD R14 | Dark background kept               | 450nm purple instead of red — coherent with the laser domain   |
| Text drop-down menus             | AutoCAD R14 | Yes, no icons in menus             | Clean typography, right-aligned shortcuts                      |
| Colored snap markers             | AutoCAD R14 | Yes, similar palette               | Antialiasing, tooltip with the name                            |

What we do **not** bring from R14: the ribbon, an always-open properties palette, model/paper space, 3D gradients on buttons, system sounds, splash screen, model viewport tabs.

## Visual summary in one sentence

`#0A0612` background, geometry in `#6E00FF`, crosshair sweeping the screen, single-column toolbar, command line alive at the bottom, and nothing more — the lightness of R14 with the light of a 450nm laser diode.
