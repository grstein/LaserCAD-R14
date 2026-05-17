# LCV-004 — CI workflow (fmt + clippy + test, Linux)

- **Status**: Done
- **Phase**: 0
- **Depends on**: LCV-001
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust (task #12); fe4f5c94; AC9 pending push to remote

## Problem

Phase 1 onwards lands large amounts of geometry, document, and rendering code. Without an enforced CI gate, "green on my machine" cannot stop a regression from reaching `main`, and `reviewer-rust` has no neutral evidence that a demand actually passes the three project gates declared in `AGENTS.md` § Implementation Rules (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --all`). This demand adds a single GitHub Actions workflow that runs all three gates on every pull request and every push to `main`, on Linux only — matching the project's primary platform per `AGENTS.md` and `PLAN.md`. Multi-platform expansion lands in Phase 9 (LCV-092).

## Scope

- A single workflow file at `.github/workflows/ci.yml`.
- One job named `linux` (or equivalent, but with `runs-on` set to a pinned Ubuntu image — see acceptance criterion 5).
- Triggers: every `pull_request` and every `push` to `main`. No nightly cron, no manual dispatch (`workflow_dispatch`) — KISS.
- Three named steps, in order: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all`.
- Toolchain comes from `rust-toolchain.toml` (installed by `rustup show` or the equivalent built-in mechanism). No second toolchain pin inside the workflow.
- Cargo registry, git db, and `target/` directories cached via [`Swatinem/rust-cache`](https://github.com/Swatinem/rust-cache) pinned to a tagged release (`v2`-line minor pin acceptable; a SHA pin is also acceptable). Cache key must include `Cargo.lock`.
- System packages required by `eframe`'s glow backend (xkb, xcb, GL) installed via `apt-get`. This is already proven by the scaffold workflow; LCV-004 simply locks it in.
- Every third-party action used in the workflow is pinned to a tagged release (e.g. `actions/checkout@v4`, `actions/cache@v4` or `Swatinem/rust-cache@v2`).

## Out of scope

- Release / packaging workflows (Phase 8 — LCV-085, LCV-086, LCV-088).
- Multi-platform matrix (Windows / macOS) — owned by **LCV-092**.
- Dependabot / Renovate configuration.
- Code coverage (tarpaulin, llvm-cov), benchmarks, or audit (`cargo audit`).
- Caching tuned for incremental rust-analyzer state, sccache, or any non-default cache backend.
- Branch protection rules (configured in the GitHub web UI, not in the repo).
- Status-badge embedding in `README.md`.

## Acceptance criteria

1. The file `.github/workflows/ci.yml` exists at the repository root and is tracked by git.
2. The file parses as valid YAML (e.g. `python -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"` exits 0, or `yq` parses it without error).
3. The `on:` block declares both `pull_request` and `push` triggers, and the `push` trigger restricts to the `main` branch (`branches: [main]`).
4. The workflow contains exactly one job. The job's `runs-on:` is a pinned Ubuntu image — specifically `ubuntu-24.04` (not `ubuntu-latest`). If a newer Ubuntu LTS becomes the GitHub default before this demand ships, the implementer may pick the newer pinned tag and record the choice in the demand's `Implementation:` line, but `ubuntu-latest` is not acceptable.
5. The job contains three named steps that run, in this exact order: (a) `cargo fmt --all -- --check`, (b) `cargo clippy --all-targets -- -D warnings`, (c) `cargo test --all`. Each step has a human-readable `name:` that identifies which gate it runs.
6. No step inside the workflow pins a Rust toolchain channel directly. The toolchain installation step relies on `rust-toolchain.toml` (e.g. via `rustup show`).
7. A cache step is present that caches the cargo registry, the cargo git db, and the `target/` directory, with a cache key that incorporates the hash of `Cargo.lock`. Either `actions/cache@v4` with explicit paths or `Swatinem/rust-cache@v2` is acceptable.
8. Every `uses:` reference in the workflow is pinned to either a tag (e.g. `@v4`) or a full commit SHA. No `@main`, `@master`, or unversioned reference appears.
9. After this demand is merged on `main`, the workflow run on the merge commit completes successfully (all three gates green) on GitHub Actions. The implementer records the successful run URL in the demand's `Implementation:` line.

## Expected tests

- **Static check (criterion 1)**: `git ls-files .github/workflows/ci.yml` returns the file path.
- **Static check (criterion 2)**: parse the file with a YAML parser locally before committing; non-zero exit fails the demand.
- **Static check (criterion 3, 4, 5, 6, 7, 8)**: implementer reads the committed `ci.yml` and confirms each criterion against the file's literal content. No automated test is required because the workflow file *is* the artifact under contract.
- **Build gate (criterion 5, transitively 9)**: the three commands listed in criterion 5 must pass locally before pushing; this is also enforced by `AGENTS.md` § Implementation Rules.
- **Manual smoke (criterion 9)**: after the PR that introduces this workflow is opened, observe that the workflow runs and reports all three steps as green on the PR's checks tab. After merge, the same workflow runs on the `main` push event and is green. The implementer records both run URLs (PR run + main run) in the demand's `Implementation:` line.

## Open questions

(none)

## Notes

- The repository already has a `ci.yml` from the LCV-001 scaffold that uses `runs-on: ubuntu-latest`. This demand explicitly tightens that to a pinned tag (`ubuntu-24.04`) so a future GitHub default change cannot silently swap the runner image under us.
- The system-package list for the glow backend (`libxkbcommon-dev`, `libxcb-render0-dev`, `libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libgl1-mesa-dev`) is required because `cargo test --all` compiles the full binary including the eframe entry point. Removing any of these breaks the test step on the Ubuntu image.
- `RUSTFLAGS: "-D warnings"` as an env var is acceptable as a belt-and-braces measure but is not required — the `cargo clippy -- -D warnings` argument is the contract.
- This workflow consumes the configuration delivered by LCV-002 (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`) without modifying it.
- Phase 9 multi-platform expansion (LCV-092) will add a matrix on top of this single-job workflow rather than rewriting it; keep the job shape extensible-by-matrix but do not pre-add the matrix in LCV-004.
