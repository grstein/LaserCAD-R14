# LCV-149 — Tasks

- [x] T1 [AC2] [AC3] Test: `ac3_both_halves…` asserts the bytes the fixture read *before* writing its reply hold the full request (`POST /chat/completions`, `"model":"m"` body); fixture thread returns them. Red today: nothing is read before the write (files: src/agent/transport.rs)
- [x] T2 [AC1] [AC2] Fix fixture order: test-local `read_request` reads head + `Content-Length` body before `timeout_case` writes `reply`; doc the ordering rule (files: src/agent/transport.rs)
- [ ] T3 [AC1] Verify: fresh build in own `CARGO_TARGET_DIR`, run the lib test binary 25× beside `3 × nproc` busy loops, 0 failures; record the result in this file (files: docs/specs/LCV-149-transport-stall-test-is-timing-flaky/tasks.md)
