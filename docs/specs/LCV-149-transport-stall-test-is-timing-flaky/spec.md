# LCV-149 - Transport stall test is timing-flaky

- **Status**: Done
- **Depends on**: none
- **Implementation**: 44d393f, 9599eb6, 211f3d6

## Problem

`src/agent/transport.rs::tests::ac3_both_halves_of_a_call_give_up_when_the_endpoint_stalls`
failed intermittently during the 2026-09-27 drive — about 1 in 5 full-suite
runs in one reviewer's measurement — but passes when run standalone. The
panic site was near the test's timing-budget assertion. A flaky test in the
authoritative local gate (CI is on billing hold) erodes gate integrity: a red
run cannot be trusted to mean the code is broken, and a green run cannot be
trusted to mean it is not.

## Scope

Diagnosed in `/design` (see `plan.md` §Diagnosis): a fixture race, not a
timing budget. `transport.rs::tests::timeout_case` writes its response head
before reading the request; under CPU contention hyper receives the head
before it has sent the request and fails with `UnexpectedMessage` instead of
timing out. The fix is test-only: the fixture reads the whole request first.

## Out of scope

- Any production change in `src/agent/transport.rs` (it is correct).
- Other fixtures: `owned_socket_closed_before_a_reply_is_a_request_error`
  never writes a reply, so it has no reply-before-read race.
- Timing budgets, `recv_timeout` values, dependencies, ADRs, CHANGELOG.

## Acceptance criteria

1. WHEN the full lib test binary runs under CPU contention (25 runs beside
   `3 × nproc` busy loops, fresh build in its own `CARGO_TARGET_DIR`) THE
   SYSTEM SHALL pass `ac3_both_halves_of_a_call_give_up_when_the_endpoint_stalls`
   in every run (0 failures).
2. WHEN the body-stall fixture answers THE SYSTEM SHALL have read the whole
   request first (head through the `Content-Length` body), and the test SHALL
   assert on the bytes read before the reply that they contain
   `POST /chat/completions` and the JSON body (`"model":"m"`).
3. WHILE either endpoint stall is exercised THE SYSTEM SHALL still end the
   call in `Timeout { secs: 0 }` with no sleep, bounded by
   `recv_timeout(5 s)`, as LCV-129 AC 3 requires.

## Expected tests

- AC 1: stress run recorded in `tasks.md` T3 (25 loaded runs, 0 failures).
- AC 2, AC 3: `ac3_both_halves…` extended to assert the received request
  bytes; both stall cases still assert `Timeout { secs: 0 }`.

## Open questions

- None. Reproduced under load (5 of 25 runs failed, always the body-stall
  case, hyper `UnexpectedMessage`); no broader pattern in the transport tests.

## Notes

Opened from the 2026-09-27 drive; standalone runs were always green.
