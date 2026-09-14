# LCV-135 — CI bills Windows and macOS on every push; gate them to tags and dispatch

- **Status**: Done
- **Implementation**: a3b9769
- **Phase**: 11
- **Depends on**: none. LCV-092 (Done) built the four-job workflow this narrows; LCV-107 (Done) last touched it. Both are read-for-vocabulary, not blockers.
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

`.github/workflows/ci.yml` runs the `test` and `build` jobs across
`ubuntu-24.04`, `windows-2022` and `macos-15` on every pull request and every
push to `main`. That is six billed legs for a change that, in this repository,
is almost always pure Rust that compiles identically on all three. GitHub bills
Windows minutes at 2x and macOS minutes at 10x the Linux rate, so the two legs
that almost never catch anything dominate the bill for the two jobs that run
most often.

The account is now on a **billing hold: private-repository Actions minutes are
exhausted and the quota renews 1 October 2026**. The user has decided to wait
for the renewal rather than change repository visibility or move to a paid plan.
This demand is the shape the workflow should be in *when the meter starts
again* — the first run after renewal should be one Linux leg, not six.

The consequence for this demand is unusual and has to be stated up front: **it
ships without a single CI run to prove it works.** There is no green check to
point at. The evidence available is static — the file parses, the job structure
is what was asked for, and the conditional expressions evaluate correctly when
worked through by hand. §Expected tests says why that is sufficient here and
would not be sufficient normally.

## Scope

`.github/workflows/ci.yml` only.

- Add `workflow_dispatch:` to the `on:` block, alongside the existing
  `pull_request` and `push` (`branches: [main]`, `tags: ['v*.*.*']`) clauses.
- Replace the static matrix of the `test` job so that it resolves to
  `ubuntu-24.04` alone by default, and to all three runners
  (`ubuntu-24.04`, `windows-2022`, `macos-15`) when the run is a `v*.*.*` tag
  push or a `workflow_dispatch`.
- Do the same for the `build` job, whose matrix is an `include:` list of
  mappings (`os`, `artifact-name`, `binary-path`) rather than a bare `os:` list.
  The mapping objects carry over verbatim; only which of them exist changes.
- Confirm — and record — that on a tag the `package` job's inputs still resolve:
  it downloads `binary-linux`, `binary-windows` and `binary-macos` from `build`.

## Out of scope

- **The `package` and `release` jobs.** They are already tag-gated by
  `if: startsWith(github.ref, 'refs/tags/')`. They are not to be edited, not
  re-indented, not re-commented. Their hunks must not appear in the diff.
- **Any file other than `.github/workflows/ci.yml`.** No `Cargo.toml`, no
  `src/`, no `scripts/`, no `rust-toolchain.toml`, no `README.md` badge.
- **The three quality gates** inside `test` (`cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo test --all --no-fail-fast`), their order, and the `--no-fail-fast`
  flag that ADR 0008 makes load-bearing. Untouched.
- **The Linux-only Xvfb and apt steps**, the `rustup show` toolchain idiom, the
  `Swatinem/rust-cache@v2` key, the `actions/*` version pins, `fail-fast: false`,
  and the `env:` block (`CARGO_TERM_COLOR`, `RUSTFLAGS`). Untouched.
- **Runner image changes.** `macos-15` stays `macos-15`; this demand does not
  revisit the choice LCV-107 made in `958fb12`.
- **Cron / scheduled triggers.** Only `workflow_dispatch` is added.
- **Any attempt to run, re-run, trigger, or diagnose GitHub Actions.** No
  `gh run rerun`, no `gh run watch`, no pushing a throwaway tag, no
  investigation of the billing state. The hold is a settled user decision, not
  a problem to solve.
- **Repository visibility, billing plan, or self-hosted runners.** Explicitly
  considered and declined by the user.

## Acceptance criteria

1. `git diff --stat` for the implementing commit lists exactly one changed
   file: `.github/workflows/ci.yml`.

