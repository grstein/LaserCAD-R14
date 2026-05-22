# LCV-088 — Icons + .desktop entry

- **Status**: Ready
- **Phase**: 8
- **Depends on**: LCV-085
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

LCV-085 ships `assets/icon-256.png` as a solid-colour placeholder so that `scripts/build-appimage.sh`
can complete. On a Linux desktop with many applications in the launcher, a placeholder icon makes
LaserCAD v2 invisible and indistinguishable. A laser-cutter operator who downloads the AppImage and
installs it system-wide finds no recognizable icon in their application menu, taskbar, or file
manager. This demand delivers the production icon family (SVG source + five PNG sizes) and the
canonical XDG `.desktop` entry so that: (a) `scripts/build-appimage.sh` keeps working unchanged
(it already reads `assets/icon-256.png` by path), (b) the future `.deb` package (LCV-086) has a
ready-made desktop entry and icons to install into the hicolor theme tree, and (c) users who
install manually (`cp lasercad ~/.local/bin/`) have a standard `.desktop` file to drop into
`~/.local/share/applications/`.

## Scope

- `assets/icons/lasercad.svg` — SVG source icon, tracked by git, authored to the design
  constraints listed in the Notes section.
- `assets/icons/lasercad-16.png`, `lasercad-32.png`, `lasercad-64.png`, `lasercad-128.png`,
  `lasercad-256.png` — five PNG renders of the SVG at exactly those pixel dimensions, all tracked
  by git as static artifacts.
- `assets/icon-256.png` — replaced with the production 256 × 256 artwork (identical content to
  `assets/icons/lasercad-256.png`), ensuring `scripts/build-appimage.sh` continues to work
  without modification.
- `assets/lasercad.desktop` — canonical XDG desktop entry, tracked by git, with the fields
  specified in the Acceptance criteria.
- `scripts/generate-icons.sh` — a tracked, executable bash script that regenerates all five PNGs
  from `assets/icons/lasercad.svg` using `rsvg-convert`. It is not called by
  `scripts/build-appimage.sh`; it exists to document and reproduce how the PNGs were produced.

## Out of scope

- Changes to `scripts/build-appimage.sh` — LCV-085 owns that file; no modifications needed.
- Linux `.deb` packaging that installs the icons and desktop entry into system paths — LCV-086.
- macOS `.icns` or Windows `.ico` format icons — LCV-090, LCV-091.
- Animated, high-DPI (`@2x`), or SVG-in-icon-theme variants beyond the five PNG sizes.
- CI pipeline changes to run `scripts/generate-icons.sh` automatically — keep icon generation
  a manual step to avoid a mandatory `librsvg` dependency in CI.
- Any changes to `src/` — no Rust code modifications.
- Brand guidelines or logo usage policy documentation.

## Acceptance criteria

1. `assets/icons/lasercad.svg` is tracked by git (`git ls-files assets/icons/lasercad.svg` exits 0).
   Running `xmllint --noout assets/icons/lasercad.svg` exits 0 (well-formed XML).
   The file contains a `viewBox="0 0 64 64"` attribute on the root `<svg>` element.
   The SVG contains no `<filter>`, no `<mask>`, no `<clipPath>`, no `<linearGradient>`,
   no `<radialGradient>`, and no embedded raster data (`data:image/png` or `data:image/jpeg`).
   The design includes a recognizable laser-cutting motif (see Notes for the reference design).

2. `assets/icons/lasercad-16.png`, `lasercad-32.png`, `lasercad-64.png`, `lasercad-128.png`, and
   `lasercad-256.png` all exist and are tracked by git. For each file, `file assets/icons/lasercad-NNN.png`
   contains `PNG image data, NNN x NNN` at the matching resolution. Specifically:
   - `file assets/icons/lasercad-16.png` → `PNG image data, 16 x 16`
   - `file assets/icons/lasercad-32.png` → `PNG image data, 32 x 32`
   - `file assets/icons/lasercad-64.png` → `PNG image data, 64 x 64`
   - `file assets/icons/lasercad-128.png` → `PNG image data, 128 x 128`
   - `file assets/icons/lasercad-256.png` → `PNG image data, 256 x 256`

3. `assets/icon-256.png` is a valid 256 × 256 PNG: `file assets/icon-256.png` contains
   `PNG image data, 256 x 256`. Its content is the production artwork (i.e. visually identical to
   `assets/icons/lasercad-256.png`, not the previous solid-colour placeholder). The file is tracked
   by git.

