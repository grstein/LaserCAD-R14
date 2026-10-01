# LCV-201 — Plan

## Approach

Packaging stays shell scripts plus the existing `ci.yml`. No new crate and no build dependency,
so the plan holds on top of LCV-180's eframe 0.36 (glow) refresh: nothing here names an
egui/eframe API.
- **Windows.** `src/main.rs` gets
  `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` (AC 2).
  A new `scripts/build-zip.ps1` (pwsh on `windows-2022`) reads the version from `Cargo.toml` and
  stages `lasercad.exe`, `LICENSE-APACHE`, `LICENSE-MIT` and `assets/FIRST-RUN.txt`. It then runs
  `Compress-Archive` into `dist/lasercad-<version>-windows-x86_64.zip` (AC 1).
  `scripts/build-msi.ps1`, `wix/main.wxs` and the `cargo-wix` CI step are deleted (spec decision).
- **macOS.** `scripts/build-dmg.sh` refuses any host that is not arm64. It names the bundle
  `LaserCAD.app`, keeps the `Info.plist` (`CFBundleShortVersionString` = `Cargo.toml` version) and
  the `sips`/`iconutil` `.icns`, and adds `codesign --force --deep -s - LaserCAD.app`. It also
  copies `FIRST-RUN.txt` into the image and writes `dist/lasercad-<version>-macos-aarch64.dmg`
  (AC 3).
- **CI.** The `package` job runs on `startsWith(github.ref, 'refs/tags/') ||
  github.event_name == 'workflow_dispatch'` (AC 4). Its Windows step runs `build-zip.ps1`, and
  the upload paths and the tag-only `release` job's file list use the new names. The `test`
  matrix already widens to the three OSes on dispatch (AC 9).
- **Release script.** `scripts/release.sh` gets a `--list-assets` mode. It prints the assets it
  would attach, plus one `missing: <file>` line for an absent `.zip` or `.dmg`, then exits 0
  before any git, gate or `gh` step. The real run uses the same list, so an absent Windows or
  macOS artifact warns but does not fail; the AppImage and `.deb` stay required (AC 7).
- **Docs.** The new `docs/install.md` covers per-OS download, first run (SmartScreen
  "More info → Run anyway"; Gatekeeper right-click → Open or `xattr -dr com.apple.quarantine`)
  and the file locations. It is linked from `README.md` and named in `FIRST-RUN.txt` (AC 5, 6).
  ADR 0006 is unchanged. Paths still come from `ProjectDirs::from("", "", "lasercad")` in the
  two boot resolvers (`settings_store.rs::platform_path`, `autosave.rs::platform_path`); recent
  files live inside `settings.json`. `docs/build-local.md` gains a "Windows cross-check"
  section: `rustup target add x86_64-pc-windows-gnu` and then
  `cargo check --release --target x86_64-pc-windows-gnu` (AC 8).

## Touches

- `src/main.rs`: the `windows_subsystem` attribute (the only Rust source change).
- `scripts/build-zip.ps1` (new), `scripts/build-dmg.sh`, `scripts/release.sh`;
  `scripts/build-msi.ps1` and `wix/main.wxs` (deleted).
- `.github/workflows/ci.yml`: the `package` `if:`, the Windows step, upload paths and the
  release file list.
- `assets/FIRST-RUN.txt` (new), `docs/install.md` (new), `docs/build-local.md`, `README.md`,
  `CHANGELOG.md`.
- Tests: `tests/it/repo/packaging.rs` (new; registered in `tests/it/repo/mod.rs`).
- ADRs: none. The change is packaging only; ADR 0006 holds as written.

## Risks

- LOC cap: none; `src/main.rs` stays under 10 lines.
- Mutation testing: no (no `src/agent/`, `export.rs` or `History` change).
- **No macOS/Windows host locally.** `build-dmg.sh` and `build-zip.ps1` run only in CI. Locally
  they are checked by `bash -n` and by the repo test's assertions on their text (output name,
  `codesign -s -`, the arm64 refusal). Done needs one green `workflow_dispatch` run on all three
  OSes. If CI minutes stay on hold after 2026-10-01, the spec goes to Blocked (spec decision).
- **Windows test failures (AC 9).** `.gitattributes` already forces `eol=lf`, so fixtures
  compare byte for byte. Risky spots are source scans that compare paths as `/`-joined strings,
  e.g. `tests/it/agent/canvas_capture.rs` matching `"src/render/camera.rs"`. First-choice fix:
  normalise with `.replace('\\', "/")` in the scan helper. A test that truly cannot run gets
  `#[cfg_attr(windows, ignore = "<reason>")]`, never a delete. Unix-only tests already carry
  `#[cfg(unix)]`.
- **Windows cross-check (AC 8)** needs only `rustup target add` (no linker for `cargo check`).
  If the pinned 1.98 toolchain cannot fetch the target offline, the check is documented and
  left to CI's `windows-2022` leg.
- **AC tests are text scans by nature** (CI YAML, scripts, docs). The one behavioural test is
  `release.sh --list-assets`, run against a temporary `dist/` (Unix only).
