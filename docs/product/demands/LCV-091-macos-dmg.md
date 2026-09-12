# LCV-091 — macOS .dmg / app bundle

- **Status**: Done
- **Phase**: 9
- **Depends on**: LCV-087, LCV-088, LCV-089
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 3ca6879 — feat(LCV-091): macOS .dmg app bundle — build-dmg.sh with sips/iconutil/hdiutil

## Problem

A laser-cutter operator on macOS cannot download and run LaserCAD v2 without a Rust
toolchain installed. The standard macOS distribution format — a drag-to-install `.dmg`
containing a `.app` bundle — removes that barrier: the user opens the disk image, drags
`lasercad.app` to `/Applications`, and launches it from Spotlight or the Dock. Without
this artifact, macOS users who want to drive a laser cutter via LaserGRBL (or the macOS
port) have no practical path to running the tool. This demand produces a
`scripts/build-dmg.sh` script that assembles a correct `.app` bundle and wraps it in a
`.dmg` using only tooling that ships with macOS (Xcode Command Line Tools), targeting
both Intel (`x86_64`) and Apple Silicon (`aarch64`) hosts from the same script.

## Scope

- New file `scripts/build-dmg.sh` — a self-contained bash script, tracked by git,
  runnable as `./scripts/build-dmg.sh` from the repo root on any macOS 12.0+ host that
  has Rust 1.88 (via `rust-toolchain.toml`) and Xcode Command Line Tools installed
  (`xcode-select --install`). No additional tool downloads are required.

- The script builds `target/release/lasercad` via `cargo build --release` for the host
  architecture (detected with `uname -m`).

- The script constructs a `build/lasercad.app` staging tree with the following exact
  structure:

  | Path inside `lasercad.app` | Required content |
  |---|---|
  | `Contents/MacOS/lasercad` | The stripped Mach-O release binary from `target/release/lasercad` |
  | `Contents/Resources/lasercad.icns` | The `.icns` icon generated from `assets/icon-256.png` (see below) |
  | `Contents/Info.plist` | Valid Apple property list; keys as specified in AC#6 |

- The `.icns` file is generated inline by the script using the macOS built-in `sips` and
  `iconutil` tools. Starting from `assets/icon-256.png`, the script produces a
  `build/lasercad.iconset/` directory with at minimum `icon_256x256.png`,
  `icon_128x128.png`, `icon_64x64x.png`, `icon_32x32.png`, and `icon_16x16.png` (all
  resampled by `sips -z H W`), then runs `iconutil -c icns build/lasercad.iconset -o
  build/lasercad.icns` and copies the result into the app bundle.

- The `Contents/Info.plist` is written by the script as a static XML property list with
  the keys specified in AC#6.

- A drag-to-install `.dmg` is assembled with `hdiutil`: the script creates a temporary
  staging directory `build/dmg-staging/` containing the `.app` bundle and a symbolic
  link `Applications → /Applications`, then calls `hdiutil create` with
  `-format UDZO -volname "LaserCAD"` to produce the final disk image.

- Output artifact path:
  - Intel host (`uname -m` → `x86_64`): `dist/lasercad-x86_64.dmg`
  - Apple Silicon host (`uname -m` → `arm64`): `dist/lasercad-aarch64.dmg`
  - The script maps `arm64` to `aarch64` in the filename; all other `uname -m` values
    cause the script to exit 1 with a clear error message.

- `dist/` and `build/` are gitignored (already covered by LCV-085; this demand verifies
  both patterns remain in `.gitignore`).

- The script is idempotent: it removes and recreates `build/lasercad.app`,
  `build/lasercad.iconset`, `build/dmg-staging/`, and the target `.dmg` on every run.

## Out of scope

- Code signing and notarization — a separate demand or part of LCV-092 (CI matrix).
- CI pipeline integration of the `.dmg` build step — LCV-092.
- Universal binary (fat binary / `lipo` merge) combining Intel and Apple Silicon into a
  single `.app` — requires a cross-compilation setup that belongs to LCV-092.
