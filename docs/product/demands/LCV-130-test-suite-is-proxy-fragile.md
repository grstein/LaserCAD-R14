# LCV-130 — The test suite's isolation from the network depends on the absence of a proxy

- **Status**: Draft
- **Phase**: 12
- **Depends on**: none (LCV-124's own fix for the leak landed separately at `a1b37aa`; what it did not reach is AC 1)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

This demand exists because a **credential leak was reproduced**, not theorised.

LCV-124's transport tests pointed at `http://127.0.0.1:1` on the assumption that
a closed loopback port cannot be reached. It can.
`reqwest::blocking::Client::new()` sets `auto_sys_proxy: true`, and reqwest 0.12
has **no loopback bypass** — so with `HTTP_PROXY` set in the environment, the
request is handed to the proxy instead of being refused by the kernel. The
reviewer stood up a listener on the proxy address and captured a real `POST`
carrying `authorization: Bearer sk-test-DO-NOT-LEAK` and the full system prompt,
**while `cargo test` reported 11 passed, 0 failed**. A test that believed it was
touching nothing had sent a bearer token and the agent's whole system prompt to
a third party, and reported success.

That specific leak is closed inside LCV-124 (`a1b37aa`) by making the endpoint
*unparseable* rather than *unreachable*. **It is closed in one file, not two.**
`a1b37aa` fixed `tests/lcv124_command_line_routing.rs`, whose
`UNPARSEABLE_ENDPOINT` now carries the whole story in a doc comment. It did not
touch `src/agent/transport.rs::unreachable_endpoint_is_a_request_error`, which
still calls `chat_completion("http://127.0.0.1:1", DUMMY_KEY, …)` and therefore
still posts `authorization: Bearer sk-test-DO-NOT-LEAK` to whatever `HTTP_PROXY`
names. The key is a dummy and no system prompt rides along — this is not a
second incident — but it is the same defect, live in the tree, and it is the
concrete thing AC 1 has to remove. Its stated purpose ("an unreachable endpoint
is a `Request` error, not a panic") is why it cannot simply copy LCV-124's
unparseable string: an unparseable URL tests the builder path, not the connect
path. And it belongs **here, not in a reopened LCV-124**:
`git diff 96a8fb4~1 HEAD -- src/agent/transport.rs` comes back empty, so LCV-124
never opened the file. Holding it open for a defect in a file it never touched
would be scope creep, and the fix would land unreviewed against acceptance
criteria that say nothing about it. Confirmed independently by `reviewer-rust`.

### Two severities, and they must not be triaged together

`reviewer-rust` ran the suite at HEAD under a capturing proxy rather than
reasoning about it. The measured split:

| | requests out | tests failing | class |
|---|---|---|---|
| mockito-bound tests | 24 | **23** | **loud** — announces itself |
| `unreachable_endpoint_is_a_request_error` | 1 | **0** | **silent — leaks and passes** |
| total at HEAD | 25 | 23 | (was 29 / 23 at `96a8fb4`) |

That one row is the whole point of this demand's ordering. **The silent one is
the same failure mode LCV-124 had**: a test that believes it touches nothing,
sends a bearer token, and reports success. The 23 are an inconvenience.

Three things are left over, and they are this demand, in that order:

**1. One fixture leaks silently.**
`src/agent/transport.rs::unreachable_endpoint_is_a_request_error` — named above.
Its payload is bounded: `{"model":"m","messages":[{"role":"user","content":"x"}]}`
plus a synthetic literal key that exists only in test code. No system prompt, no
operator text, no real credential. The *payload* is why this is not an incident;
the *shape* is why it leads the acceptance criteria. A test that passes while
leaking teaches nobody anything, and the next one to take this shape may not
have a bounded payload.

**2. The suite does not run behind a proxy.** Under the same `HTTP_PROXY`, the
**23 mockito-backed tests** across `src/agent/transport.rs`,
`src/app/agent_turn.rs` and `tests/lcv123_agent_turn.rs::a_whole_turn_lands_on_the_bed`
fail loudly — every mockito server is a loopback socket and every request to one
gets posted to the proxy instead. Loud failure is the safe direction, so this
half is **not urgent**. What it costs is a session: the suite is simply not
runnable behind a corporate proxy, the failures look like agent bugs, and the
person who hits it will spend an afternoon before finding the environment
variable. The file-level inventory, by `mockito::Server::new()` construction
site — `src/agent/transport.rs` 11, `src/app/agent_turn.rs` 7,
`tests/lcv123_agent_turn.rs` 2 — is smaller than 23 because several sites sit in
loops and shared helpers that back more than one test.

**3. The rule underneath both was never written down.** "A test is isolated from
the network because the address is unreachable" is the assumption that failed,
and nothing in `AGENTS.md` forbids the next person making it. Unreachability is
a property of the *environment*, which a test does not control; unparseability
and a bound socket the test owns are properties of the *test*. That distinction
is what needs recording, because it generalises past reqwest and past the agent.

## Scope

- Make the mockito-backed tests reach their loopback servers regardless of the
  ambient proxy environment, **without touching the production client**.
- Record the rule in `AGENTS.md` §Implementation Rules: a test's isolation from
  the network must be a property the test controls, never a property of the
  environment.
- A guard that fails if a test reintroduces an "unreachable address" fixture.
- Add the proxy run to the repository's stated gate as a **periodic** check, not
  a per-demand one.

## Out of scope

- **`.no_proxy()` (or any proxy configuration) on the production transport.**
  Explicitly rejected — flagged by the reviewer and upheld here. It would break
  real operators behind corporate proxies, which is the exact population most
  likely to be running this on a work machine, and it trades a production
  regression for a test convenience. `src/agent/transport.rs::chat_completion`
  must keep reqwest's default proxy behaviour. A demand that proposes changing
  it is rejected on sight unless it comes with an ADR about how a proxied
  operator reaches OpenRouter.
- **Removing mockito, or rewriting the HTTP tests to not use a socket.** They
  earn their place: `a_whole_turn_lands_on_the_bed` is the only test that runs a
  real worker thread against a real socket, and the transport tests are the only
  ones that exercise the wire format end to end.
- **Re-fixing what `a1b37aa` already fixed.** `tests/lcv124_command_line_routing.rs`
  is done and its `UNPARSEABLE_ENDPOINT` doc comment is the record — leave both
  alone. AC 1 is about the one site that commit did not reach.
- **Making the suite pass with no network stack at all**, or in a sandbox that
  forbids `bind()`. Loopback binding is a requirement of the suite and stays one.
- **Anything about real endpoints in CI.** No CI test may reach one; that rule
  is unchanged and is not what this demand is about.

## Acceptance criteria

**Ordered by severity, not by convenience: AC 1 is the silent leak and it leads.
An implementer who runs out of room ships AC 1 to AC 3 and comes back.**

1. **The silent leak is closed.**
   `src/agent/transport.rs::unreachable_endpoint_is_a_request_error` sends no
   request to a proxy, and keeps testing exactly what it tests today — that a
   **connect** failure is `TransportError::Request` and not a panic. LCV-124's
   unparseable string is the wrong substitute: it fails in `Url::parse` on the
   builder path, so the test would stop testing its own subject. Two candidates,
   both endorsed by `reviewer-rust`; pick one and say why:
   - a **scoped `.no_proxy()` client built inside that one test**. Legitimate
     here and only here — it is test code in the one file allowed to import
     `reqwest`, it configures nothing in production, and AC 2's scan is bounded
     to the implementation section precisely so this stays possible;
   - a **`TcpListener` bound to `127.0.0.1:0`, its port read, then dropped**, so
     the port is closed *and* the closure is a fact the test owns rather than an
     assumption about the environment. Needs AC 4's mechanism to be proxy-proof.

   Verified by re-running AC 4's command and seeing this test's request gone
   from the proxy's log.

2. **The production client is untouched.** A bounded scan over
   `src/agent/transport.rs`'s implementation section asserts the needles
   `no_proxy` and `proxy` (built with `concat!`) appear nowhere in it. The fix
   is in the test environment or in test code, never in `chat_completion`. The
   bound at the bare `#[cfg(test)]` is load-bearing here, not boilerplate: it is
   what lets AC 1's first candidate exist.

3. **No test's isolation rests on an address being unreachable, and a guard
   says so in general terms.** A scan over `src/` and `tests/` finds no
   occurrence of `127.0.0.1:1` or any other "nothing-listens-here" endpoint
   literal used as a stand-in for "the network is off". **Follow the shape
   LCV-124 already shipped rather than inventing one**:
   `tests/lcv124_command_line_routing.rs::the_test_endpoint_cannot_reach_a_proxy`
   asserts `reqwest::Url::parse(UNPARSEABLE_ENDPOINT).is_err()` — a general
   invariant, not a blocklist, so planting `https://api.openai.com/v1` fails it
   exactly as restoring `http://127.0.0.1:1` would, and it carries a positive
   control proving the replaced spelling really was a valid URL. Whatever
   LCV-130 asserts must have that property: a sendable endpoint cannot quietly
   re-land. The scan lives in `tests/`, so it cannot match itself; needles built
   with `concat!`; paths rebuilt from `components()` joined with `/`; shown to
   discriminate.

4. **`cargo test --all --no-fail-fast` is green with `HTTP_PROXY`,
   `HTTPS_PROXY` and `ALL_PROXY` all set to an address nothing is listening on**
   (e.g. `http://127.0.0.1:9`). Same pass/fail set as with them unset. This is
   the loud half: 23 failures at HEAD, 0 after.

5. **The rule is written where the next person will read it.**
   `AGENTS.md` §Implementation Rules gains one entry, in the existing voice,
   saying: a test's isolation from the network must be a property the test
   controls — an unparseable URL, or a socket the test binds and owns — never a
   property of the environment such as a closed port or an unroutable address.
   It cites this demand and the captured `Bearer` as the witness, the way the
   path-comparison rule cites its two CI breakages. If `architect` would rather
   this were an ADR, that is their call — file the task rather than deciding it
   here.

6. **The proxy run is in the release checklist, not in every demand's gate.**
   The per-demand gate stays `fmt` + `clippy` + `cargo test --all
   --no-fail-fast`; adding a second full test run to every demand is not worth
   it for a loud failure. Instead the proxy run joins LCV-089's manual smoke
   list (owned by `demand-manager`) as a step before tagging.

## Expected tests

- **Integration / AC 1 and AC 4** — this is a *run*, not a test. Stand up a
  capturing listener, point all three proxy variables at it, and run
  `HTTP_PROXY=… HTTPS_PROXY=… ALL_PROXY=… cargo test --all --no-fail-fast`
  **before and after**, reporting both the pass/fail line and the **request
  count seen by the listener**. The expected numbers are in §Problem: 25 out /
  23 failing before, 0 out / 0 failing after. A run that reports failures going
  to zero without reporting requests going to zero has not checked AC 1 — the
  silent leak is invisible in the pass/fail line, which is the entire defect.
  Do not write a test that mutates the process environment to prove any of this:
  `std::env::set_var` is process-global and `cargo test` runs tests in parallel
  threads, so such a test would be a flake generator and would poison unrelated
  tests in the same binary.
- **Unit / AC 2** — the bounded transport scan, shown to discriminate.
- **Unit / AC 3** — the tree scan plus the general-invariant guard, each shown
  to discriminate: plant `https://api.openai.com/v1`, prove it goes red; plant
  `http://127.0.0.1:1`, prove it goes red; carry a positive control the way
  `the_test_endpoint_cannot_reach_a_proxy` does.
- **Unit / AC 1** — the existing assertion of
  `unreachable_endpoint_is_a_request_error` still holds: a closed port is
  `TransportError::Request`, not a panic, and not a builder error. If the chosen
  remedy cannot keep that, stop and report rather than weakening the assertion.
- **Manual / AC 5** — the `AGENTS.md` entry exists and reads in the file's
  voice.
- **[manual] smoke** — none beyond the two proxy runs above. This demand changes
  no user-visible behaviour.

## Test hygiene (mandatory)

- Bound every source scan to the implementation section (bare `#[cfg(test)]` at
  column 0) and build every needle with `concat!`. Canonical correct example:
  `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- **No test may mutate the process environment.** See §Expected tests.
- **No test may reach a real endpoint.**

## Open questions

1. **Which mechanism?** The leading candidate is a `[env]` entry in
   `.cargo/config.toml` setting `NO_PROXY` to cover loopback
   (`127.0.0.1,localhost,::1`), which cargo applies to `cargo test` and
   `cargo run` alike, costs no code, and is harmless in production — "do not
   proxy loopback" is what an operator would want anyway. Needs checking: that
   reqwest 0.12 honours `NO_PROXY` in `auto_sys_proxy` mode (it should), whether
   `force = true` or `force = false` is right (does a developer's own `NO_PROXY`
   win?), and whether `cargo nextest` and a bare `./target/debug/deps/...`
   invocation inherit it (they do not — is that acceptable?). Alternatives, if
   it does not hold: a `#[ctor]`-style one-time env scrub (rejected in advance —
   same global-mutation problem ruled out in §Expected tests), or teaching each mockito test to talk
   to its server over a raw `TcpStream` (a rewrite, and it would lose the wire
   coverage).
2. **Does the rule in AC 5 belong in `AGENTS.md` or in an ADR?** Product view:
   `AGENTS.md`, because it is a one-line implementation rule with a witness, and
   that file already carries three rules of exactly that shape. `architect`
   decides.

## Notes

- Origin: `reviewer-rust`, 2026-09-13, during the LCV-124 review, from a
  **reproduced** leak — a listener on the proxy address captured a real `POST`
  with `authorization: Bearer sk-test-DO-NOT-LEAK` and the full system prompt
  while the suite reported 11 passed, 0 failed.
- Priority: **split, and the body is ordered to match.** The loud half (AC 4,
  23 failing tests) is low priority — it announces itself and costs one
  afternoon, once. The silent half (AC 1, one request that leaks while the suite
  reports green) is the same failure mode as the original incident and leads the
  criteria, even though its payload is bounded to a synthetic key and the string
  `"x"`. Nothing an operator can do is affected either way. It is filed
  so the afternoon it costs is spent once, by someone who reads this file first.
- LCV-129 adds two loopback `TcpListener` fixtures reached through `reqwest`
  (its AC 3). They join this demand's inventory and are subject to the same fix;
  LCV-129's body already says so.
- Related: `src/agent/transport.rs::chat_completion`, `src/app/agent_turn.rs`
  (mockito section), `tests/lcv123_agent_turn.rs::a_whole_turn_lands_on_the_bed`,
  `AGENTS.md` §Implementation Rules, LCV-089 (manual smoke list),
  [ADR 0008](../../adr/0008-test-gates-run-with-no-fail-fast.md).
