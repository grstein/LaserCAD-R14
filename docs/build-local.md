# Building LaserCAD locally (Linux)

This is a from-a-clean-checkout guide, written from an actual run of every step
below on 2026-09-14 (Fedora Linux 43, x86_64). If a step here ever stops
matching what the scripts do, the scripts win — this document describes them,
it does not replace them.

## 1. Toolchain

LaserCAD pins its Rust toolchain in [`rust-toolchain.toml`](../rust-toolchain.toml):

```toml
[toolchain]
channel = "1.98"
components = ["rustfmt", "clippy", "rust-analyzer"]
profile = "minimal"
```

Install [`rustup`](https://rustup.rs/) if you don't already have it; `cd` into
the repository and any `cargo`/`rustc` invocation will fetch and use 1.98
automatically (verified: `rustup show` reports `1.98-x86_64-unknown-linux-gnu
(active)` inside the checkout). `Cargo.toml` declares `rust-version = "1.98"`;
older toolchains will fail to compile.

## 2. System packages

`eframe`'s `glow` backend links against the platform's windowing and GL
libraries. On this Fedora 43 host the packages the [README](../README.md)
lists were already present and sufficient to build cleanly:

```bash
# Fedora / RHEL — confirmed present and sufficient on this host
sudo dnf install -y gcc libxkbcommon-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel mesa-libGL-devel
```

```bash
# Debian / Ubuntu — carried over from the README; not independently
# re-verified in this session (this host is Fedora, not Debian/Ubuntu)
sudo apt install -y build-essential libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgl1-mesa-dev
```

Packaging needs two more things beyond the base build:

- **`curl`** — `scripts/build-appimage.sh` uses it to fetch `appimagetool` the
  first time (cached afterwards at `build/appimagetool-x86_64.AppImage`).
- **`dpkg-deb`** — `scripts/build-deb.sh` uses it to assemble the `.deb`. On
  Debian/Ubuntu this ships with the base system. On Fedora it is **not**
  installed by default; it comes from the `dpkg` package in Fedora's own
  `updates` repo (no third-party repo needed): `sudo dnf install -y dpkg`.
  This host already had it installed; a genuinely clean Fedora checkout will
  need that one extra package that neither the README nor the script
  documents today.

Launching the built AppImage (not building it) additionally needs FUSE
(`fuse-libs`/`fuse3-libs` on Fedora, `libfuse2`/`fuse3` on Debian/Ubuntu) so it
can mount itself; the build itself does not need FUSE because
`build-appimage.sh` invokes `appimagetool` with `--appimage-extract-and-run`.

## 3. Plain release build

```bash
cargo build --release
```

Produces `target/release/lasercad`. This is also the first step of both
packaging scripts below, so running it standalone first is optional — but on
a machine this disk-constrained it's worth doing once up front so a failed
packaging run doesn't also eat the compile time. Verified: a cold
`cargo build --release` on this host took about 1m25s; a subsequent no-op
rebuild (nothing in `src/` changed) took 0.14s.

`cargo run` / `cargo run --release` build and launch the app directly, without
packaging.

## 4. AppImage

```bash
./scripts/build-appimage.sh
```

What it does: builds the release binary, downloads (and caches in `build/`)
`appimagetool` if it isn't already on `PATH`, assembles a throwaway `AppDir/`
staging tree from `assets/icon-256.png` and `assets/lasercad.desktop` plus the
binary, and packs it.

**Output:** `dist/lasercad-x86_64.AppImage` (about 5.1 MiB, stripped release
binary + `appimagetool`'s own squashfs overhead).

To run it:

```bash
./dist/lasercad-x86_64.AppImage
```

Verified end to end on this host: the script ran clean, and the resulting
AppImage was launched once. The process started, forked into its mounted
`usr/bin/lasercad`, stayed up with no crash and near-zero idle CPU for the
duration it was observed, and shut down cleanly on `SIGTERM` with no leftover
`/tmp/.mount_lasercad*` FUSE mount. This session's Wayland compositor does not
support the screenshot capture protocol (`grim` reports "compositor doesn't
support the screen capture protocol") and `wmctrl -l` lists no windows for a
native Wayland client, so **the window itself was not visually confirmed in
this session** — only that the process launches and runs without error. A
human with an ordinary desktop session should confirm the window renders
before relying on this artifact for a release.

### Script fix made while validating this guide

`scripts/build-appimage.sh` downloaded `appimagetool` from
`https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage`.
That numbered release ("13") has been retired upstream — the URL now 404s —
and AppImage/AppImageKit publishes only a rolling `continuous` tag. The script
now points at
`https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage`,
confirmed reachable (HTTP 200 after the GitHub release-asset redirect) on
2026-09-14.

## 5. `.deb` package

```bash
./scripts/build-deb.sh
```

What it does: reads `VERSION` out of `Cargo.toml` (never hardcoded), builds
the release binary, assembles a throwaway `build/deb-staging/` tree
(`DEBIAN/control`, the binary, the `.desktop` file, the icon), computes
`Depends:` (`libc6`, plus `libssl3 | libssl1.1` only if the binary is
dynamically linked against OpenSSL), and calls `dpkg-deb --build`.

**Output:** `dist/lasercad_<version>_amd64.deb` (e.g.
`dist/lasercad_0.1.0_amd64.deb`, about 3.7 MB).

To inspect it without installing: `dpkg-deb -c dist/lasercad_0.1.0_amd64.deb`
(file listing) and `dpkg-deb -I dist/lasercad_0.1.0_amd64.deb` (control
metadata). To install: `sudo dpkg -i dist/lasercad_0.1.0_amd64.deb` (or
`pkexec dpkg -i …` if your session's `sudo` isn't interactive).

### Script fix made while validating this guide

The first run printed a `dpkg-deb` warning: the staging directory's files
carried the build user's own uid/gid (`1000:1000`) instead of `root:root`,
which is wrong for files destined for `/usr/bin` and `/usr/share` on the
installing machine — `dpkg-deb` itself suggested the fix. The script now
passes `--root-owner-group` to `dpkg-deb --build`; a rebuild produced no
warning and `dpkg-deb -c` confirms every entry is now owned `root/root`.

## 6. Where things land

| Artifact | Path | Produced by |
|---|---|---|
| Debug binary | `target/debug/lasercad` | `cargo build` / `cargo run` |
| Release binary | `target/release/lasercad` | `cargo build --release` |
| AppImage | `dist/lasercad-x86_64.AppImage` | `scripts/build-appimage.sh` |
| `.deb` | `dist/lasercad_<version>_amd64.deb` | `scripts/build-deb.sh` |
| Cached `appimagetool` | `build/appimagetool-x86_64.AppImage` | `scripts/build-appimage.sh` (first run only) |
| `.deb` staging tree | `build/deb-staging/` | `scripts/build-deb.sh` (recreated every run) |
| AppImage staging tree | `AppDir/` (repo root) | `scripts/build-appimage.sh` (recreated every run) |

`dist/`, `build/` and `AppDir/` are all `.gitignore`d — none of this is meant
to be committed. (`AppDir/` was not covered before this guide was written; it
now is.)

## 7. Cutting an actual release

Building the AppImage and `.deb` is not the same as publishing a release.
That flow — version bump, `CHANGELOG.md`, the annotated git tag, and the
GitHub release with these two files attached — lives in `scripts/release.sh`
(see LCV-089) and is out of scope for this document. The Windows `.zip`
(`scripts/build-zip.ps1`) and macOS `.dmg` (`scripts/build-dmg.sh`) are built
by CI's `package` job on Windows and Apple Silicon hosts; `release.sh`
attaches them when they are in `dist/` and names any that is missing
(`scripts/release.sh --list-assets` shows the list without releasing).

## 8. Faster local linking with `mold` (optional)

The dev profile in `Cargo.toml` and the single integration-test binary
(`tests/it/`) already keep the edit-build-test loop short (LCV-152). Linking
can be shortened further with [`mold`](https://github.com/rui314/mold), but the
repository does **not** enable it: `.cargo/config.toml` has no way to say "use
mold only if it is installed", and naming a missing linker there breaks every
build. Without `mold` the default system linker is used and nothing changes.

To opt in on your own machine, install it and enable it in your **per-user**
`~/.cargo/config.toml`, never in the repository's:

```bash
sudo dnf install -y mold clang    # Fedora
sudo apt install -y mold clang    # Debian / Ubuntu
```

```toml
# ~/.cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

Undo it by deleting those three lines. Not measured on the reference host
(`mold` is not installed there); the timings in LCV-152's `plan.md` are with the
default linker.

## 9. Windows cross-check (optional)

A Linux host can type-check the Windows build, including the
`windows_subsystem` attribute in `src/main.rs` and any `cfg(windows)` code,
without a Windows machine or a MinGW linker (`cargo check` does not link):

```bash
rustup target add x86_64-pc-windows-gnu
cargo check --release --target x86_64-pc-windows-gnu
```

It must finish with no warnings. `scripts/gate.sh` does not run it, so the
Linux gate never needs the target; CI's `windows-2022` leg builds and tests the
real thing on `workflow_dispatch` and tags. Verified on the reference host
(Rust 1.98) with no warnings (LCV-201).
