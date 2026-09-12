# LCV-092 — CI matrix: build + test + release on all platforms

- **Status**: Done
- **Phase**: 9
- **Depends on**: LCV-085, LCV-086, LCV-089, LCV-090, LCV-091
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 263e7cc — feat(LCV-092): CI matrix — test/build/package/release on Linux/Windows/macOS
- **Note**: the workflow file has never executed because the repository has no git remote, so the matrix has never run. Windows package job is broken (tracked as LCV-107).

## Problem

After LCV-089 ships a manual release script for Linux, a user attempting to build or
test LaserCAD on Windows or macOS has no CI gate: formatting violations, clippy
regressions, and broken compilations can merge silently on those platforms. At the
same time, assembling the four platform-specific release artifacts (AppImage, `.deb`,
MSI, `.dmg`) and uploading them to a GitHub release is a manual, multi-machine
operation that is easy to forget or to do out of order. This demand replaces both
pain points with a single GitHub Actions workflow that enforces the three quality
gates on all target platforms on every PR and push to `main`, then — automatically
on every `v*.*.*` tag — builds, packages, and publishes all four artifacts to a
GitHub release.

## Scope

- Replace `.github/workflows/ci.yml` (introduced by LCV-004) with a multi-platform
  workflow that defines four jobs: `test`, `build`, `package`, `release`.
