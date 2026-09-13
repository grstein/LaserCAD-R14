# LCV-089 — First 0.1.0 release tag + GitHub release

- **Status**: Blocked
- **Phase**: 8
- **Depends on**: LCV-085, LCV-086, LCV-088
- **Suggested agent**: project-manager
- **Suggested model**: sonnet
- **Implementation**: b936570 — chore(LCV-089): bump Cargo.toml to v0.1.0, finalize CHANGELOG, add release.sh
- **Note**: b936570 bumped Cargo.toml to 0.1.0, finalized the CHANGELOG and added scripts/release.sh, but no git tag and no GitHub release exist (the repository has no git remote). Blocked on the user choosing a remote.

## Problem

Once the AppImage (LCV-085), `.deb` (LCV-086), and production icons/desktop entry
(LCV-088) are in place, there is no automated way to cut the actual 0.1.0 release:
the version string is still `0.1.0-dev`, the CHANGELOG has no dated entry, no git
tag exists, and no GitHub release page has been created. A laser-cutter operator
who finds the project cannot download a ready-to-run binary because the release
artifacts are not published anywhere. This demand closes that gap: it bumps the
version, finalises the changelog, writes a release script that pre-flight-checks
every precondition and then creates the annotated tag and GitHub release with the
AppImage and `.deb` as downloadable assets.

## Scope

- Bump `Cargo.toml`: `version = "0.1.0-dev"` → `version = "0.1.0"`.
- Update `CHANGELOG.md`: rename the `## [Unreleased]` section to
  `## [0.1.0] - YYYY-MM-DD` (date at time of implementation), then insert a fresh
  empty `## [Unreleased]` section above it (with no items).
- Create `scripts/release.sh` — a bash script that:
  1. Verifies preconditions (clean working tree, `gh` CLI present, assets built).
  2. Runs the full CI gate locally.
  3. Creates an annotated git tag `v0.1.0`.
  4. Pushes the tag to `origin`.
  5. Extracts the `[0.1.0]` CHANGELOG block as release notes.
  6. Creates the GitHub release via `gh release create` attaching
     `dist/lasercad-x86_64.AppImage` and the first `dist/lasercad_*.deb` found.
  7. Prints a post-release reminder checklist to stdout.
- `scripts/release.sh` is tracked by git with the execute bit set.
- `dist/` and `build/` remain gitignored (already enforced by LCV-085; this demand
  must not remove those entries).

## Out of scope

- Automatically building the AppImage or `.deb` from within `scripts/release.sh` —
  those are the responsibility of `scripts/build-appimage.sh` (LCV-085) and the
  equivalent `.deb` build script (LCV-086). The release script verifies they exist.
- Windows or macOS release assets — LCV-090, LCV-091.
- CI automation of the release pipeline — LCV-092 or a future dedicated demand.
- Code-signing the AppImage, the `.deb`, or the git tag.
- Generating or updating a `docs/build-local.md` build guide.
- Bumping any version beyond `0.1.0` — future releases are out of scope.
- Any change to `[profile.release]` in `Cargo.toml` — that is LCV-087.
- A `scripts/release.sh` that auto-bumps `Cargo.toml` or edits `CHANGELOG.md` at
  runtime. Version bumping and changelog editing are done by the implementer manually
  before committing this demand; the script only verifies those edits are in place.

## Acceptance criteria

1. `Cargo.toml` `[package]` table contains exactly `version = "0.1.0"` (no `-dev`
   suffix). Verify with:
   ```
   grep '^version' Cargo.toml
   ```
   Output must be `version = "0.1.0"`.

2. `CHANGELOG.md` contains a level-2 heading `## [0.1.0]` with an ISO date suffix
   (`## [0.1.0] - YYYY-MM-DD`) and a level-2 heading `## [Unreleased]` that appears
   above it with no list items. Verify with:
   ```
   grep -n '## \[' CHANGELOG.md | head -3
   ```
   The first match must be `## [Unreleased]`; the second match must be
   `## [0.1.0] - <date>`.

3. `scripts/release.sh` exists and is tracked by git:
   ```
   git ls-files scripts/release.sh
   ```
   returns `scripts/release.sh`.

