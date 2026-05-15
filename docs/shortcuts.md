# Keyboard shortcuts — LaserCAD R14

## Tools

| Key   | Command                       |
| ----- | ----------------------------- |
| `L`   | Line                          |
| `P`   | Polyline                      |
| `R`   | Rectangle                     |
| `C`   | Circle                        |
| `A`   | Arc                           |
| `S`   | Select                        |
| `M`   | Move                          |
| `T`   | Trim                          |
| `E`   | Extend                        |
| `Del` | Delete selected entities      |

## Camera

| Key / gesture                       | Action                          |
| ----------------------------------- | ------------------------------- |
| Mouse wheel                         | Zoom with pivot at cursor       |
| Middle button + drag                | Pan                             |
| `Space` + drag (left button)        | Pan                             |
| Command `zoom extents`              | Frame the entire document       |

## Modifiers while drawing

| Key               | Behavior                                 |
| ----------------- | ---------------------------------------- |
| `Shift` (hold)    | Locks ortho (0° / 90°) on active segment |
| `F8`              | Toggle permanent ortho                   |
| `F3`              | Toggle snaps                             |
| `F7`              | Toggle grid                              |

## Editing

| Key                                   | Action                                          |
| ------------------------------------- | ----------------------------------------------- |
| `Esc`                                 | Cancel active tool, re-arm `select`             |
| `Enter` or `Space` (in command line)  | Confirm input                                   |
| `↑` / `↓` (in command line)           | Command history                                 |
| `Ctrl+Z`                              | Undo                                            |
| `Ctrl+Y`                              | Redo                                            |
| `Ctrl+S`                              | Save SVG (cut preset)                           |
| `Ctrl+N`                              | New document (via File menu)                    |

## Command-line input

| Form              | Example                                          | Meaning                                                 |
| ----------------- | ------------------------------------------------ | ------------------------------------------------------- |
| Command           | `line`, `l`, `circle`, `c`                       | Triggers a tool                                         |
| Toggle            | `snap`, `grid`, `ortho`                          | Toggles state                                           |
| Zoom              | `zoom in`, `zoom out`, `zoom extents` (or `z e`) | Zoom                                                    |
| Absolute coord    | `124.5, 87.3`                                    | Point in mm                                             |
| Relative coord    | `@50, 0`                                         | Offset from the last point                              |
| Distance          | `50`                                             | Distance along the cursor direction (after first point) |