- **`test` job** — matrix: `ubuntu-24.04`, `windows-2022`, `macos-13`
  - Triggers: `pull_request`, `push: branches: [main]`, `push: tags: ['v*.*.*']`
  - On Linux only: install apt system packages required by eframe's glow backend,
    start `Xvfb :99` and export `DISPLAY=:99` before the test step
  - Steps (in order): toolchain (from `rust-toolchain.toml`), cargo cache,
    `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    `cargo test --all`
- **`build` job** — same matrix, same triggers
  - `needs: [test]`
  - Steps: toolchain, cache, `cargo build --release`, upload release binary as
    named GitHub Actions artifact (`binary-linux`, `binary-windows`, `binary-macos`)
- **`package` job** — same matrix, triggers: `push: tags: ['v*.*.*']` only
  - `needs: [build]`
  - Linux runner: calls `./scripts/build-appimage.sh` then `./scripts/build-deb.sh`;
    uploads artifact group `package-linux`
  - Windows runner: calls `./scripts/build-msi.sh` (delivered by LCV-090);
    uploads artifact `package-windows`
  - macOS runner: calls `./scripts/build-dmg.sh` (delivered by LCV-091);
    uploads artifact `package-macos`
- **`release` job** — single job (no matrix), triggers: `push: tags: ['v*.*.*']` only
  - `needs: [package]`
  - Downloads all three artifact groups, extracts the CHANGELOG section for the
    pushed tag, creates a GitHub release via `gh release create` attaching all
    four packaged artifacts
- Rust toolchain always sourced from `rust-toolchain.toml`; no toolchain channel
  hardcoded in the workflow
- All `uses:` references pinned to a tagged release (`@vN`) or full SHA
- Cargo cache via `Swatinem/rust-cache@v2` with a key that includes the runner OS
  and the hash of `Cargo.lock`

## Out of scope

- LCV-004's single-Linux job content — LCV-092 replaces `ci.yml` entirely
- Code coverage (tarpaulin, llvm-cov), benchmarks, `cargo audit`, `cargo deny`
- Dependabot / Renovate configuration
- Code signing of binaries, MSI, or `.deb`
- macOS notarization — that responsibility belongs to `scripts/build-dmg.sh` (LCV-091)
- ARM builds (aarch64 Linux, Apple Silicon macOS) — x86_64 only for v0.1.0
- Cross-compilation: each platform builds natively on its own runner
- `workflow_dispatch` or scheduled (cron) triggers
- Branch protection rules — configured in the GitHub web UI
- Status badges in `README.md`
- The packaging scripts themselves — those are LCV-085, LCV-086, LCV-090, LCV-091

## Acceptance criteria

1. `.github/workflows/ci.yml` exists and is tracked by git:
   ```
   git ls-files .github/workflows/ci.yml
   ```
   returns the path.

2. The file parses as valid YAML without error:
   ```
   python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"
   ```
   exits 0.

3. The `on:` block declares exactly three trigger clauses:
   - `pull_request` (no branch filter)
   - `push: branches: [main]`
   - `push: tags: ['v*.*.*']`
   Confirm by reading the YAML.

4. The workflow defines exactly four top-level jobs named `test`, `build`, `package`,
   and `release`. No other job names appear. Confirm by reading the YAML.

5. Jobs `test` and `build` each declare a `matrix` (or `include:` list) containing
   exactly these three runner values: `ubuntu-24.04`, `windows-2022`, `macos-13`.
   No other runner images appear in their matrix entries.

6. In the `test` job:

   a. A step guarded by `if: runner.os == 'Linux'` (or `startsWith(matrix.os, 'ubuntu')`)
      installs at minimum these packages via `apt-get`:
      `libxkbcommon-dev`, `libxcb-render0-dev`, `libxcb-shape0-dev`,
      `libxcb-xfixes0-dev`, `libgl1-mesa-dev`, `xvfb`

   b. A step guarded for Linux only starts Xvfb on display `:99` and writes
      `DISPLAY=:99` to `$GITHUB_ENV` so the value is available to all subsequent
      steps in the job, e.g.:
      ```yaml
      run: |
        Xvfb :99 -screen 0 1024x768x24 &
        echo "DISPLAY=:99" >> $GITHUB_ENV
      ```

   c. The three quality-gate steps appear in this exact order and each carries a
      human-readable `name:`:
      1. `cargo fmt --all -- --check`
      2. `cargo clippy --all-targets -- -D warnings`
      3. `cargo test --all`
      No other `cargo` command appears between them.

   d. No step in the workflow hardcodes a Rust toolchain channel (`stable`,
      `1.xx`, `nightly`). Toolchain setup delegates entirely to `rust-toolchain.toml`
      (e.g. via a `rustup show` step or `dtolnay/rust-toolchain@master` with no
      `toolchain:` key). Confirm by reading every `uses:` and `run:` block that
      mentions `rustup` or `toolchain`.

7. In the `build` job:

   a. `needs: [test]` is declared, so the job starts only after all three `test`
      matrix legs pass.

   b. A step runs `cargo build --release` unconditionally (no `|| true`).

   c. A step uploads the release binary via `actions/upload-artifact@v4` with the
      following artifact names and paths:

      | Runner | Artifact name | Path |
      |---|---|---|
      | `ubuntu-24.04` | `binary-linux` | `target/release/lasercad` |
      | `windows-2022` | `binary-windows` | `target/release/lasercad.exe` |
      | `macos-13` | `binary-macos` | `target/release/lasercad` |

      The name/path distinction between Linux and macOS (same binary name, different
      artifact name) must be handled with a matrix variable or `if:` guards.

8. In the `package` job:

   a. `needs: [build]` is declared.

   b. The job (or each matrix leg) carries:
      ```yaml
      if: startsWith(github.ref, 'refs/tags/')
      ```
      so packaging never runs on branch pushes or pull requests.

   c. The Linux runner calls both packaging scripts unconditionally and in order:
      ```
      ./scripts/build-appimage.sh
      ./scripts/build-deb.sh
      ```
      Both calls must appear as separate `run:` steps or as lines in one `run:` block
      under `set -euo pipefail`. Neither call may be silenced with `|| true`.

   d. The Windows runner calls `bash ./scripts/build-msi.sh` (bash is available on
      `windows-2022` via Git Bash). The call must appear unconditionally.

   e. The macOS runner calls `./scripts/build-dmg.sh` unconditionally.

   f. After the packaging step(s), artifacts are uploaded via `actions/upload-artifact@v4`:

      | Runner | Artifact name | Paths included |
      |---|---|---|
      | `ubuntu-24.04` | `package-linux` | `dist/lasercad-x86_64.AppImage`, `dist/lasercad_*_amd64.deb` |
      | `windows-2022` | `package-windows` | `dist/lasercad-x86_64.msi` |
      | `macos-13` | `package-macos` | `dist/lasercad-x86_64.dmg` |

9. In the `release` job:

   a. `needs: [package]` is declared.

   b. The job carries `if: startsWith(github.ref, 'refs/tags/')`.

   c. Three download steps retrieve each artifact group via `actions/download-artifact@v4`:
      `package-linux`, `package-windows`, `package-macos`.

   d. A step extracts the CHANGELOG section for the pushed tag
      (e.g., tag `v0.1.0` → section `## [0.1.0]`) and writes the body to a
      temporary file (e.g. `/tmp/release-notes.md`). The extraction must handle
      the case where the section is not found by aborting with a non-zero exit
      rather than creating an empty notes file silently.

   e. A step calls `gh release create "${{ github.ref_name }}"` attaching at
      minimum:
      - `dist/lasercad-x86_64.AppImage`
      - `dist/lasercad_*_amd64.deb` (glob expanded before passing to `gh`)
      - `dist/lasercad-x86_64.msi`
      - `dist/lasercad-x86_64.dmg`
      The `--notes-file /tmp/release-notes.md` flag is passed. The
      `--title "LaserCAD v2 ${{ github.ref_name }}"` flag is passed.

   f. The release step has `GH_TOKEN` available, supplied via:
      ```yaml
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      ```
      The job or workflow declares `permissions: contents: write` so that the
      default `GITHUB_TOKEN` has sufficient scope.