4. `assets/lasercad.desktop` is tracked by git. It is a valid XDG desktop entry: the file begins
   with the line `[Desktop Entry]` and contains every one of the following key=value lines (order
   within the file is flexible; key names are case-sensitive):

   | Key | Required value |
   |---|---|
   | `Version` | `1.0` |
   | `Type` | `Application` |
   | `Name` | `LaserCAD` |
   | `Comment` | `2D CAD for laser cutting — LaserGRBL compatible` |
   | `Exec` | `lasercad %F` |
   | `Icon` | `lasercad` |
   | `Terminal` | `false` |
   | `Categories` | `Graphics;Engineering;` |
   | `MimeType` | `image/svg+xml;` |
   | `Keywords` | `CAD;laser;cutting;LaserGRBL;` |

   Verify each field is present with its exact value:
   ```
   grep -P '^Version=1\.0$'      assets/lasercad.desktop
   grep -P '^Type=Application$'  assets/lasercad.desktop
   grep -P '^Name=LaserCAD$'     assets/lasercad.desktop
   grep -P '^Exec=lasercad %F$'  assets/lasercad.desktop
   grep -P '^Icon=lasercad$'     assets/lasercad.desktop
   grep -P '^Terminal=false$'    assets/lasercad.desktop
   ```
   All six `grep` commands exit 0.

5. `scripts/generate-icons.sh` is tracked by git and has the execute bit set
   (`ls -l scripts/generate-icons.sh` shows `-rwxr-xr-x` or `-rwxrwxr-x`).
   The first two lines of the script are exactly:
   ```
   #!/usr/bin/env bash
   set -euo pipefail
   ```
   The script calls `rsvg-convert` for each of the five sizes with explicit `-w` and `-h` flags
   and writes output to `assets/icons/lasercad-NNN.png`. It also copies `assets/icons/lasercad-256.png`
   to `assets/icon-256.png` as its final step.
   Confirm by reading the script: it contains five `rsvg-convert` invocations covering 16, 32, 64,
   128, and 256, and a `cp assets/icons/lasercad-256.png assets/icon-256.png` line (or equivalent).

6. No Rust source file is added or modified: `git diff --name-only HEAD` (after the commit) must
   not include any path matching `src/**/*.rs` or `tests/**/*.rs`.

7. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
   `cargo test --all` all exit 0 after the changes introduced by this demand are committed.

## Expected tests

- **Static / AC#1**: `git ls-files assets/icons/lasercad.svg` returns the path; `xmllint --noout assets/icons/lasercad.svg` exits 0; `grep 'viewBox="0 0 64 64"' assets/icons/lasercad.svg` exits 0; `grep -E '<(filter|mask|clipPath|linearGradient|radialGradient)' assets/icons/lasercad.svg` exits 1 (no matches).

- **Static / AC#2**: for each of the five sizes, `git ls-files assets/icons/lasercad-NNN.png` exits 0 and `file assets/icons/lasercad-NNN.png` contains the matching `PNG image data, NNN x NNN` string.

- **Static / AC#3**: `git ls-files assets/icon-256.png` exits 0; `file assets/icon-256.png` contains `PNG image data, 256 x 256`; visually compare `assets/icon-256.png` and `assets/icons/lasercad-256.png` (manual smoke: they must look identical).

- **Static / AC#4**: `grep -P '^Version=1\.0$' assets/lasercad.desktop`, `grep -P '^Type=Application$' assets/lasercad.desktop`, and all four other grep commands from AC#4 exit 0.

- **Static / AC#5**: `ls -l scripts/generate-icons.sh` shows the execute bit; `head -2 scripts/generate-icons.sh` matches the shebang and `set -euo pipefail` lines exactly; `grep -c 'rsvg-convert' scripts/generate-icons.sh` outputs `5`; `grep 'cp assets/icons/lasercad-256.png assets/icon-256.png' scripts/generate-icons.sh` exits 0.

- **Static / AC#6**: `git show --stat HEAD` (or `git diff --name-only`) confirms no `.rs` file is touched.

- **Manual smoke / regeneration**: on a host with `librsvg2-bin` installed (`sudo apt install librsvg2-bin`), run `./scripts/generate-icons.sh` from the repo root; confirm it exits 0 and the five PNGs and `assets/icon-256.png` are regenerated. `file assets/icons/lasercad-NNN.png` still reports the correct dimensions.

- **Manual smoke / AppImage still builds**: run `./scripts/build-appimage.sh` (LCV-085 script) after this demand lands; confirm it exits 0 and produces `dist/lasercad-x86_64.AppImage` without modification. The AppImage's `AppDir/lasercad.png` is now the production artwork.

