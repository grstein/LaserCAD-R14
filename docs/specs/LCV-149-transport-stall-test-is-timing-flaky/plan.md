# LCV-149 — Plan

## Diagnosis (done during /design)

Reproduced: the lib test binary run 25× with 48 busy-loop processes on 16 cores failed 5×, always
`ac3_both_halves_of_a_call_give_up_when_the_endpoint_stalls`, always the second case:

```
a header promising a body that never comes must be a Timeout, got Request(reqwest::Error {
kind: Request, source: hyper_util::client::legacy::Error(SendRequest, hyper::Error(UnexpectedMessage)) })
```

Unloaded: 15/15 green. So it is not the 250 ms budget, the 5 s `recv_timeout`, or the connect/request
window naming. It is a fixture race: `transport.rs::tests::timeout_case` writes the response head
right after `accept`, before reading the request. Under contention the head reaches hyper before
it has finished writing the request, and hyper rejects a response to a request not yet sent. The
production code (`chat_completion_with_timeout`, `request_error`) is correct; only the fixture lies.

## Acceptance criteria

Approved by the user and recorded in `spec.md` as AC 1 (0 failures in 25 loaded runs),
AC 2 (fixture reads the whole request before replying, asserted on the bytes) and AC 3
(both stalls still end in `Timeout { secs: 0 }`, no sleep, bounded by `recv_timeout(5 s)`).

## Approach

Test-only fix inside `src/agent/transport.rs`'s `#[cfg(test)]` module. `timeout_case` gets a small
helper `read_request(&mut TcpStream) -> Vec<u8>` that reads until `\r\n\r\n`, parses
`Content-Length`, reads that many body bytes, and returns them; the fixture calls it before
writing `reply`, then drains until hang-up as today. The fixture thread returns the bytes it read *before* writing;
the test asserts (AC 2) they contain `POST /chat/completions` and the JSON body (`"model":"m"`), so a
regression to reply-before-read is visible deterministically, not only under load. Doc comment of
`timeout_case` gains the ordering rule and why (hyper `UnexpectedMessage`).

## Touches

- `src/agent/transport.rs::tests::timeout_case` (+ new test-local `read_request`) — fixture order.
- `src/agent/transport.rs::tests::ac3_both_halves_of_a_call_give_up_when_the_endpoint_stalls` —
  asserts on the received request.
- No production code, no dependency, no ADR. Not user-visible: no CHANGELOG line.

## Risks

- Broader pattern (spec open question): the only other hand-rolled fixture,
  `tests::owned_socket_closed_before_a_reply_is_a_request_error`, never writes a reply, so it has
  no reply-before-read race; in 40 loaded runs nothing else in the lib binary failed. Out of scope.
- LOC cap: change is below `#[cfg(test)]`; implementation LOC stays 251.
- Mutation testing: no — `src/agent/` is high-risk, but no production line changes.
- Stress verification must use its own `CARGO_TARGET_DIR` and a fresh build (gate integrity).
