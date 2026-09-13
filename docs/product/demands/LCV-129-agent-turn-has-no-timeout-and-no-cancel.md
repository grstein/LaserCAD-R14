# LCV-129 — A hung endpoint wedges the app: the agent turn has no timeout and no cancel

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-123 (the live turn a cancel must end through), LCV-125 (file-level: both edit `src/agent/panel.rs`, and LCV-125 closes the transcript role vocabulary this demand must not widen — LCV-125 lands first)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`src/agent/transport.rs::chat_completion` builds its HTTP client as
`reqwest::blocking::Client::new()` with **no `.timeout(...)`**. reqwest's
default is unbounded, so a slow, black-holed or unresponsive endpoint blocks the
worker thread for as long as the socket stays open — which can be forever.

The consequence chain is what makes this a demand rather than a footnote:

1. the worker blocks in `chat_completion` and never sends a terminal event;
2. so `src/app/agent_poll.rs::end_turn` is never reached;
3. so `agent_busy` is never cleared — `end_turn` is the only place in the
   program that writes `agent_busy = false` or clears `agent_rx`;
4. so `src/app/mod.rs`, which requests a repaint on every frame while
   `agent_busy` is set, repaints for the rest of the session.

That is **LCV-120 reopening through a different door**, days after LCV-120
closed. It is exactly the latch ADR 0007 §D11 warns about: a repaint condition
that can no longer fall is an unconditional repaint wearing a guard.

The insult on top of the injury is that the operator cannot even start over.
LCV-124 AC 10 refuses a second prompt while `agent_busy` is true, and there is
no cancel affordance anywhere in the agent panel, so the only recovery is
quitting the application — losing unsaved work unless autosave happened to fire.

**Pre-existing.** The transport has been built this way since LCV-077. But
LCV-123 and LCV-124 are the commits that put a real operator in front of a real
endpoint, and the next thing that happens after Marco 2 closes is the first live
run against OpenRouter. That is the moment a network timeout stops being
hypothetical.

## One demand, not two

A timeout and a cancel are different mechanisms — one bounds a hung endpoint,
the other bounds a *slow but working* one and an operator who changed their
mind — and the question of splitting them was asked explicitly. They ship
**together, in one demand**, for three reasons:

1. **They share one invariant and it is the risky part.** ADR 0007 §D11 states
   a closure property: `end_turn` is the only writer of `agent_busy = false`,
   and every terminal path is a tail call to it. Both halves have to be argued
   against that property, and both are tempted to break it the same way (a
   timeout that clears the flag from the transport side; a cancel that clears it
   from the panel side). Split into two demands, that argument is made twice and
   the second one re-litigates the first.
2. **Neither half closes the defect alone.** Timeout-only leaves the operator
   watching a spinner for the whole window with no way out. Cancel-only frees
   the UI but leaks a worker thread on a wedged socket that nothing will ever
   end, and requires the operator to be present and to guess that the endpoint
   is dead.
3. **They share a manual smoke.** The fixture that proves the timeout works — an
   endpoint that accepts and never answers — is the same fixture that proves
   cancel works, and you cannot exercise either one without standing it up.

What is *not* bundled: a per-turn deadline, a retry policy, streaming, and a
Settings field for the duration. See §Out of scope.

## The timeout value, and why it is a constant rather than a setting

**The values: 120 s total per HTTP call, 10 s to connect.** Both named
constants in `src/agent/transport.rs`. The reasoning, written down so it can be
argued with later rather than re-guessed:

- **The unit is one HTTP call, not one turn.** `chat_completion` is one round
  trip; a turn is up to `step_budget` of them (default 12, max 32). Per-call is
  where the client is built, so it is the only place the value can be applied
  without threading a deadline through `loop_.rs`. The turn as a whole is
  already bounded — by the step budget — so a second, per-turn deadline would
  be a second bound on the same thing.
- **Nothing is streamed, so time-to-first-byte is generation time.** The
  transport reads the whole response before returning. A reasoning model asked
  for a long tool-call sequence can legitimately spend a minute or more before
  the first byte exists. A 30 s timeout would turn a working feature into a
  flaky one, and the failure would look exactly like a broken endpoint.
