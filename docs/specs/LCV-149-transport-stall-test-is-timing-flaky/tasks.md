# LCV-149 — Tasks

- [x] T1 [AC2] [AC3] Test: `ac3_both_halves…` asserts the bytes the fixture read *before* writing its reply hold the full request (`POST /chat/completions`, `"model":"m"` body); fixture thread returns them. Red today: nothing is read before the write (files: src/agent/transport.rs)
- [x] T2 [AC1] [AC2] Fix fixture order: test-local `read_request` reads head + `Content-Length` body before `timeout_case` writes `reply`; doc the ordering rule (files: src/agent/transport.rs)
- [x] T3 [AC1] Verify: fresh build in own `CARGO_TARGET_DIR`, run the lib test binary 25× beside `3 × nproc` busy loops, 0 failures; record the result in this file (files: docs/specs/LCV-149-transport-stall-test-is-timing-flaky/tasks.md)

## T3 result (2026-09-27)

16 cores, 48 busy loops, fresh `cargo test --lib --no-run` in its own `CARGO_TARGET_DIR`.
Built on `0e2a05c` plus this branch's `transport.rs`: at `d374c97` the lib test binary does not
compile (LCV-143 red-test commit `6dde9ef`); `transport.rs` is otherwise identical.

- Fixed fixture: 2 × 25 runs, `ac3_both_halves…` ok 50/50, **0 failures**.
- Control (old fixture, same load): 1 of 25 runs failed with hyper `UnexpectedMessage`.
- Every run of both binaries also failed two unrelated, deterministic source scans at `0e2a05c`
  (`app::persist::tests::project_dirs_is_resolved_in_exactly_two_files` and
  `the_pathless_wrappers_have_no_definition_and_no_call_site` still expect `io/settings.rs`,
  moved to `io/settings_store.rs` by LCV-143). Not LCV-149 scope.