2. The file still parses as valid YAML:
   `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
   exits 0.

3. The `on:` block contains `workflow_dispatch:` and still contains, unchanged,
   `pull_request`, `push: branches: [main]` and `push: tags: ['v*.*.*']`.
   Nothing else is added to `on:` — in particular no `schedule:`.

4. The `test` job's `strategy.matrix.os` is no longer a literal three-element
   list. It is a single GitHub Actions expression that resolves to:
   - `["ubuntu-24.04"]` — one leg — for a `pull_request` and for a `push` to
     `main`;
   - `["ubuntu-24.04", "windows-2022", "macos-15"]` — three legs — for a
     `push` of a `v*.*.*` tag and for a `workflow_dispatch`.

5. The `build` job's `strategy.matrix.include` resolves the same way, and each
   surviving entry carries the same three keys with the same values as today:

   | `os` | `artifact-name` | `binary-path` |
   |---|---|---|
   | `ubuntu-24.04` | `binary-linux` | `target/release/lasercad` |
   | `windows-2022` | `binary-windows` | `target/release/lasercad.exe` |
   | `macos-15` | `binary-macos` | `target/release/lasercad` |

   No artifact name and no binary path changes.

6. The condition sub-expression that selects between the one-leg and three-leg
   forms is **character-for-character identical** in `test` and `build`. Verify
   by extracting both expressions and diffing them. Drift between the two is the
   failure this criterion exists to prevent: `test` running three legs while
   `build` runs one would leave a tag with no `binary-windows` artifact and a
   release with no MSI.

7. Neither the one-leg nor the three-leg branch of the expression can evaluate
   to an empty array. The fallback branch is the literal one-element
   `ubuntu-24.04` array — never `[]`, never an expression that could produce
   `[]`. A matrix that resolves to zero legs is reported by GitHub as a skipped
   job, not an error, and would silently disable CI entirely.

8. The gate is implemented by **not creating** the Windows and macOS legs, not
   by creating them and skipping their work. A full three-entry matrix whose
   steps carry `if:` guards, or a `continue-on-error`, or an early-exit first
   step, does not satisfy this demand: the runner still starts and still bills.
   Confirm no new step-level `if:` appears in `test` or `build`, and that no
   job-level `if:` is added to `test` or `build` (a job-level `if:` would gate
   the Linux leg too).

9. The `package` and `release` jobs are byte-identical to their state at the
   parent commit. Verify with
   `git diff <parent> -- .github/workflows/ci.yml` and confirm no hunk falls
   inside either job.

10. On a tag push, `package`'s inputs still resolve. The implementer states, in
    the handover, that `build` produces `binary-linux`, `binary-windows` and
    `binary-macos` on a tag, and that `package`'s `matrix.binary-artifact`
    values (`binary-linux`, `binary-windows`, `binary-macos`) each name one of
    them. This is expected to hold — on a tag the gate opens and all three
    `build` legs run — but it must be checked and written down, because the
    outcome this change risks is a release that silently ships without its
    Windows MSI.

11. The handover contains the three hand-evaluations required by
    §Expected tests, written out in full, showing the value of `github.ref` and
    `github.event_name` for each case, the value of each clause, and the
    resulting leg count.

12. The handover pastes the actual output of the YAML parse and of
    `gh workflow view`. If `gh` cannot reach the API under the billing hold, the
    handover says so and quotes the error verbatim. It does not claim a check
    that did not happen.

13. The close-out records **`CI: not run (billing hold)`** in the place a run id
    or Actions URL would normally go. No run id is invented, and no earlier
    run's id is reused.

## Expected tests

**Why the usual standard does not apply here.** The house rule is that a change
is proven by a test that fails when the change is reverted. For a workflow file
the only executor is GitHub's, and the only mutation test is *push a tag and
watch which legs appear* — which costs the minutes that do not exist until
1 October 2026. So the proof is substituted, not waived. It is substituted with
something that happens to be complete rather than merely indicative: the gating
expression reads exactly two values out of the event context, `github.ref` and
`github.event_name`, and both are fully determined by which of the four trigger
shapes fired. Enumerating the trigger shapes therefore enumerates the entire
input space of the expression. A hand-evaluation is not a weaker substitute for
a run here; for this one expression it is exhaustive.

- **AC#1** — `git diff --stat HEAD~1` shows one file.
- **AC#2** — the `yaml.safe_load` one-liner; paste exit status.
- **AC#3** — read the `on:` block; paste it.
- **AC#4, AC#5** — `gh workflow view` (or `gh workflow view CI --yaml`); paste
  the job list it returns. Under the billing hold this may fail; if it does,
  quote the error and fall back to reading the `strategy:` blocks and pasting
  them.
- **AC#4, AC#5, AC#7 — the three hand-evaluations.** Write each out in full.
  Given the condition
  `startsWith(github.ref, 'refs/tags/') || github.event_name == 'workflow_dispatch'`
  and the GitHub ternary idiom `<cond> && <three-leg array> || <one-leg array>`:

  1. **Ordinary push to `main`.** `github.event_name` is `push`,
     `github.ref` is `refs/heads/main`.
     `startsWith('refs/heads/main', 'refs/tags/')` → `false`.
     `'push' == 'workflow_dispatch'` → `false`.
     Condition → `false`. `false && <three>` → `false`;
     `false || <one>` → the one-element array. **One leg: `ubuntu-24.04`.**

  2. **`workflow_dispatch`.** `github.event_name` is `workflow_dispatch`,
     `github.ref` is the dispatched branch, normally `refs/heads/main`.
     `startsWith('refs/heads/main', 'refs/tags/')` → `false`.
     `'workflow_dispatch' == 'workflow_dispatch'` → `true`.
     Condition → `true`. `true && <three>` → the three-element array, which is
     non-empty and therefore truthy; `<three> || <one>` short-circuits to
     `<three>`. **Three legs: `ubuntu-24.04`, `windows-2022`, `macos-15`.**

  3. **Tag push `v1.0.0`.** `github.event_name` is `push`, `github.ref` is
     `refs/tags/v1.0.0`.
     `startsWith('refs/tags/v1.0.0', 'refs/tags/')` → `true`.
     Condition → `true` on the first clause; the second is not reached.
     **Three legs.** And `package`'s existing
     `if: startsWith(github.ref, 'refs/tags/')` → `true`, so all three package
     legs run against the three binaries `build` just uploaded.

  A fourth case falls out for free and should be stated: a **`pull_request`**
  has `github.event_name == 'pull_request'` and `github.ref` of the form
  `refs/pull/N/merge`, so both clauses are `false` and it resolves exactly as
  case 1 — one Linux leg.

  The truthiness step in case 2 is the one to get right, and it is why AC#7
  exists: the idiom `A && B || C` returns `C` whenever `B` is falsy, and an
  **empty** array is falsy. A three-leg branch that evaluated to `[]` would
  collapse to the one-leg branch, on a release tag, silently — the Windows and
  macOS legs would simply never appear again and the first symptom would be a
  release with no MSI attached.

- **AC#6** — extract the condition text from both `test` and `build` and diff
  the two strings; they must be identical. Paste the comparison.
- **AC#8** — `grep` for `if:` within `test` and `build`; the only matches are
  the pre-existing `runner.os == 'Linux'` step guards. No new ones, and no
  job-level `if:` on either job.
- **AC#9** — `git diff HEAD~1 -- .github/workflows/ci.yml`; confirm by reading
  that no hunk touches `package` or `release`. Paste the diff.
- **AC#10** — read `build`'s `artifact-name` values and `package`'s
  `binary-artifact` values; state the correspondence explicitly.
- **AC#11, AC#12, AC#13** — satisfied by the handover text itself.

- **Not run, deliberately**: any `gh run` subcommand, any workflow trigger, any
  tag push. Not a gap in coverage — an instruction.

## Open questions

*(none)*

## Notes

### Reference shape

Not normative — any formulation that hand-evaluates identically for the four
cases above is acceptable — but this is the idiom the criteria were written
against, for `test`:

```yaml
    strategy:
      fail-fast: false
      matrix:
        os: ${{ (startsWith(github.ref, 'refs/tags/') || github.event_name == 'workflow_dispatch') && fromJSON('["ubuntu-24.04","windows-2022","macos-15"]') || fromJSON('["ubuntu-24.04"]') }}