- Windows packaging — LCV-090.
- Any change to Rust source files or `Cargo.toml`.
- `cargo-bundle` or `create-dmg` tooling — both require downloading or `cargo install`;
  the `hdiutil` / `sips` / `iconutil` approach uses only tools present on every macOS
  host with Xcode Command Line Tools and is preferred here.
- Uploading the `.dmg` to a GitHub Release — LCV-089 / LCV-092.
- Retina (`@2x`) icon variants — the single 256 × 256 PNG is sufficient for the first
  distribution artifact.

## Acceptance criteria

1. `scripts/build-dmg.sh` exists, is tracked by git
   (`git ls-files scripts/build-dmg.sh` returns the path), and its file permissions
   include the execute bit (`ls -l scripts/build-dmg.sh` shows `-rwxr-xr-x` or
   `-rwxrwxr-x`).

2. The first two lines of `scripts/build-dmg.sh` are exactly:
   ```
   #!/usr/bin/env bash
   set -euo pipefail
   ```

3. Running `./scripts/build-dmg.sh` from the repo root on a macOS 12.0+ host (Intel or
   Apple Silicon) with Rust 1.88 and Xcode Command Line Tools produces the correctly
   named artifact (`dist/lasercad-x86_64.dmg` on Intel, `dist/lasercad-aarch64.dmg` on
   Apple Silicon) and exits 0.

4. The produced `.dmg` mounts without errors:
   `hdiutil attach dist/lasercad-*.dmg -nobrowse -mountpoint /tmp/lcv091-test` exits 0.
   After mounting, `/tmp/lcv091-test/lasercad.app` exists as a directory and
   `/tmp/lcv091-test/Applications` exists as a symbolic link pointing to `/Applications`.
   Unmount with `hdiutil detach /tmp/lcv091-test`.

5. `build/lasercad.app/Contents/MacOS/lasercad` is a valid Mach-O executable:
   `file build/lasercad.app/Contents/MacOS/lasercad` contains `Mach-O` and either
   `x86_64` (on Intel) or `arm64` (on Apple Silicon). The binary is the host
   architecture; no cross-compilation is performed by this script.

6. `build/lasercad.app/Contents/Info.plist` is a valid XML property list:
   `plutil -lint build/lasercad.app/Contents/Info.plist` exits 0. The file contains
   exactly the following key–value pairs (confirmed via `plutil -extract <key> raw
   build/lasercad.app/Contents/Info.plist`):

   | Key | Required value |
   |---|---|
   | `CFBundleExecutable` | `lasercad` |
   | `CFBundleIdentifier` | `io.lasercad.lasercad` |
   | `CFBundleName` | `LaserCAD` |
   | `CFBundleDisplayName` | `LaserCAD` |
   | `CFBundleVersion` | The exact version string from `Cargo.toml` `[package].version` (e.g. `0.1.0`) |
   | `CFBundleShortVersionString` | Same as `CFBundleVersion` |
   | `CFBundleIconFile` | `lasercad` |
   | `CFBundlePackageType` | `APPL` |
   | `LSMinimumSystemVersion` | `12.0` |
   | `NSHighResolutionCapable` | `true` (Boolean, not a string) |

7. `build/lasercad.app/Contents/Resources/lasercad.icns` exists after the script runs.
   `file build/lasercad.app/Contents/Resources/lasercad.icns` contains `Mac OS X icon`
   (the `file` magic for `.icns`). The file size is greater than 1 KiB (confirming
   it is not an empty placeholder).

8. The `.icns` is generated by the script from `assets/icon-256.png` using `sips` and
   `iconutil`. Confirm by reading the script: it contains a `sips -z` invocation for
   at least five sizes (256, 128, 64, 32, 16), an `iconutil -c icns` invocation, and
   does not call `rsvg-convert`, `convert` (ImageMagick), or any other tool not
   included with Xcode Command Line Tools.