- **Cancel is what handles impatience, which inverts the trade-off.** The
  demand's original constraint — "short enough that a human does not conclude
  the application crashed" — stops binding once there is a Cancel button. The
  human never has to wait the window out. The timeout therefore only has to
  bound the *unattended* hang, so it can afford to be generous. 120 s covers
  every non-streamed generation a 2026 model plausibly produces under a 32-step
  budget.
- **A separate 10 s connect timeout is worth one extra line.** The most common
  real failure is a typo'd endpoint or a host that black-holes packets, where a
  bare TCP connect can sit for the OS default (~130 s on Linux). Failing that
  case in 10 s with a readable message is the difference between "wrong URL" and
  "the app is broken".

**It does not become a `Settings` field**, and LCV-125 must not grow a fifth
row for it. Rejected because:

- an operator cannot set it correctly — it is a function of the model's
  generation latency, which they do not know and which changes per prompt;
- it is a backstop, not a workflow parameter. `agent_model` and
  `agent_step_budget` are configurable because they change *what the agent does*.
  A timeout changes only how long a broken thing takes to admit it is broken;
- the machinery is not free: a serde field with a default, a clamp at the read
  site (ADR 0007 §D7 — `settings.json` is hand-editable, so `0` and
  `u64::MAX` are both things a real file can say), a UI row, and a set of tests,
  all in `src/agent/settings_ui.rs`, the file LCV-125 is editing right now;
- **it is reversible in the cheap direction.** A constant can become a setting
  the day a real operator reports a real model that legitimately exceeds it. A
  shipped setting can never be taken back. If that day comes, open a demand and
  cite the model.

## Scope

- `src/agent/transport.rs`: build the blocking client through
  `reqwest::blocking::Client::builder()` with a total request timeout and a
  connect timeout, both named constants; add a `TransportError::Timeout`
  variant and map `reqwest::Error::is_timeout()` onto it.
- `src/app/agent_poll.rs`: a `cancel_turn(app)` entry point that ends the
  in-flight turn **by tail-calling the existing `end_turn`**, and is a no-op
  when no turn is in flight.
- `src/app/mod.rs`: re-export `cancel_turn` alongside `poll_agent_rx`, so the
  panel needs no deep path.
- `src/agent/panel.rs`: a `Cancel` button in the existing thinking row, visible
  only while `agent_busy`, whose whole body is one call into
  `crate::app::cancel_turn(app)` — the same shape as the existing
  `crate::app::start_turn(app, &prompt)` call.
- Tests, including the sole-writer scan that currently exists only as prose in
  ADR 0007 §D11 and the `agent_poll.rs` module header.

## Out of scope

- **A `Settings` field for the timeout.** Ruled on above. A bounded scan
  asserts `src/io/settings.rs` gained no timeout field.
- **A per-turn deadline, a retry, a backoff, or a second attempt on timeout.**
  One call, one window, one failure the operator can read. The step budget
  already bounds the turn.
- **Streaming responses / SSE.** That is a transport redesign and a panel
  redesign, and it is the only thing that would let a long generation show
  progress. If it is ever wanted it needs its own ADR.
- **`.no_proxy()` on the transport.** Explicitly forbidden: it would break real
  operators behind corporate proxies to buy a test convenience. The proxy
  fragility of the test suite is LCV-130's problem, not this demand's.
- **A keyboard binding for cancel (`Esc` or otherwise).** `Esc` is already
  routed (tool cancel, command-line clear) and adding a fourth claimant needs a
  precedence argument this demand does not want to make. The button is the
  affordance; LCV-126 can consider a key once the shortcut set is documented.
- **A cancel affordance anywhere but the agent panel.** No status-bar button, no
  toolbar button, no modal. If the panel is closed mid-turn the toolbar's 🤖
  toggle reopens it (`src/ui/toolbar.rs`), and LCV-124 AC 8 opens it
  automatically for any command-line-routed prompt.
