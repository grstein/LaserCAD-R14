# LCV-123 — The agent turn draws on the operator's real drawing

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-121, LCV-122
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

This is the demand that closes the sentence in `CHANGELOG.md`: *"The agent can
read and narrate the drawing but does not yet modify it end-to-end from chat."*

`src/agent/panel.rs::submit` spawns the background thread like this:

```rust
let mut doc = Document::default();
let mut history = History::default();
let result = crate::agent::run_agent_turn(&text, &endpoint, &api_key, &mut doc, &mut history);
```

The turn runs to completion against a **throwaway document**, commits real
`CreateLine` / `DeleteEntities` / `MoveEntities` commands into a throwaway
history, and then drops both on the floor when the thread ends. Only the final
assistant string survives. An operator who types "draw a 20 mm square at the
origin" is told, truthfully as far as the model knows, that four lines were
created — and the bed stays empty. Worse after LCV-121, because the model can
now actually call the tools.

Every other piece is in place: a transport that speaks tool calls (LCV-121), an
`AgentAction` vocabulary, an apply path that goes through `Command` + `History`,
a composite command and a coalescing history, and a fence type (LCV-122). What
is missing is the wiring decided in ADR 0007: the thread stops owning any
document state and instead **asks the UI thread to apply one action at a time
and blocks for the real answer**. This demand does that wiring, and adds the two
read-only tools (`query_entities`, `query_selection`) that make the ask path
useful in both directions — without them the model can only write blind, which
is exactly how a wrong `delete_entity 3` happens.

## Scope

- `run_agent_turn` becomes control-flow-only: no `Document`, no `History`, one
  injected ask-callback.
- `src/app/agent_turn.rs` (created by LCV-122, which is where `run_agent_turn`
  already lives) — arms the turn, spawns the thread, holds the fence and the
  coalesce gate (ADR 0007 §D8).
- `src/app/agent_poll.rs` rewritten to drain until `Empty` and to clear
  `agent_rx` only on a terminal event.
- `src/agent/panel.rs::submit` deleted; the panel calls into
  `app::agent_turn::start_turn`.
- `query_entities` and `query_selection`: schema entries and `parse_tool_call`
  arms.
- The system prompt gains the positional-index contract (ADR 0007 §D5).
- The step budget is read from `Settings` and clamped at this read site.
- The end-of-turn note that tells the operator whether the turn is one undo step
  or several.
- The tool-call transcript: one `agent_chat` row per action, written at the
  apply site.

## Out of scope

- **Rendering the transcript.** The seam is: **this demand produces the rows,
  LCV-125 renders them.** Producing a row is data and it happens where the
  outcome is already known (AC 23); how a row *looks* — per-role render arms,
  colours, a distinct tool row, prefixes, layout, scrolling — is **LCV-125**,
  along with the settings fields for model and budget and the plaintext-key
  warning. Until LCV-125 lands these rows render through the panel's existing
  fallback label, which is legible enough to assert against. This demand writes
  all six roles LCV-125 closes over: `user` (AC 3), `tool` and `refused`
  (AC 23), `assistant` and `error` (AC 7), `note` (AC 11).
- **Command-line routing** (`:` / `/ai`, the classifier, the
  `! Agent unavailable: …` path). **LCV-124.** After this demand lands, LCV-124
  and LCV-125 are independent of each other and may run in parallel.
- **Partial rejection of a turn.** Named non-goal (ADR 0007 §D6). Two undo
  behaviours exist — coalesced, and per-action when the fence aborted — and both
  are correct.
- **Blocking the canvas during a turn.** No modal, no lock, no greyed-out
  viewport. Drawing during a turn is allowed; it ends the turn (ADR 0007 §D4).
- **More than one turn in flight**, queued prompts, or a turn that outlives the
  frame loop. The single `agent_rx` and the single fence both assume one turn,
  and ADR 0007 §Revisit criteria says to come back to the ADR first.
- **Stable entity IDs.** Positional indices ship, disclosed in the prompt and in
  every outcome (ADR 0007 §D5). The real fix is a separate ADR.
- **New mutating tools.** Exactly the five that exist, plus the two read-only
  queries. No `create_rect`, no `trim`, no `offset`, no `select`.
- **Touching `src/app/viewport.rs` or its repaint predicate.** That is LCV-120's
  and it is shipped. See AC 19 for the one repaint site this demand must
  *preserve*.
- **`tokio`, `Arc<Mutex<Document>>`, `#[derive(Clone)]` on `Document`, an entity
  snapshot, a shadow document.** Each is a review blocker.

## Acceptance criteria