4. `scripts/release.sh` has the execute bit set for at least the owner:
   ```
   ls -l scripts/release.sh
   ```
   shows `-rwx` as the first four characters.

5. The first two lines of `scripts/release.sh` are exactly:
   ```
   #!/usr/bin/env bash
   set -euo pipefail
   ```

6. `scripts/release.sh` contains a precondition block that explicitly checks all of
   the following before performing any destructive action (read the script to verify
   each guard):
   - `gh` CLI is on `PATH` (`command -v gh`).
   - The git working tree is clean (`git diff --quiet && git diff --cached --quiet`);
     the script prints an error and exits non-zero if uncommitted changes are present.
   - `dist/lasercad-x86_64.AppImage` exists as a regular file (`[ -f ... ]`).
   - At least one file matching `dist/lasercad_*.deb` exists
     (`ls dist/lasercad_*.deb 2>/dev/null | head -1`).
   - `Cargo.toml` version is `0.1.0` (not `0.1.0-dev`); the script reads the version
     with `grep` or `sed` and aborts if the `-dev` suffix is present.
   - `CHANGELOG.md` contains a `## [0.1.0]` section; the script greps for it and
     aborts if not found.

7. `scripts/release.sh` runs the CI gate before tagging:
   ```
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```
   All three commands must appear in the script in that order, and the script must
   exit non-zero if any of them fails (enforced by `set -euo pipefail` and the calls
   appearing unconditionally).

8. `scripts/release.sh` creates an annotated git tag:
   ```
   git tag -a v0.1.0 -m "LaserCAD v2 0.1.0"
   ```
   This line (or equivalent using a variable for the version) must appear in the
   script. The script must only attempt this if the tag does not already exist
   (`git tag -l v0.1.0` is empty); if the tag already exists the script prints a
   warning and continues to the push and release steps so the script is idempotent.

9. `scripts/release.sh` pushes the tag to `origin`:
   ```
   git push origin v0.1.0
   ```
   This line must appear in the script after the tag creation block.

10. `scripts/release.sh` extracts the release notes from `CHANGELOG.md` by capturing
    the text between the `## [0.1.0]` heading and the next `## [` heading (exclusive),
    and writes them to a temporary file (e.g. `/tmp/release-notes-0.1.0.md`) that is
    passed to `gh release create` via `--notes-file`. Verify by reading the script.

11. `scripts/release.sh` creates the GitHub release via:
    ```
    gh release create v0.1.0 \
      --title "LaserCAD v2 0.1.0" \
      --notes-file /tmp/release-notes-0.1.0.md \
      dist/lasercad-x86_64.AppImage \
      dist/lasercad_*.deb
    ```
    (exact flag spelling; `--notes-file` path may vary, `dist/lasercad_*.deb` may be
    expanded via a variable). The `--title` value must be `"LaserCAD v2 0.1.0"`.

12. `scripts/release.sh` prints a post-release checklist to stdout (plain text, at
    minimum three items) after the `gh release create` call. Required items:
    - Verify the GitHub release page shows the AppImage and `.deb` as assets.
    - Update `Cargo.toml` version to the next dev version (e.g. `0.2.0-dev`) in a
      follow-up commit.
    - Announce the release (forum, README badge, etc.).

13. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and
    `cargo test --all` all exit 0 after the `Cargo.toml` version change introduced by
    this demand. The version string change (`0.1.0-dev` → `0.1.0`) must not break any
    test that hard-codes the version.

## Expected tests

- **Static / AC#1**: `grep '^version' Cargo.toml` returns `version = "0.1.0"`.

- **Static / AC#2**: `grep -n '## \[' CHANGELOG.md | head -3` shows `[Unreleased]`
  first and `[0.1.0] - <date>` second.

- **Static / AC#3**: `git ls-files scripts/release.sh` returns the path.

- **Static / AC#4**: `ls -l scripts/release.sh` shows `-rwx` prefix.

- **Static / AC#5**: `head -2 scripts/release.sh` shows the shebang and `set` line.

- **Static / AC#6**: read `scripts/release.sh`; locate the six guard conditions and
  confirm each uses the exact check described (exit on failure, before any tagging).