- **Killing the worker thread.** Rust has no safe thread kill and this design
  does not need one: the worker unwinds on its own at its next rendezvous (AC
  10). A cancelled turn's thread may stay alive for up to the timeout window
  while its in-flight HTTP call drains. That is accepted, and AC 11 pins the
  only thing that actually matters about it.
- **A seventh transcript role.** LCV-125 AC 1 closes the vocabulary at six
  (`user`, `assistant`, `error`, `tool`, `refused`, `note`). The cancel row
  reuses `note` (AC 7). Do not add a `cancelled` role and do not edit LCV-125's
  match arms.
- **Changing ADR 0007.** §D11 already anticipates this work: *"A fifth exit may
  yet exist; the rule it must obey is the invariant above, not this list."* A
  cancel is a new **caller** of `end_turn`, not a new writer of `agent_busy`, so
  the invariant is preserved as written. If the implementer concludes §D11's
  four-item enumeration needs a fifth entry, that is an `architect` call — file
  a task, do not edit the ADR.

## Acceptance criteria

### Part A — the timeout (land these first; they stand alone)

1. **The client is bounded, and the bound is named.** `chat_completion` builds
   its client through `reqwest::blocking::Client::builder()` with
   `.timeout(Duration::from_secs(AGENT_REQUEST_TIMEOUT_SECS))` and
   `.connect_timeout(Duration::from_secs(AGENT_CONNECT_TIMEOUT_SECS))`, where
   the two constants are declared in `src/agent/transport.rs` as
   `AGENT_REQUEST_TIMEOUT_SECS: u64 = 120` and
   `AGENT_CONNECT_TIMEOUT_SECS: u64 = 10`, each carrying a doc comment that
   states its reasoning in one sentence. `builder().build()` returns a
   `Result`; its error is mapped, never `unwrap()`ed or `expect()`ed. A bounded
   scan over the implementation section asserts both constant names and both
   builder calls are present **and that `Client::new()` appears nowhere in it**.

2. **A timeout is a timeout, not a generic request failure.**
   `TransportError` gains
   `Timeout { secs: u64 }`, whose `Display` is exactly:
   `The endpoint did not answer within {secs} s. It may be slow, unreachable, or the endpoint URL may be wrong — check Help > Agent settings, or press Cancel and try a shorter prompt.`
   Every `reqwest::Error` raised inside `chat_completion` — from `send()` **and**
   from `text()`, because the body read can time out too — passes through one
   mapper that checks `is_timeout()` before falling back to
   `TransportError::Request`. No `?` is applied directly to a `reqwest` result
   inside `chat_completion`. The API key appears nowhere in the message
   (`assert_no_key`, the existing helper).

3. **The timeout actually fires, proved without a sleep in the passing path.**
   `chat_completion` delegates to a private
   `chat_completion_with_timeout(..., request: Duration, connect: Duration)`;
   the public surface stays exactly one `pub fn chat_completion`. A test drives
   the private form with a 250 ms window against two loopback
   `std::net::TcpListener` fixtures it owns — (a) accept and never write, (b)
   write `HTTP/1.1 200 OK` plus a `Content-Length` header and then stall — and
   asserts `TransportError::Timeout` in both cases. The green path costs ~500 ms
   total and sleeps nowhere; the call runs on a helper thread whose result is
   collected with `recv_timeout(5 s)`, so a regression that removes the timeout
   **fails in 5 s instead of hanging the suite**. State that reasoning in the
   test's doc comment: the 5 s guard bounds the failure, it does not produce the
   pass.

4. **A timed-out call ends the turn through the existing exit.** No new event,
   no new variant of `AgentEvent`: the worker's `Err` becomes
   `AgentError::Transport(..)` and then `AgentEvent::Failed(..)`, which is ADR
   0007 §D11 exit (2). A deterministic test — `arm_turn`, push
   `AgentEvent::Failed(TransportError::Timeout { secs: 120 }.to_string())` into
   the channel by hand, `poll_agent_rx` — asserts `agent_busy == false`,
   `agent_rx.is_none()`, and a last chat row of `("error", <the AC 2 sentence>)`.
   No thread, no socket, no sleep.