9. The script reads the version string from `Cargo.toml` at runtime (not hardcoded).
   Confirm by reading the script: it uses a command such as
   `grep -m1 '^version' Cargo.toml | cut -d'"' -f2` (or equivalent `sed` / `awk`) and
   assigns the result to a variable that is then interpolated into `Info.plist`.

10. `.gitignore` contains lines matching `dist/` and `build/` so that neither directory
    can be committed. Verify:
    `grep -x 'dist/' .gitignore` and `grep -x 'build/' .gitignore` both exit 0.

11. Running `./scripts/build-dmg.sh` a second time on an already-built tree completes
    without error and regenerates the `.dmg` (idempotent). The script removes and
    recreates `build/lasercad.app`, `build/lasercad.iconset`, and `build/dmg-staging/`
    at the start of each run.

12. No Rust source file or `Cargo.toml` is added or modified:
    `git diff --name-only HEAD` after the commit must not include any path matching
    `src/**/*.rs`, `tests/**/*.rs`, or `Cargo.toml`.

13. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0 after the changes introduced by this demand are
    committed (CI gate on the Linux runner where Rust tests run).

## Expected tests

- **Static / AC#1**: `git ls-files scripts/build-dmg.sh` returns the path;
  `ls -l scripts/build-dmg.sh` shows the execute bit set.

- **Static / AC#2**: `head -2 scripts/build-dmg.sh` shows the shebang and `set -euo pipefail`
  lines exactly as specified.

- **Static / AC#8**: read `scripts/build-dmg.sh` and confirm it calls `sips -z` with at
  least five sizes, calls `iconutil -c icns`, and contains no reference to `convert`,
  `rsvg-convert`, or `brew`.

- **Static / AC#9**: read `scripts/build-dmg.sh` and confirm the version string is
  extracted from `Cargo.toml` at runtime, not hardcoded.

- **Static / AC#10**: `grep -x 'dist/' .gitignore && grep -x 'build/' .gitignore` exits 0.

- **Static / AC#12**: `git show --stat HEAD` confirms no `.rs` or `Cargo.toml` file is
  touched.

- **Post-build / AC#4** (macOS only): mount the `.dmg` with `hdiutil attach -nobrowse
  -mountpoint /tmp/lcv091-test`; verify `lasercad.app` and the `Applications` symlink are
  present; detach cleanly.

- **Post-build / AC#5** (macOS only): `file build/lasercad.app/Contents/MacOS/lasercad`
  contains `Mach-O` and the expected architecture string.

- **Post-build / AC#6** (macOS only): `plutil -lint build/lasercad.app/Contents/Info.plist`
  exits 0; run `plutil -extract LSMinimumSystemVersion raw
  build/lasercad.app/Contents/Info.plist` and confirm output is `12.0`; run
  `plutil -extract CFBundleIdentifier raw ...` and confirm `io.lasercad.lasercad`; run
  `plutil -extract NSHighResolutionCapable raw ...` and confirm `true`.

- **Post-build / AC#7** (macOS only): `file build/lasercad.app/Contents/Resources/lasercad.icns`
  contains `Mac OS X icon`; `wc -c build/lasercad.app/Contents/Resources/lasercad.icns`
  shows more than 1024 bytes.

- **Manual smoke / AC#3 — full build run** (macOS only): on a clean macOS 12.0+ host
  with Rust 1.88 and Xcode Command Line Tools, run `./scripts/build-dmg.sh`; confirm
  it exits 0 and `dist/lasercad-x86_64.dmg` (Intel) or `dist/lasercad-aarch64.dmg`
  (Apple Silicon) is produced.

- **Manual smoke / drag-to-install** (macOS only): mount the `.dmg`, drag `lasercad.app`
  to `/Applications`; launch from Spotlight; the LaserCAD window opens. macOS may show
  a Gatekeeper warning ("unidentified developer") because the binary is unsigned —
  right-click → Open to bypass. No missing-library crash, no `SIGILL`.

- **Manual smoke / AC#11 — idempotence** (macOS only): run the script twice back-to-back;
  both runs exit 0 and produce a valid `.dmg`.

- **CI gate / AC#13**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` all exit 0 (runs on Linux CI, gating the non-macOS portions of the commit).

