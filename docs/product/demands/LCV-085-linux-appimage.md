# LCV-085 — Linux AppImage build

- **Status**: Ready
- **Phase**: 8
- **Depends on**: LCV-075, LCV-076, LCV-077, LCV-078, LCV-079, LCV-080
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

After LaserCAD v2 achieves feature parity, a laser-cutter operator on Linux needs a
single-file executable they can download, `chmod +x`, and run — no Rust toolchain, no
`apt install`, no dependency resolution. Without a packaging artifact every non-developer
user who wants to import a job into LaserGRBL is blocked at the distribution step:
"clone and compile" is not a user workflow. The AppImage format solves this for x86_64
Linux by embedding the application binary and a minimal ELF runtime into one portable
executable that runs on any reasonably modern distro without root or system package
installation.

## Scope

- New file `scripts/build-appimage.sh` — a self-contained bash script, tracked by git,
  runnable as `./scripts/build-appimage.sh` from the repo root on any Ubuntu 24.04
  host that has Rust 1.88 and `curl` installed.
- The script builds `target/release/lasercad` via `cargo build --release`, constructs
  an `AppDir/` staging tree, then calls `appimagetool` to produce
  `dist/lasercad-x86_64.AppImage`.
- If `appimagetool` (or `appimagetool-x86_64.AppImage`) is not already on `PATH`, the
  script downloads it from the pinned GitHub release URL
  `https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage`
  into `build/appimagetool-x86_64.AppImage` and caches it there for subsequent runs.
  No download occurs on a run where the cached file already exists.
- `appimagetool` is invoked with `--appimage-extract-and-run` so FUSE is not required
  on the build host (CI-safe).
- libssl is not a dynamic dependency of the produced binary (static linkage — see
  Scope item below and Notes for the two acceptable implementation paths).
- libc (glibc) remains dynamic — the AppImage does not bundle glibc, matching standard
  AppImage practice.
- A placeholder icon `assets/icon-256.png` (256 × 256 PNG, any solid-colour content) is
  committed as a tracked git artifact and used by the build script. LCV-088 replaces it
  with production artwork.
- `dist/` and `build/` are added to `.gitignore` so no generated artifact is
  accidentally committed.
- `Cargo.toml` may add `openssl = { version = "0.10", features = ["vendored"] }` to
  `[dependencies]` if — and only if — a `ldd` check after `cargo build --release`
  shows `libssl.so` as a dynamic dependency. If the TLS backend in use (e.g. rustls
  via reqwest) already links openssl statically or avoids it entirely, no Cargo.toml
  change is needed. No new Rust source files are introduced.

## Out of scope

- Linux `.deb` packaging — LCV-086.
- Production icons and final `.desktop` entry content — LCV-088.
- Windows (MSI/NSIS) or macOS (dmg) packaging — LCV-090, LCV-091.
- CI pipeline integration of the AppImage build step — LCV-092 or a dedicated CI demand.
- ARM (`aarch64`) or 32-bit (`i686`) AppImage targets.
- Code-signing or AppImage update metadata (`update information` section).
- Bundling GPU/display-server libraries (`libGL`, `libX11`, `libxkbcommon`, etc.) — these
  are present on every Linux desktop that can drive a display, and bundling them causes
  driver-version conflicts.
- Release tag creation or GitHub Release uploads — LCV-089.

## Acceptance criteria

1. `scripts/build-appimage.sh` exists, is tracked by git
   (`git ls-files scripts/build-appimage.sh` returns the path), and its file
   permissions include the execute bit (`chmod +x` equivalent, i.e. `-rwxr-xr-x` or
   `-rwxrwxr-x`).

2. The first two lines of `scripts/build-appimage.sh` are exactly:
   ```
   #!/usr/bin/env bash
   set -euo pipefail
   ```

3. Running `./scripts/build-appimage.sh` from the repo root on a clean Ubuntu 24.04
   host with Rust 1.88 (via `rust-toolchain.toml`), `curl`, and `libfuse2` installed
   produces `dist/lasercad-x86_64.AppImage`. The script exits 0.

