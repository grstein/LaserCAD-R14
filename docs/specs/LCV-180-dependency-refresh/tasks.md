# LCV-180 — Tasks

Base: `2211ad0`. One commit per crate family: T1 alone, T2–T7 together as the egui/eframe
commit, then T8–T13 one each, then T14. Each commit passes `scripts/gate.sh` and
`cargo deny check`. Use `CARGO_TARGET_DIR` per worktree.

- [x] T1 [AC1] [AC5] [AC6] Test: Cargo.lock lists no `wgpu`/`egui-wgpu`. Cargo.toml's eframe has
      `default-features = false` with `"glow"`, `winit` is declared, Cargo.toml has no `tokio`,
      and reqwest has `"blocking"` and `"native-tls"`. It fails until T2 and T12
      (files: tests/it/repo/dependencies.rs, tests/it/repo/mod.rs)
- [x] T2 [AC1] [AC5] egui/eframe 0.36 plus `winit = "0.30"` with its comment. App shell:
      `App::ui`, `update_ui(&mut Ui)`, `Panel`/`CentralPanel::show(ui, …)`, `content_rect`,
      `global_style`. deny.toml gets `Ubuntu-font-1.0` (files: Cargo.toml, Cargo.lock, deny.toml,
      src/lib.rs, src/app/mod.rs, src/app/panels.rs, src/app/viewport.rs, src/app/input.rs)
- [x] T3 [AC3] Render: `StrokeKind` on `rect_stroke`, `CornerRadius`, `TextStyle` resolution
      through `global_style` (files: src/render/bed.rs, src/render/cursor.rs,
      src/app/viewport/paint.rs, src/render/snaps/label.rs)
- [x] T4 [AC3] UI: theme (`CornerRadius`/`Margin`/`Shadow`), `MenuBar` with `close_menu` removed,
      Object Snap kept open with `CloseOnClickOutside`, `clip_rect_margin`, statusbar pill and
      toolbar (files: src/ui/theme.rs, src/ui/menubar.rs, src/ui/menubar/recent.rs,
      src/ui/menubar/object_snap.rs, src/ui/shortcuts_dialog.rs, src/ui/statusbar/pill.rs,
      src/ui/toolbar.rs, src/ui/statusbar.rs, src/ui/command_line.rs, src/ui/icons.rs)
- [x] T5 [AC3] Agent panel: Panel API, `Frame` and style renames; `panel.rs` stays ≤300 by
      moving a helper into `src/agent/panel/` if needed (files: src/agent/panel.rs,
      src/agent/settings_ui.rs)
- [x] T6 [AC3] Test harness and call sites: `run_ui`, `Event::ModifiersChanged` in
      `raw_input_at`, `harness::canvas_rect` via `viewport::VIEWPORT_ID` and `read_response`.
      Sweep `ctx.run`→`run_ui` in `tests/it/**` and `src/**/tests.rs`, plus `RawInput`
      literals without `modifiers`. Change no expected value (files: tests/harness/mod.rs,
      tests/harness/paint.rs, the call sites as found)
- [x] T7 [AC3] Paint triage: run every `harness::paint` user. Fix each drift in
      `src/ui/theme.rs` first. Change an expectation that cannot be restored only together with
      a CHANGELOG line and a DESIGN.md note. List each such change in the commit body
      (files: src/ui/theme.rs, CHANGELOG.md, DESIGN.md)
- [x] T8 [AC10] Test: on Linux, `settings::platform_path()` and `autosave::platform_path()` equal
      `$XDG_CONFIG_HOME|$HOME/.config` + `lasercad/settings.json` and
      `$XDG_DATA_HOME|$HOME/.local/share` + `lasercad/autosave.json`. The test reads the
      environment and never sets it. Then bump directories to 6 (files: src/app/persist/tests.rs,
      Cargo.toml, Cargo.lock)
- [x] T9 [AC2] Test: a `<path>` that carries both a foreign-namespace `x:d`/`x:stroke` and plain
      `d`/`stroke` imports the plain values; the same holds for `data-layer` on `<g>`. Bump
      roxmltree to 0.21, and filter to no-namespace attributes only if the test fails
      (files: tests/it/io_svg/import.rs, Cargo.toml, src/io/svg/layers.rs)
- [x] T10 [AC1] base64 0.23. The existing `wire.rs` data-URL tests pin the output
      (files: Cargo.toml, Cargo.lock)
- [x] T11 [AC4] rfd 0.17 with default features. ADR 0005 tests (`src/io/dialogs.rs`,
      `src/lib.rs`) stay unchanged; `ldd` shows no new hard dependency (files: Cargo.toml,
      Cargo.lock, src/io/dialogs.rs only if the API forces it)
- [ ] T12 [AC6] reqwest 0.13 with native-tls (plan §TLS). The transport tests stay unchanged and
      T1 passes. If `transport.rs` changes, run `scripts/mutants.sh` on it
      (files: Cargo.toml, Cargo.lock)
- [ ] T13 [AC1] [AC7] [AC8] [AC9] Run `cargo update`, then `cargo update -p flate2 --precise
      1.1.9`. Drop the quick-xml ignores from deny.toml. Record the duplicate count (≤9), the
      stripped binary bytes (≤13,174,059), `cargo deny check` and the gate in the commit body
      (files: Cargo.lock, deny.toml)
- [ ] T14 CHANGELOG: GUI stack moves to egui/eframe 0.36 (glow), rfd 0.17 and reqwest 0.13
      (files: CHANGELOG.md)