- **Manual smoke / icon at 16 px**: open `assets/icons/lasercad-16.png` in an image viewer or render in a file manager; the icon is legible (the laser motif and square outline are both distinguishable at 16 × 16 pixels, with no rendering artifact spills outside the canvas).

- **CI gate / AC#7**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` all exit 0.

## Open questions

*(none)*

## Notes

### Reference icon design

The implementer has latitude on the exact visual but must satisfy AC#1's structural constraints.
The following description is the reference design — a minimal, flat, laser-cutting–themed mark:

- **Canvas**: 64 × 64 px, transparent background.
- **Bed outline**: a 44 × 44 px axis-aligned rectangle, centered at (32, 32), `fill="none"`,
  `stroke="#303030"`, `stroke-width="3"`, `rx="2"` (slight rounding). This represents the
  laser-cutter work bed.
- **Laser ray**: a diagonal line from the top-left region (approximately (10, 10)) to the
  center (32, 32), `stroke="#E53935"` (laser red), `stroke-width="2"`,
  `stroke-linecap="round"`. This represents the laser beam path.
- **Focal point**: a filled circle of `r="4"` at (32, 32), `fill="#E53935"`. This represents
  the laser focus spot.
- No `<text>` nodes. No text anywhere.
- Total distinct colors: at most three (background transparent, bed dark gray, laser red).

The key requirement is legibility at 16 × 16. If the reference design above is not legible at
16 px, the implementer may simplify (e.g., omit the bed outline at 16 px by using a distinct
simplified variant — but only if a single SVG cannot render well at both ends of the size range;
the preferred solution is a single SVG that degrades gracefully).

### How `assets/icon-256.png` relates to LCV-085

LCV-085's `scripts/build-appimage.sh` copies `assets/icon-256.png` to `AppDir/lasercad.png`.
LCV-085 committed a solid-colour placeholder at that path. LCV-088 replaces the file content;
the path stays the same. No change to `scripts/build-appimage.sh` is needed.

### `assets/lasercad.desktop` vs. the AppDir desktop entry

The `.desktop` file in `AppDir/` (created inline by `scripts/build-appimage.sh`) is the
AppImage-specific entry. `assets/lasercad.desktop` is the system-wide source of truth, intended
for installation by the `.deb` (LCV-086) into `/usr/share/applications/` and for manual installs
to `~/.local/share/applications/`. The two entries intentionally differ on `Exec=`: the AppDir
entry uses `Exec=lasercad` (bare binary name, resolved by AppRun inside the AppImage), while the
canonical entry uses `Exec=lasercad %F` (allows files to be passed from a file manager). LCV-086
may choose to copy `assets/lasercad.desktop` verbatim into the `.deb` control tree.

### `Icon=lasercad` — XDG icon resolution

`Icon=lasercad` (no path, no extension) is the XDG icon name. The desktop environment resolves it
by searching `/usr/share/icons/hicolor/NxN/apps/lasercad.png` at each installed size, falling back
to `/usr/share/pixmaps/lasercad.png`. LCV-086 is responsible for the `postinst` / `install` steps
that copy the PNG family from `assets/icons/` to the hicolor tree and call `gtk-update-icon-cache`.
This demand only delivers the source assets; installation is out of scope.

### `rsvg-convert` invocation pattern

Each conversion line in `scripts/generate-icons.sh` should follow this pattern:

```bash
rsvg-convert -w 16  -h 16  assets/icons/lasercad.svg -o assets/icons/lasercad-16.png
rsvg-convert -w 32  -h 32  assets/icons/lasercad.svg -o assets/icons/lasercad-32.png
rsvg-convert -w 64  -h 64  assets/icons/lasercad.svg -o assets/icons/lasercad-64.png
rsvg-convert -w 128 -h 128 assets/icons/lasercad.svg -o assets/icons/lasercad-128.png
rsvg-convert -w 256 -h 256 assets/icons/lasercad.svg -o assets/icons/lasercad-256.png
cp assets/icons/lasercad-256.png assets/icon-256.png
```

`librsvg2-bin` is an optional dev dependency, not a CI or runtime dependency. The committed PNGs
are the authoritative artifacts; the script is the reproducibility record.

### Commit strategy

All assets (SVG, five PNGs, `icon-256.png`, `lasercad.desktop`, `generate-icons.sh`) should land
in a single commit with message `feat(LCV-088): icons and .desktop entry`. If Git LFS is configured
for `.png` in the future, PNGs should migrate; for now, standard git object storage is acceptable
given the combined size of the five PNGs is well under 500 KiB.