4. `dist/lasercad-x86_64.AppImage` produced by the script is executable and passes the
   sanity check `file dist/lasercad-x86_64.AppImage`, which must contain the string
   `ELF` (the embedded AppImage runtime is an ELF binary prepended to the payload).

5. The AppDir staging tree constructed by the script before `appimagetool` runs contains
   exactly these four entries (relative paths inside `AppDir/`):

   | Path | Required content |
   |---|---|
   | `AppRun` | Executable shell script; body sets `HERE` to its own directory and `exec`s `$HERE/usr/bin/lasercad "$@"` |
   | `lasercad.desktop` | Valid freedesktop `.desktop` file with `Name=LaserCAD`, `Exec=lasercad`, `Icon=lasercad`, `Type=Application`, `Categories=Graphics;`, `Terminal=false` |
   | `lasercad.png` | Copy or symlink of `assets/icon-256.png` |
   | `usr/bin/lasercad` | The stripped x86_64 ELF release binary from `target/release/lasercad` |

6. `file AppDir/usr/bin/lasercad` contains `ELF 64-bit` and `x86-64` after a build run.

7. `ldd AppDir/usr/bin/lasercad | grep libssl` produces no output — libssl is not a
   dynamic dependency of the installed binary.

8. `appimagetool` is invoked with `--appimage-extract-and-run` so that FUSE is not
   required on the build host. Confirm by reading the script: the call to
   `appimagetool` (or the downloaded `build/appimagetool-x86_64.AppImage`) includes
   the `--appimage-extract-and-run` argument.

9. If `appimagetool` is absent from `PATH` and `build/appimagetool-x86_64.AppImage`
   does not yet exist, the script downloads it from
   `https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage`
   and saves it to `build/appimagetool-x86_64.AppImage`. On a second run, the download
   is skipped (the script checks for the cached file with `[ -f ... ]` before calling
   `curl`).

10. `assets/icon-256.png` is tracked by git (`git ls-files assets/icon-256.png` returns
    the path) and is a valid 256 × 256 PNG (`file assets/icon-256.png` contains
    `PNG image data, 256 x 256`).

11. `.gitignore` contains lines that match `dist/` and `build/` so that neither
    directory's contents can be committed. Verify with:
    `grep -x 'dist/' .gitignore` and `grep -x 'build/' .gitignore` both exit 0.

12. Running `./scripts/build-appimage.sh` a second time on an already-built tree
    completes without error and regenerates `dist/lasercad-x86_64.AppImage` (the script
    is idempotent — it removes and recreates `AppDir/` before each build to avoid stale
    staging artifacts).

13. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0 after any `Cargo.toml` changes introduced by this
    demand.

## Expected tests

- **Static / AC#1**: `git ls-files scripts/build-appimage.sh` returns the file; manually
  inspect that the execute bit is set (`ls -l scripts/build-appimage.sh`).

- **Static / AC#2**: `head -2 scripts/build-appimage.sh` shows the shebang and `set`
  line exactly as specified.

- **Static / AC#8**: read `scripts/build-appimage.sh` and confirm `--appimage-extract-and-run`
  appears in the appimagetool invocation line.

- **Static / AC#9**: read `scripts/build-appimage.sh` and confirm the download block uses
  `[ -f build/appimagetool-x86_64.AppImage ]` (or equivalent) as a guard before `curl`.

- **Static / AC#10**: `git ls-files assets/icon-256.png` returns the path;
  `file assets/icon-256.png` contains `PNG image data, 256 x 256`.

- **Static / AC#11**: `grep -x 'dist/' .gitignore && grep -x 'build/' .gitignore` exits 0.

- **Post-build / AC#4**: after one run of the script, `file dist/lasercad-x86_64.AppImage`
  contains `ELF`.

- **Post-build / AC#5 — desktop file fields**: `grep -E '^(Name|Exec|Icon|Type|Categories|Terminal)=' AppDir/lasercad.desktop`
  shows all six required fields with the specified values.

- **Post-build / AC#6**: `file AppDir/usr/bin/lasercad` contains `ELF 64-bit` and `x86-64`.