## Open questions

*(none)*

## Notes

### Why `hdiutil` instead of `create-dmg` or `cargo-bundle`

`hdiutil` ships with macOS and requires no install step. `create-dmg` (a common third-party
tool) produces a prettier DMG with a background image and icon layout but requires either
Homebrew or a manual download — adding a dependency to the CI matrix without meaningful
user-facing gain for a first distribution artifact. `cargo-bundle` reads
`[package.metadata.bundle]` from `Cargo.toml` and handles `Info.plist` generation, but
requires `cargo install cargo-bundle` and adds Cargo.toml churn. Both alternatives are
listed here for reference; a future demand may adopt them if the build toolchain already
includes them.

### `.app` bundle structure requirement

macOS requires the `Info.plist` to live at `<Name>.app/Contents/Info.plist` and the
executable to match `CFBundleExecutable` exactly. A bundle that is missing `Info.plist` or
has a mismatched `CFBundleExecutable` will fail to launch with `LSOpenURLsWithRole() failed`
or show as a generic document in the Dock.

### `NSHighResolutionCapable` and Retina displays

Setting `NSHighResolutionCapable` to `true` (Boolean) tells macOS to render the window at
the native pixel density of Retina displays. Without it, egui's framebuffer is rendered at
half resolution and then upscaled, producing a blurry window. This key must be a
`<true/>` element in the plist, not a string.

### Icon generation: `sips` + `iconutil`

`sips` (Scriptable Image Processing System) and `iconutil` are present on every macOS
system with Xcode Command Line Tools. The iconset convention requires specific filenames:

```
build/lasercad.iconset/
  icon_16x16.png
  icon_32x32.png
  icon_64x64.png        # not strictly required but recommended
  icon_128x128.png
  icon_256x256.png
```

`iconutil -c icns build/lasercad.iconset -o build/lasercad.icns` will silently succeed with
any subset of the above. A Retina (`@2x`) variant (e.g. `icon_128x128@2x.png` at 256 × 256
pixels) can be added as a future improvement; it is not required by this demand.

### Gatekeeper and unsigned binaries

Without a Developer ID code signature and Apple notarization, macOS Gatekeeper will
quarantine the `.app` and show a warning dialog. Users can bypass it with right-click → Open
or `xattr -dr com.apple.quarantine lasercad.app`. Code signing and notarization require
an Apple Developer account and belong to a future demand (LCV-092 or successor). The warning
must be documented in the project's release notes alongside the first macOS artifact.

### Version string extraction

The script must read the version at build time from `Cargo.toml` to avoid drift. A portable
one-liner that works on both GNU grep and BSD grep (macOS default):

```bash
VERSION=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
```

Both `CFBundleVersion` (build number) and `CFBundleShortVersionString` (marketing version)
should be set to this value for the first release.

### `LSMinimumSystemVersion`

`12.0` (macOS Monterey, released October 2021) is the floor. Rust 1.88's minimum for
`aarch64-apple-darwin` is macOS 11.0 and for `x86_64-apple-darwin` is macOS 10.12. Setting
`12.0` gives a comfortable margin above the compiler minimum while excluding only hardware
that cannot run Monterey.

### Relation to LCV-088 (icons)

LCV-088 delivers `assets/icon-256.png` (production artwork, 256 × 256 PNG). LCV-091's
script reads that file as the sole icon source and generates the `.icns` from it. If LCV-088
has not yet landed, the placeholder `assets/icon-256.png` from LCV-085 satisfies AC#7
(non-empty `.icns`) and the demand can still be implemented; the icon will be production
quality once LCV-088 ships.

### Relation to LCV-092 (CI matrix)

LCV-092 will add a `macos-latest` runner to the GitHub Actions matrix that calls
`./scripts/build-dmg.sh` and uploads the produced `.dmg` as a release artifact. LCV-091
only produces the script; CI wiring is deliberately out of scope to keep this demand small
and independently shippable.