```

and for `build`, where the entries are mappings, the same condition in front of
two `fromJSON` arrays of objects — the three-entry one carrying the `os` /
`artifact-name` / `binary-path` triples exactly as they appear today, the
one-entry fallback carrying only the `ubuntu-24.04` triple.

### `startsWith(github.ref, 'refs/tags/')`, not a `v`-prefix test

This is LCV-092's vocabulary and the form `package` and `release` already use.
It does not need to check for the `v` prefix, because the `on: push: tags:`
filter already admits only `v*.*.*`; no other tag can start a run. Using the
same predicate keeps all four jobs agreeing about what "a tag run" means, which
is worth more than a redundant prefix check.

### This amends one line of LCV-092's §Out of scope

LCV-092 listed "`workflow_dispatch` or scheduled (cron) triggers" as out of
scope, and its AC#3 required the `on:` block to declare *exactly three* trigger
clauses. LCV-135 deliberately supersedes the `workflow_dispatch` half of that
exclusion: without a manual trigger there is no way to exercise the Windows and
macOS legs at all except by cutting a release, which is worse. **Cron remains
out of scope** and LCV-092's exclusion of it stands.

### The dispatch button will not appear immediately

GitHub only offers a `workflow_dispatch` trigger for a workflow file that exists
on the default branch. The button appears once this lands on `main` and is
pushed — and dispatching it will itself consume minutes, so in practice it is
also unusable until 1 October 2026. That is expected, not a defect.

### `workflow_dispatch` runs the matrix but does not package

On a dispatch, `build` runs all three legs, but `package` and `release` stay
skipped, because their `if: startsWith(github.ref, 'refs/tags/')` is false.
That is the intended shape: dispatch is "smoke-test the full matrix without
cutting a release". Nobody should later "fix" this by loosening `package`'s
guard.

### Stale label in LCV-092

LCV-092 §Scope names `macos-13`. The file has said `macos-15` since LCV-107's
`958fb12`. Use `macos-15`, matching the file. LCV-092's text is a dated record
and is not being edited.

### The `package` job rebuilds on Windows anyway

Worth knowing while checking AC#10: `package`'s Windows leg downloads
`binary-windows` *and* then runs `scripts/build-msi.ps1`, which itself calls
`cargo build --release --target x86_64-pc-windows-msvc`. The download is still a
real dependency the job declares and still has to resolve, so AC#10 stands as
written; it is simply not the only thing keeping that leg honest.
