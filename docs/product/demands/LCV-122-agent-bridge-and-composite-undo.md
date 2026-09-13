# LCV-122 — The bridge: one action, one command, one undo entry

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-121
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`src/agent/tools.rs::dispatch_tool_call(name, args, doc, history)` does two
unrelated jobs in one function: it validates the model's JSON (finite numbers,
positive radius, non-negative integer index, per-field error messages) **and**
it commits a `Box<dyn Command>` against a `Document` and a `History`. That
second half is why the background thread was handed a document at all, and
ADR 0007 §D1 forbids the thread from owning one. The two halves have to be cut
apart before the turn can run against the live drawing: validation is pure and
belongs on the agent side; committing needs `&mut App` and belongs on the UI
side.

Two things are missing entirely. There is no value type for "the model wants
this done" — the intent only ever exists as a `&str` name plus a
`serde_json::Value`, which cannot cross a channel with any guarantee. And there
is no way to turn *n* commits into one undo entry: an operator who asks for a
20 mm square and gets four `CreateLine` commits has to press `Ctrl+Z` four times
to take it back, which is not what "undo what the agent did" means in an R14
clone.

The channel payload is also in the wrong place. `AgentPanelMsg` lives in
`src/agent/panel.rs`, which imports `egui` and `crate::app::App` — so the
protocol between a kernel-pure background thread and the UI thread is declared
inside the UI file, and `agent → app` is imported backwards.

This demand builds the vocabulary. It changes nothing the operator can see:
after it lands the agent still narrates a drawing nobody can see, exactly as
today. LCV-123 is what closes that.

## Scope

- New `src/agent/bridge.rs` — `AgentAction`, `AgentOutcome`, `AgentEvent`, and
  the turn fence type. Kernel-pure.
- `AgentPanelMsg` renamed to `AgentEvent` and moved out of `panel.rs`.
- `src/agent/tools.rs` — `dispatch_tool_call` becomes
  `parse_tool_call(name, &Value) -> Result<AgentAction, ToolCallError>`, keeping
  every existing hand-rolled check.
- New `src/app/agent_apply.rs` — `AgentAction` → `Box<dyn Command>` →
  `App::commit`, plus the outcome strings ADR 0007 §D5 requires.
- New `document::commands::CompositeCommand` and
  `History::coalesce_last(n, label)`.

## Out of scope

- **Running a turn against the live document.** `run_agent_turn` keeps its
  `&mut Document` / `&mut History` parameters and `panel.rs::submit` keeps its
  throwaway `Document::default()` here. Both die in LCV-123, and a reviewer must
  not flag either in this demand.
- **`src/app/agent_turn.rs`, the rewritten `src/app/agent_poll.rs`, spawning,
  the ask-callback, the system-prompt change, the budget read site.** All
  LCV-123.