5. **`end_turn` is still the only writer, and now a test says so.** A scan over
   every `.rs` file under `src/`, each haystack bounded at the offset of its
   bare `#[cfg(test)]` at column 0, finds the needle `agent_busy = false`
   **exactly once**, in `src/app/agent_poll.rs`. The scan lives in
   `tests/` (never under `src/`, so it cannot match itself), builds its needle
   with `concat!`, rebuilds every path it reports from `components()` joined
   with `/` (AGENTS.md — this has broken CI twice), and carries a positive
   control proving it would find a second occurrence. This is the guard that
   makes the rest of the demand safe; it is not optional.

### Part B — the cancel

6. **One entry point, and it is a tail call.** `pub fn cancel_turn(app: &mut
   App)` lives in `src/app/agent_poll.rs` next to `end_turn`, is re-exported
   from `src/app/mod.rs`, returns immediately when `!app.agent_busy`, and
   otherwise does nothing but push nothing itself and tail-call
   `end_turn(app, Some(("note", AGENT_CANCELLED_MESSAGE.to_owned())))`. It
   assigns neither `agent_busy` nor `agent_rx` (AC 5 enforces the first).

7. **The operator is told what a cancel left behind, in the existing
   vocabulary.** `AGENT_CANCELLED_MESSAGE` is a `pub const` in
   `src/app/agent_poll.rs`, role `note`, text exactly:
   `Turn cancelled. The agent stopped; anything it already applied stays applied and stays undoable.`
   It is followed — in the same frame, in `agent_chat` order — by the ordinary
   end-of-turn undo-shape note from `finish_turn` when the turn applied
   anything. No seventh role is added and `panel.rs`'s six match arms are
   untouched.

8. **The button exists only while a turn does.** In `panel.rs`'s existing
   `if app.agent_busy` thinking row, after the spinner and the `Thinking…`
   label, a button labelled `Cancel`; clicking it calls
   `crate::app::cancel_turn(app)` and nothing else. When `agent_busy` is false
   the row — and therefore the button — is not rendered at all. Bounded scan:
   the `Cancel` literal and the `cancel_turn` call both sit inside the
   `agent_busy` block, and `panel.rs` still spawns no thread and constructs no
   `Document`/`History` (LCV-125 AC 7's scans cover the second half; do not
   duplicate them).

9. **After a cancel the app is idle and usable.** Following `cancel_turn`:
   `agent_busy == false`, `agent_rx.is_none()`, the `src/app/mod.rs` repaint
   gate no longer fires (assert the condition, in the style of
   `the_agent_repaint_is_guarded_on_the_busy_flag` in
   `tests/lcv123_agent_turn.rs`), and a subsequent `start_turn` / agent-routed
   command line is **accepted**, not refused by LCV-124 AC 10's busy gate.

10. **The worker can always notice and unwind.** Both halves of the rendezvous
    fail as soon as the UI drops the receiver, and both are
    `AgentError::Cancelled`, on which `start_turn`'s closure returns **without**
    sending a terminal event (there is nobody to read one):
    - the receiver is already gone when the worker sends: `ask_ui`'s
      `tx.send(Act { .. })` returns `Err` → `Cancelled`;
    - the receiver is dropped while an `Act` is queued and unread: dropping a
      `std::sync::mpsc::Receiver` drops the queued message, which drops the
      reply `Sender` riding inside it, so the worker's `answer.recv()` returns
      `Err` → `Cancelled`.
    Both are tested deterministically with **no thread and no sleep**, in the
    style of LCV-122's fourth-exit test. The second case is the one the demand
    called the genuinely hard part, and it is the one that must be pinned: it is
    a property of `std::sync::mpsc` that this design depends on and that nothing
    currently asserts. ADR 0007 §D11 arm (4) is its mirror and stays as it is.

11. **A cancelled turn can never speak into a later one.** After `cancel_turn`,
    a new turn armed with `arm_turn`, and the **old** `Sender` used to send
    `AgentEvent::Done("ghost")`: the old send returns `Err`, the new turn's
    `agent_chat`, `agent_busy`, `agent_fence` and `agent_applied` are
    untouched, and `poll_agent_rx` on the new turn observes nothing. Each turn
    owns its own channel; this test is what proves the zombie thread accepted in
    §Out of scope is harmless.