10. Every `uses:` reference in the workflow is pinned to a semantic-version tag
    (`@v4`, `@v2`, etc.) or a full 40-character SHA. No `@main`, `@master`,
    `@latest`, or unversioned reference appears. Verify with:
    ```
    grep 'uses:' .github/workflows/ci.yml | grep -vE '@v[0-9]|@[0-9a-f]{40}'
    ```
    This command must return no output.

11. A `Swatinem/rust-cache@v2` step (or `actions/cache@v4` with explicit cargo paths)
    appears in both the `test` and `build` jobs on every matrix leg. The cache key
    incorporates both the runner OS and the hash of `Cargo.lock`. Confirm by reading
    the YAML.

12. After this workflow is merged to `main`, all six `test` legs and all six `build`
    legs on the merge-commit push complete green on GitHub Actions. The implementer
    records the Actions run URL in the `Implementation:` line.

13. After a `v*.*.*` tag is pushed to the repository, all three `package` legs and
    the single `release` leg complete green. The resulting GitHub release page shows
    all four packaged artifacts attached. The implementer records the Actions run URL
    in the `Implementation:` line.

## Expected tests

- **Static / AC#1**: `git ls-files .github/workflows/ci.yml` returns the path.

- **Static / AC#2**: run the YAML parser command; confirm exit 0.

- **Static / AC#3**: read the `on:` block and confirm the three trigger clauses
  (`pull_request`, `push: branches: [main]`, `push: tags: ['v*.*.*']`); no other
  trigger clause is present.

- **Static / AC#4**: list top-level job keys from the YAML and confirm exactly
  `test`, `build`, `package`, `release`.

- **Static / AC#5**: read the matrix definition for `test` and `build`; confirm
  exactly three OS values: `ubuntu-24.04`, `windows-2022`, `macos-13`.

- **Static / AC#6a**: read the `test` job; locate the apt-install step; confirm it
  carries the Linux `if:` guard and lists all six packages including `xvfb`.

- **Static / AC#6b**: read the `test` job; locate the Xvfb step; confirm the Linux
  `if:` guard and `DISPLAY=:99` written to `$GITHUB_ENV`.

- **Static / AC#6c**: read the `test` job; confirm the three quality-gate steps
  appear in order (fmt → clippy → test) with no other `cargo` call interleaved.

- **Static / AC#6d**: `grep -n 'toolchain' .github/workflows/ci.yml` must not reveal
  any hardcoded channel string (`stable`, `nightly`, `1.[0-9]`).

- **Static / AC#7**: read the `build` job; confirm `needs: [test]`,
  `cargo build --release`, and the three `upload-artifact` steps with the exact
  artifact names and paths from AC#7c.

- **Static / AC#8**: read the `package` job; confirm `needs: [build]`, the
  `startsWith(github.ref, 'refs/tags/')` guard, the script invocations for all
  three platforms, and the three `upload-artifact` steps with correct artifact names.

- **Static / AC#9**: read the `release` job; confirm `needs: [package]`, the tag
  guard, three `download-artifact` steps, CHANGELOG extraction with abort-on-empty
  guard, `gh release create` with `--notes-file`, `--title`, and all four artifact
  paths, and `GH_TOKEN` env var.

- **Static / AC#10**:
  ```
  grep 'uses:' .github/workflows/ci.yml | grep -vE '@v[0-9]|@[0-9a-f]{40}'
  ```
  returns no output.

- **Static / AC#11**: read `test` and `build` jobs; confirm `Swatinem/rust-cache@v2`
  (or `actions/cache@v4`) is present on every leg with `Cargo.lock` in the cache key.

- **Manual smoke / AC#12**: open a PR, observe six green `test` legs and six green
  `build` legs in the GitHub Actions checks tab; record the run URL.

