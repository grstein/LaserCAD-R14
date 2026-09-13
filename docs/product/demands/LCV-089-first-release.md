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

- **Manual smoke / the first live agent run (Marco 2: LCV-121…LCV-125)**: the
  whole agent stack is validated in CI against `mockito` only — ADR 0007 commits
  Marco 2 to that deliberately, because no CI run may spend the user's tokens or
  depend on a third party being up. Nothing therefore proves the real endpoint
  answers until a human asks it to, and steps 10–15 are rendering criteria only
  a human looking at the screen can accept. **Marco 2 is complete, so this list
  is live.** Fifteen steps, in this order: 1 gates everything downstream, 2–3 are
  the headline behaviours only a real model can demonstrate, 4 is the prompt
  contract, **5 decides whether the build is taggable**, 6–7 are the error paths,
  8–9 are security sweeps, and 10–15 are what LCV-125 shipped. Report any step
  that fails; do not silently retry.

  **Setup.** `Help > Agent settings` exposes four fields — Endpoint URL, Model,
  API Key, and a Steps-per-turn slider bounded 1 to 32. Defaults: endpoint
  `https://openrouter.ai/api/v1`, model `anthropic/claude-sonnet-4.6`, budget
  12. Requests go to `{endpoint}/chat/completions` with
  `Authorization: Bearer <key>`. Open the chat panel with the 🤖 toolbar button.
  Enter a real key, close the dialog, and confirm it survives a restart before
  starting. (Model and budget are **form fields since LCV-125** — nothing here
  requires hand-editing `settings.json`.)

  1. **Auth, model slug, and one action end to end.** Prompt
     `draw a line from 0,0 to 100,0`. The most informative first prompt: it
     exercises auth, the model slug, tool-schema acceptance, one tool call, one
     commit, the undo note and the terminal row in one round trip. Correct: a
     user row, a spinner, a tool-call row, an assistant row confirming, and a
     note row reading exactly `Applied 1 action — Ctrl+Z undoes it.` A 100 mm
     horizontal line lands on the bed, the app goes dirty, one Ctrl+Z removes
     it. Failure modes in likelihood order: **401** — the key is blank or was
     never saved; **404 on the model slug** — OpenRouter renamed it, check
     `openrouter.ai/models`; **an assistant row with no tool-call row and no
     note row** — the schemas were accepted but not used, a prompt issue rather
     than a defect; **a spinner that never clears** — the no-timeout hazard,
     which is LCV-129.
  2. **The multi-action fold.** Prompt `draw a 40 mm square centred at the
     origin`. This is the headline behaviour and nothing in CI can prove it,
     because no test makes a real model emit four calls. Correct: four
     transcript rows, then `Applied 4 actions — Ctrl+Z undoes the whole turn.`,
     and **one** Ctrl+Z removes all four. Worth reporting: a note saying the
     actions stay four separate undo steps while nothing else touched the
     drawing — that is a real coalesce-gate defect. Mis-placed lines are a
     prompt issue, not one.
  3. **A query stays out of the fold.** With the square on the bed, prompt
     `what is on the drawing?`. Correct: one query row, an assistant row listing
     4 lines with indices 0 to 3 in millimetres, and **no note row at all** — a
     query is an action but not an applied one. Ctrl+Z still undoes the square.
     Failure: a note row appears, meaning queries are being counted.
  4. **The positional-index contract under renumbering.** Prompt
     `delete the first line, then tell me what is left`. This is what ADR 0007
     §D5 and the extended system prompt exist for, and it is pure prompt
     behaviour no test can reach. Correct: a delete row with index 0, then a
     query row showing the model re-read rather than assumed, then a report of 3
     entities. Failure: it deletes index 0 and then reasons about a stale index
     3, meaning the prompt's contract is not landing. Highest-value signal
     available in one turn.
  5. **The fence — the release-critical step.** Prompt `draw three circles of
     radius 10 in a row 30 mm apart`, and **while the spinner is up**, click an
     entity on the canvas or press Ctrl+Z. Correct: the next action comes back
     refused, the transcript shows a row beginning
     `The drawing changed outside this turn —`, the model stops and explains
     rather than retrying, the turn ends, the spinner clears, and the window
     stops burning CPU. Failures by severity: **the spinner never clears** —
     LCV-120 reopened, stop and report; **the model retries in a loop** until
     the budget is exhausted; **no refusal row and the circles all land anyway**
     — the fence is not being consulted. Both reviewers independently called
     this the step that decides whether the build is taggable, because it is the
     only property no test in the suite can reach.
  6. **Budget exhaustion is graceful.** Drag the Steps-per-turn slider to 1,
     close the dialog, prompt `draw a square`. Correct: one action applied, an
     error row naming the exhausted budget,
     `Applied 1 action — Ctrl+Z undoes it.`, spinner clears. The cheapest way to
     see the error path with a live model. **Set it back to 12 afterwards.**
  7. **Abandonment.** Closing the agent panel does **not** cancel, because the
     receiver lives on the app rather than the panel and there is no cancel
     button yet (LCV-129). Start a turn, close the panel, keep drawing, reopen:
     the turn should either have landed or been fenced off by your drawing, and
     the spinner should be clear. Observing the cancelled path itself requires
     quitting mid-turn and leaves nothing in-app to see — skip it unless chasing
     a hang.
  8. **Key hygiene sweep, once at the end.** Scroll the entire transcript and
     check the terminal for tracing output. The API key must appear in **no**
     chat row, no command feedback, no error text and no log line. Then grep the
     autosave file for it. This is ADR 0007 §D10 and there is no automated
     check, because no test may carry a real key.
  9. **Proxy isolation.** Run `cargo test --all --no-fail-fast` once with
     `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY` all set to
     `http://127.0.0.1:9`, and confirm the same pass set as an unproxied run and
     **zero outbound connections**. Manual because no CI job here sets a proxy,
     and the failure mode is silent: the LCV-124 reviewer captured real POSTs
     leaving the machine carrying the bearer token, the system prompt and the
     operator's prompt text while the suite reported 11 passed / 0 failed.
     `reqwest::blocking::Client::new()` enables system-proxy discovery and
     reqwest 0.12 has no loopback bypass, so a test endpoint on a closed
     loopback port is dialled through the proxy instead of failing locally.
     LCV-124 fixed its own fixture at `a1b37aa`; the rest of the suite is
     **LCV-130** and is still open, so until that ships expect this step to find
     failures — a green run here is the thing being tested, not a formality.
  10. **The plaintext warning is visible before anything is typed.** With an
      **empty** API key, open `Help > Agent settings` and confirm the warning is
      on screen before you type. This is the exact state a surviving mutant made
      invisible, and it is the state every first-time operator is in.
  11. **Two consecutive tool rows stack vertically.** Ask for something taking
      two actions — a line then a circle — and confirm the two marker rows stack
      rather than flowing side by side.
  12. **Transcript order is oldest at top.** Trivial to eyeball and worth one
      glance.
  13. **The slider stops at both ends.** Drag it to each extreme and confirm it
      stops at 1 and at 32, rather than only testing that a hand-edited 200
      clamps.
  14. **A narrow panel folds rather than clips.** Resize the agent panel narrow
      and confirm a long tool outcome wraps. Watch the **user row**: its
      right-to-left layout defaults to extending, and the wrap call is the only
      thing holding it.
  15. **Read the renumbering sentence off the screen.** After a delete, find the
      ADR 0007 §D5 sentence in the monospace row. That sentence being findable
      is the entire justification for LCV-125 — if it is not findable, the
      demand did not deliver.

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
