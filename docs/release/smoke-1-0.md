# LaserCAD 1.0 — release smoke checklist

Run this list by hand before tagging `v1.0.0`, once on each system, with the artifact built from
the 1.0 commit (not a debug build). Note the result of each step per system; any failure blocks
the tag.

| System | Artifact |
|---|---|
| Linux x86-64 | `lasercad-x86_64.AppImage` and `lasercad_1.0.0_amd64.deb` |
| Windows x86-64 | `lasercad-1.0.0-windows-x86_64.zip` |
| macOS, Apple Silicon | `lasercad-1.0.0-macos-aarch64.dmg` |

Install and first start follow [`../install.md`](../install.md). Keep LaserGRBL at hand (on
Linux and macOS, under Wine or on the machine's Windows PC).

1. Start: the app opens to an empty bed with the tool rail, command line and status bar; Help >
   About says 1.0.0.
2. Draw: `L`, type `10,10`, `@100,0`, `@0,60`, `@-100,0`, `10,10`, Enter; then `C` a
   circle of radius `15` at `60,40`; `A` an arc; `D` a text `LCV 1.0` at height `5`.
3. Snap: with F3 on, start a line on a corner (endpoint), a side's midpoint and the circle's
   center; each marker shows and the point lands exactly (check with `dist`).
4. Edit: select the circle, `copy` it `@30,0`; `rotate`, `mirror` and `scale` a copy; `TRIM` a
   line at a crossing; `EXTEND` one back; Delete a copy. Ctrl+Z undoes each step, Ctrl+Y redoes.
5. Layers: Format > Layers… adds `Engrave` (blue, Output on) and `Notes` (Output off); select
   the text and use Move Selection Here on `Engrave`; it turns blue. Pick `Notes` in the
   status-bar layer dropdown and draw a line: it lands on `Notes`.
6. Save: File > Save As… writes `smoke.svg`; the window title loses its `*`.
7. Reopen: File > New, then File > Open Recent > `smoke.svg`; every entity, layer, color, Output
   switch and the current layer come back as saved.
8. Export layers: File > Export Layers writes `smoke-cut.svg` and `smoke-engrave.svg` beside
   `smoke.svg`, and nothing for `Notes`.
9. LaserGRBL: open `smoke-cut.svg` and `smoke-engrave.svg` in LaserGRBL; the size matches the
   drawing in mm, nothing is mirrored, arcs and circles are smooth, and the text is engraved.
10. Autosave: edit, wait two seconds, kill the app (not Exit); the next start reopens the edit.
11. AI panel (optional, needs a key): Help > AI Settings…, then `:draw a 20 mm square` draws on
    the bed and one Ctrl+Z removes it.
