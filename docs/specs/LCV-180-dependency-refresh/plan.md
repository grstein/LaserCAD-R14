# LCV-180 — Plan

## Approach

Bump one crate family per commit, egui/eframe first. Behaviour is unchanged; only API call
sites move. Targets (crates.io, 2026-10-01): egui/eframe **0.36.2**, rfd **0.17.2**, reqwest
**0.13.5**, directories **6.0.0**, roxmltree **0.21.1**, base64 **0.23.1**. png 0.18.1,
mockito 1.7.2 and proptest 1.11.0 are already the latest. serde 1.0.229, serde_json 1.0.151
and thiserror 2.0.21 need only `cargo update`: caret requirements stay.
- **eframe**: `default-features = false` with
  `["accesskit", "default_fonts", "glow", "wayland", "web_screen_reader", "x11"]`, the 0.29
  default set without `wgpu` and `links`. The app opens no URLs, and `links` costs 0.28 MB.
- **winit `"0.30"`** (0.30.13, the latest stable) becomes a direct dependency for feature
  unification only. Without eframe's defaults, `winit/default` goes, and with it the Wayland
  CSD (`wayland-csd-adwaita`) and `wayland-dlopen` that 0.29 shipped (measured with
  `cargo tree -e features`). No `src/` file imports it; a Cargo.toml comment says so.
- **Frame entry.** 0.35 removed `App::update`, `Context::run`, `SidePanel`/`TopBottomPanel` and
  context-level panels. `fn ui(&mut self, ui, _frame)` delegates to
  `App::update_ui(&mut self, ui: &mut egui::Ui)`, which keeps its name because scans split on
  it. Its first line, `let ctx = &ui.ctx().clone();`, keeps the body and the repaint scans
  unchanged. Panels: `Panel::{top,left,right,bottom}(id).show(ui, …)`, `CentralPanel::default()
  .show(ui, …)`; `Window` still takes `&Context`. Tests: `ctx.run_ui(raw, |ui| app.update_ui(ui))`.
- **TLS (measured; deviates from the spec's reqwest bullet).** Size cost: rustls with aws-lc
  (0.13 default) +3.35 MB; rustls with ring +1.24 MB; egui 0.36 +1.42 MB. Either rustls option
  breaks AC 8, so reqwest uses `default-features = false` with `["json", "blocking",
  "native-tls", "charset", "http2", "system-proxy"]` (+2 KB). That is today's TLS stack.
  rustls needs AC 8 relaxed first.
- **Lock pin**: `cargo update -p flate2 --precise 1.1.9` keeps one `miniz_oxide` (png uses 0.8).

## Baselines (HEAD 2211ad0 = v0.7.0, standing in for v0.5.0 in AC 8)

- AC 7: names with more than one *distinct* version in
  `cargo tree --duplicates -e normal --depth 0` (same-version repeats such as zvariant 4.2.0 ×2
  do not count). **9**: calloop, calloop-wayland-source, getrandom, linux-raw-sys, quick-xml,
  rustix, smithay-client-toolkit, thiserror, thiserror-impl. Target set probed with the pin:
  **9** (getrandom and quick-xml leave; hashbrown 0.16/0.17 and syn 2/3 arrive); 10 without it.
- AC 8: stripped release binary **11,455,704 B**; cap **13,174,059 B**; probe estimate ≈12.9 MB.
- AC 10: `directories` 5.0.1 resolves `$XDG_CONFIG_HOME|~/.config/lasercad` (settings) and
  `$XDG_DATA_HOME|~/.local/share/lasercad` (autosave). 6.0.0 resolves the same in a
  side-by-side probe, with and without the XDG variables.
- AC 5/6: no `wgpu` in `cargo tree -e normal` and no `tokio` in Cargo.toml (transitive is allowed).

## Touches

- `Cargo.toml`, `Cargo.lock`. `deny.toml`: `Ubuntu-font-1.0` replaces `LicenseRef-UFL-1.0`
  (epaint_default_fonts 0.36), and the two quick-xml ignores go (the crate is gone).
- Shell: `src/lib.rs::run`, `src/app/mod.rs::{update_ui, App::ui}`, `src/app/viewport.rs::draw`,
  `src/app/input.rs`, `src/app/panels.rs` (`screen_rect`→`content_rect`, `style`→`global_style`).
- Paint/UI: `rect_stroke` + `StrokeKind` (`render/{bed,cursor}.rs`, `app/viewport/paint.rs`);
  `Rounding`→`CornerRadius`, `Margin`/`Shadow` i8/u8 (`ui/theme.rs`, `ui/statusbar/pill.rs`);
  `menu::bar`→`MenuBar`, no `close_menu` (`ui/menubar{,/recent}.rs`); `clip_rect_margin`
  (`ui/shortcuts_dialog.rs`); `agent/{panel,settings_ui}.rs`.
- Tests: `tests/harness/{mod,paint}.rs`. 0.36 removed `RawInput::modifiers`, so the harness
  prepends `Event::ModifiersChanged`. A new `harness::canvas_rect` replaces 19
  `c.available_rect()` sites by reading the viewport response through `Context::read_response`
  (stable `Id`: `src/app/viewport.rs::VIEWPORT_ID`). About 60 files move to `ctx.run_ui`.
- `src/io/svg/layers.rs` only if the roxmltree test fails (0.21 `attribute` matches local names).
- New tests: `tests/it/repo/dependencies.rs` (AC 1/5/6), `src/app/persist/tests.rs` (AC 10).
- ADRs: none (version bump plus a feature-unification dependency; no boundary moves).

## Risks

- **Paint harness, high (AC 3).** Expect galley widths and positions to drift: skrifa and
  hinting (0.34), harfrust kerning (0.35), `Frame` padding that now includes the stroke (0.31),
  the window margin and `clip_rect_margin` rework (0.35/0.36), and panel separator width
  (0.36). Restore each drift in `ui/theme.rs` first. Change an expected value only with a
  CHANGELOG line and a DESIGN.md note.
- **Interaction.** Menus now close on click (0.32), so Object Snap keeps `CloseOnClickOutside`
  (`tests/it/ui/object_snap_menu.rs`). A press that leaves a widget now counts as a drag (0.36),
  which affects harness rule 5. Panels can be dragged closed and open: the agent panel turns
  that off if the width tests fail.
- **Export bytes (AC 2), low**: no updated crate in `export.rs`; proof is green io_svg tests and
  an empty `git diff 2211ad0 -- tests/it/io_svg tests/fixtures src/io/svg/export.rs`.
- **Duplicates: 9 = 9, no margin.** If the set has drifted, pin the newcomer `--precise`.
- **rfd 0.17** swaps ashpd/zbus for libdbus: `ldd` shows no new hard dep; the smoke opens Open/Save.
- LOC cap: `src/app/mod.rs` is at 295; seam: `impl eframe::App` → `src/app/frame.rs`.
  `src/agent/panel.rs` is at 288; seam: a row helper → `src/agent/panel/`. `src/io/svg/import.rs`
  is at 276, so any attribute helper goes in `layers.rs` (141).
- Mutation testing: no (`export.rs`, `History` untouched; agent edits are egui renames). If T12
  edits `transport.rs`, run `scripts/mutants.sh` on it.
