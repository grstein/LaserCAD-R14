# LCV-087 — Release profile tuning + binary strip

- **Status**: Ready
- **Phase**: 8
- **Depends on**: LCV-007
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust — binary size: `-rwxr-xr-x 8.4M target/release/lasercad` (Linux x86-64, stripped ELF, well within 20 MiB envelope)

## Problem

LaserCAD v2 ships as a single-binary desktop application. For laser-cutting operators,
responsiveness during geometry operations (snapping, live preview, undo/redo) directly
affects whether the tool is usable in a workshop setting. The current `[profile.release]`
in `Cargo.toml` uses `opt-level = "s"` (size-optimised codegen), which trades runtime
performance for a slightly smaller binary. For a desktop CAD application whose binary
size is dominated by egui's compiled assets regardless of opt-level, the right default is
`opt-level = 3` (maximum speed). All other release-profile keys — `lto = true`,
`codegen-units = 1`, `strip = true`, `panic = "abort"` — are already present and
correct; this demand only corrects the one key that is wrong and then confirms the
resulting binary meets the size envelope expected for an AppImage / .deb payload.

## Scope

- Change `opt-level = "s"` to `opt-level = 3` in the `[profile.release]` table of
  `Cargo.toml`.
- Verify that `cargo build --release` succeeds and that the stripped binary at
  `target/release/lasercad` is ≤ 20 MiB.
- Record the actual binary size (output of `ls -lh target/release/lasercad`) in the
  `Implementation:` line of this demand.

## Out of scope

- Adding or changing any other `[profile.*]` table (debug, bench, test, dev).
- Changing any dependency version or feature flag.
- Splitting the release profile into workspace-level overrides per crate.
- CI configuration (owned by LCV-004).
- AppImage / .deb packaging configuration (owned by LCV-085, LCV-086).
- Windows / macOS build verification (owned by LCV-090, LCV-091).

## Acceptance criteria

1. `Cargo.toml` contains exactly one `[profile.release]` table. That table contains
   `opt-level = 3` (integer, not the string `"s"` or `"z"`).
2. The `[profile.release]` table also contains all four additional keys with these exact
   values: `lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`. No
   other keys are present in the table.
3. `cargo build --release` exits 0 on Linux with the pinned toolchain (`rust-toolchain.toml`,
   Rust 1.88 stable).
4. The file `target/release/lasercad` exists, is executable, and its size as reported by
   `ls -lh` is ≤ 20 MiB.
5. `cargo test --all` continues to exit 0 after the `Cargo.toml` change (the profile
   change must not break the test suite).

## Expected tests

- **Unit / static (AC 1, AC 2)**: implementer reads the committed `Cargo.toml` and
  confirms the `[profile.release]` table character-for-character against the two criteria
  before pushing.
- **Build smoke (AC 3, AC 4)**: run `cargo build --release`; then run
  `ls -lh target/release/lasercad` and confirm exit 0 and size ≤ 20 MiB. Paste the
  `ls -lh` output line into the `Implementation:` metadata of this demand file.
- **Regression gate (AC 5)**: run `cargo test --all` after the `Cargo.toml` edit and
  confirm it exits 0. This is also enforced by `AGENTS.md` § Implementation Rules before
  any commit.

## Open questions

(none)

## Notes

- The existing `[profile.release]` was introduced during the Phase 0 scaffold with
  `opt-level = "s"`. That choice is appropriate for resource-constrained embedded targets
  or CLI tools where startup latency matters; it is not appropriate for an interactive
  egui desktop application where frame-render time and geometry-kernel throughput dominate
  the user experience.
- `opt-level = 3` with `lto = true` and `codegen-units = 1` enables whole-program
  link-time optimisation across all egui, wgpu/glow, and lasercad crates. Compile time
  in CI will be longer than a debug build; this is expected and acceptable for release
  builds.
- `strip = true` instructs the linker to remove debug symbols from the final ELF. The
  resulting binary is suitable for direct inclusion in an AppImage or `.deb` payload
  without a separate `strip` step in the packaging Makefile.
- `panic = "abort"` removes the stack-unwinding machinery. Combined with `strip = true`,
  this is one of the primary contributors to keeping the binary under the 20 MiB target.
- 20 MiB is a conservative upper bound. With the current dependency tree (egui 0.29,
  eframe 0.29, rfd 0.14, serde_json, roxmltree, reqwest+tokio), a stripped release
  binary on Linux x86-64 is expected to land in the 8–15 MiB range. If the binary
  exceeds 20 MiB, the implementer must flag this as a blocker and open an investigation
  issue before merging.
- This demand has no dependency on any Phase 4–7 demands because the profile change
  applies to whatever code is present at the time it is merged; it can land as soon as
  the bootstrap window (LCV-007) exists to give `cargo build --release` a compilable
  binary to measure.