12. **Undo shape survives a cancel.** A turn that applied 3 actions and is then
    cancelled coalesces into **one** undo entry exactly as any other exit does
    (ADR 0007 §D6), because `cancel_turn` goes through `end_turn` →
    `finish_turn`. One `Ctrl+Z` takes the whole partial turn back, and the note
    row says so. A cancel that bypassed `end_turn` would leave three separate
    entries — this criterion is what makes that visible.

### Part C — joint

13. **Purity, caps, gates.** `src/agent/transport.rs` remains the only file
    importing `reqwest`; `panel.rs` and `settings_ui.rs` remain the only files
    under `src/agent/` importing `egui`; no file under `src/agent/` imports
    `eframe` or `rfd`. Implementation LOC measured with ADR 0004's `awk`
    recipe, never `wc -l`, and **reported** for all four touched files
    (before this work: `transport.rs` 156, `agent_poll.rs` 179, `panel.rs` 116
    *before LCV-125*, `app/mod.rs` 292 — `app/mod.rs` is between 270 and 300, so
    do **not** split it on sight; if the re-export pushes it over, stop and flag
    `architect`). `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and
    `cargo test --all --no-fail-fast` all exit 0. **`--no-fail-fast` is not
    optional** (ADR 0008).

14. **No setting was added.** A bounded scan over `src/io/settings.rs`'s
    implementation section finds no occurrence of `timeout`, and
    `src/agent/settings_ui.rs` gains no row. The two constants live in
    `transport.rs` and nowhere else.

## Expected tests

Every criterion above is checkable by a local Linux run. **No acceptance
criterion here may be verified only by CI** — GitHub Actions is down for this
repository and the local gate is the authoritative standard.

- **Unit / AC 1** — the bounded builder scan, including the negative on
  `Client::new()`, shown to discriminate.
- **Unit / AC 2** — `TransportError::Timeout { secs: 120 }.to_string()` is the
  exact sentence (needle built with `concat!`); `assert_no_key` on it; and the
  mapper is reached rather than the generic arm (covered behaviourally by AC 3,
  which must produce `Timeout` and not `Request`).
- **Unit / AC 3** — the two `TcpListener` fixtures with an injected 250 ms
  window, result collected via `recv_timeout(5 s)`.
- **Integration / AC 4** — the hand-pushed `Failed` event through `arm_turn` +
  `poll_agent_rx`.
- **Integration / AC 5** — the tree-wide sole-writer scan, with its positive
  control.
- **Unit / AC 6, AC 7** — `cancel_turn` on an idle `App` is a no-op (nothing
  pushed to `agent_chat`); on a busy `App` it produces the `note` row with the
  exact sentence.
- **Unit / AC 8** — the two bounded panel scans; plus a headless
  `App::update_ui` frame with `agent_panel_open = true` and `agent_busy = true`
  that completes (ADR 0002 §A2: `App::default()`, never `App::new()`).

  > **Conditional, added 2026-09-13.** If LCV-132 has landed by the time this
  > demand is implemented, assert AC 8 and AC 7 as **painted text** instead of
  > as scans: `harness::lines_on_surface_of` over a frame with
  > `agent_busy = true` puts a line reading `Cancel` on the panel surface, the
  > same frame with `agent_busy = false` does not, and after `cancel_turn` the
  > cancel note and the undo-shape note come back as consecutive lines in that
  > order. A scan proves the `Cancel` literal is written; it cannot prove the
  > button was painted, and it cannot prove the two notes are in the right
  > order — which is the whole of AC 7's second sentence. If LCV-132 has not
  > landed, keep the scans and **do not block on it**; do not write a private
  > shape collector here, because LCV-132 AC 3 exists to delete exactly that.
- **Integration / AC 9** — post-cancel state, the repaint condition, and a
  second `start_turn` being accepted.
- **Unit / AC 10** — both rendezvous-failure cases, no thread, no sleep.
- **Integration / AC 11** — the ghost-`Sender` test.
- **Integration / AC 12** — three commits, cancel, one `Ctrl+Z`, drawing back to
  its pre-turn state; assert the note row's wording matches the coalesced shape.
- **Unit / AC 13, AC 14** — the purity scans, the settings scan, the LOC numbers.
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result by name**:
  (a) delete `.timeout(..)` from the builder → AC 1's scan fails by name, and
  AC 3 fails (in 5 s, via the `recv_timeout` guard) rather than hanging —
  **report both, and report the wall time**;
  (b) let the `reqwest::Error` fall through to the generic `Request` arm
  (drop the `is_timeout()` check) → AC 3 fails by name;
  (c) make `cancel_turn` write `app.agent_busy = false; app.agent_rx = None;`
  directly instead of calling `end_turn` → AC 5's sole-writer scan **and**
  AC 12's coalesce test both fail by name. This is the demand's most important
  mutation: it is the exact shortcut a future implementer will reach for;
  (d) drop the `agent_busy` guard so the Cancel button always renders → AC 8
  fails by name;
  (e) make `cancel_turn` clear `agent_rx` but leave `agent_busy` true → AC 9
  fails by name;
  (f) introduce a `cancelled` role for the cancel row instead of `note` →
  AC 7 fails by name.
- **[manual] smoke** — `cargo run`, with a scratch endpoint in
  `Help > Agent settings` (restore the real one afterwards):
  1. **Cancel on a wedged socket.** Run `nc -l 127.0.0.1 8099` in a terminal and
     set the endpoint to `http://127.0.0.1:8099/v1`. Send a prompt: the spinner
     appears and a `Cancel` button sits beside it. Click `Cancel` — the spinner
     goes immediately, the cancelled note row appears, and the panel says
     nothing red. Type a second prompt and confirm it is **accepted** (no
     `Agent is busy` refusal).
  2. **The app idles again.** With the turn cancelled and the app untouched,
     watch its CPU in `top` for ten seconds: it must sit at rest, not spin. This
     is the LCV-120 symptom and no automated test observes it.
  3. **Connect timeout.** Set the endpoint to `http://192.0.2.1:81/v1`
     (RFC 5737 TEST-NET-1, guaranteed unroutable). Send a prompt, do **not**
     press Cancel, and confirm that within roughly 10 s a red row appears
     carrying the AC 2 sentence — not a hang, and not a raw reqwest string.
  4. **Partial work survives a cancel.** Set the endpoint back to the real one,
     ask for a shape that takes several actions, and press `Cancel` after the
     first one or two land. Confirm the geometry that landed is still on the
     bed, the note row states the undo shape, and one `Ctrl+Z` removes exactly
     what the note said it would.
  - The 120 s request timeout is deliberately **not** waited out by hand; AC 3
    covers it with an injected window.

