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
concrete thing AC 1 has to remove.

### What that fixture is actually worth, measured

Its stated purpose is *"an unreachable endpoint is a `Request` error, not a
panic"*, and the first reading of that was that it therefore cannot copy
LCV-124's unparseable string, because an unparseable URL fails on the **builder**
path rather than the **connect** path. Read at HEAD, the code under test does not
make that distinction. Every `reqwest::Error` a bounded call can raise — from
`Client::builder().build()`, from `.send()` and from `.text()` alike — goes
through `src/agent/transport.rs::request_error`, which asks exactly **one**
question, `is_timeout()`, and returns `TransportError::Request(error)` for
everything else. A refused socket and a rejected URL land on the same arm.

Two consequences, and they set the shape of AC 1. The unparseable endpoint
**does** keep the mapping covered, so replacing the address costs no coverage
today. And the socket-level half is still worth keeping — from a socket the test
**binds and owns**, not from an address nobody owns — so that the arm cannot be
split later (an `is_connect()` branch is a plausible future change) without a
test noticing. AC 1 asks for both, and §Expected tests carries the mutation that
proves the second is not a duplicate of the first.

This all belongs **here, not in a reopened LCV-124**:
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
and nothing in `AGENTS.md` forbids the next person making it. **Unreachability is
not isolation.** It is a property of the *environment*, which a test does not
control: behind a proxy, on a captive portal, or on a CI runner with a transparent
redirect, the connection is attempted anyway, the bytes leave the machine, and —
this is the part that matters — **the test still passes**. Unparseability, and a
socket the test binds and owns, are properties of the *test*. That distinction is
what needs recording, because it generalises past reqwest and past the agent.

## Scope

- **Remove the one fixture that points the transport at an address no test
  owns**, keeping both claims it makes: the error mapping, and the socket-level
  failure — the latter against a socket the test binds itself.
- **Check in a `.cargo/config.toml` `[env]` entry** so the suite's loopback
  fixtures stop consulting the ambient proxy, **without touching the production
  client**.
- **Two guards** so a sendable endpoint cannot quietly re-land: the per-file
  `Url::parse(..).is_err()` invariant LCV-124 already ships, extended to the
  transport's own test module, and one tree-wide scan for loopback URL literals.
- Record the rule in `AGENTS.md` §Implementation Rules: a test's isolation from
  the network must be a property the test controls, never a property of the
  environment.
- Add the proxy run to the repository's stated gate as a **periodic** check, not
  a per-demand one — by filing the task, not by editing the checklist here.

## Out of scope

- **`.no_proxy()`, anywhere — production *or* test-scoped.** Evaluated and
  **rejected**; do not resurrect it in either form. On the production transport
  it would break real operators behind corporate proxies, which is the exact
  population most likely to be running this on a work machine, and it trades a
  production regression for a test convenience.
  `src/agent/transport.rs::chat_completion` keeps reqwest's default proxy
  behaviour. A demand that proposes changing it is rejected on sight unless it
  comes with an ADR about how a proxied operator reaches OpenRouter. A
  *test-scoped* `.no_proxy()` client was the second candidate in an earlier draft
  of AC 1 and is withdrawn: it puts proxy configuration into the one file the
  purity rules are trying to keep proxy-agnostic, and it fixes exactly one test
  while AC 2 fixes the whole class for less code.
- **Changing `request_error`'s mapping**, or adding an `is_connect()` branch to
  it. §Problem measures what it does today; this demand tests that behaviour, it
  does not change it. The `is_connect()` branch appears in §Expected tests only
  as a mutation, applied and reverted.
- **Removing mockito, or rewriting the HTTP tests to not use a socket.** They
  earn their place: `a_whole_turn_lands_on_the_bed` is the only test that runs a
  real worker thread against a real socket, and the transport tests are the only
  ones that exercise the wire format end to end.
- **Re-fixing what `a1b37aa` already fixed.** `tests/lcv124_command_line_routing.rs`
  is done and its `UNPARSEABLE_ENDPOINT` doc comment is the record — leave both
  alone. AC 1 is about the one site that commit did not reach.
- **Making the suite pass with no network stack at all**, or in a sandbox that
  forbids `bind()`. Loopback binding is a requirement of the suite and stays one.
- **Making the suite proxy-proof under runners `.cargo/config.toml` does not
  reach** — `cargo nextest`, or a bare `./target/debug/deps/<binary>`
  invocation. AC 2 asks for those two answers to be *measured and written down*,
  not for them to be fixed. Neither is this repository's stated gate.
