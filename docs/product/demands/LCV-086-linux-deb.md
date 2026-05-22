# LCV-086 — Linux .deb package

- **Status**: Ready
- **Phase**: 8
- **Depends on**: LCV-085
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

A laser-cutting operator on a Debian/Ubuntu system often prefers a native `.deb` package
over an AppImage: it integrates with `apt`/`dpkg` for installation, removal, and upgrade
tracking; it places the binary in `/usr/bin` so no `chmod +x` or path-gymnastics is
needed; and it registers the application in the system launcher (GNOME Shell, KDE
Application Menu) via the standard `/usr/share/applications` path without any manual
setup. Without a `.deb`, users who want LaserCAD available system-wide — or who manage
workstations with configuration-management tooling — have no clean installation path.
This demand produces a single bash script that packages the already-built release binary
into a standards-compliant Debian package and deposits it in `dist/`.

## Scope

- New file `scripts/build-deb.sh` — a self-contained bash script, tracked by git,
  runnable as `./scripts/build-deb.sh` from the repo root on any Ubuntu 24.04 host that
  has Rust 1.88 and `dpkg-deb` installed (`dpkg-deb` is part of the default `dpkg`
  package and is present on every Debian/Ubuntu system).
- The script builds `target/release/lasercad` via `cargo build --release`, constructs
  a staging directory tree at `build/deb-staging/` mirroring the target filesystem
  layout, writes a `DEBIAN/control` file, and calls `dpkg-deb --build` to produce
  `dist/lasercad_VERSION_amd64.deb`, where `VERSION` is the exact version string read
  from `Cargo.toml`.
- `VERSION` is extracted from `Cargo.toml` by the script (e.g. via `grep`/`sed`) — it
  is never hardcoded in the script.