- **Static / AC#7**: read `scripts/release.sh`; confirm the three CI commands appear
  in order before the `git tag` call.

- **Static / AC#8**: read `scripts/release.sh`; confirm `git tag -a v0.1.0` with
  idempotency guard (`git tag -l v0.1.0`).

- **Static / AC#9**: read `scripts/release.sh`; confirm `git push origin v0.1.0`
  appears after the tag block.

- **Static / AC#10**: read `scripts/release.sh`; confirm CHANGELOG extraction logic
  and `--notes-file` usage.

- **Static / AC#11**: read `scripts/release.sh`; confirm `gh release create v0.1.0`
  with `--title "LaserCAD v2 0.1.0"`, `--notes-file`, AppImage, and `.deb` arguments.

- **Static / AC#12**: read `scripts/release.sh`; confirm the post-release checklist
  block contains at least three items including the three required ones.

- **CI gate / AC#13**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` all exit 0 after the version bump.

- **Manual smoke / dry-run AC#6**: set `DRY_RUN=1` or manually remove the AppImage
  from `dist/` and run `./scripts/release.sh`; confirm the script exits non-zero and
  prints the precondition error, and that no tag was created (`git tag -l v0.1.0` is
  empty).

- **Manual smoke / full run**: on a machine with `gh` authenticated, AppImage and
  `.deb` present in `dist/`, and a clean working tree at the tagged commit, run
  `./scripts/release.sh`; confirm tag `v0.1.0` exists locally and on the remote,
  and the GitHub release page shows both assets and the extracted changelog text.

- **Manual smoke / native file dialogs (ADR 0005, LCV-118)**: no automated test
  can observe `arm_native_dialogs()` actually flipping the flag to true — a test
  that armed it would poison the shared `AtomicBool` for the rest of that test
  binary, and ADR 0005 deliberately provides no disarm function, so if the
  arming were ever wrong a real user would hit a panic the first time they
  opened a file and nothing in CI would catch it. Before tagging: launch the
  app and use **File > Open** — a real OS dialog must open rather than
  panicking, and **Cancel** must leave the document untouched; draw a line and
  press **Ctrl+S** — the save dialog must open prefilled, and the save must
  succeed; then **File > Open** the saved file and confirm the line
  round-trips.

- **Manual smoke / configurable bed size (LCV-114)**: the LCV-114 reviewer
  confirmed these four paths are reachable by no automated test in the suite.
  Before tagging:
  - Open a file exported at 300 x 180 in real LaserGRBL and confirm the job
    lands in the same corner it appears in on screen. This is the criterion no
    unit test can cover.
  - Click `File > Bed size…` and drag both `DragValue` widgets. Real pointer
    hit-testing on a nested menu button and a drag on a `DragValue` are
    simulated nowhere in the suite.
  - Restart the app cold and confirm a blank document starts at the bed size
    last set. `App::new()` cannot be called from any test under ADR 0002
    §A2, so this is covered only by a source scan.
  - Confirm the bed rectangle on the canvas visibly redraws at the new size
    after OK, and that `View > Fit to Bed` frames it.

- **Manual smoke / export preset selector (LCV-115)**: the LCV-115 reviewer
  found that nested menu button and radio item hit-testing is simulated
  nowhere in the suite. Before tagging:
  - The status bar reads `CUT` on a cold start.
  - `File > Export preset` then Mark flips the status bar to `MARK`.
  - Save, and confirm the geometry sits inside the mark group with the cut
    and engrave groups empty.
  - Open that file in real LaserGRBL and confirm it is recognised as blue and
    marked.
  - `File > New` keeps `MARK`, and a restart resets to `CUT`.

- **Manual smoke / chrome completion (LCV-116)**: before tagging:
  - The three mode indicators render unselected on a cold start and the bar
    reads no autosave yet.
  - Clicking each of the three indicators visibly toggles it, actually
    changes the behaviour, and agrees with the matching function key and
    View menu checkbox.
  - Draw a line, stop touching the app, and watch the indicator settle from
    pending to autosaved hands off. This will probably pass today for the
    wrong reason, because of the unconditional repaint recorded in LCV-120 —
    which is exactly why it stays a manual check.
  - F1 opens the dialog, it scrolls on a short window, and the close button
    works.
  - With the Text tool active and characters typed, F1 still opens, and
    pressing the letter L types the character rather than switching tools.