- **A checked-in proxy-test script, harness or fixture server.** The run in
  §Expected tests is done with a throwaway listener outside the repository. No
  new file under `scripts/`, no new dev-dependency.
- **Any change to the production default endpoint** (`src/io/settings.rs`), or to
  what the settings UI paints for it.
- **Anything about real endpoints in CI.** No CI test may reach one; that rule is
  unchanged and is not what this demand is about. **CI is on a billing hold and
  the local gate is the acceptance gate** — no criterion here depends on a CI run.

## Acceptance criteria

**Ordered by severity, not by convenience.** AC 1 is the silent leak and it
leads; **AC 2 lands with it**, because AC 1's socket half needs AC 2's entry to
be proxy-proof (AC 1's unparseable half needs nothing and is safe under any
environment). AC 5's run is the only evidence that either worked, so it is not
deferrable. AC 6 and AC 7 are the tail and may land in a second commit.

1. **The silent leak is closed, and both of its claims survive.**
   `src/agent/transport.rs::unreachable_endpoint_is_a_request_error` no longer
   names `http://127.0.0.1:1`, or any other address the test does not own. It is
   replaced by **two** tests in the same module:

   - **(a) the unsendable endpoint.** A module constant that `reqwest::Url::parse`
     rejects — use LCV-124's spelling, `"not-a-url"`, so the tree has one
     vocabulary for this — passed to `chat_completion`, asserting
     `TransportError::Request(_)` and not a panic, and keeping the existing
     `assert_no_key` on the rendered message. **No socket is opened and no proxy
     is consulted, by construction, in any environment.** The constant carries a
     doc comment in `UNPARSEABLE_ENDPOINT`'s voice, naming the leak and why the
     address it replaced was not isolation.
   - **(b) the socket the test owns.** A `TcpListener` bound to `127.0.0.1:0`
     that accepts one connection and drops it without writing a byte; the call
     against it asserts `TransportError::Request(_)` and **not**
     `TransportError::Timeout`. Same fixture shape as
     `src/agent/transport.rs::timeout_case` in the same file: one thread, no
     sleep, no new dependency, and the thread ends on its own. Either spelling of
     the underlying failure — connection closed before the response, or reset by
     peer — satisfies the assertion, which is on the variant and never on the
     message text.

   Neither test may use `.no_proxy()` (§Out of scope). **A bound-then-dropped
   port is not an acceptable substitute for (b)**: the port can be handed to an
   unrelated process between the drop and the connect, which is a flake and, worse,
   a connection to something else on the developer's machine.

2. **The suite's loopback fixtures stop consulting the ambient proxy.** A new,
   checked-in `.cargo/config.toml`:

   ```toml
   [env]
   NO_PROXY = { value = "127.0.0.1,localhost,::1", force = false }
   ```

   with a comment naming this demand and the reason, not just the value.
   `force = false` is deliberate and is part of the criterion: a developer who
   already sets `NO_PROXY` owns their own bypass list and cargo must not clobber
   it — this entry also reaches `cargo run`, which starts the real app. If their
   list does not cover loopback, the suite fails **loudly** (the 23), which is the
   safe direction and is the whole reason this half is not urgent.

   **Measured, not asserted.** AC 5's run is the evidence that reqwest 0.12
   honours `NO_PROXY` while `auto_sys_proxy` is on. If it does not, **stop and
   report**: the alternatives in §Notes are named, not pre-approved. The handover
   additionally records two answers as measured lines, and they go in the file's
   comment: whether `cargo nextest run` inherits the entry (if `nextest` is not
   installed, say so — do not install it), and whether a bare
   `./target/debug/deps/<binary>` invocation does (it will not).

3. **The production client is untouched.** A bounded scan over
   `src/agent/transport.rs`'s implementation section asserts the needles
   `no_proxy` and `proxy` (built with `concat!`) appear nowhere in it. The bound
   at the bare `#[cfg(test)]` at column 0 is load-bearing here, not boilerplate.
   `git diff` over `src/agent/transport.rs` touches the `#[cfg(test)] mod tests`
   block and nothing else — in particular `request_error`, `chat_completion` and
   `chat_completion_with_timeout` are byte-identical to their parent-commit state.