- **Adding `query_entities` / `query_selection` to `tool_definitions()` or to
  `parse_tool_call`.** LCV-123. The two `AgentAction` variants and their
  `agent_apply` arms *are* delivered here (ADR 0007 §D2 fixes the enum's shape),
  so the `match` is exhaustive and tested from day one. They are unreachable
  from the model until LCV-123 adds their schema entries — that is staging, not
  dead code, and it is written here so nobody deletes them.
- **Making `AgentAction` a `serde` deserialization target.** Explicitly rejected
  by ADR 0007 §Alternatives: it would throw away `tools.rs`'s finite-number,
  positive-radius and non-negative-integer checks and its per-field error
  messages, which is most of the value of that file.
- **Partial rejection of a turn** (undoing some of the agent's actions and
  keeping others). Named non-goal in ADR 0007 §D6. The cure is undo and
  re-prompt.
- **Deferring application to turn end**, a shadow document, an entity snapshot,
  `Arc<Mutex<Document>>`, or `#[derive(Clone)]` on `Document`. Each is a review
  blocker (ADR 0007 §D1, AGENTS.md §Implementation Rules).
- **Stable entity IDs.** Deferred to its own ADR. Positional indices ship, with
  the renumbering disclosed (ADR 0007 §D5).
- **New tools.** Exactly the five that exist, plus the two queries LCV-123
  exposes. No `create_rect`, no `offset`, no `trim`.

## Acceptance criteria

1. **`src/agent/bridge.rs` declares the protocol, verbatim per ADR 0007 §D2.**
   `AgentAction` has exactly these seven variants and no others:
   `CreateLine { x1, y1, x2, y2 }`, `CreateCircle { cx, cy, r }`,
   `CreateArc { cx, cy, r, start, end, ccw }`, `Delete { index }`,
   `Move { index, dx, dy }`, `QueryEntities`, `QuerySelection`. All coordinates,
   lengths and radii are `f64` **millimetres**; `start` / `end` are `f64`
   **radians** (the degrees→radians conversion happens in `parse_tool_call`,
   where the schema's `start_deg` / `end_deg` arrive, so the kernel side of the
   boundary is radians like the rest of the kernel). `index` is `usize`.
   `AgentOutcome` is `Ok(String) | Refused(String)`. `AgentEvent` is
   `Act { action: AgentAction, reply: std::sync::mpsc::Sender<AgentOutcome> }`,
   `Done(String)`, `Failed(String)`.

2. **`bridge.rs` is kernel-pure.** Its implementation section contains none of
   `egui`, `eframe`, `rfd`, `reqwest`. Bounded scan with a positive control.

3. **No file under `src/agent/` imports `crate::document` or holds document
   state.** A scan over every `src/agent/*.rs` implementation section finds no
   `crate::document`, no `Document`, no `History`, no `Arc<Mutex`, and no
   `Vec<Entity>` — **except** `panel.rs`, whose throwaway construction LCV-123
   deletes and which this AC lists as the single named exception, by file, with
   a comment saying so. This is ADR 0007 §D1 stated as a check.

4. **`AgentPanelMsg` is gone.** The identifier appears nowhere in `src/` or
   `tests/` (bounded scan, `concat!` needle). `crate::agent::AgentEvent` is the
   re-export; `panel.rs` declares no channel payload type; `App::agent_rx` is
   `Option<Receiver<crate::agent::AgentEvent>>`. The two existing
   `poll_agent_rx` unit tests in `src/app/mod.rs` are rewritten against
   `AgentEvent::Done` / `AgentEvent::Failed` and keep asserting the same
   behaviour (`agent_chat` row, `agent_busy` cleared, `agent_rx` cleared).

5. **`parse_tool_call(name: &str, args: &serde_json::Value) ->
   Result<AgentAction, ToolCallError>`** replaces `dispatch_tool_call`, which no
   longer exists. It handles the five mutating tool names and returns
   `ToolCallError::UnknownTool` for anything else. Every existing validation is
   preserved with its existing message text: missing field, non-finite number,
   non-positive or non-finite radius, an `index` that is negative / fractional /
   non-finite.

6. **The one check that must move, and why.** `get_index`'s range test
   (`index >= doc_len`) cannot stay on the agent side, because the agent side no
   longer knows `doc_len` — that is the point of ADR 0007 §D1. So:
   `parse_tool_call` keeps the *shape* check (non-negative, integral, finite)
   with its current message; the *range* check moves to `src/app/agent_apply.rs`
   where the live document is, and produces
   `AgentOutcome::Refused("index 7 is out of range (the drawing has 3 entities)")`
   rather than an error. A refusal is information the model can act on, not a
   failed turn. **Both halves have their own test.**

7. **`tools.rs` keeps hand-rolled validation.** Bounded scan: the `AgentAction`
   declaration carries no `Deserialize` derive, and `tools.rs` contains no
   `serde_json::from_value::<AgentAction>`. Every one of the 17 existing
   `#[test]`s in `tools.rs` survives, retargeted at `parse_tool_call` — none may
   be deleted silently, and the final count is reported.

8. **`src/app/agent_apply.rs` applies one action through `Command` +
   `History`.** `pub fn apply(app: &mut App, action: &AgentAction) ->
   AgentOutcome`. The five mutating variants build the **same commands as
   today** — `CreateLine`, `CreateCircle`, `CreateArc`, `DeleteEntities`,
   `MoveEntities` — and commit them through `App::commit`, never by touching
   `app.document.entities`. The two query variants read and format, and commit
   nothing: `app.history.revision()` is unchanged across a query.

9. **Every mutating outcome reports the resulting entity count** (ADR 0007 §D5).
   Format, fixed so tests can assert on it: the existing sentence, then
   ` The drawing now has N entities.` — e.g.
   `Line created: (0.000, 0.000) → (20.000, 0.000) mm. The drawing now has 3 entities.`
   Millimetres to 3 decimals and degrees to 1 decimal, exactly as the current
   `dispatch_tool_call` strings do; the arc outcome keeps reporting degrees.

10. **A delete says what it deleted and which indices shifted.** The `Delete`
    outcome names the entity **captured before the mutation** (e.g.
    `Deleted entity 1 (circle, center (10.000, 10.000) mm, r = 5.000 mm).`) and,
    when higher indices existed, states the renumbering:
    ` Indices 2..4 are now 1..3.` When the deleted entity was the last one, it
    says ` No indices shifted.` The `Move` outcome likewise echoes a one-line
    description of the entity it moved.

11. **`document::commands::CompositeCommand`** exists in its own file under
    `src/document/commands/`, re-exported from that module's `mod.rs`. It holds
    `Vec<Box<dyn Command>>` and a `String` label; `do_` runs them **forward**,
    `undo` runs them **in reverse**; `label()` returns the stored label. It
    satisfies the round-trip invariant every concrete command is gate-checked on
    (AGENTS.md / `src/document/commands/mod.rs`): `do_` then `undo` leaves the
    document `PartialEq`-equal to the pre-`do_` state, including after a
    `do_ → undo → do_ → undo` cycle (redo calls `do_` a second time).

12. **`History::coalesce_last(n: usize, label: &str)`** pops the last `n` undo
    entries, wraps them in a `CompositeCommand` **in their original order**, and
    pushes that back as one entry. Precisely:
    - it does **not** call `do_` or `undo` on anything — the document is not
      touched;
    - `revision()` is **unchanged** by the call (the document did not change);
    - the redo stack is not touched;
    - `n > len()` coalesces what is there (`HISTORY_DEPTH` is 200 and the step
      budget caps at 32, so this is a tolerance, not a path);
    - `n < 2` is a no-op — wrapping a single command in a composite would only
      relabel it, and `len()` would not change.
    After `coalesce_last(4, "Agent: draw a square")`, `len()` has dropped by 3,
    one `undo` reverses all four actions, and one `redo` reapplies all four in
    the original order.

13. **The fence type is declared here and driven by LCV-123.** `bridge.rs`
    declares `TurnFence` with `new(start_revision: u64)`,
    `check(current: u64) -> Result<(), String>`, `advance(current: u64)` and
    `may_coalesce(current: u64, n: usize) -> bool`. Semantics, from ADR 0007
    §D4 and §D6:
    - `check` is `Ok(())` iff `current` equals the revision the fence expects;
      otherwise `Err` carrying the refusal text
      `The drawing changed outside this turn — someone drew, deleted or selected something since I last looked. Nothing was applied. Undo is unaffected; ask again and I will re-read the drawing.`
    - once `check` has returned `Err`, the fence is **tripped** and every later
      `check` returns the same `Err` regardless of `current`;
    - `advance` sets the expectation to `current` (used after a successful
      apply, where `current == expected + 1`);
    - `may_coalesce` is `true` iff the fence was never tripped, `n >= 2`, and
      `current - start == n as u64` — the ADR §D6 step-3 gate, which proves the
      top `n` entries are contiguously the agent's.
    `TurnFence` is pure: it is unit-tested with plain `u64`s, no `App`, no
    `Document`, no egui.

14. **Nothing the operator can see changes.** The agent still runs against a
    throwaway document; the chat still shows the same rows; no new menu item, no
    new field, no new command-line behaviour. Stated here so a reviewer does not
    read the absence as an omission.

15. **Caps, purity, gates.** ADR 0004's `awk` recipe, never `wc -l`:
    `bridge.rs`, `tools.rs`, `agent_apply.rs`, `composite.rs` and `history.rs`
    are each at or under 300 implementation LOC, and the numbers are reported.
    `src/document/` stays free of `egui` / `eframe` / `rfd`;
    `src/app/agent_apply.rs` imports neither `eframe` nor `rfd` and spawns no
    thread. `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and `cargo test --all` exit 0.

## Expected tests

- **Unit / AC 1, AC 2** — `bridge.rs`: constructing each `AgentEvent` variant
  compiles and a `Sender<AgentOutcome>` fits inside `Act`; bounded purity scan
  with a positive control.
- **Unit / AC 3** — one scan per file under `src/agent/`, with `panel.rs`
  named as the single allowed exception and that exception asserted to be
  exactly one file (so the exception list cannot silently grow).
- **Unit / AC 4** — bounded scan over `src/` and `tests/` for `AgentPanelMsg`
  (`concat!` needle); the two rewritten `poll_agent_rx` tests.
- **Unit / AC 5, AC 7** — the 17 retargeted `tools.rs` tests: each of the five
  names produces the right `AgentAction` with the right field values; a missing
  field, a NaN coordinate, `r = 0`, `r = -1`, `r = inf`, `index = -1`,
  `index = 1.5`, an unknown name each produce the documented `ToolCallError`
  with its existing message text. Degrees→radians: `start_deg = 90` becomes
  `start` within `1e-12` of `FRAC_PI_2`.
- **Unit / AC 6** — two tests. Agent side: `parse_tool_call("delete_entity",
  {"index": 7})` succeeds with no document in sight. App side:
  `apply(&mut app, &AgentAction::Delete { index: 7 })` against a 3-entity
  document returns `Refused` whose text contains `out of range` and `3`, and
  leaves `history.revision()` unchanged and `entities.len() == 3`.
- **Unit / AC 8** — for each of the five mutating variants: the entity appears
  (or moves, or disappears) in `app.document`, `history.revision()` bumps by
  exactly 1, `history.can_undo()` is true, and one `undo` restores the previous
  state. Plus: `apply` of `QueryEntities` and `QuerySelection` leaves
  `revision()` and `entities.len()` untouched.
- **Unit / AC 9** — the outcome string for each mutating variant contains the
  post-mutation count; a document that had 2 entities reports `3` after a
  create and `1` after a delete. The needle is the rendered count, and the
  test is run at two different counts so it cannot pass on a hardcoded string.
- **Unit / AC 10** — deleting index 1 of 4 reports the deleted entity's kind and
  its mm geometry and says `Indices 2..3 are now 1..2`; deleting the last index
  says `No indices shifted`; a `Move` outcome names the entity it moved.
- **Unit / AC 11** — `CompositeCommand` round-trip on a three-command sequence
  including one `DeleteEntities` (order-sensitive: a forward-order `undo` would
  restore the wrong entity, so this test must be built to fail against a
  forward-order `undo` — demonstrate it).
- **Unit / AC 12** — `coalesce_last` with `n = 4` on a 4-deep stack: `len()`
  becomes 1, `revision()` is identical before and after, one `undo` empties the
  document, one `redo` restores all four in order. Plus `n = 0`, `n = 1`
  (no-op, `len()` unchanged), and `n = 9` on a 4-deep stack (coalesces 4).
  Plus: a stack holding **user** commands below the agent's — `coalesce_last(2)`
  on a 5-deep stack leaves the lower 3 untouched and individually undoable.
- **Unit / AC 13** — `TurnFence`: a matching revision passes; a mismatch returns
  the refusal text; after one mismatch a *matching* revision still returns the
  refusal (the trip is sticky); `advance` moves the expectation;
  `may_coalesce` is true for `(start = 10, current = 14, n = 4)`, false for
  `n = 3`, false for `n = 1`, and false after a trip.
- **Unit / AC 15** — the LOC measurements (reported, not asserted) and the
  purity scans.
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result**:
  (a) make `CompositeCommand::undo` run forward → AC 11 fails by name;
  (b) make `coalesce_last` bump `revision` → AC 12 fails by name;
  (c) make `coalesce_last` re-run `do_` on the composite → the document
  doubles and AC 12 fails by name;
  (d) drop the sticky behaviour from `TurnFence` (recompute per call) → AC 13
  fails by name;
  (e) delete the ` The drawing now has N entities.` suffix → AC 9 fails by name;
  (f) make `agent_apply` mutate `app.document.entities` directly instead of
  committing → AC 8's `revision()` and `can_undo()` assertions fail.
- **[manual] none.** Nothing operator-visible ships here.

## Test hygiene (mandatory)

- **Bound every source scan** to the implementation section (slice at the offset
  of the bare `#[cfg(test)]` at column 0) and **build every needle with
  `concat!`**, or the scan matches its own literal and cannot fail. Canonical
  correct example: `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`.
  This bug class has shipped six times on this project; every scan must be
  **shown to discriminate** and the demonstration reported.
- **Positive controls must be tight** — a control that also passes against the
  mutant is not a control.
- ADR 0002 §A2: build `App::default()`, never `App::new()`. ADR 0002 §A4 rule 1:
  no test sends `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`.
- ADR 0006 / ADR 0007 §D10: **no test may cause a settings write to a real
  per-user path**, and a test that injects a `settings_path` must not also set a
  real API key.

## Risks

- **`Box<dyn Command>` is not `Clone`, so `coalesce_last` must move the entries
  out of the `VecDeque`, not copy them.** Popping `n` from the back yields them
  in reverse; reverse again before constructing the composite or `undo` will
  run in the wrong order and a delete/create pair will corrupt the document.
  AC 11's order-sensitive test is the guard.
- **A `usize` underflow is one line away.** `n > len()` and `current - start`
  are both subtraction on unsigned integers. Use `saturating_sub` / `min`, and
  test both edges.
- **The exception list in AC 3 is the thing that rots.** It names `panel.rs` and
  only `panel.rs`, and LCV-123 removes it. Assert the list's length, not just
  its contents.
- **`agent_apply` is where a silent direct mutation would be easiest to write.**
  AGENTS.md §"State and mutation" forbids it; AC 8's revision assertions are
  what catch it.

## Notes

- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D2 (the enum shapes), §D3 (the rename and the single channel), §D4 (the
  fence), §D5 (index disclosure), §D6 (coalesce, not defer), §D8 (file
  responsibilities and the three boundary rules).
- **The fence lives in two places by design, and that is not a contradiction.**
  ADR §D4 says `src/app/agent_turn.rs` records `expected` and checks it; ADR
  §D8's file table says `agent_turn.rs` *holds* the fence. This demand declares
  the fence **type** in the pure `bridge.rs` so it can be unit-tested with no
  `App` and no frame loop; LCV-123 owns the instance, records the start
  revision, and calls it. Nothing about §D4's semantics changes.
- Measured before the work (ADR 0004 `awk`): `tools.rs` 151,
  `src/document/history.rs` 160, `src/app/mod.rs` 265, `panel.rs` 155.
- `HISTORY_DEPTH` is 200 (`src/document/history.rs`); `AGENT_STEP_BUDGET_MAX` is
  32 (LCV-121). That is why `coalesce_last` never meets a real `n > len()`.
- LCV-123 depends on this demand and on LCV-121.