## Test hygiene (mandatory)

- **Bound every source scan** to the implementation section — slice at the
  offset of the bare `#[cfg(test)]` at column 0, never a pattern that matches
  `#[cfg(test)]` anywhere on a line — and **build every needle with `concat!`**.
  Six self-matching, vacuously-passing scans have shipped in this repository.
  Canonical correct example: `guard_is_runtime_not_cfg` at
  `src/io/dialogs.rs:179-194`. Every scan must be **shown to discriminate** and
  the demonstration reported.
- **A path compared against a literal is rebuilt from `components()` joined with
  `/`** — never `Path::display()` or `to_string_lossy()` on the whole path. AC
  5's tree scan is exactly the shape that has broken CI twice.
- **A test that passes because of a `sleep` is not a test.** This demand is the
  single most likely place in the codebase for one. AC 3's `recv_timeout(5 s)`
  bounds a *failure*; the pass comes from the implementation giving up after
  250 ms. AC 10's two cases and AC 4, AC 11 and AC 12 use no thread and no sleep
  at all. If any test here needs `thread::sleep` to go green, it is the wrong
  test — say so and come back rather than shipping it.
- **No CI test may reach a real endpoint.** The only sockets in this demand are
  two loopback `TcpListener`s the test itself binds and drops. Do not add a
  mockito server for the timeout path — mockito cannot stall deterministically
  and the suite's mockito tests are already proxy-fragile (LCV-130).
