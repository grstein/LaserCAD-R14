# LCV-201 — Windows and macOS packaging

- **Status**: Planned
- **Depends on**: LCV-180
- **Implementation**: -

## Problem

LaserGRBL runs on Windows, and many operators prepare their files on a Mac, but LaserCAD ships
only a Linux AppImage and `.deb`. `scripts/build-msi.ps1` (WiX) and `scripts/build-dmg.sh` exist,
along with a tag-only CI `package` job, but none of them has ever produced an artifact anyone ran.
The Windows binary would also open a console window next to the GUI. We have no signing
certificates and no Apple notarization, so the artifacts are unsigned, and operators need
first-run steps that say so.

## Stories

- As a Windows operator, I want a download I unzip and run, so that I can draw beside LaserGRBL.
- As a macOS operator, I want an app I drag to Applications and can open despite Gatekeeper.

## Acceptance criteria

1. WHEN the CI `package` job runs on Windows THE SYSTEM SHALL produce
   `dist/lasercad-<version>-windows-x86_64.zip` holding `lasercad.exe`, `LICENSE-*` and
   `FIRST-RUN.txt`.
2. WHEN `lasercad.exe` is started on Windows THE SYSTEM SHALL open the GUI without a console
   window in release builds; debug builds keep the console.
3. WHEN the CI `package` job runs on macOS THE SYSTEM SHALL produce
   `dist/lasercad-<version>-macos-aarch64.dmg`. It holds `LaserCAD.app` with an `Info.plist`
   whose `CFBundleShortVersionString` equals the `Cargo.toml` version, plus an `.icns` icon and
   an ad-hoc signature (`codesign -s -`).
4. WHEN the workflow is started by `workflow_dispatch` THE SYSTEM SHALL run the `package` job as
   well as on a `v*.*.*` tag, so that artifacts can be verified without tagging.
5. WHEN an operator reads `docs/install.md` THE SYSTEM SHALL give the first-run steps for an
   unsigned app: Windows SmartScreen ("More info → Run anyway") and macOS Gatekeeper
   (right-click → Open, or `xattr -dr com.apple.quarantine`). The page is linked from
   `README.md` and named in `FIRST-RUN.txt`.
6. WHEN the app resolves user paths on Windows or macOS THE SYSTEM SHALL use the
   `directories::ProjectDirs` locations, resolved once in `App::new()` (ADR 0006). `docs/install.md`
   lists the settings, autosave and recent-files location per OS.
7. WHEN `scripts/release.sh` runs and the Windows `.zip` and macOS `.dmg` are in `dist/` THE
   SYSTEM SHALL attach them to the release next to the AppImage and `.deb`. IF either is missing
   THEN it SHALL attach what exists and print which artifact is missing.
8. WHEN `cargo check --release --target x86_64-pc-windows-gnu` runs on a Linux host that has the
   target THE SYSTEM SHALL compile with no warnings. `docs/build-local.md` documents this check;
   the Linux gate does not require the target.
9. WHEN the `test` matrix runs on `windows-2022` and `macos-15` THE SYSTEM SHALL pass the full
   test suite. A test that cannot run headless there is skipped with `#[cfg_attr(..., ignore)]`
   and a reason, never deleted.

## Out of scope

- Code signing, notarization, MSI/NSIS installers, auto-update, Intel/universal macOS builds,
  a Windows `.exe` icon resource (it needs a build dependency).

## Open questions

- None. Decided (self-approved per user goal, 2026-09-30):
  - Windows ships a portable `.zip`. `build-msi.ps1`, `wix/` and the `cargo-wix` CI step are
    removed, because they are unverified and need WiX.
  - macOS ships arm64 only (the `macos-15` runner); Intel Macs build from source.
  - CI minutes are on billing hold until 2026-10-01. The workflow is written now, and Done
    needs one green `workflow_dispatch` run on all three OS. If minutes stay unavailable, the
    spec moves to Blocked rather than Done.
