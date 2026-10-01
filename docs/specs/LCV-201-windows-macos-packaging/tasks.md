# LCV-201 — Tasks

- [x] T1 [AC2] Test: `src/main.rs` carries `windows_subsystem = "windows"` gated on
      `all(windows, not(debug_assertions))` (files: tests/it/repo/packaging.rs, tests/it/repo/mod.rs)
- [x] T2 [AC2] Add the `cfg_attr` attribute (files: src/main.rs)
- [x] T3 [AC1] Test: `scripts/build-zip.ps1` exists, reads the version from `Cargo.toml`, stages
      `lasercad.exe`, both `LICENSE-*` and `FIRST-RUN.txt`, and writes
      `dist/lasercad-<version>-windows-x86_64.zip`; `build-msi.ps1` and `wix/` are gone
      (files: tests/it/repo/packaging.rs)
- [x] T4 [AC1] `scripts/build-zip.ps1` (new), `assets/FIRST-RUN.txt` (new, names `docs/install.md`);
      delete `scripts/build-msi.ps1`, `wix/main.wxs` (files: scripts/build-zip.ps1, assets/FIRST-RUN.txt; deletions: scripts/build-msi.ps1, wix/main.wxs)
- [x] T5 [AC3] Test: `build-dmg.sh` refuses non-arm64, bundles `LaserCAD.app`, ad-hoc signs with
      `codesign --force --deep -s -`, writes `dist/lasercad-<version>-macos-aarch64.dmg`; `bash -n`
      passes (files: tests/it/repo/packaging.rs)
- [x] T6 [AC3] Update `scripts/build-dmg.sh` (arm64 only, bundle name, codesign, FIRST-RUN.txt,
      versioned output) (files: scripts/build-dmg.sh)
- [x] T7 [AC4] [AC9] Test: `ci.yml` `package` job runs on tags or `workflow_dispatch`, the Windows
      step runs `build-zip.ps1` with no `cargo-wix`, uploads and the release list use the new
      names; the `test` matrix lists `windows-2022` and `macos-15` on dispatch (files: tests/it/repo/packaging.rs)
- [x] T8 [AC4] Edit `ci.yml` accordingly (files: .github/workflows/ci.yml)
- [ ] T9 [AC7] Test (Unix): `release.sh --list-assets` in a temp copy with a fake `dist/` lists
      AppImage, `.deb`, `.zip`, `.dmg`; with the `.dmg` removed it prints `missing: …dmg`, exits 0
      (files: tests/it/repo/packaging.rs)
- [ ] T10 [AC7] `release.sh`: shared asset list, `--list-assets` mode, optional `.zip`/`.dmg`
      attached when present, missing ones printed (files: scripts/release.sh)
- [ ] T11 [AC5] [AC6] Test: `docs/install.md` exists with SmartScreen and Gatekeeper/`xattr`
      steps and settings/autosave paths for Linux, Windows and macOS; `README.md` links it;
      `FIRST-RUN.txt` names it (files: tests/it/repo/packaging.rs)
- [ ] T12 [AC5] [AC6] Write `docs/install.md`; link from `README.md` (files: docs/install.md, README.md)
- [ ] T13 [AC8] `docs/build-local.md` "Windows cross-check" section; run
      `cargo check --release --target x86_64-pc-windows-gnu` if the target installs and fix any
      `cfg(windows)` warnings (files: docs/build-local.md)
- [ ] T14 [AC9] Make path-string source scans separator-agnostic (`replace('\\', "/")`); any test
      that cannot run headless on Windows/macOS gets `cfg_attr(…, ignore = "<reason>")`
      (files: tests/it/agent/canvas_capture.rs, tests/it/repo/*.rs as found)
- [ ] T15 CHANGELOG line: Windows `.zip` and macOS `.dmg` (unsigned, see install guide) (files: CHANGELOG.md)
- [ ] T16 [AC4] [AC9] After merge and with CI minutes available: one green `workflow_dispatch`
      run on all three OSes with `package-windows`/`package-macos` artifacts; record the run URL
      in `spec.md` (otherwise Status → Blocked) (files: docs/specs/LCV-201-windows-macos-packaging/spec.md)