- **Manual smoke / first real prompt against OpenRouter (LCV-123)**: the whole
  agent stack is validated in CI against `mockito` only — ADR 0007 commits
  Marco 2 to that deliberately, because no CI run may spend the user's tokens
  or depend on a third party being up. Nothing therefore proves the real
  endpoint answers until a human asks it to. **This item becomes relevant once
  LCV-123 lands** (before that, an agent turn mutates a throwaway document and
  there is nothing to smoke-test); if the release is cut before LCV-123, record
  it as not applicable rather than as passed.
  - Enter a real OpenRouter API key in `Help > Agent settings`, close the
    dialog, and confirm it survives a restart.
  - Type a free-text request into the command line — for example
    `draw a 50 mm square at the origin` — and confirm the transcript shows the
    prompt was sent, the tool rows that came back, and a final answer.
  - The geometry appears **in the open drawing**, at the coordinates asked for,
    in millimetres, and the status bar agrees.
  - A single `Ctrl+Z` undoes the entire turn, and `Ctrl+Y` puts it back.
  - Clear the key, restart, and confirm the same free text now answers
    `! Agent unavailable: set the API key in Help > Agent settings` and draws
    nothing.

## Open questions

*(none)*

## Notes

### Version string convention

`Cargo.toml` currently carries `version = "0.1.0-dev"` as established during the
scaffold. The `-dev` pre-release suffix signals "not yet released" to any tooling that
parses `Cargo.toml`. Removing it to `"0.1.0"` is the canonical release act. The
implementer bumps this before committing the demand; the script in AC#6 then guards
against accidentally running a release while the suffix is still present.

### Why the script does not auto-edit files

`scripts/release.sh` is a pre-flight + execution script, not a build-system
replacement. Letting a script overwrite `Cargo.toml` and `CHANGELOG.md` in-place
creates a class of "half-edited, half-committed" bugs that are harder to review and
revert than a normal human-authored commit. The implementer makes the textual changes
(Cargo.toml version, CHANGELOG rename) as part of this demand, commits them, and
the script verifies they are already in the commit before proceeding.

### Extracting the CHANGELOG section

A portable `awk` one-liner is sufficient:
```bash
awk '/^## \[0\.1\.0\]/{found=1; next} found && /^## \[/{exit} found{print}' CHANGELOG.md \
  > /tmp/release-notes-0.1.0.md
```
The implementer may use `sed`, `grep -A`, or any POSIX tool available on Ubuntu 24.04.
The test in AC#10 is "read the script and confirm the extraction logic"; it does not
mandate the specific tool used.

### `gh` CLI authentication

`gh release create` requires an authenticated GitHub CLI session
(`gh auth login` or the `GH_TOKEN` environment variable). The release script is
designed to be run by a human on a workstation with `gh auth status` passing; it is
not designed for unattended CI use (that is LCV-092).

### Relation to LCV-085 and LCV-086

`scripts/release.sh` does **not** call `scripts/build-appimage.sh` or the `.deb`
build script. The operator is expected to have already built the artifacts:
```
./scripts/build-appimage.sh
./scripts/build-deb.sh   # produced by LCV-086
```
before running `./scripts/release.sh`. The release script checks that both artifact
files exist (AC#6) and aborts if they are missing.

### Idempotency of the tag step

If `v0.1.0` was already pushed (e.g. after a failed `gh release create`), re-running
`scripts/release.sh` must not abort at the tag step. The idempotency guard in AC#8
covers this: if the tag exists locally, the script skips `git tag -a` and continues
to the push and release creation steps, allowing the operator to retry publishing
without needing to delete and recreate the tag.

### Post-0.1.0 version bump

The `scripts/release.sh` post-release checklist (AC#12) reminds the operator to bump
`Cargo.toml` to the next dev version (e.g. `0.2.0-dev`) in a follow-up commit. That
bump is **not** part of this demand; it belongs to whatever demand opens the 0.2.0
development cycle.
