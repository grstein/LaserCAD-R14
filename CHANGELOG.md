# Changelog

All notable changes to LaserCAD R14 are documented in this file. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-05-12

First stable release. Migrated to TypeScript + Vite + Tauri 2.x; ships a native desktop executable in addition to the file:// web build.

### Added

- Drawing tools: Line, Polyline, Rect, Circle, Arc with live preview and ortho lock.
- Modify tools: Select, Move, Trim (line×line, line×circle), Extend (line→line, line→circle), Delete.
- Snaps: endpoint, midpoint, center, intersection.
- Undo/Redo stack (200 commands).
- Command line input (absolute `X,Y`, relative `@X,Y`, distance).
- Export plain SVG with cut/mark/engrave presets compatible with LaserGRBL.
- Autosave with 800 ms debounce; restore on boot.
- Native desktop shell via Tauri 2 (Linux AppImage/deb/rpm, macOS dmg arm64+x64, Windows msi/exe) with native Save/Open dialogs and `tauri-plugin-store`-backed autosave.
- CI multi-platform release pipeline using `tauri-apps/tauri-action`.

### Security

- Strict CSP without `'unsafe-inline'` for scripts.
- Tauri capabilities scoped to `$DOCUMENT`, `$DESKTOP`, `$DOWNLOAD`, `$HOME` with specific `fs:allow-read-text-file` / `fs:allow-write-text-file` and `dialog:allow-open` / `dialog:allow-save` permissions.

### Known limitations

- No fillet/chamfer/offset.
- Trim of circles by line not implemented.
- SVG import is limited to LaserCAD-emitted plain SVG (see [LCD-005](docs/product/demands/LCD-005-normal-file-menu-svg-workflow.md)); third-party SVG is rejected with a user-facing message.
- Mobile not supported.

[1.0.0]: https://github.com/grstein/LaserCAD-R14/releases/tag/v1.0.0