4. **A sendable endpoint cannot quietly re-land.** Two guards, each shown to
   discriminate:

   - **(a) per file.** Every module that configures a transport endpoint from a
     constant proves, in the same file, that
     `reqwest::Url::parse(THE_CONSTANT).is_err()` — the shape
     `tests/lcv124_command_line_routing.rs::the_test_endpoint_cannot_reach_a_proxy`
     already ships, including its positive control that the spelling it replaced
     *was* a valid URL. After AC 1 those files are `tests/lcv124_command_line_routing.rs`
     and `tests/lcv129_agent_timeout_and_cancel.rs` (both already carry it) and
     `src/agent/transport.rs` (new, for AC 1a's constant).
   - **(b) tree-wide.** One scan over `src/` and `tests/`: **no string literal on
     a code line parses as an `http`/`https` URL whose host is a loopback or
     unspecified address** — anything in `127.0.0.0/8`, `::1`, `0.0.0.0`, or the
     domain `localhost`. A general invariant, not a blocklist: it flags
     `http://127.0.0.1:1`, `http://localhost:8080` and `http://127.0.0.2:9`
     alike, and it leaves alone every legitimate spelling in the tree today —
     `TcpListener::bind("127.0.0.1:0")` (not a URL), the template literal in
     `format!("http://{}", addr)` (whatever `Url::parse` makes of a `{}` host,
     it is not a loopback address), `server.url()` (not a literal), every
     `http://www.w3.org/2000/svg` namespace literal under `src/io/svg/` and
     `tests/lcv057_svg_import.rs`, and the production
     `https://openrouter.ai/api/v1` default.

   **State the guard's limit in its own doc comment rather than overclaiming:**
   (b) is a claim about *loopback URL literals*, not about every sendable
   endpoint. A planted `https://api.openai.com/v1` is caught by (a) wherever a
   constant carries it, and by nothing in (b). Saying so is the criterion; a doc
   comment that implies more than the code checks is the failure mode this
   repository keeps paying for.

   §Test hygiene binds both: the scan lives in `tests/`, so it scans itself, and
   neither its needles nor its control fixture may leave a whole offending literal
   in its own **source text** — which is where `concat!` earns its keep, and is
   why `tests/lcv124_command_line_routing.rs`'s
   `concat!("http://127.0.0.", "1:1")` control survives the scan (neither piece
   parses as a loopback URL). Paths rebuilt from `components()` joined with `/`.

5. **`cargo test --all --no-fail-fast` is green behind a proxy, and nothing
   leaves the machine.** Run with `HTTP_PROXY`, `HTTPS_PROXY` and `ALL_PROXY` all
   pointed at a **capturing** listener, before and after the change, reporting
   both the pass/fail line and the **request count the listener saw**. Expected:
   **25 out / 23 failing** before, **0 out / 0 failing** after. The same pass/fail
   set as with the three variables unset. A run that reports failures going to
   zero without reporting requests going to zero **has not checked AC 1** — the
   silent leak is invisible in the pass/fail line, which is the entire defect.

6. **The rule is written where the next person will read it.**
   `AGENTS.md` §Implementation Rules gains one entry, in the existing voice, with
   three clauses:
   - a test that must **not** reach the network makes its endpoint
     **unparseable** — a property of the test, true in every environment;
   - a test that must reach a loopback fixture **binds and owns that socket**,
     and the repository's `.cargo/config.toml` `NO_PROXY` entry is what keeps the
     ambient environment from redirecting it; when that entry is missing or
     ignored the failure is **loud**, never silent;
   - never *"this address is unreachable, so nothing happens"*: unreachability is
     a property of the environment, the test does not control it, and behind a
     proxy the request is sent anyway **and the test still passes**.

   It cites this demand and the captured `Bearer` as the witness, the way the
   path-comparison rule cites its two CI breakages, and it cites symbols, never
   line numbers (§Documentation Hygiene). If `architect` would rather this were an
   ADR, that is their call — **file the task, do not decide it here**.

7. **The proxy run is on the release checklist, not in every demand's gate.** The
   per-demand gate stays `fmt` + `clippy` + `cargo test --all --no-fail-fast`;
   adding a second full test run to every demand is not worth it for a loud
   failure. The implementer **opens a task for `demand-manager`** to add one step
   to LCV-089's manual smoke list and writes **the exact text to insert** in the
   handover — naming the command and the two numbers to report. It does not edit
   LCV-089 or `docs/product/backlog.md` itself.

## Expected tests

- **Integration / AC 1, AC 2, AC 5 — the proxy run.** This is a *run*, not a
  test. Stand up a capturing listener outside the repository (a few lines of
  Python, a `nc` loop, a scratch binary — not a checked-in file), point all three
  proxy variables at it, and run
  `HTTP_PROXY=… HTTPS_PROXY=… ALL_PROXY=… cargo test --all --no-fail-fast`
  **before and after**, reporting the pass/fail line **and** the request count the
  listener saw, both times. Do the "after" run early: it is the only thing that
  can tell you whether AC 2's mechanism works at all, and if it does not, the
  right move is to stop and report rather than to reach for an alternative.
  **Do not write a test that mutates the process environment to prove any of
  this:** `std::env::set_var` is process-global and `cargo test` runs tests in
  parallel threads, so such a test would be a flake generator and would poison
  unrelated tests in the same binary.
- **Unit / AC 1a — the constant is unsendable.** *Mutation:* change it to
  `http://127.0.0.1:1`; AC 4a's guard in that file goes red, and AC 4b's tree
  scan goes red naming `src/agent/transport.rs`. Two reds from one mutation is
  the point — the per-file guard and the tree scan overlap here deliberately.
  Revert.
- **Unit / AC 1b — the owned socket binds, and is not a duplicate of (a).**
  *Mutation:* add an `is_connect()` branch to `request_error` —
  `if error.is_connect() { return TransportError::Timeout { secs: connect.as_secs() }; }`
  — and confirm **(b) goes red while (a) stays green**. That single mutation is
  the entire justification for (b) existing. If (b) does **not** go red, (b) is a
  duplicate of (a): report that, and say so, rather than keeping a test for
  appearance. Revert.
- **Unit / AC 3 — the bounded transport scan**, shown to discriminate: each
  needle first looked up in a witness line, through the same helper that does the
  real work, so a misspelt `concat!` fails against the witness before the
  expected-empty haystack is consulted.
- **Unit / AC 4b — the tree scan, both directions.** *Mutations, one at a time:*
  plant `http://127.0.0.1:1` on a code line, prove red; plant
  `http://localhost:3000`, prove red. *False-positive control, not optional:* the
  scan stays green on `https://openrouter.ai/api/v1`, on
  `http://www.w3.org/2000/svg`, on `"127.0.0.1:0"` and on `"http://{}"` — a scan
  that flagged those would be weakened until it flagged nothing, which is how a
  guard dies. Run the control fixture through the **same** extractor the real scan
  uses; a control that takes a different path proves nothing.
- **Unit / AC 1 — the original assertion still holds.** A socket-level failure is
  `TransportError::Request`, not a panic and not a `Timeout`. If the chosen
  remedy cannot keep that, **stop and report** rather than weakening the
  assertion.
- **Manual / AC 2** — the two inheritance answers (`cargo nextest`, bare deps
  binary), measured and quoted into the config file's comment.
- **Manual / AC 6** — the `AGENTS.md` entry exists, carries all three clauses,
  and reads in the file's voice; the `architect` task exists if one was filed.
- **Manual / AC 7** — the `demand-manager` task exists and the handover carries
  the exact checklist text.
- **[manual] smoke** — none beyond the two proxy runs above. This demand changes
  no user-visible behaviour. `git diff` over `src/` outside
  `src/agent/transport.rs`'s test module is empty, and `Cargo.toml` / `Cargo.lock`
  are unchanged.

## Test hygiene (mandatory)

- Bound every source scan to the implementation section (bare `#[cfg(test)]` at
  column 0) and build every needle with `concat!`. Canonical correct example:
  `guard_is_runtime_not_cfg` in `src/io/dialogs.rs`.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- **Every absence assertion carries a positive control run through the same
  helper.** An absence assertion over an empty or mis-sliced haystack passes for
  the wrong reason, silently.
- **No test may mutate the process environment.** See §Expected tests.
- **No test binds a fixed port.** `127.0.0.1:0` and read the assigned address;
  every fixture thread ends on its own, with no sleep anywhere.
- **No test may reach a real endpoint.**

## Open questions

None. Both are closed in §Notes — the mechanism is AC 2's `.cargo/config.toml`
entry, validated by AC 5's run rather than by assertion; the rule goes in
`AGENTS.md`, with `architect` free to promote it to an ADR through a filed task.

## Notes

- Origin: `reviewer-rust`, 2026-09-13, during the LCV-124 review, from a
  **reproduced** leak — a listener on the proxy address captured a real `POST`
  with `authorization: Bearer sk-test-DO-NOT-LEAK` and the full system prompt
  while the suite reported 11 passed, 0 failed.
- **Closed: which mechanism (was open question 1).** The answer is AC 2's
  `[env]` entry in `.cargo/config.toml`. It costs no code, it is checked in and
  reviewable, cargo applies it to `cargo test` and `cargo run` alike, and "do not
  proxy loopback" is what an operator would want anyway — a laser operator
  pointing the agent at a local OpenAI-compatible server on `127.0.0.1` is a real
  configuration, and nothing here makes it worse. What it is **not** is a proof:
  whether reqwest 0.12 honours `NO_PROXY` under `auto_sys_proxy` is decided by
  AC 5's run, not by this paragraph. Rejected alternatives, named so they are not
  re-proposed: a `#[ctor]`-style one-time environment scrub (the same
  process-global mutation ruled out in §Expected tests); teaching each mockito
  test to talk to its server over a raw `TcpStream` (a rewrite that loses the wire
  coverage those tests exist for); and `.no_proxy()` in any form (§Out of scope).
- **Why an environment-provided mechanism does not contradict the rule it writes
  down.** AC 6's rule forbids resting *isolation* on the environment. AC 2 is not
  an isolation claim: a mockito test, or AC 1's owned listener, is *supposed* to
  reach a socket, and that socket is a fact the test created. What AC 2 removes is
  an ambient redirect between the test and its own socket. The distinction that
  makes this safe is the failure mode, and it is worth stating in the config
  file's comment: when this entry is missing or ignored, **23 tests go red**.
  Nothing passes while leaking. The rule exists because the other shape —
  "unreachable, therefore isolated" — fails the opposite way: it leaks and
  reports success.
- **Non-normative sketch for AC 4b.** Reading string literals out of source does
  not need a parser: for each code line, take each run between double quotes, hand
  it to `reqwest::Url::parse`, and judge only the ones that parse — `Host::Ipv4` /
  `Host::Ipv6` answering `is_loopback()` or `is_unspecified()`, or
  `Host::Domain("localhost")`. Raw strings need no special handling: the pieces
  a naive split yields out of `r#"<svg xmlns="http://www.w3.org/2000/svg"…"#` are
  `<svg xmlns=` (does not parse) and `http://www.w3.org/2000/svg` (parses, not
  loopback). The `{}` in a `format!` template is the one case to check rather
  than assume: judge only literals that parse, and it falls out either way. That is guidance, not a criterion; what the criterion requires is the
  claim in AC 4b and the four mutations in §Expected tests.
- **The `request_error` reading is a measurement, and it will rot like any other.**
  Re-read at HEAD by `product-owner` while refining this demand: one `is_timeout()`
  question, everything else to `TransportError::Request`. AC 1b's mutation is what
  keeps that reading honest after this demand ships — if someone splits the arm,
  (b) goes red and says so.
- Priority: **split, and the body is ordered to match.** The loud half (AC 5,
  23 failing tests) is low priority — it announces itself and costs one afternoon,
  once. The silent half (AC 1, one request that leaks while the suite reports
  green) is the same failure mode as the original incident and leads the criteria,
  even though its payload is bounded to a synthetic key and the string `"x"`.
  Nothing an operator can do is affected either way. It is filed so the afternoon
  it costs is spent once, by someone who reads this file first.
- LCV-129 adds two loopback `TcpListener` fixtures reached through `reqwest`
  (its AC 3), and AC 1b adds a third of the same shape. They are all in AC 2's
  class: sockets their tests own, kept proxy-free by the config entry, loud when
  it is not there.
- Related: `src/agent/transport.rs::chat_completion`,
  `src/agent/transport.rs::request_error`,
  `src/agent/transport.rs::timeout_case` (the fixture shape AC 1b copies),
  `src/app/agent_turn.rs` (mockito section),
  `tests/lcv123_agent_turn.rs::a_whole_turn_lands_on_the_bed`,
  `tests/lcv124_command_line_routing.rs::the_test_endpoint_cannot_reach_a_proxy`
  (the guard AC 4a copies), `AGENTS.md` §Implementation Rules, LCV-089 (manual
  smoke list), [ADR 0008](../../adr/0008-test-gates-run-with-no-fail-fast.md).
