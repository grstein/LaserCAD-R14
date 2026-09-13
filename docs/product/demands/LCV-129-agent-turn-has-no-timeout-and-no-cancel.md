# LCV-129 — A hung endpoint wedges the app: the agent turn has no timeout and no cancel

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-123 (the live turn wiring a cancel must end through)
- **Suggested agent**: product-owner (refinement) → implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`src/agent/transport.rs:127` builds its HTTP client as
`reqwest::blocking::Client::new()` with **no `.timeout(...)`**. reqwest's
default is unbounded, so a slow or unresponsive endpoint blocks the worker
thread for as long as the connection stays open — which can be forever.

The consequence chain is what makes this a demand rather than a footnote:

1. the worker blocks in `chat_completion` and never sends a terminal event;
2. so `src/app/agent_poll.rs::end_turn` is never reached;
3. so `agent_busy` is never cleared — `end_turn` is the only place in the
   program that writes `agent_busy = false` or clears `agent_rx`;
4. so `src/app/mod.rs`, which requests a repaint on every frame while
   `agent_busy` is set, repaints for the rest of the session.

That is **LCV-120 reopening through a different door**, two days after LCV-120
closed. It is exactly the latch ADR 0007 §D11 warns about: a repaint condition
that can no longer fall is an unconditional repaint wearing a guard.

And there is no cancel affordance anywhere in the agent panel, so the
operator's only recovery is to quit the application — losing unsaved work
unless autosave happens to have fired.

**Pre-existing.** The transport has been built this way since LCV-077; this
predates LCV-123 and is not a regression from it. But LCV-123 is the commit
that puts a real operator in front of a real endpoint for the first time, which
is what turns a latent defect into a reachable one.

## Two parts, and both are needed

### 1. A client timeout on the blocking reqwest client

The worker must eventually give up and send a terminal event, so the turn ends
through the normal path.

**Do not invent a new exit.** ADR 0007 §D11 enumerates exactly four turn exits
and every one of them is a tail call to `agent_poll::end_turn`. A timeout must
surface as one of the **existing** terminal events — an ordinary transport
failure the worker reports — not as a fifth exit with its own path to
`agent_busy`.

The duration is left to refinement. The only constraint stated here: long
enough that a slow model can think, short enough that a human does not conclude
the application crashed.

### 2. A Cancel affordance in the agent panel, ending the turn through `end_turn`

The reviewer's sketch is to drop `agent_rx`.

**The sharp edge, which refinement must not gloss.** The worker thread blocks
on a rendezvous reply channel (ADR 0007 §D2/§D3: the reply `Sender` is built
per request and carried inside the `Act`, and the worker waits on its other
half for the real outcome). Cancelling from the UI side therefore has to leave
the worker able to *notice* and unwind, rather than blocking forever on a send
nobody will answer. Note that §D11 arm (4) already covers the mirror case — the
UI thread finding nobody waiting for its reply — so the two halves of this
interaction are related and should be reasoned about together. This is the
genuinely hard part of this demand.

### Why this is not "just add a timeout"

The timeout alone leaves the operator staring at a spinner for the whole
timeout window with no way out. The cancel alone leaves a truly wedged
connection hanging. Both, or the defect is only half closed.

## Notes for refinement

- Room for the button: `src/agent/panel.rs` is **116** implementation LOC after
  LCV-123 stripped `submit` out of it. `src/agent/transport.rs` is **156**.
  Neither is near the 300-LOC cap.
- `panel.rs` renders and reports; it never spawns a thread and never constructs
  a `Document` or a `History` (ADR 0007 §D8). Whatever the Cancel button does,
  the wiring side of it belongs app-side.
- Only `src/agent/transport.rs` may import `reqwest` (AGENTS.md §Purity rule).

## Traps

Both have already bitten this repository. They belong in the acceptance
criteria, not in the implementer's rediscovery.

1. **The self-matching source scan — wrong six separate times here.** A test
   that scans source text with `include_str!` matches its own source and passes
   vacuously. The haystack must be bounded at the offset of a bare
   `#[cfg(test)]` at column 0, and every needle built with `concat!`. Canonical
   correct example: `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179`.
2. **A test that passes because of a `sleep` is not a test.** A timeout demand
   is the single most likely place in this codebase for someone to write one:
   it is slow, it is flaky, and it proves nothing about the property. The model
   to copy is LCV-122's fourth-exit test, which reproduces a channel race
   deterministically with no thread and no sleep.

## Scheduling

- **Not part of Marco 2.** Must not be driven ahead of LCV-124 or LCV-125.
- **Strongest candidate to be driven immediately after Marco 2 closes**, ahead
  of LCV-126, LCV-127 and LCV-128. It is the only known defect that can wedge
  the application in the hands of a real user, and the user is about to run
  their first live prompt against OpenRouter — the exact situation in which a
  slow or unresponsive endpoint is not hypothetical.

## Notes

- Origin: `reviewer-rust`, 2026-09-13, during the LCV-123 review. Recorded as
  pre-existing and non-blocking for LCV-123.
- Body is deliberately thin: opened by `demand-manager` for registration.
  `product-owner` writes Scope / Out of scope / Acceptance criteria / Expected
  tests, and decides the timeout duration.
- Related: `src/agent/transport.rs::chat_completion`,
  `src/app/agent_turn.rs::arm_turn`, `src/app/agent_poll.rs::end_turn`,
  `src/agent/panel.rs`,
  [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md) §D2, §D3,
  §D8 and §D11, `AGENTS.md` §Event flow → Repaint policy, LCV-120,
  `src/io/dialogs.rs:179`.