- The staging tree contains exactly four filesystem entries (see Acceptance criteria AC#5).
- The `DEBIAN/control` file fields are specified in AC#6.
- The script is idempotent: it removes and recreates `build/deb-staging/` at the start
  of every run so stale staging artifacts cannot corrupt the output.
- `dist/` is already covered by `.gitignore` (LCV-085, AC#11); no additional gitignore
  entries are required. The script verifies `dist/` exists and creates it if absent.
- No new Rust source files or `Cargo.toml` dependency changes are introduced by this
  demand.

## Out of scope

- Linux AppImage build — LCV-085.
- Production icons and final `.desktop` entry content — LCV-088.
  The icon and `.desktop` content used here are the same placeholder artifacts already
  committed by LCV-085 (`assets/icon-256.png`, freedesktop fields matching LCV-085 AC#5).
- Uploading the `.deb` to a PPA, apt repository, or GitHub Release — LCV-089.
- Windows (MSI/NSIS) or macOS (dmg) packaging — LCV-090, LCV-091.
- CI pipeline integration of the `.deb` build step — LCV-092 or a dedicated CI demand.
- `post-install` / `pre-remove` maintainer scripts (`DEBIAN/postinst`, etc.) — the
  package contains no system service, no shared library, and no file needing special
  post-install handling. A plain binary + desktop entry + icon require no maintainer
  scripts.
- ARM (`aarch64`) or 32-bit (`i686`) packages — `amd64` only for v0.1.0.
- GPG signing of the `.deb` — LCV-089 handles release signing.
- Generating a `Makefile` or any build system integration beyond the shell script.

## Acceptance criteria

1. `scripts/build-deb.sh` exists, is tracked by git (`git ls-files scripts/build-deb.sh`
   returns the path), and its file permissions include the execute bit
   (`ls -l scripts/build-deb.sh` shows `-rwxr-xr-x` or equivalent).

2. The first two lines of `scripts/build-deb.sh` are exactly:
   ```
   #!/usr/bin/env bash
   set -euo pipefail
   ```

3. The script extracts `VERSION` from `Cargo.toml` — not from any hardcoded string.
   Confirm by reading the script: a `grep`/`sed` (or equivalent POSIX text tool)
   expression targeting `^version\s*=` in `Cargo.toml` is present. Changing the
   `version` field in `Cargo.toml` and re-running the script must produce a file named
   `dist/lasercad_<new-version>_amd64.deb`.

4. Running `./scripts/build-deb.sh` from the repo root on a clean Ubuntu 24.04 host
   with Rust 1.88 (via `rust-toolchain.toml`) and `dpkg` installed produces
   `dist/lasercad_VERSION_amd64.deb` where `VERSION` matches the value of the `version`
   key in `Cargo.toml`. The script exits 0.

5. The staging directory constructed by the script at `build/deb-staging/` before
   `dpkg-deb --build` runs contains exactly these entries (relative paths inside
   `build/deb-staging/`):

   | Path | Required content |
   |---|---|
   | `DEBIAN/control` | Valid Debian control file — see AC#6 |
   | `usr/bin/lasercad` | The stripped x86_64 ELF release binary from `target/release/lasercad`; installed with mode `0755` |
   | `usr/share/applications/lasercad.desktop` | A copy of a valid freedesktop `.desktop` file — see AC#7 |
   | `usr/share/icons/hicolor/256x256/apps/lasercad.png` | A copy of `assets/icon-256.png` |

   No other files or directories are present inside `build/deb-staging/DEBIAN/` beyond
   `control`. No `postinst`, `prerm`, or other maintainer scripts are included.

6. `build/deb-staging/DEBIAN/control` contains the following fields with these exact
   values (field names are case-insensitive per Policy; values must match):

   | Field | Required value |
   |---|---|
   | `Package:` | `lasercad` |
   | `Version:` | Exactly the `VERSION` string extracted from `Cargo.toml` (e.g. `0.1.0-dev`) |
   | `Architecture:` | `amd64` |
   | `Maintainer:` | `LaserCAD contributors <noreply@lasercad.invalid>` |
   | `Section:` | `graphics` |
   | `Priority:` | `optional` |
   | `Description:` | First line (short): `KISS 2D CAD for laser cutting`. Extended description (indented continuation lines) is permitted and must be valid `control` syntax (each continuation line starts with a single space). |
   | `Depends:` | See AC#8 |

   The `control` file must end with a single newline character (no trailing blank lines).

7. `build/deb-staging/usr/share/applications/lasercad.desktop` is a valid freedesktop
   `.desktop` file containing at minimum these six key-value pairs under the
   `[Desktop Entry]` section:

   | Key | Required value |
   |---|---|
   | `Name` | `LaserCAD` |
   | `Exec` | `lasercad` |
   | `Icon` | `lasercad` |
   | `Type` | `Application` |
   | `Categories` | `Graphics;` |
   | `Terminal` | `false` |

8. The `Depends:` field in `DEBIAN/control` MUST contain `libc6`. Additionally, the
   script MUST check whether `ldd target/release/lasercad` lists `libssl.so`; if it
   does, the script MUST append `libssl3 | libssl1.1` to the `Depends:` field.
   Since LCV-085 AC#7 mandates that the release binary carries no dynamic libssl
   linkage, the produced `Depends:` will typically be `libc6` alone — but the
   conditional check must be present in the script to guard against future regressions
   where native-tls replaces rustls.

9. `dpkg-deb --info dist/lasercad_VERSION_amd64.deb` exits 0 after a successful build
   run and its output contains `Package: lasercad`, `Architecture: amd64`, and
   `Version: VERSION` (where `VERSION` matches `Cargo.toml`).

10. `file build/deb-staging/usr/bin/lasercad` contains `ELF 64-bit` and `x86-64` after a
    build run. (The binary is the same `target/release/lasercad` ELF artifact produced
    by `cargo build --release` with the `strip = true` profile.)

11. The script is idempotent: running `./scripts/build-deb.sh` a second time on an
    already-built tree completes without error and regenerates
    `dist/lasercad_VERSION_amd64.deb`. Confirm by running the script twice back-to-back
    and verifying both runs exit 0 and produce a valid `.deb` (AC#9 passes on both runs).

12. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0 after any changes introduced by this demand. (No Rust
    source changes are expected; this AC guards against accidental breakage.)

## Expected tests

- **Static / AC#1**: `git ls-files scripts/build-deb.sh` returns the file; `ls -l scripts/build-deb.sh`
  shows execute bit.

- **Static / AC#2**: `head -2 scripts/build-deb.sh` shows the shebang and `set` line
  exactly as specified.

- **Static / AC#3**: read `scripts/build-deb.sh` and confirm a `grep`/`sed` expression
  targeting `^version` in `Cargo.toml` is used to populate `VERSION` — no hardcoded
  version string. Mutate `version` in `Cargo.toml` to `9.9.9`, run the script, verify
  the output is named `dist/lasercad_9.9.9_amd64.deb`; then restore `version` to its
  original value.

- **Static / AC#8 — ldd guard**: read `scripts/build-deb.sh` and confirm there is a
  conditional block that runs `ldd target/release/lasercad | grep libssl` (or
  equivalent) and appends `libssl3 | libssl1.1` to `Depends` only when the output is
  non-empty.

- **Post-build / AC#5 — staging tree**: after one script run, verify via `find build/deb-staging -type f | sort` that exactly the four required files are present (plus `DEBIAN/control`).

- **Post-build / AC#6 — control fields**: `grep -E '^(Package|Version|Architecture|Maintainer|Section|Priority|Description|Depends):' build/deb-staging/DEBIAN/control`
  shows all eight required fields.

- **Post-build / AC#7 — desktop file fields**:
  `grep -E '^(Name|Exec|Icon|Type|Categories|Terminal)=' build/deb-staging/usr/share/applications/lasercad.desktop`
  shows all six required key-value pairs.

- **Post-build / AC#9 — dpkg-deb info**: `dpkg-deb --info dist/lasercad_VERSION_amd64.deb`
  exits 0 and prints `Package: lasercad`, `Architecture: amd64`, and the correct version.

- **Post-build / AC#10 — binary arch**: `file build/deb-staging/usr/bin/lasercad`
  contains `ELF 64-bit` and `x86-64`.

- **Manual smoke / AC#4 — full build**: on Ubuntu 24.04 with Rust 1.88, execute
  `./scripts/build-deb.sh`; confirm `dist/lasercad_VERSION_amd64.deb` is produced and
  the script exits 0.

- **Manual smoke / AC#11 — idempotence**: run `./scripts/build-deb.sh` twice back-to-back;
  both runs must exit 0 and `dpkg-deb --info` must pass on the final artifact.

- **Manual smoke — install/uninstall cycle**: on a test Ubuntu 24.04 VM, run
  `sudo dpkg -i dist/lasercad_VERSION_amd64.deb`; confirm `which lasercad` returns
  `/usr/bin/lasercad`; confirm the application appears in the desktop launcher; run
  `sudo dpkg -r lasercad`; confirm `which lasercad` returns nothing and the `.desktop`
  file is removed.

- **CI gate / AC#12**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all`.

## Open questions

*(none)*

## Notes

### Staging directory convention

`build/deb-staging/` follows the same `build/` convention established by LCV-085 for
intermediate build tooling and staging artifacts. It is already covered by the
`build/` entry in `.gitignore` (LCV-085 AC#11). The script removes the entire
`build/deb-staging/` tree at the start of each run (`rm -rf build/deb-staging`) to
guarantee a clean staging state before rebuilding.

### Control file syntax

Debian Policy §5.1 requires the `control` file to be a RFC 822-style stanza. Key rules
the implementer must observe:

- Each field starts at column 0: `Field: value` (single space after the colon).
- Multi-line values (e.g., the extended `Description:`) use continuation lines starting
  with a single space. A lone `.` (space followed by dot) represents a blank line in the
  extended description.
- The file must end with a newline; trailing blank lines are not allowed.
- `dpkg-deb --build` will reject a malformed control file with a clear error message —
  build the package and verify with `dpkg-deb --info` before considering the demand Done.

Minimum valid control file (single-line description, no extended body):
```
Package: lasercad
Version: 0.1.0-dev
Architecture: amd64
Maintainer: LaserCAD contributors <noreply@lasercad.invalid>
Section: graphics
Priority: optional
Description: KISS 2D CAD for laser cutting
Depends: libc6
```

### libssl Depends logic

LCV-085 AC#7 mandates `ldd AppDir/usr/bin/lasercad | grep libssl` returns nothing.
The same binary is packaged by this demand. In practice the `Depends:` field will
therefore be `libc6` alone. The ldd conditional (AC#8) is a correctness guard, not
dead code: if reqwest is ever reconfigured to use native-tls instead of rustls the
binary will acquire a dynamic libssl dependency and the script must advertise it
correctly rather than produce a broken package.

### `.desktop` file re-use

The `.desktop` fields used here are identical to those in LCV-085's AppDir spec
(`Name=LaserCAD`, `Exec=lasercad`, `Icon=lasercad`, `Type=Application`,
`Categories=Graphics;`, `Terminal=false`). The implementer may share a single
`assets/lasercad.desktop` source file copied by both `build-appimage.sh` and
`build-deb.sh`, or generate it inline in each script — either approach is acceptable
as long as both AC sets are satisfied.

### Icon path convention

The `.deb` places the icon at
`usr/share/icons/hicolor/256x256/apps/lasercad.png` inside the staging tree, which
installs to `/usr/share/icons/hicolor/256x256/apps/lasercad.png` on the target system.
This follows the freedesktop Icon Theme Specification and is the path that
`gtk-update-icon-cache` uses when the package is installed. The source file is
`assets/icon-256.png`, committed by LCV-085. LCV-088 replaces the artwork; the
packaging script reads it by path and requires no changes when the file content changes.

### Why `dpkg-deb --build` and not `fakeroot`

`dpkg-deb --build` does not require root privileges when the file ownership inside the
staging tree is already correct for the target system (i.e., all files owned by the
current user, which dpkg will treat as uid 0/gid 0 on installation). The script must
NOT use `chown` (which requires root); instead it relies on `dpkg-deb`'s default
behaviour of accepting files owned by the build user and remapping them to root:root
at installation time. If this causes `dpkg-deb` to emit ownership warnings on the build
host, the implementer may add `fakeroot dpkg-deb --build …` — `fakeroot` is available
on all Ubuntu 24.04 hosts and requires no root.

### Relation to LCV-088 and LCV-089

LCV-088 (production icons + final `.desktop` entry) replaces `assets/icon-256.png` and
may add `MimeType=` to the `.desktop` file. Neither change requires edits to
`scripts/build-deb.sh` — the script reads the icon by path and writes the `.desktop`
by value. LCV-089 (0.1.0 release tag + GitHub Release) depends on this demand being
Done; the `.deb` artifact from `dist/` is uploaded as a release asset by LCV-089.