1. **`run_agent_turn` owns no document state.**
   ```rust
   pub fn run_agent_turn<A>(
       prompt: &str, endpoint: &str, api_key: &str, model: &str,
       step_budget: u8, ask: &mut A,
   ) -> Result<String, AgentError>
   where A: FnMut(AgentAction) -> Result<AgentOutcome, AgentError>
   ```
   The **thread's** dispatch is the ask seam: the closure `run_agent_turn`
   hands to `agent_loop` parses the tool call and then calls `ask`, so the only
   seam that carries a real outcome is `ask` and the worker holds no document.
   `agent_loop`'s own `dispatch_fn: FnMut(&str, &str) -> Result<String,
   AgentError>` signature is **unchanged**, and that is deliberate: ADR 0007
   §D8 gives `loop_.rs` control flow only — send, cap, feed tool results back —
   and puts `parse_tool_call` in `tools.rs`, so `loop_.rs` must never name
   `AgentAction`. Pushing `AgentAction` down into `agent_loop` would satisfy a
   literal reading of "the same seam" by breaking §D8; do not. `Document`,
   `History`, `crate::document`, `Arc<Mutex`, `Vec<Entity>` and
   `Document::default()` appear **nowhere** under `src/agent/` — the LCV-122
   AC 3 exception list is now empty, and the scan asserts the list is empty
   rather than merely not containing `panel.rs`.

2. **The throwaway document is deleted, not adapted.** `panel.rs::submit` no
   longer exists. `panel.rs` contains no `std::thread::spawn`, no
   `mpsc::channel`, no `Document`, no `History` (bounded scan, `concat!`
   needles, each shown to discriminate). AGENTS.md's rule — *"`panel.rs` renders
   and reports; it never spawns a thread and never constructs a `Document` or a
   `History`"* — is now enforced by a test.

3. **`src/app/agent_turn.rs` arms and spawns, and the arming is the test seam.**
   - `pub fn arm_turn(app: &mut App, prompt: &str) ->
     std::sync::mpsc::Sender<AgentEvent>` records the user row in
     `agent_chat`, sets `busy`, creates the channel and stores the `Receiver` on
     `App`, constructs `TurnFence::new(app.history.revision())`, zeroes the
     applied-action counter, stores the turn label, and returns the `Sender`.
   - `pub fn start_turn(app: &mut App, prompt: &str)` calls `arm_turn` and then
     `std::thread::spawn`s the thread that runs `run_agent_turn`. It does
     nothing else.
   - A turn is refused while one is in flight: `start_turn` on a busy `App` is a
     no-op and leaves `agent_chat`, `agent_rx` and the fence untouched.

   `arm_turn` is `pub`, not `pub(crate)`, and the visibility is driven by AC 8's
   test rather than by a design preference: that test lives in `tests/`, which
   is a **separate crate**, so `pub(crate)` would make it impossible to write.
   Moving it into `src/` is the worse trade — it drives `App::update_ui` frames
   and needs `tests/harness`, which would have to be duplicated or made public.
   The exposure is nil because the library has no external consumer. A later
   cleanup that "tightens" this to `pub(crate)` breaks the integration suite;
   the doc comment on `arm_turn` says so in place, and this clause is why.

4. **The rendezvous.** The thread's ask-callback creates a fresh one-shot
   `mpsc::channel::<AgentOutcome>()` per action, sends
   `AgentEvent::Act { action, reply }`, and **blocks on `recv()`** until the UI
   answers. `App` keeps exactly one agent channel field; the reply `Sender`
   rides inside the message and is never parked on `App`.

5. **Dropping the reply `Sender` is cancellation, and it is free.** `recv()`
   returning `Err(RecvError)` — and `send()` on a dead `agent_rx` — both map to
   `AgentError::Cancelled`, on which the thread returns **without** sending a
   terminal event. A unit test drives `run_agent_turn` with an ask-callback that
   returns `Err(AgentError::Cancelled)` and asserts the turn returns immediately
   and issues no further HTTP call.

6. **`agent_poll` drains until `Empty`.** `poll_agent_rx` loops. In one frame it
   handles an unbounded number of `Act`s and then the terminal event, in the
   order received. The pre-LCV-122 `if let Ok(msg) = rx.try_recv()` /
   `agent_rx = None` shape must not come back: it drained exactly one message
   per frame and cleared the channel unconditionally, which would swallow the
   first `Act` and kill every turn. (LCV-122 landed the loop and the
   `take()`/put-back; this demand preserves them.)

7. **`agent_rx` and `agent_busy` are cleared on every turn exit, and only on a
   turn exit.** After a frame that carried only `Act`s,
   `app.agent_rx.is_some()` and `app.agent_busy` are both still true. ADR 0007
   §D11 names **four** exits, and each one is a tail call to the single
   `end_turn` that clears both flags and writes at most one row:
   1. `Done` — the assistant's text, role `assistant`.
   2. `Failed` — the worker's error, role `error`.
   3. `TryRecvError::Disconnected` — the worker's `Sender` dropped without a
      verdict: it panicked, it returned after cancellation, or the app is
      closing. Row: the constant `agent_poll::AGENT_LOST_MESSAGE`, role
      `error`.
   4. **`reply.send(outcome)` returning `Err` while the UI thread is holding an
      `Act`** — the worker vanished between asking and reading the answer.
      Row: **the same constant and the same `error` role as (3)**, asserted
      equal to (3)'s row rather than to a literal. Why the same row and not
      silence: the reply `Sender` is built per request and rides inside the
      `Act` (§D3), and a live worker blocks on its other half until the answer
      arrives (§D2), so it cannot drop the receiving end while an `Act` is
      outstanding. An `Err` from `reply.send` therefore means the worker is
      gone, which means the event channel is closed too and (3) would have
      written that row an instant later. Two spellings of "the worker is gone"
      must not show the operator two different things. This arm **returns**; it
      does not fall through to the next `try_recv`, because a dead reply
      channel has to be sufficient on its own (§D11). The action that was
      already applied stays applied and stays undoable.

   The property all four share, and the one the tests pin: **a turn that does
   not end in a clean `Done` says so in the transcript with role `error`, and
   every exit leaves the app idle.** A turn that leaves `agent_busy` set is a
   permanent repaint loop — `src/app/mod.rs` requests a repaint on every frame
   while the flag is set — which is LCV-120 reopened with a symptom that
   surfaces nowhere near the agent. The **wording** of `AGENT_LOST_MESSAGE`
   belongs to the code, not to this demand: every assertion names the constant,
   so polishing the sentence can never fail a test or stale this file.

8. **An action really lands on the operator's drawing.** Deterministic
   integration test, no HTTP and no thread: the test calls `arm_turn`, keeps the
   returned `Sender`, pushes `AgentEvent::Act { CreateLine … }`, runs **one**
   `App::update_ui` frame, and then reads its own reply `Receiver`. Assertions:
   `app.document.entities.len()` grew by 1, the entity is the line with the
   exact millimetre coordinates requested, `app.history.revision()` bumped by
   exactly 1, `app.history.can_undo()`, and the `AgentOutcome::Ok` the test
   received carries the post-mutation entity count.

9. **The fence refuses a foreign commit and stays tripped.** Same shape: arm the
   turn, apply one `Act` successfully, then commit something as the user
   (`app.commit(Box::new(CreateCircle…))` — and, in a second test,
   a `SelectionCommand`, which bumps the revision too and must abort just the
   same), then push a second `Act`. The second action is answered
   `AgentOutcome::Refused` with the ADR 0007 §D4 text; the drawing does **not**
   gain the second entity; every later `Act` in the same turn is refused
   identically; and the turn ends normally when the thread's terminal event
   arrives. The first action stays applied and stays undoable.

10. **One turn, one undo entry.** A turn that applied `n >= 2` actions with no
    foreign commit calls `History::coalesce_last(n, &label)` at turn end, where
    `label` is `format!("Agent: {}", …)` with the trimmed prompt truncated to 40
    characters and an `…` appended when truncated. After the turn:
    `history.len()` grew by exactly 1 over its pre-turn value, `revision()` is
    the same as it was immediately before the coalesce, one `Ctrl+Z` reverses
    the whole turn, and one `Ctrl+Y` reapplies it in order.

11. **A fence-aborted turn is not coalesced, and the operator is told which
    happened.** When `may_coalesce` is false the actions remain individual undo
    entries. In both cases `agent_chat` gains one row with role `note` at turn
    end:
    - coalesced: `Applied 4 actions — Ctrl+Z undoes the whole turn.`
    - not coalesced: `Applied 2 actions — the drawing changed mid-turn, so they stay 2 separate undo steps.`
    - exactly one action: `Applied 1 action — Ctrl+Z undoes it.`
    - zero actions: **no note row at all.**

12. **The user is never blocked.** No dialog flag on `App` is set by arming,
    polling or finishing a turn, and no widget is disabled on `agent_busy`
    except the agent panel's own Send button (which already is). A test commits
    a user command through the normal path while a turn is armed and an `Act` is
    outstanding, and it succeeds.

13. **`tool_definitions()` exposes exactly seven tools, in this order**:
    `create_line`, `create_circle`, `create_arc`, `delete_entity`,
    `move_entity`, `query_entities`, `query_selection`. The two query schemas
    take no parameters (`"properties": {}`, `"required": []`).
    `tools.rs`'s existing `tool_definitions_array_length_is_five` and
    `tool_definitions_names_in_order` tests are **updated, not deleted** — they
    pin 5 tools by index today and break here **by design**.

14. **`query_entities` answers from the live document.** `parse_tool_call`
    accepts the name with any arguments object (including absent/empty), and
    `agent_apply` formats:
    ```
    The drawing has 3 entities. Bed 400.000 × 400.000 mm.
    0: line (0.000, 0.000) → (20.000, 0.000) mm
    1: circle center (10.000, 10.000) mm, r = 5.000 mm
    2: arc center (0.000, 0.000) mm, r = 8.000 mm, 0.0°→90.0° ccw
    ```
    Millimetres to 3 decimals, angles to 1 decimal **in degrees** (the same
    boundary convention `create_arc` already uses). An empty document answers
    `The drawing is empty (0 entities). Bed 400.000 × 400.000 mm.` The bed line
    reports `Document::bed_mm`, never a constant.

15. **`query_selection` answers from the live selection.**
    `2 of 3 entities are selected: 0, 2.` — indices ascending — or
    `Nothing is selected (the drawing has 3 entities).`

16. **Neither query mutates.** `history.revision()`, `history.len()` and
    `document.entities.len()` are identical before and after a query action, and
    a query does **not** advance the fence. A turn made only of queries applies
    zero actions, so AC 11 produces no note row and no coalesce is attempted.

17. **The system prompt discloses the index contract (ADR 0007 §D5).** The
    prompt states, in words: entity handles are **positional indices** into the
    drawing; deleting an entity **renumbers** every higher index down by one;
    the drawing may have changed since the last read, so call `query_entities`
    before a `delete_entity` or `move_entity` that was not derived from the
    current turn; and if an action is refused because the drawing changed, stop
    and tell the operator rather than retrying. It also keeps the existing
    millimetre / degree statement. A bounded scan asserts the presence of the
    key terms (`positional`, `renumber`, `query_entities`) with a positive
    control.

18. **The budget is read and clamped here.** `src/app/agent_turn.rs` computes
    `crate::agent::loop_::clamp_step_budget(app.settings.agent_step_budget)` and
    passes it into the thread. `io/settings.rs` still does not import
    `crate::agent` (bounded scan). A settings value of `0` yields `1` and `200`
    yields `32` at the call site, asserted directly on `agent_turn`'s helper.

19. **The progress repaint is preserved and its reason is written down.**
    `src/app/mod.rs`'s `if self.agent_busy { ctx.request_repaint(); }` survives
    this demand unchanged in behaviour, and gains a comment saying **why it is
    now load-bearing for turn progress and not merely for the spinner**: the
    rendezvous only advances while frames are running, so an app that stops
    painting stalls the turn forever. `src/app/viewport.rs`'s
    `every_repaint_request_in_src_is_conditional` still passes and still counts
    **three** conditional implementation call sites. That scan pins the count
    and that each site sits inside an `if`; it does **not** pin *which*
    condition, so `if true {` would satisfy it. A second scan therefore pins
    that this site's condition is `agent_busy`. A scan and not a runtime test:
    every test here drives `App::update_ui` on its own schedule and never
    consults egui's repaint request, so no headless test can observe this at
    all — see mutation (g). **If — and only if — the AgentState seam in AC 20
    is executed**, both scans' needles and `AGENTS.md` §Event flow → Repaint
    policy are updated to the new field path in the same commit, and the count
    stays three.

20. **The LOC seam is executed only if it is crossed.** Measure
    `src/app/mod.rs` with ADR 0004's `awk` recipe (it is 265 before this work)
    and report the number.
    - If it would exceed **300**, execute the seam the architect already named:
      move the agent fields (`agent_panel_open`, `agent_chat`,
      `agent_input_draft`, `agent_busy`, `agent_rx`, plus the fence, the applied
      counter and the turn label) into a `pub struct AgentState` declared in
      `src/app/agent_turn.rs`, held on `App` as one `pub agent: AgentState`, and
      update every call site including tests.
    - If it lands at or under 300, **do not split**, and simply report the
      number. Between 270 and 300 the ADR 0004 rule is "flag, do not split on
      sight".
    `src/app/agent_turn.rs`, `src/app/agent_poll.rs`, `src/agent/loop_.rs`,
    `src/agent/tools.rs` and `src/app/agent_apply.rs` are each at or under 300
    implementation LOC, measured and reported.

21. **The key never leaves the transport.** The API key appears in no
    `agent_chat` row, no `command_feedback`, no `AgentEvent`, no transcript row
    and no `tracing` line. A test sets a recognisable dummy key
    (`"sk-test-DO-NOT-LEAK"`), drives a full turn, and asserts the substring
    (built with `concat!`) is absent from every `agent_chat` content and from
    `command_feedback`.

22. **Turn-end work is the same on every exit.** `end_turn` is the only place a
    turn ends (ADR 0007 §D11), so the AC 10 coalesce and the AC 11 note row run
    on **all four** exits, under the same `may_coalesce` gate and the same
    applied-action counter — not only on `Done`. An operator whose worker died
    after three applied actions gets the same undo shape as one whose turn
    finished: one `Ctrl+Z` if the fence held, three separate steps if it did
    not. Anything else leaves half a turn's geometry on the bed with no single
    way to take it off again before cutting. The action applied in exit (4)
    **counts**: it is already in `History`, so a coalesce that excluded it would
    strand it as a separate undo step. Exit (4) therefore always produces a note
    row; an exit that applied zero actions produces none (AC 11). Row order at
    turn end is fixed and asserted: the terminal row first (`assistant` or
    `error`), then the `note` row — the tail of the total ordering AC 23 fixes
    for the whole turn.

23. **Every action the agent takes leaves a row in the transcript.** The panel
    shows the final assistant message **and a tool-call transcript**; v1 showed
    only the answer, and an operator who cannot see what the agent did cannot
    catch a wrong `delete_entity` before it costs a part. The rows are produced
    in `agent_apply::apply`, the one place that already knows both the action
    and its narration:
    - an applied action appends one row, role `tool`, content the
      `AgentOutcome::Ok` string **verbatim** — the same sentence handed back to
      the model (LCV-122 AC 9, AC 10), so the operator and the model read the
      same fact and a disagreement between them is impossible by construction;
    - a refused action appends one row, role `refused`, content the
      `AgentOutcome::Refused` string verbatim — the fence text (AC 9) or the
      out-of-range text (LCV-122 AC 6);
    - a **query** appends a `tool` row like any other action. One row per
      action, no variant is special-cased: a rule with an exception is a rule
      someone has to ask about. The listing can be long; that is accepted and
      bounded by the ≤ 32 step budget, and AC 5 of LCV-125 makes long rows wrap.

    A transcript row is **not** an applied action: a query appends its row and
    still advances neither the applied-action counter, nor the fence, nor the
    coalesce (AC 16). Exit (4) appends its `tool` row too — the action was
    applied before the answer was lost, so the drawing really did change and
    the row is the only place the operator can see it.

    **Ordering within a turn is total, and asserted:** the `user` row (AC 3),
    then one row per action in apply order, then the terminal row (AC 7), then
    the `note` row (AC 11, AC 22). No row is ever inserted earlier in
    `agent_chat` than a row already there.

24. **Gates.** `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings`, `cargo test --all` exit 0.

## Expected tests

- **Unit / AC 1, AC 2** — bounded scans over every `src/agent/*.rs`
  implementation section for `crate::document`, `Document`, `History`,
  `Arc<Mutex`, `thread::spawn`, `mpsc::channel`; the LCV-122 exception list is
  asserted **empty**. Each scan shown to discriminate.
- **Unit / AC 3** — `arm_turn` sets `busy`, stores the `Receiver`, records the
  user row, and returns a live `Sender`; `start_turn` on a busy `App` changes
  nothing.
- **Unit / AC 4, AC 5** — `run_agent_turn` with a recording ask-callback: one
  call per tool call, in order, and the returned `AgentOutcome` string is fed
  back as the `tool` message content (assert on the message list handed to the
  send stub). A second test with a callback returning
  `Err(AgentError::Cancelled)` asserts an immediate return and that the send
  stub was called exactly once. **Both of these substitute their own callback
  for the real `ask_ui`, so neither covers the seam**, and neither does AC 8's
  test, which drives `arm_turn` and pushes an `Act` by hand. A third test drives
  `ask_ui` directly, in two deterministic halves and with no timeout: against a
  `Sender` whose `Receiver` is already dropped it returns
  `AgentError::Cancelled` rather than a success; and against a live rendezvous —
  the test thread playing the UI — it puts an `Act` on the channel carrying the
  action intact, blocks, and returns the outcome the UI answered with. Without
  it, a seam that fabricates an answer and never sends an `Act` is caught only
  by the mockito end-to-end test.
- **Integration / AC 6, AC 7** — `tests/lcv123_agent_turn.rs`, `mod harness;`.
  Push `Act`, `Act`, `Done` into the channel **before** running a single frame;
  assert both actions applied in that one frame and the turn terminated in that
  same frame. A second test pushes only `Act` and asserts
  `agent_rx.is_some() && agent_busy` after the frame. A third **drops** the
  event `Sender` without a terminal event and asserts the next frame clears
  `agent_busy` / `agent_rx` and appends an `error` row carrying
  `AGENT_LOST_MESSAGE` — the anti-stall guard. A fourth covers exit (4): push
  one `Act`, **drop its reply `Receiver` before the frame and keep the event
  `Sender` alive** — so no `Disconnected` can arrive and only the dead reply
  channel says anything is wrong — then assert the frame clears `agent_busy` /
  `agent_rx`, that the action really was applied, and that the row it appended
  is **equal to the row the third test asserted** (compare against the
  constant, and against the other exit's `(role, text)` pair; never against a
  copy of the sentence).
- **Integration / AC 8** — the deterministic single-action test described in
  AC 8, including reading the outcome back off the test's own reply receiver.
- **Integration / AC 9** — two tests, one with a user `CreateCircle` and one
  with a `SelectionCommand` as the foreign commit. Both assert: refusal text,
  no second entity, a third `Act` also refused, first action still undoable.
- **Integration / AC 10, AC 11** — a four-action turn: `history.len()` +1,
  `revision()` unchanged across the coalesce, one undo empties what the turn
  drew, one redo restores it, and the `note` row reads the coalesced sentence.
  The fence-aborted counterpart asserts `n` separate entries and the other
  sentence. A one-action turn and a zero-action turn assert their own sentences
  (and the absence of one).
- **Integration / AC 22** — a three-action turn ended by `Disconnected`, and a
  three-action turn ended by exit (4): in both, `history.len()` grew by exactly
  1, one `Ctrl+Z` empties what the turn drew (including exit (4)'s last
  action), and the `note` row is present and sits **after** the `error` row. A
  fence-aborted counterpart ended by `Disconnected` asserts three separate undo
  entries and the other sentence.
- **Integration / AC 23** — three tests on `agent_chat`'s contents, not its
  appearance. (a) A turn of two `CreateLine`s and one `query_entities`: the
  roles, in order, are `user`, `tool`, `tool`, `tool`, `assistant`, `note`; each
  `tool` row's content equals the `AgentOutcome::Ok` string the test read back
  off its own reply receiver; and the `note` row still reads *Applied 2
  actions…* because the query is not an applied action. (b) A fence-aborted
  turn: the refused action's row has role `refused` and content equal to the
  `AgentOutcome::Refused` string, and the applied action's `tool` row precedes
  it. (c) Exit (4): the `tool` row for the applied action, then the `error` row,
  then the `note` row, in that order.
- **Integration / AC 12** — a user commit succeeds while an `Act` is
  outstanding; plus a scan that no `*_open` flag is written by the three turn
  functions.
- **Unit / AC 13** — the updated length (7) and order tests, plus the two query
  schemas' empty `properties` / `required`.
- **Unit / AC 14, AC 15, AC 16** — query formatting on an empty document, on a
  3-entity mixed document, with a non-default bed (assert the bed line follows
  `Document::bed_mm` by running it at two different bed sizes), and on a
  selection of 2 of 3 and of 0 of 3. Plus the no-mutation and no-fence-advance
  assertions.
- **Unit / AC 17** — bounded scan of the system prompt for the three key terms,
  with a positive control (a term that is deliberately absent).
- **Unit / AC 18** — `0 → 1`, `12 → 12`, `200 → 32` through `agent_turn`'s read
  site; bounded scan proving `io/settings.rs` has no `crate::agent`.
- **Unit / AC 19** — the existing
  `every_repaint_request_in_src_is_conditional` still passes unchanged (or, if
  AC 20's seam ran, with its needle updated and the count still three), plus a
  **new** scan on the guard's *condition*: the nearest statement opening the
  block that holds `src/app/mod.rs`'s `request_repaint` must mention
  `agent_busy`. Presence alone is not enough — LCV-120's scan already has that,
  and `if true {` passes it. Run the same scan over a witness
  (`if self.agent_busy {`) and over an `if true {` control before running it
  over the real file, so a needle that could not find a real guard fails here
  instead of passing over the real file for free. This scan, not any runtime
  test, is what answers mutation (g).
- **Integration / AC 21** — the dummy-key absence test.
- **mockito end-to-end / the whole demand** — one test, and it earns its place
  because it is the only thing that proves the CHANGELOG gap is closed. A
  mockito server answers request 1 with two `create_line` tool calls and
  request 2 with final assistant text. `App::default()` with
  `settings.agent_endpoint = server.url()`, a dummy key, `settings_path: None`
  and `autosave_path: None`. Call `start_turn`, then run `App::update_ui`
  frames in a bounded loop (at most 400 frames with a 5 ms sleep, roughly 2 s)
  until `!app.agent_busy`, failing with a named message on timeout. Assert: two
  lines exist at the exact millimetre coordinates, `history.len()` grew by
  exactly 1 (coalesced), the final assistant text is the last `assistant` row,
  and one undo empties the drawing. **This is the only test in the suite that
  runs a real thread against a real socket; if it proves flaky, report it
  rather than weakening the assertions.**
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result**:
  (a) restore `if let Ok(msg) = rx.try_recv()` (drain one per frame) → AC 6
  fails by name;
  (b) clear `agent_rx` after an `Act` → AC 7 fails by name;
  (c) delete the `Disconnected` arm → AC 7's dropped-sender test hangs the
  turn and fails by name;
  (d) make the fence check always `Ok` → AC 9 fails by name;
  (e) drop the `may_coalesce` gate and coalesce unconditionally → AC 11's
  fence-aborted test fails by name;
  (f) make `ask_ui` return a fabricated success without sending `Act` → **not**
  AC 8, which never reaches the seam: it arms the turn and pushes the `Act`
  itself. The mutant is killed by AC 4's dedicated ask-seam test — its live
  half sees the `Sender` dropped without an `Act` and fails on `recv` — and
  end to end by the mockito test, which then finds nothing on the bed;
  (g) delete `if self.agent_busy { ctx.request_repaint(); }` → **the suite
  stays green, and no runtime test can change that.** An earlier revision of
  this demand claimed the mockito end-to-end test times out; it does not. That
  test pumps `App::update_ui` frames on its own schedule and never consults
  egui's repaint request, so it finishes either way. The property at stake is
  about the shipped binary, whose frames egui schedules — which is precisely
  what a headless test replaces. So the mutant is killed by the **source scan**
  in AC 19 on the guard's condition, and only by that. LCV-120's
  `every_repaint_request_in_src_is_conditional` is not sufficient on its own:
  it pins that the call sits inside *an* `if`, which `if true {` satisfies
  while repainting every frame forever. Report the scan's discrimination
  demonstration in place of a mutation result;
  (h) delete the `reply.send` failure arm and let it fall through to the next
  `try_recv` → AC 7's exit-(4) test, which deliberately keeps the event
  `Sender` alive, leaves `agent_busy` set and fails by name;
  (i) make exit (4) end the turn **silently** (`end_turn(app, None)`, the
  pre-amendment behaviour) → AC 7's same-row assertion fails by name;
  (j) skip the coalesce on the two lost exits → AC 22 fails by name;
  (k) write a refusal with role `tool` instead of `refused` → AC 23 (b) fails
  by name;
  (l) append a row for mutating actions only, skipping queries → AC 23 (a)'s
  role sequence fails by name.
- **[manual] smoke, recorded on LCV-089's checklist** — first real prompt
  against OpenRouter with the user's own API key. No CI test may reach a real
  endpoint.

## Test hygiene (mandatory)

- **mockito only in CI.** Real-endpoint validation is a human step (ADR 0007
  §Consequences).
- **Bound every source scan** to the implementation section (slice at the offset
  of the bare `#[cfg(test)]` at column 0) and **build every needle with
  `concat!`**. Canonical correct example: `guard_is_runtime_not_cfg` at
  `src/io/dialogs.rs:179-194`. This project has shipped an un-failable test six
  times; every scan must be **shown to discriminate**, and the demonstration
  reported.
- **Positive controls must be tight.** A control that also passes against the
  mutant is not a control.
- ADR 0002 §A2: `App::default()`, never `App::new()`. §A4 rule 1: no test sends
  `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`. §A4 rule 2 (as amended 2026-09-13): an
  injected path points only at a directory the test owns, and a test that wants
  no write leaves `settings_path` / `autosave_path` as `None`.
- ADR 0006 / ADR 0007 §D10: **no test may cause a settings write to a real
  per-user path**, and a test that injects a `settings_path` must not also set a
  real API key. Every test here uses a dummy.

## Risks

- **The borrow in `poll_agent_rx` is what shaped that function.**
  `&app.agent_rx` cannot be held across `apply(app, …)`, so LCV-122 already
  takes the `Receiver` out with `app.agent_rx.take()` at the top of the loop and
  puts it back only on `Empty`. Preserve that shape: it is what makes AC 7
  explicit rather than incidental — every path that leaves the loop without
  putting the receiver back is a tail call to `end_turn`.
- **A stalled turn is a busy loop.** `agent_busy` stuck true means
  `ctx.request_repaint()` every frame forever, which is exactly the defect
  LCV-120 just closed. All four exit paths — `Done`, `Failed`, `Disconnected`,
  a failed `reply.send` — plus a `start_turn` that spawns a thread that panics,
  must clear it. AC 7 is the guard.
- **The fence is deliberately conservative and will surprise people.** Clicking
  an entity mid-turn bumps the revision through `SelectionCommand` and aborts
  the turn. That is correct — `query_selection` means something different
  afterwards — and it costs a re-prompt in a case that barely happens, since the
  operator is watching a spinner. Do not "fix" it by exempting selection.
- **A tripped fence still costs budget.** After the first refusal the model may
  keep asking; every later `Act` is refused and the step budget (≤32) is what
  bounds the waste. Accepted: it is a handful of refusals on a path that ends
  the turn anyway.
- **Coalescing after an autosave.** The autosave debounce watches
  `History::revision()`, which `coalesce_last` deliberately does not move. An
  autosave that fired mid-turn already wrote the right bytes; the coalesce only
  reshapes the undo stack. Do not "fix" this by bumping the revision — AC 10
  asserts it does not move.
- **Roughly 30 existing tests move with the four changed signatures** —
  `run_agent_turn`, `dispatch_tool_call` (already split by LCV-122),
  `chat_completion` (already changed by LCV-121) and the two `tool_definitions`
  tests that pin exactly five tools by index. None may be deleted silently; the
  before/after counts are reported.

## Notes

- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D1 (the thread owns nothing), §D2 (rendezvous, one at a time), §D3 (one
  channel, reply rides inside), §D4 (the fence), §D5 (index disclosure), §D6
  (coalesce, gated), §D7 (budget, clamped at the read site), §D8 (file
  responsibilities and the three boundary rules), §D11 as amended 2026-09-13
  (**four** turn exits, all through one `end_turn`, and exits (3) and (4)
  showing the operator the same row). This demand implements it; it does not
  re-decide it.
- **No sentence the operator reads is quoted from the code into an AC.** AC 7
  names `agent_poll::AGENT_LOST_MESSAGE` instead of reproducing it, because a
  demand that hardcodes copy forces a demand round-trip every time the wording
  is polished, and this repo has already spent a review round on stale doc
  comments. Non-normative, for the reader only: today the constant reads
  *"Agent turn ended without a reply."* — if that changes, nothing here needs
  to. The sentences in AC 11 are different: nothing in `src/` owns them yet,
  so the demand is where they are decided.
- **`src/app/mod.rs`'s `if self.agent_busy { ctx.request_repaint(); }` is the
  one repaint site this demand must leave alive.** LCV-120 was explicitly told
  not to remove it, and ADR 0007 §Consequences explains why: the rendezvous only
  advances while frames run. It is pinned by AC 19's source scan on the guard's
  condition, **not** by a runtime test and not by mutation (g): every test here
  drives `App::update_ui` itself, so none of them ever asks egui for a frame,
  and deleting the guard leaves the whole suite green.
- `src/agent/panel.rs` importing `crate::app::App` inverts the `app → agent`
  direction. It is left alone deliberately; moving `submit` into
  `src/app/agent_turn.rs` is what resolves the part that mattered.
- Measured before the work (ADR 0004 `awk`): `loop_.rs` 178, `panel.rs` 155,
  `tools.rs` 151, `transport.rs` 116, `src/app/mod.rs` 265,
  `src/app/agent_poll.rs` 28.
- When this lands, `CHANGELOG.md`'s *"The agent can read and narrate the drawing
  but does not yet modify it end-to-end from chat"* stops being true.
  `demand-manager` owns that edit.
- **The transcript is a requirement, not a nicety.** The direction for the
  agent panel was *show the final assistant message **and** a tool-call
  transcript — v1 never did, do not repeat that gap.* AC 23 is where that lands.
  It was previously implied by LCV-125 (whose AC 1 closes the role vocabulary at
  six) and written down in neither demand, which left two render arms with no
  producer. The seam is now explicit in both: **LCV-123 produces rows, LCV-125
  renders them.**
- **LCV-124 and LCV-125 are parallelizable once this demand lands.** They touch
  disjoint files: LCV-124 is `src/agent/classifier.rs` + `src/app/cmdline.rs`;
  LCV-125 is `src/agent/panel.rs` + `src/agent/settings_ui.rs`.