- **Post-build / AC#7**: `ldd AppDir/usr/bin/lasercad | grep libssl` is empty.

- **Manual smoke / AC#3**: on Ubuntu 24.04 with `libfuse2` installed, execute
  `./scripts/build-appimage.sh`; confirm `dist/lasercad-x86_64.AppImage` is produced
  and exits 0.

- **Manual smoke / running the AppImage**: on a desktop Ubuntu 24.04 with `libfuse2`,
  execute `dist/lasercad-x86_64.AppImage`; the LaserCAD window opens without a
  missing-library error or segfault (no `error while loading shared libraries` in
  stderr).

- **Manual smoke / AC#9 — cached download**: delete `build/appimagetool-x86_64.AppImage`,
  run the script, observe a `curl` download line in output; run again without deleting,
  confirm no download output.

- **Manual smoke / AC#12 — idempotence**: run the script twice back-to-back; both runs
  exit 0 and produce a valid `dist/lasercad-x86_64.AppImage`.

- **CI gate / AC#13**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all`.

## Open questions

*(none)*

## Notes

### Two acceptable paths for static libssl

**Option A — `openssl` vendored feature (preferred for portability):**
Add to `[dependencies]` in `Cargo.toml`:
```toml
openssl = { version = "0.10", features = ["vendored"] }
```
The `vendored` feature compiles a pinned OpenSSL source tree bundled inside `openssl-src`
and links it statically, regardless of what system headers are installed. No environment
variables needed. This is the recommended path when building for distribution.

**Option B — `OPENSSL_STATIC=1` with system headers:**
Keep `Cargo.toml` unchanged and set `OPENSSL_STATIC=1 OPENSSL_LIB_DIR=<path>` before
`cargo build --release` inside the script. Requires `libssl-dev` on the build host.
Less portable but avoids the extra Cargo.toml entry.

In both cases `ldd AppDir/usr/bin/lasercad | grep libssl` must return nothing (AC#7).
If reqwest is built with the `rustls-tls` feature (and native-tls is disabled) the binary
will never link openssl at all — AC#7 is then trivially satisfied and no Cargo.toml change
is needed.

### AppImage runtime and FUSE

End-users running the AppImage need `libfuse2` installed on their system
(`sudo apt install libfuse2` on Ubuntu 22.04 / 24.04). This is a documented runtime
requirement, not something the packaging script can fix. The `--appimage-extract-and-run`
flag is only for the _build host_ when calling `appimagetool` itself; it does not affect
how end-users run the produced AppImage.

### Why appimagetool release 13

Release 13 (2020) is the last stable tagged release of AppImageKit. The `continuous`
channel changes without notice and has broken AppImage magic-byte placement in the past.
Pinning to `13` gives reproducible tooling. If a newer stable release ships before this
demand is implemented, the implementer may update the version and record the change in
the `Implementation:` line.

### AppImage magic bytes

Bytes at offset 8 in the output file are `0x41 0x49` ("AI"), identifying the AppImage
type. AC#4 uses `file` for a human-readable check because `file` recognises the magic
without requiring a hex dump.

### Relation to LCV-088 (icons and desktop entry)

The `assets/icon-256.png` placeholder and the `.desktop` fields committed in this demand
are functional but not production-quality. LCV-088 replaces `assets/icon-256.png` with
the real artwork and may revise `Categories=` or add `MimeType=` to the desktop entry.
No changes to `scripts/build-appimage.sh` should be needed for LCV-088 — the script
reads the icon from `assets/icon-256.png` by path, so replacing the file is sufficient.

### `build/` directory convention

`build/` is used exclusively for downloaded build tooling (appimagetool) that is not part
of the Rust compilation. It is gitignored (AC#11). `dist/` holds packaged output
artifacts. Neither directory appears in `src/`, `tests/`, or `Cargo.toml`.

### Release profile

`Cargo.toml` already carries `strip = true`, `lto = true`, `codegen-units = 1`,
`opt-level = "s"`, and `panic = "abort"` in `[profile.release]` (landed in LCV-001).
No release-profile changes are needed for this demand.
