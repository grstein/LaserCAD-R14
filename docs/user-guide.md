# LaserCAD user guide

LaserCAD is a small 2D CAD for laser cutting. You draw in millimetres on a bed the size of your
machine, put each kind of cut on its own layer, and save an SVG that LaserGRBL opens as is.
To install it, see [`install.md`](install.md).

## The screen

- **Menu bar**: File, Edit, View, Format, Tools, Help.
- **Tool rail** (left): one button per tool, in the order of the table below, and the **AI**
  button that opens the AI panel.
- **Canvas**: the bed outline, the grid and your drawing. Scroll to zoom at the cursor, drag
  with the middle button to pan. The crosshair picks what is under its box.
- **Command line** (bottom): type commands, coordinates and distances; the prompt above it says
  what the active tool wants next. Up and Down recall earlier lines.
- **Status bar**: cursor position in mm, the Snap, Grid and Ortho switches, and the current
  layer.

## Tools

Start a tool with its rail button, its Tools menu row, its key (when the canvas has the
keyboard), or any of its command words typed on the command line. Words are not case
sensitive. Esc cancels the tool; Enter on an empty command line finishes it, and at rest repeats
the last command.

| Tool | Key | Command words | What it does |
|---|---|---|---|
| Select | — | `s`, `select` | Click an entity, or drag a box: left to right selects what is inside (window), right to left what it touches (crossing). Shift-click toggles. |
| Line | `L` | `l`, `line` | Lines from point to point; Enter ends the chain. |
| Polyline | `P` | `p`, `pline`, `polyline` | A chain of joined lines; Enter finishes it. |
| Rect | `R` | `r`, `rect`, `rectangle` | A rectangle from two opposite corners. |
| Circle | `C` | `c`, `circle` | Center, then radius (a point or a typed distance). |
| Arc | `A` | `a`, `arc` | Start point, end point, then a point on the arc. |
| Text | `D` | `text` | Type a line of text; it becomes engravable single-stroke lines. |
| Move | `M` | `m`, `move` | Select, then a base point and a destination. |
| Copy | — | `copy`, `co`, `cp` | Like Move, but leaves the originals. |
| Rotate | — | `rotate`, `ro` | Select, then a base point and an angle in degrees. |
| Mirror | — | `mirror`, `mi` | Select, then two points of the mirror line. |
| Scale | — | `scale`, `sc` | Select, then a base point and a factor. |
| Trim | `T` | `t`, `trim` | Click the part of a line or arc to cut away at its crossings. |
| Extend | `X` | `extend` | Click a line or arc end to stretch it to the next boundary. |
| Delete | `E` | `e`, `delete`, `del`, `erase` | Removes the selection, or what you pick. |
| Dist | — | `dist`, `di` | Measures the distance and angle between two points. |

Ellipses and Bézier curves come from opened SVG files. They draw, snap, move, copy, rotate,
mirror, scale and delete like the rest; Trim and Extend refuse them.

## Other commands

| Command words | What it does |
|---|---|
| `layer`, `la` | Opens Format > Layers…. |
| `check` | Checks the drawing for open ends, duplicates and degenerate geometry before export (also Tools > Check). |
| `snap`, `grid`, `ortho` | Toggle object snap (F3), the grid (F7) and ortho lock (F8). |
| `ze`, `zoom extents` | Zoom to the whole drawing (also `F`). |
| `zoom in`, `zoom out` | Zoom one step. |

## Typing points and distances

When a tool asks for a point you can click, or type:

- `X,Y` — an absolute point, in mm from the bed origin (bottom left), for example `50,25`.
- `@dX,dY` — relative to the last point, for example `@10,-5`.
- `@d<a` — relative, `d` mm at `a` degrees counter-clockwise from +X, for example `@50<30`;
  `d<a` measures from the origin.
- A plain number — a distance along the cursor direction (with Ortho, along the axis).

## Snaps

With Snap on (F3), the cursor locks to the nearest object snap: endpoint, midpoint, center,
intersection, quadrant, perpendicular, tangent and nearest. View > Object Snap chooses which
kinds are active; the choice is remembered.

## Undo

Every change, including what the AI assistant draws, undoes with Ctrl+Z and redoes with Ctrl+Y,
up to 200 steps.

## Layers

Every entity sits on a layer. A layer has a name, a color (the stroke color in the SVG) and an
**Output** switch. Use one layer per kind of laser work, for example `Cut`, `Mark`, `Engrave`.

- **Format > Layers…** (or `layer`) adds, renames, recolors and deletes layers and switches
  Output. Enter applies, Esc closes.
- The **layer dropdown** in the status bar sets the current layer; new entities go there.
- A new drawing starts with one layer, `Cut` (red, Output on).

## Files

- **File > New / Open… / Open Recent / Save / Save As…** (Ctrl+N, Ctrl+O, Ctrl+S,
  Ctrl+Shift+S). LaserCAD saves the drawing as one SVG, the **mother** file, with every layer.
  Open reads LaserCAD files and SVG from other programs (Inkscape, Illustrator, LightBurn);
  what it could not import is listed on the command line.
- **File > Export Layers** writes one SVG per layer that has Output on and holds entities,
  beside the mother: `plate.svg` gives `plate-cut.svg`, `plate-engrave.svg`, … Load each into
  LaserGRBL with its own speed and power.
- **File > Bed Size…** sets the bed of this drawing (and of new ones).
- An **autosave** keeps your work; after a crash the next start reopens it.

The SVG LaserCAD writes is fixed for 1.x: the same drawing exports the same bytes in every 1.x
release (ADR 0018).

## AI panel

The optional AI assistant draws and edits for you from plain-language requests. It is off until
you set it up.

1. **Help > AI Settings…**: the endpoint (any OpenAI-compatible API; OpenRouter by default),
   your API key and the model. Optional switches let it see the canvas, read attached images,
   and get feedback after each change.
2. Open the panel with the **AI** button on the rail, type a request and send it. On the
   command line, start a line with `:` or `/ai` to send it to the assistant.
3. **Attach image…** sends a sketch or photo with the next request (needs `Model supports
   images`).

The assistant edits the same drawing you do: each reply is one undo step, and you can stop it
at any time. It cannot open or save files, change settings, or create or delete layers.

## Keyboard

| Key | Action |
|---|---|
| F1 | Keyboard shortcuts (Help > Keyboard Shortcuts…) |
| F3 / F7 / F8 | Snap / Grid / Ortho |
| F | Zoom extents |
| Ctrl+Z / Ctrl+Y | Undo / Redo |
| Ctrl+A | Select all |
| Del | Delete the selection |
| Esc | Cancel the tool or the selection |
| Enter | Finish the tool; on an empty line, repeat the last command |