- ADR 0002 §A2: `App::default()`, never `App::new()`. §A4 rule 1: no test sends
  `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`. ADR 0006 / ADR 0007 §D10: no test causes
  a settings write to a real per-user path, and no test sets a real API key.
- **The suite must be runnable behind a proxy is *not* a requirement of this
  demand** — it is LCV-130's. But do not make it worse: the two `TcpListener`
  fixtures in AC 3 are reached through `reqwest`, which honours `HTTP_PROXY`, so
  note in the test's doc comment that it belongs to LCV-130's inventory.

## Risks

- **The tempting shortcut breaks the one invariant that matters.** Clearing
  `agent_busy` from `cancel_turn` (or from the panel, or from the transport)
  "works" in every manual test and quietly turns §D11's closure property into
  prose. AC 5 plus mutation (c) exist solely to catch that, and they are the two
  things a reviewer should read first.
- **A cancelled turn leaks a live thread for up to 120 s.** Accepted. It holds
  no `Document`, no `History` and no borrow (ADR 0007 §D1); its channel is dead
  so it cannot speak (AC 11); it ends itself at the next rendezvous or when the
  HTTP call gives up. The alternative — killing a thread — does not exist in
  safe Rust.
- **A worst-case turn is still long.** 32 steps × 120 s is theoretically over an
  hour of wall time before the step budget alone would end it. That is
  acceptable *because* cancel exists and the operator is watching a spinner the
  whole time; it is the second reason the two halves ship together. If a real
  operator ever hits it, the fix is a per-turn deadline in `loop_.rs` — a new
  demand, not a widening of this one.
- **120 s is a judgement, not a measurement.** It is written down above so that
  the first operator report of a legitimate slower model can argue with the
  reasoning instead of the number.
- **`panel.rs` is edited by two demands.** LCV-125 lands first and rewrites the
  render arms; this demand adds three lines to the thinking row. Rebase, do not
  merge blind, and re-measure the LOC after LCV-125.

## Open questions

None. This demand is Ready when `demand-manager` flips it.

## Notes

- Origin: `reviewer-rust`, 2026-09-13, during the LCV-123 review; recorded as
  pre-existing and non-blocking for LCV-123. Refined by `product-owner`
  2026-09-13, which set the two constants, ruled the timeout a constant rather
  than a setting, ruled the demand single rather than split, and chose `note`
  over a seventh transcript role.
- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D1 (the thread owns no document state), §D2 (`recv()` returning `Err` **is**
  the cancellation signal — "dropping the `Sender` without answering … aborts
  the turn cleanly with no extra machinery"; this demand implements exactly that
  sentence and adds no machinery), §D3 (the reply `Sender` rides inside the
  `Act`), §D6 (one turn is one undo entry), §D8 (`panel.rs` renders and reports;
  only `transport.rs` imports `reqwest`), §D11 (the closure property, and its
  explicit allowance for a fifth exit that obeys it).
- Also: `AGENTS.md` §Event flow → Repaint policy, LCV-120 (the defect this
  reopens), LCV-124 AC 10 (the busy gate a cancel re-opens), LCV-125 AC 1 (the
  six-role vocabulary), LCV-130 (the proxy-fragile mockito suite),
  [ADR 0004](../../adr/0004-measuring-the-300-loc-cap.md) (the `awk` recipe),
  [ADR 0008](../../adr/0008-test-gates-run-with-no-fail-fast.md).
- Call sites to read before starting: `src/agent/transport.rs::chat_completion`,
  `src/app/agent_turn.rs::ask_ui` and `::start_turn`,
  `src/app/agent_poll.rs::poll_agent_rx` / `::end_turn` / `::finish_turn`,
  `src/agent/panel.rs::draw_agent_panel`, `src/ui/toolbar.rs` (the 🤖 toggle).
