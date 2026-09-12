# LCV-107 — CI: Windows package job invokes `build-msi.ps1` via PowerShell

- **Status**: Ready
- **Phase**: 10
- **Depends on**: none
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

`.github/workflows/ci.yml:171` runs the Windows packaging step as
`bash ./scripts/build-msi.sh`. That file does not exist and never did — LCV-090
shipped `scripts/build-msi.ps1`, a PowerShell script. The step is named
"Package (Windows) — MSI via Git Bash", which describes a plan that was never
implemented.

The `package` job runs only on `v*.*.*` tag pushes, and the repository has no
git remote, so the workflow has never executed once. The first thing that will
happen when the user pushes the v0.1.0 tag is that the Windows leg dies on
`bash: ./scripts/build-msi.sh: No such file or directory`, after the `test` and
`build` matrices have already spent their minutes, and the `release` job — which
`needs: [package]` — never runs. Marco 0 claims "Done" for packaging; a
guaranteed-red release pipeline is not done.

A second, independent failure sits behind the first: the `package` job only
checks out the repo and downloads the prebuilt binary. `scripts/build-msi.ps1`
calls `cargo build --release --target x86_64-pc-windows-msvc` and then
`cargo wix`, and `cargo-wix` is not present on the runner and is not installed
by any step. Fixing only the shell would move the failure from step 1 to step 2.

## Scope

`.github/workflows/ci.yml` only — the Windows leg of the `package` job:

- Rename the step to "Package (Windows) — MSI via PowerShell".
- Replace the invocation with an explicit PowerShell run:
  ```yaml
      - name: Package (Windows) — MSI via PowerShell
        if: runner.os == 'Windows'
        shell: pwsh
        run: ./scripts/build-msi.ps1
  ```
- Add, immediately before it, the two prerequisites the script needs and the job
  does not currently provide, both guarded by `if: runner.os == 'Windows'`:
  ```yaml
      - name: Install Rust toolchain (from rust-toolchain.toml) (Windows)
        if: runner.os == 'Windows'
        run: rustup show

      - name: Install cargo-wix (Windows)
        if: runner.os == 'Windows'
        run: cargo install cargo-wix --locked
  ```
  `rustup show` is the same toolchain-pinning idiom the `build` job already uses
  (`.github/workflows/ci.yml:104-105`).
- No other job, step, matrix entry or trigger is modified.

## Out of scope — hard boundary

These are outward-facing actions that require the user's decision on repository
name and visibility, and must **not** be taken as part of this demand:

- **Creating a git remote**, choosing a hosting account, or setting repository
  visibility.
- **`git push`** of any kind, including pushing a branch to seed the workflow.
- **Creating or pushing the `v0.1.0` tag** (LCV-089, currently `Blocked` on
  exactly this).
- **Watching, re-running or debugging a live matrix run.** Validating CI end to
  end is a follow-up that starts the moment a remote exists.

Also out of scope:

- Rewriting `scripts/build-msi.ps1`, `build-appimage.sh`, `build-deb.sh`,
  `build-dmg.sh` or `release.sh`.
- Changing the `test`, `build` or `release` jobs, the trigger list, or the
  runner images.
- Adding an `actionlint` gate to CI.
- Making the Windows leg reuse the downloaded binary instead of rebuilding
  (a real inefficiency, but a behaviour change that cannot be validated without
  a live run).

## Acceptance criteria

1. `grep -rn "build-msi.sh" .github/` returns no match.

2. The Windows packaging step declares `shell: pwsh` and its `run:` is exactly
   `./scripts/build-msi.ps1`; the step name no longer mentions Git Bash.

3. No step guarded by `if: runner.os == 'Windows'` invokes `bash` or a `.sh`
   script.

4. The `package` job contains a Windows-only `rustup show` step and a
   Windows-only `cargo install cargo-wix --locked` step, both ordered **before**
   the packaging step.

5. Every script path referenced anywhere in `.github/workflows/` exists in the
   working tree:
   ```
   grep -ohE '\./scripts/[A-Za-z0-9._-]+' .github/workflows/*.yml | sort -u | \
     while read -r p; do test -f "${p#./}" || echo "MISSING $p"; done
   ```
   prints nothing.

6. `.github/workflows/ci.yml` is valid YAML:
   `python3 -c "import yaml;yaml.safe_load(open('.github/workflows/ci.yml'))"`
   exits 0.

7. The Linux and macOS legs of the `package` job are byte-identical to their
   current content; `git diff` touches only the Windows leg.

8. No file outside `.github/workflows/ci.yml` is modified by this demand.

9. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
   and `cargo test --all` still exit 0 (the change is CI-only; this is a
   regression guard, not a claim about the workflow).

## Expected tests

- **(AC 1, 2, 3, 4)**: static checks — `grep` on `.github/`, plus a read of the
  `package` job's Windows steps.
- **(AC 5)**: the script-existence loop above; expected output is empty.
- **(AC 6)**: the `yaml.safe_load` parse check.
- **(AC 7, 8)**: `git diff --stat` shows one file changed, and
  `git diff .github/workflows/ci.yml` shows no hunk inside the Linux or macOS
  steps.
- **(AC 9)**: the three build gates.
- **Manual (deferred, not part of this demand)**: once a remote exists, push a
  throwaway `v0.0.0-test` tag and confirm the Windows `package` leg produces
  `dist/lasercad-x86_64.msi` and uploads the `package-windows` artifact. Record
  the result under LCV-089.

## Risks

- **Unverifiable until a remote exists.** Nothing in this demand can be proven
  green by running CI, because CI cannot run. The acceptance criteria are
  therefore all static; they remove two *certain* failures without claiming the
  job succeeds.
- **Runner-image assumptions.** The fix assumes `windows-2022` ships WiX
  Toolset v3 (`candle.exe` / `light.exe`) and a rustup-managed Rust, so only
  `cargo-wix` needs installing. If the first real run disproves that, the
  follow-up is a `wix` install step — a one-line addition, not a redesign.
- **`cargo install cargo-wix` adds several minutes** to a job that only runs on
  tag pushes. Acceptable for a release-only path; caching it is a later
  optimisation, not a Marco 0 concern.
- **The script rebuilds the binary** that the job just downloaded, so the
  `binary-windows` artifact is effectively unused on that leg. Left as-is
  deliberately (see Out of scope) — changing it without a live run to verify
  would trade a known-correct build for an unproven one.

## Open questions

*(none — demand is Ready)*

## Notes

- `scripts/build-msi.ps1` sets `$ErrorActionPreference = 'Stop'` and
  `Set-StrictMode -Version Latest` at the top, and checks `$LASTEXITCODE` after
  both `cargo build` and `cargo wix`, so a failure inside the script propagates
  to the step without extra flags. It also resolves and `Set-Location`s to the
  repo root itself, so no `working-directory:` key is needed.
- Expected output path is `dist/lasercad-x86_64.msi`, which is what the existing
  "Upload package artifact (Windows)" step already publishes — unchanged.
- LCV-089 ("First 0.1.0 release tag + GitHub release") is `Blocked` awaiting the
  git remote. This demand removes one of its blockers; the remote decision
  remains the user's.