- **Manual smoke / AC#13**: push a `v*.*.*` tag; observe three green `package` legs
  and one green `release` leg; verify the GitHub release page lists
  `lasercad-x86_64.AppImage`, `lasercad_*_amd64.deb`, `lasercad-x86_64.msi`, and
  `lasercad-x86_64.dmg` as downloadable assets; record the run URL.

## Open questions

*(none)*

## Notes

### Relation to LCV-004

LCV-004 established `.github/workflows/ci.yml` with a single Linux job
(`ubuntu-24.04`) running fmt + clippy + test. LCV-092 replaces that file entirely.
The three quality gates are preserved unchanged; the file is rewritten, not extended.
The runner pin `ubuntu-24.04` from LCV-004 is carried forward in the matrix.

### Runner pinning rationale

Pinned runner images (`ubuntu-24.04`, `windows-2022`, `macos-13`) are used instead of
`ubuntu-latest` / `windows-latest` / `macos-latest` to prevent silent runner-image
upgrades from altering build behaviour, consistent with LCV-004's precedent. If a
pinned image is removed by GitHub before this demand ships, the implementer selects
the next available stable pinned image and records the substitution in
`Implementation:`.

`macos-13` is the Intel (x86_64) runner. `macos-14` and above are Apple Silicon
(arm64). The x86_64 target for v0.1.0 requires `macos-13`. If GitHub deprecates
`macos-13` before this demand ships, the implementer may fall back to `macos-12` or
add a cross-compilation step on `macos-14` (targeting `x86_64-apple-darwin`); either
choice must be documented in `Implementation:`.

### Xvfb on Linux

`cargo test --all` compiles the full binary (including the eframe entry point) and
runs all test binaries. Unit tests in `geometry/` and `document/` are display-free.
However, any future integration test that exercises `eframe::run_native` would hang
on a headless runner without a display. Starting Xvfb proactively (AC#6b) costs
negligible time and prevents a class of silent CI hangs as the test suite grows.
The `DISPLAY=:99` value exported to `$GITHUB_ENV` is available to all subsequent
steps in the job.

### Package script interface contract (LCV-090 and LCV-091)

LCV-092 calls `./scripts/build-msi.sh` and `./scripts/build-dmg.sh`. This imposes
a hard interface on LCV-090 and LCV-091:

- `scripts/build-msi.sh` must be a bash-compatible script runnable in Git Bash on
  `windows-2022`, must exit 0 on success, and must produce exactly
  `dist/lasercad-x86_64.msi`.
- `scripts/build-dmg.sh` must be a bash script, must exit 0 on success, and must
  produce exactly `dist/lasercad-x86_64.dmg`.

If LCV-090 or LCV-091 ship a different output filename or script name, the `package`
job YAML must be updated accordingly. The implementer must confirm the actual
interface before finalising the workflow step.

### GitHub release permissions

`gh release create` requires the `GITHUB_TOKEN` to have `contents: write` permission.
GitHub Actions grants this by default for `push: tags` events when the repository's
workflow permissions are set to "Read and write". If the repository uses the more
restrictive default ("Read repository contents and packages"), the workflow must
declare `permissions: contents: write` at the job level (AC#9f).

### Artifact retention

Package artifacts uploaded in the `package` job are ephemeral build intermediates.
Once attached to the GitHub release they do not need to persist in Actions storage.
Setting `retention-days: 1` on each `upload-artifact` step reduces storage
consumption; this is a SHOULD, not a MUST.

### CHANGELOG extraction

The `release` job can use the same `awk` pattern from LCV-089:
```bash
TAG="${{ github.ref_name }}"    # e.g. "v0.1.0"
VERSION="${TAG#v}"              # strip leading "v" → "0.1.0"
awk "/^## \[${VERSION}\]/{found=1; next} found && /^## \[/{exit} found{print}" \
  CHANGELOG.md > /tmp/release-notes.md
[ -s /tmp/release-notes.md ] || { echo "No CHANGELOG section for ${VERSION}"; exit 1; }
```
The empty-file guard (`[ -s … ]`) satisfies the abort-on-empty requirement in AC#9d.

### Idempotency of `gh release create`

If the release already exists (e.g., on a CI re-run), `gh release create` exits
non-zero. The simplest acceptable behaviour is to let the step fail on a duplicate
release — re-running after a successful publish is an operator error. The implementer
may add `gh release upload <tag> <artifacts>` as a re-run fallback, but this is not
required by the acceptance criteria.
