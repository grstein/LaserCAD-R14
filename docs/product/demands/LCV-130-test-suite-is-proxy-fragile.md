# LCV-130 — The test suite's isolation from the network depends on the absence of a proxy

- **Status**: Draft
- **Phase**: 12
- **Depends on**: none (LCV-124's own fix for the leak landed separately at `a1b37aa`; what it did not reach is AC 3)
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
concrete thing AC 3 has to remove. Its stated purpose ("an unreachable endpoint
is a `Request` error, not a panic") is why it cannot simply copy LCV-124's
unparseable string: an unparseable URL tests the builder path, not the connect
path. Three things are left over, and they are this demand:

**1. The suite does not run behind a proxy.** Under the same `HTTP_PROXY`, the
**23 mockito-backed tests** across `src/agent/transport.rs`,
`src/app/agent_turn.rs` and `tests/lcv123_agent_turn.rs::a_whole_turn_lands_on_the_bed`
fail loudly — every mockito server is a loopback socket and every request to one
gets posted to the proxy instead. Loud failure is the safe direction, so this is
**not urgent**. What it costs is a session: the suite is simply not runnable
behind a corporate proxy, the failures look like agent bugs, and the person who
hits it will spend an afternoon before finding the environment variable. The
file-level inventory, by `mockito::Server::new()` construction site —
`src/agent/transport.rs` 11, `src/app/agent_turn.rs` 7,
`tests/lcv123_agent_turn.rs` 2 — is smaller than 23 because several sites sit in
loops and shared helpers that back more than one test.

**2. One leaking fixture survives.** Named above:
`src/agent/transport.rs::unreachable_endpoint_is_a_request_error`.

**3. The rule underneath it was never written down.** "A test is isolated from
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
  alone. AC 3 is about the one site that commit did not reach.
- **Making the suite pass with no network stack at all**, or in a sandbox that
  forbids `bind()`. Loopback binding is a requirement of the suite and stays one.
- **Anything about real endpoints in CI.** No CI test may reach one; that rule
  is unchanged and is not what this demand is about.

## Acceptance criteria

1. **`cargo test --all --no-fail-fast` is green with `HTTP_PROXY`,
   `HTTPS_PROXY` and `ALL_PROXY` all set to an address nothing is listening on**
   (e.g. `http://127.0.0.1:9`). Same pass/fail set as with them unset. This is
   the criterion; everything below serves it.

2. **The production client is untouched.** A bounded scan over
   `src/agent/transport.rs`'s implementation section asserts the needles
   `no_proxy` and `proxy` (built with `concat!`) appear nowhere in it. The fix
   is in the test environment, never in the transport.

3. **No test depends on an address being unreachable.**
   `src/agent/transport.rs::unreachable_endpoint_is_a_request_error` stops
   using `http://127.0.0.1:1` while keeping what it actually tests — that a
   connect failure is `TransportError::Request` and not a panic. It must still
   exercise the **connect** path, so LCV-124's unparseable string is the wrong
   substitute (that is the builder path). The likely shape is a
   `TcpListener` bound to `127.0.0.1:0`, its port read, and the listener
   dropped, so the port is closed *and* the test owns the fact — which only
   works once AC 1's mechanism is in place, which is why the two ship together.
   Then a scan over `src/` and `tests/` finds no occurrence of `127.0.0.1:1`
   or any other "nothing-listens-here" endpoint literal used as a stand-in for
   "the network is off". The scan lives in `tests/`, so it cannot match itself;
   needles built with `concat!`; paths rebuilt from `components()` joined with
   `/`; shown to discriminate.

4. **The rule is written where the next person will read it.**
   `AGENTS.md` §Implementation Rules gains one entry, in the existing voice,
   saying: a test's isolation from the network must be a property the test
   controls — an unparseable URL, or a socket the test binds and owns — never a
   property of the environment such as a closed port or an unroutable address.
   It cites this demand and the captured `Bearer` as the witness, the way the
   path-comparison rule cites its two CI breakages. If `architect` would rather
   this were an ADR, that is their call — file the task rather than deciding it
   here.

5. **The proxy run is in the release checklist, not in every demand's gate.**
   The per-demand gate stays `fmt` + `clippy` + `cargo test --all
   --no-fail-fast`; adding a second full test run to every demand is not worth
   it for a loud failure. Instead the proxy run joins LCV-089's manual smoke
   list (owned by `demand-manager`) as a step before tagging.

## Expected tests

- **Integration / AC 1** — this is a *run*, not a test: the implementer runs
  `HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 cargo test --all --no-fail-fast`
  and reports the full pass/fail line, before and after the fix. Do not write a
  test that mutates the process environment to prove this: `std::env::set_var`
  is process-global and `cargo test` runs tests in parallel threads, so such a
  test would be a flake generator and would poison unrelated tests in the same
  binary.
- **Unit / AC 2** — the bounded transport scan, shown to discriminate.
- **Unit / AC 3** — the tree scan, shown to discriminate (add the literal to a
  scratch file and prove it goes red).
- **Manual / AC 4** — the `AGENTS.md` entry exists and reads in the file's
  voice.
- **[manual] smoke** — none beyond AC 1's two runs. This demand changes no
  user-visible behaviour.

## Test hygiene (mandatory)

- Bound every source scan to the implementation section (bare `#[cfg(test)]` at
  column 0) and build every needle with `concat!`. Canonical correct example:
  `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- **No test may mutate the process environment.** See AC 1.
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
   same global-mutation problem as AC 1), or teaching each mockito test to talk
   to its server over a raw `TcpStream` (a rewrite, and it would lose the wire
   coverage).
2. **Does the rule in AC 4 belong in `AGENTS.md` or in an ADR?** Product view:
   `AGENTS.md`, because it is a one-line implementation rule with a witness, and
   that file already carries three rules of exactly that shape. `architect`
   decides.

## Notes

- Origin: `reviewer-rust`, 2026-09-13, during the LCV-124 review, from a
  **reproduced** leak — a listener on the proxy address captured a real `POST`
  with `authorization: Bearer sk-test-DO-NOT-LEAK` and the full system prompt
  while the suite reported 11 passed, 0 failed.
- Priority: **low and explicitly so.** The failure mode is loud, the incident
  half is fixed in LCV-124 (`a1b37aa`), the fixture that survives carries a
  dummy key and no system prompt, and nothing an operator can do is affected. It is filed
  so the afternoon it costs is spent once, by someone who reads this file first.
- LCV-129 adds two loopback `TcpListener` fixtures reached through `reqwest`
  (its AC 3). They join this demand's inventory and are subject to the same fix;
  LCV-129's body already says so.
- Related: `src/agent/transport.rs::chat_completion`, `src/app/agent_turn.rs`
  (mockito section), `tests/lcv123_agent_turn.rs::a_whole_turn_lands_on_the_bed`,
  `AGENTS.md` §Implementation Rules, LCV-089 (manual smoke list),
  [ADR 0008](../../adr/0008-test-gates-run-with-no-fail-fast.md).
