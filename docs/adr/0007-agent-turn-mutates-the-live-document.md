# ADR 0007 — The agent turn mutates the live document through a fenced rendezvous, never a snapshot

- **Status**: Accepted
- **Amended (1)**: 2026-09-13 — refinement into LCV-121..125 found one internal
  contradiction and three silences. §D2a is new and resolves the contradiction
  (§D4 claimed `get_index` still range-checks, which §D1 forbids it from being
  able to do). §D4 gains the fence's home and its stickiness. §D11 is new and
  covers channel disconnection. §"The 121→123 window" is new and records a
  sequencing hazard. Nothing already decided is reversed.
- **Amended (2)**: 2026-09-13 — LCV-121 shipped (`d584f4d`) and put tool calls on
  the *live* send path, which §"The 121→123 window" had required it not to do.
  That section is rewritten to describe `main` as it is: the window is open now,
  not merely reachable, and the no-release-before-LCV-123 rule is the whole of
  what is left of the mitigation. No decision in §D1..§D11 changes.
- **Amended (3)**: 2026-09-13 — LCV-122's review found §D11 enumerating *three*
  terminal exits where the implementation has four. The fourth — `reply.send`
  failing while the UI thread is holding an `Act` — is now named and as
  normative as the other three, and the property all four share ("exactly one
  function ends a turn") is stated once instead of being re-derived per arm.
  §D8's one-line gloss for `agent_poll.rs` follows. No decision in §D1..§D10
  changes.
- **Amended (4)**: 2026-09-13 — LCV-123 shipped (`af5ef82`) and closed the
  hazard §"The 121→123 window" was tracking. That section is rewritten into the
  past tense and kept as a dated historical record: it is no longer a live
  warning and it no longer bars a release. `panel.rs::submit` is deleted,
  `src/agent/` names no document type, and two independent scans — both proved
  non-vacuous by mutation — hold that. No decision in §D1..§D11 changes.
- **Amended (5)**: 2026-09-13 — §D9's "open product question, not decided
  here" is now decided, by the user, against rule 4: **only `:` and `/ai` reach
  the model.** Rule 4 is rewritten and §D9a records what the flip may not
  disturb. This reverses rule 4 and nothing else — §D11's closure property is
  untouched, because the flip removes an arming path and adds none.
- **Date**: 2026-09-13
- **Deciders**: architect (Marco 2 / Agent Harness MVP)

## Context

`CHANGELOG.md` records the gap this ADR closes: *"The agent can read and
narrate the drawing but does not yet modify it end-to-end from chat."*

The mechanism behind that sentence is in `src/agent/panel.rs`. `submit` spawns
the background thread with a **throwaway document**:

```rust
let mut doc = Document::default();
let mut history = History::default();
let result = crate::agent::run_agent_turn(&text, &endpoint, &api_key, &mut doc, &mut history);
```

`run_agent_turn` takes `&mut Document` and `&mut History`, `tools.rs` commits
real `CreateLine` / `DeleteEntities` / `MoveEntities` commands against them, and
then the whole thing is dropped on the floor when the thread ends. Only the
final assistant string survives, over an `mpsc::Receiver<AgentPanelMsg>`. The
agent narrates work it did to a document nobody can see.

The pieces are all present and all correct in isolation: a blocking transport
(`transport.rs`), a JSON tool registry that already builds the right commands
(`tools.rs`), a loop with an injectable send/dispatch seam and a call cap
(`loop_.rs`), a chat panel (`panel.rs`), a once-per-frame poll
(`src/app/agent_poll.rs`), `agent_busy` / `agent_rx` on `App`. What is missing
is the one decision nobody has made: **how a thread that is not allowed to own
the `Document` causes the `Document` to change.**

### Constraints that bind the answer

- AGENTS.md §"State and mutation": all entity mutation goes through `Command`
  plus `History`; `App` owns the mutable state; setters on `App` are the only
  mutation path.
- `Document` is deliberately **not `Clone`** (`src/document/state.rs`): "a full
  document clone would be a silent O(n) cost, and no current consumer needs
  one."
- Entity handles are **positional indices** into `Document::entities`, by that
  same file's own admission ("until a stable-id demand argues otherwise").
- `History::revision()` is already a monotonic, single-writer sequencer of every
  mutation (ADR 0002 §B).
- Async is a plain `std::thread` over `reqwest::blocking`; no `tokio`.
- 300 implementation LOC per file (ADR 0004). `loop_.rs` is at 178, `panel.rs`
  at 155, `tools.rs` at 151 before any of this work lands.

### The proposal on the table, and why half of it is wrong

The team lead proposed: the thread returns a `Vec<AgentAction>` at the end of
the turn; `App::update_ui` replays them as commands; queries are served from a
**clone of the entity vector sent to the thread at turn start**; no
`Arc<Mutex<Document>>`; no tokio.

The "no mutex, no tokio, replay as `Command`s through `App`" half is right and
is adopted below. The batch-and-replay half does not survive contact with a
multi-step turn:

1. **A deferred batch makes every tool result a fabrication.** The loop feeds
   `tool`-role results back to the model between HTTP calls. If nothing has been
   applied yet, the string the model reads is a prediction. `"Entity 7 deleted."`
   is asserted before anything has looked at whether entity 7 exists.
2. **Step *k+1* depends on the effect of step *k*.** Delete index 3, then move
   index 7: after the delete, the entity the model called 7 is at 6. To answer
   honestly from a batch you must maintain a shadow document in the thread —
   which is the `Document` clone the same proposal rules out, now with a
   divergence bug waiting in it.
3. **A snapshot goes stale exactly when it matters.** It is taken at turn start
   and consulted several seconds later, across an interval in which the user is
   still drawing. Answering a query from it is answering about a drawing that no
   longer exists.

The stale-index question the lead raised — *the model asks to move entity 7 and
the user deleted entity 7 two seconds ago* — has a specific shape that matters:
if index 7 is now **out of range** that is catchable, but only somewhere that
can see the document (§D2a); if it is still in range but is now **a different
entity**, nothing anywhere notices and the drawing is silently corrupted. No
amount of care inside the thread can see either one. Both have to be checked
from outside.

## Decision

### D1 — The thread owns no document state at all. Not the document, not a clone, not a snapshot

The background thread holds the prompt, the endpoint, the key, the model, the
step budget and the message list. It holds no `Document`, no `History`, no
`Vec<Entity>`, no `Arc<Mutex<_>>` and no snapshot of any kind. `Document` stays
`!Clone`; a demand that adds `Clone` to it for the agent's benefit is a review
blocker and belongs back here first.

### D2 — Actions and queries are a rendezvous against the live document, one at a time

The thread↔UI protocol is request/response, not fire-and-forget:

```rust
// src/agent/bridge.rs — kernel-pure: no egui, no eframe, no rfd, no reqwest.
pub enum AgentAction {
    CreateLine { x1: f64, y1: f64, x2: f64, y2: f64 },
    CreateCircle { cx: f64, cy: f64, r: f64 },
    CreateArc { cx: f64, cy: f64, r: f64, start: f64, end: f64, ccw: bool },
    Delete { index: usize },
    Move { index: usize, dx: f64, dy: f64 },
    QueryEntities,
    QuerySelection,
}

/// What the UI thread answers. Both variants go back to the model as the
/// `tool` result — a refusal is information, not a failure.
pub enum AgentOutcome { Ok(String), Refused(String) }

pub enum AgentEvent {
    /// Apply `action` to the live document and answer down `reply`.
    Act { action: AgentAction, reply: std::sync::mpsc::Sender<AgentOutcome> },
    /// Terminal: the final assistant text.
    Done(String),
    /// Terminal: the turn failed.
    Failed(String),
}
```

The thread sends `Act`, then **blocks on the reply receiver** until the UI
thread's next frame applies it and answers. One frame of latency (~16 ms)
against a multi-second LLM round trip is free. Every result the model reads is
the outcome of a mutation that really happened, against the document the
operator is really looking at, at the moment it happened.

Queries ride the same path — `QueryEntities` is answered by formatting the live
`Document`. That is why no snapshot is needed: the read is as fresh as the write.

`recv()` returning `Err(RecvError)` is the cancellation signal. Dropping the
`Sender` without answering — the app is closing, the turn was abandoned, the
frame loop is gone — aborts the turn cleanly with no extra machinery.

### D2a — Validation splits where the document does: shape in the thread, range at the apply site

*(Added by amendment. §D4 originally claimed out-of-range indices were "still
caught where they already are, in `get_index`". They are not, and cannot be:
`get_index` takes a `doc_len`, and §D1 forbids the thread from holding anything
it could compute one from. The ADR was asking a pure function to validate
against data the design denies it. Resolved as product-owner proposed during the
refinement of LCV-122.)*

`tools.rs::parse_tool_call` keeps every check that is a property of the
**arguments alone**, and keeps them hand-rolled, with its per-field error
messages — that is the whole reason §Alternatives rejected deriving
`AgentAction` from serde:

- missing field, wrong JSON type;
- non-finite numbers;
- a radius that is not positive and finite;
- an index that is not a non-negative integer.

The one check that is a property of the **document** — `index < entities.len()`
— moves to `src/app/agent_apply.rs`, where the document is in hand, and yields
`AgentOutcome::Refused` naming the index and the current entity count. A refusal
is information: it goes back to the model as the tool result and the model can
re-`query_entities` and retry.

The two failures therefore travel different routes, and that is deliberate:

- a **shape** failure never leaves the thread. No `Act` is sent, no frame is
  waited on; the thread turns `ToolCallError` straight into the `tool` result.
  A malformed argument list has nothing to do with the document and must not
  reach the frame loop.
- a **range** failure is an `Act` that was sent, applied against nothing, and
  answered `Refused`.

`get_index` loses its `doc_len` parameter and its out-of-range arm; that arm's
wording is reproduced at the apply site.

### D3 — One thread→UI channel carrying a richer enum; the reply channel rides inside the message

`AgentPanelMsg` is renamed to `AgentEvent`, moved out of the egui-importing
`panel.rs` into the pure `bridge.rs`, and gains the `Act` variant. `Reply` and
`Error` become `Done` and `Failed`. `App` keeps **exactly one** agent channel
field (`agent_rx`), because the reply `Sender` is constructed per request by the
thread and carried inside `Act` rather than parked on `App`.

Two long-lived channels were considered and rejected: a second `App` field, a
second thing to clear on every exit path, and no gain — there is never more than
one outstanding request, because the thread blocks.

### D4 — `History::revision()` is the fence; a foreign commit aborts the turn

Every mutation in this program bumps `History::revision()` exactly once, and the
UI thread is its only writer. That makes it a complete sequencer, already built
and already tested (ADR 0002 §B).

The fence is a type, `TurnFence`, and it lives in **`src/app/agent_turn.rs`** —
declared there by LCV-122 and driven there by LCV-123, so it is in its final
home from the first commit that mentions it. It is a struct over two `u64`s and
a `bool`; it needs no `App`, no `Document` and no egui, so it is unit-testable
against plain integers wherever it sits.

*(It does not go in `bridge.rs`, which was the alternative considered during
refinement. `bridge.rs` is the protocol, shared with and visible to the worker
thread. The fence is the one thing in this design the thread is specifically not
allowed to evaluate — putting its type where the thread can reach it is an
invitation to a future demand evaluating it on the wrong side. The coalesce gate
in §D6 is the same revision arithmetic and belongs in the same file.)*

`agent_turn.rs` records `expected = history.revision()` when the turn starts,
and before applying each `Act`:

- if `history.revision() != expected`, the user has committed something since
  the model last looked. The action is **refused** with a message that says so
  (`AgentOutcome::Refused`), and the turn ends after that result reaches the
  model. The already-applied actions stay applied and stay undoable.
- otherwise apply, then `expected = history.revision()` — which is
  `expected + 1`.

This is what closes the move-entity-7 hole. The fence proves that no mutation
the model does not know about happened between the model's read and its write,
so "in range but now a different entity" cannot occur while the fence holds.
Out-of-range is a separate check at a separate site — §D2a.

**A tripped fence is sticky for the rest of the turn.** It does not re-arm, not
even after a `QueryEntities` that would give the model a fresh read. Every
subsequent action is refused with the same message, each refusal consumes one
step of the budget, and the turn ends when the model stops asking or the budget
runs out — at most 32 wasted round trips, which is the bound that makes "do
nothing cleverer" affordable.

Stickiness is not just conservatism. It is what keeps §D6's coalesce gate sound.
Once tripped, the agent commits nothing further, so the `n` entries the gate
wants to fold are still contiguous at the top of the undo stack or the gate's
equality fails and it declines to fold — two outcomes, both correct. A
re-arming fence would resume committing *after* a foreign commit had landed in
the middle of the run, breaking the contiguity the gate depends on and requiring
either two composites or a cleverer gate. That is the corruption door of
§Alternatives' "defer application" entry, arriving by a third route. Refuse it
the same way.

The fence is otherwise deliberately conservative: `SelectionCommand` bumps the revision
too, so clicking an entity mid-turn aborts. That is correct — `query_selection`
means something different afterwards — and it costs a re-prompt in a case that
barely happens, since the user is watching a spinner.

**The user is never blocked.** No modal, no greyed-out canvas, no lock. Drawing
during a turn is allowed; it just ends the turn.

### D5 — Indices shift, so the protocol says so out loud

Positional indices are the handle this milestone ships with. Within a single
turn the agent's *own* deletes renumber everything above them, and the fence
cannot help with that because the agent knows it did it. Three cheap
requirements make it visible instead of silent:

- the system prompt states that entity indices are positional and that a delete
  renumbers every higher index down by one;
- every mutating outcome reports the resulting entity count, and a delete says
  which indices shifted;
- `Delete` and `Move` outcomes echo a one-line description of the entity they
  touched, so a wrong target is legible in the transcript instead of invisible.

The real fix is stable entity IDs. That is a separate ADR — see §Deferred.

### D6 — One turn is one undo entry, produced by coalescing, gated on the fence

Twenty lines from one prompt must not leave twenty `Ctrl+Z` presses. Partial
rejection of half a turn is a non-goal: R14 does not offer it for its own
commands either, and the cure is to undo and re-prompt.

The mechanism must not defer application (D2 forbids that), so:

1. each action commits immediately and individually through `App::commit`, in
   order — the operator watches the geometry appear, and the undo stack is never
   out of chronological order;
2. at turn end, `History::coalesce_last(n, label)` pops the last `n` entries,
   wraps them in a new `document::commands::CompositeCommand` (`do_` forward,
   `undo` in reverse) and pushes that back as one entry. Nothing is re-run and
   the revision does not move: the document did not change.
3. Step 2 runs **only if** `history.revision() - revision_at_turn_start == n`,
   which proves the `n` entries on top of the stack are exactly the agent's,
   contiguous and unmixed. If the fence aborted the turn the equality fails, the
   coalesce is skipped, and the agent's actions remain as individual undo
   entries — correct, if less tidy.

`coalesce_last` tolerates `n` greater than the stack depth (coalesce what is
there) because `HISTORY_DEPTH` is 200 and the step budget caps at 32.

### D7 — The step budget is configurable, and the range lives with the loop

`MAX_TOOL_CALLS_PER_TURN = 10` is retired. `loop_` takes the budget as a
parameter. The three constants — `AGENT_STEP_BUDGET_DEFAULT: u8 = 12`,
`AGENT_STEP_BUDGET_MIN: u8 = 1`, `AGENT_STEP_BUDGET_MAX: u8 = 32` — live in
`src/agent/loop_.rs`, next to the code that enforces them.

`Settings` gains `agent_step_budget: u8` and `agent_model: String` (default
`"anthropic/claude-sonnet-4.6"`, matching the OpenRouter endpoint that is
already the default), both with `#[serde(default = …)]` per the existing
field-by-field rule. A hand-edited settings file can hold anything, so the value
is clamped **where it is read**, in `src/app/agent_turn.rs`, through
`loop_::clamp_step_budget`. `io/settings.rs` does not import `crate::agent`;
that edge would invert the layering for no benefit.

### D8 — Where each file's responsibility sits

```
src/agent/
  wire.rs          serde types for the OpenAI chat + tool-call protocol.      pure
  transport.rs     the ONLY file that may import reqwest. HTTP + status map.  pure of egui
  tools.rs         tool_definitions() + parse_tool_call(name,&Value)->AgentAction
  bridge.rs        AgentAction / AgentOutcome / AgentEvent.                   pure
  loop_.rs         control flow only: send, cap, feed tool results back.      pure
  classifier.rs    `:` / `/ai` routing over cmdline::parse.                   pure
  settings_ui.rs   egui form.
  panel.rs         egui chat + tool-call transcript. Renders and reports.
src/app/
  agent_turn.rs    spawn the thread; TurnFence (D4); coalesce gate (D6).
  agent_apply.rs   AgentAction -> Box<dyn Command> -> App::commit; range check
                   (D2a); outcomes.
  agent_poll.rs    drain AgentEvent; dispatch; answer; end the turn through
                   end_turn on any of D11's four exits.
```

Three rules follow, and they are the ones reviewers check:

- **Only `panel.rs` and `settings_ui.rs` may import `egui`.** No file under
  `src/agent/` may import `eframe` or `rfd`. This extends the AGENTS.md purity
  list rather than replacing it.
- **Only `transport.rs` may import `reqwest`**, as its own module header already
  claims.
- **`panel.rs` renders and reports. It does not spawn threads and does not
  construct a `Document` or a `History`.** That sentence is the whole defect
  this ADR closes, stated as a rule.

The wire types currently duplicated between `loop_.rs` (public `ToolCall`,
`AssistantMessage`, `Choice`, `ChatResponse`) and `transport.rs` (private
`ChatResponse`, `Choice`, `ChoiceMessage`) collapse into `wire.rs`. That
deduplication is what keeps `transport.rs` and `loop_.rs` under the cap once
tool calls, a `tool` role and status mapping are added to them.

### D9 — Command-line routing precedence

`classifier.rs` is pure and takes the availability of the agent as data:
`classify(raw: &str, agent_available: bool) -> Route`. Precedence, highest
first:

1. **raw-input mode wins over everything.** `src/app/cmdline.rs::submit` already
   returns early when `tool_manager.wants_raw_input()`. The classifier is called
   *after* that check, never before, or `TEXT`'s string would be posted to an
   LLM.
2. `:` or `/ai` prefix → `Route::Agent(rest)`, even when the remainder would
   parse as a CAD command. This is the escape hatch and it must be absolute.
3. `cmdline::parse(raw)` returning anything other than `Unknown` → `Route::Cad`.
   CAD verbs, toggles, zoom, coordinates and a bare Enter always win.
4. `Unknown` → `Route::Cad`, **always**. An unrecognised bare line answers
   `Unknown command: "…"` locally and makes no network call, whatever the
   settings say.

`agent_available` is "a non-empty API key is configured". Without one, a
prefixed line answers `! Agent unavailable: set the API key in Help > Agent
settings` and nothing is spawned.

*(Rule 4 as shipped in LCV-124 read: `Unknown` → `Route::Agent` when
`agent_available`, else `Route::Cad`. This ADR recorded that as an "open product
question, not decided here" and handed it to `product-owner`. Amendment (5)
closes it: the user has decided that free-form-to-agent is a v1 behaviour this
product does not carry forward. The reasoning is cost, not purity — the grammar
knows only single-letter aliases, so the most ordinary thing an R14 operator can
type, the command's own name, is `Unknown` and becomes a paid round trip. The
architecture always supported either answer: it is one arm in `classify`.)*

### D9a — What the rule-4 flip may not disturb

*(Added by amendment (5).)*

**§D11 is not in scope of the flip, and the demand must not be written as if it
were.** §D11 constrains how a turn **ends**: `agent_poll::end_turn` is the only
writer of `agent_busy = false` and the only clearer of `agent_rx` after startup.
Arming is a separate, also-single site — `agent_turn::arm_turn`, reached only
through `agent_turn::start_turn`. Rule 4 is neither of those: it decides whether
a *caller* ever reaches `start_turn`. **Deleting a caller cannot add a writer.**
`tests/lcv129_agent_timeout_and_cancel.rs`'s single-writer scan pins file paths,
not call graphs, and stays green and non-vacuous across the flip. A demand that
flips rule 4 and also touches `agent_poll.rs` or `arm_turn` has scope creep in
it; say so in review.

Three things the flip must preserve, each of which is a way to get it wrong:

1. **`classify` keeps its arity and its purity.** `agent_available` stays a
   parameter: it is still what separates `Route::Agent` from `Route::Unavailable`
   on a **prefixed** line. Dropping it to a one-argument `classify` would move
   the "is a key configured" question somewhere less pure and re-open
   `src/agent/classifier.rs`'s import scan.
2. **The escape hatches stay absolute.** Precedence rules 1–3 do not move.
   `:line` still goes to the model even though `line` will by then parse as CAD,
   and raw-input mode still wins over everything.
3. **The behaviour LCV-124 pinned by name inverts, and must be re-pinned, not
   deleted.** `tests/lcv124_command_line_routing.rs`'s "with a key configured
   `lien` is sent" asserts the accepted hazard. Under the flip `lien` stays CAD.
   That test is the flip's best witness once inverted — a demand that quietly
   removes it loses the only assertion that bare text does not reach the model.

**Consequence for the grammar demand that precedes this one.** Once rule 4 is
`Route::Cad`, every line the grammar does not recognise — `z`, a bare `zoom`,
`zoom sideways`, `lien` — is free and local by construction. So the grammar
demand must **not** build `is_reserved_word`, a `CommandInput::Incomplete`
variant, or any other mechanism to hold specific words back from the model: it
would be deleted by the very next demand. The window between the two demands, in
which `z` and `zoom` still reach the model with a key configured, is the status
quo and is accepted.

### D10 — The API key

The key stays plaintext in `settings.json` for this milestone, and the settings
UI must say so in words next to the field. It is cloned into the thread at
submit (as it already is) and it may not appear anywhere else: not in
`AgentEvent`, not in `agent_chat`, not in `command_feedback`, not in the
transcript, not in `tracing` output, not in an error string. `TransportError`
already truncates the *response* body; the *request* is never logged.

[ADR 0006](0006-real-user-paths-are-injected.md) already prevents a test from
writing the key to the developer's real file, because `persist_settings()` is a
no-op without an injected `settings_path`. One rule is added on top: **a test
that injects a `settings_path` must not also set a real API key.** `mockito`
tests use a dummy.

### D11 — A turn has four exits; every one of them goes through one function, and that function clears `agent_busy`

*(Added by amendment (1); the ADR was silent and the silence has teeth.
Amendment (3) corrected the count from three exits to four and replaced the
enumeration with the invariant underneath it.)*

`agent_busy` is not merely the spinner's flag. `App::update_ui` reads it to
decide whether to `ctx.request_repaint()`, and under §D2 it is also what keeps
frames turning so an in-flight rendezvous can advance. That makes it a
**liveness invariant with two directions**: while it is `true` the app never
idles, and the rendezvous only progresses while it is `true`.

So a turn that ends without saying so is not a cosmetic bug. A worker thread
that panics, or returns after cancellation, or is killed at shutdown, leaves
`agent_busy` stuck `true`, and the app then requests a repaint every frame for
the rest of the session — silently reopening LCV-120, the demand that shipped to
stop exactly that, with a symptom that surfaces nowhere near the agent.

Therefore, stated once rather than re-derived per arm:

> **Exactly one function ends a turn.** `agent_poll::end_turn` is the only place
> in the program that writes `agent_busy = false` or clears `agent_rx` after
> startup, and every path that leaves `poll_agent_rx` without putting the
> receiver back is a tail call to it. Liveness is checked by reading that one
> function and the arms that reach it — not by proving the same property once
> per arm and trusting the list to be complete.

The list was not complete. There are **four** such paths:

1. **`Done`** — the turn succeeded. Chat row: the assistant's text, role
   `assistant`.
2. **`Failed`** — the worker reported an error. Chat row: the error, role
   `error`.
3. **`TryRecvError::Disconnected`** — the worker's `Sender` dropped without a
   verdict: it panicked, it returned after cancellation, or the app is closing.
   Chat row: `agent_poll::AGENT_LOST_MESSAGE`, role `error`.
4. **`reply.send(outcome)` returning `Err`** — the UI thread applied an `Act`
   and found nobody left waiting for the answer. Chat row: the same message and
   the same role as (3), for the reason two paragraphs down.

The event channel's `Sender` is **moved into the thread closure** and held
nowhere else, so a panicking or returning thread closes it as a matter of
course. There is no path by which the thread stops and the channel stays open.

**(4) is (3) observed one instant earlier.** The reply `Sender` is built per
request and carried inside the `Act` (§D3), and a live worker is blocked on its
other half until the answer arrives (§D2) — so it cannot drop the receiving end
while an `Act` is outstanding. `Err` from `reply.send` therefore means the
worker is gone, which means the event channel is closed too; had arm (4) fallen
through instead of returning, arm (3) would have fired on the next iteration
and written that row. Silence at (4) does not report a different fact. It
withholds the one (3) would have printed a microsecond later.

**An already-applied action is not a reason for silence.** In (4) the `Act` was
applied before the send failed; it stays applied and stays undoable, exactly as
§D4 leaves the already-applied actions of a turn whose fence trips. The
objection that a row would then misdescribe the drawing proves too much: (3)
also fires after a run of `Act`s that were applied and answered, and this
section has required a row there since amendment (1). `AGENT_LOST_MESSAGE`
describes the **turn**, not the last action — it says the turn ended with no
reply, which is exactly what happened. What the operator otherwise cannot tell
apart is a turn that *stopped* from a turn that *finished*, and after (4) the
drawing has just changed, which is the worst moment to leave that ambiguous. If
the `error` role ever reads as too alarming, change the role or the wording for
**both** exits together; two spellings of "the worker is gone" must never show
the operator different things.

**Arm (4) returns; it does not fall through.** Deleting it and letting the next
`try_recv` report `Disconnected` is the tempting simplification, and it is
unsound: it assumes a dead reply channel implies a closed event channel *in the
same instant*. Where it does not — a future worker that drops the reply end
while still alive, a send racing the closure's drop — `try_recv` answers
`Empty`, the receiver goes back, and `agent_busy` latches `true` for the rest of
the session. The explicit arm is what makes a dead reply channel sufficient on
its own.

A fifth exit may yet exist; the rule it must obey is the invariant above, not
this list. End the turn through `end_turn`, where whatever turn-end work §D6's
coalesce gate needs also happens, once, for all of them. A reviewer checks that
every terminal arm is a tail call to `end_turn`, and a test mutating any one of
them into leaving `agent_busy` set must fail.

## Consequences

**Easier.**

- Undo, redo, autosave, dirty tracking, the title bar and the discard
  confirmation all work on agent edits for free, because agent edits are
  ordinary `Command`s committed on the UI thread.
- Every tool result the model reads is true. There is no second source of truth
  to diverge.
- `Document` stays `!Clone` and `App` stays the single owner of mutable state.
  No lock, no interior mutability, no `Send` bound on anything in `document/`.
- Cancellation and app-shutdown are the same code path as a dropped channel.
- The whole turn is unit-testable with no HTTP and no egui: `loop_` takes a send
  closure and an ask closure, and `agent_apply` takes `&mut App`.

**Harder, and accepted.**

- Drawing during a turn ends the turn. The operator loses the rest of an agent
  reply they were not watching. The alternative — replaying stale indices — is a
  corrupted drawing, which is worse.
- One frame of latency per tool call, and a turn only progresses while frames
  are running. `App::update_ui`'s `if self.agent_busy { ctx.request_repaint(); }`
  is now load-bearing for progress, not just for the spinner. **LCV-120 must not
  remove it** while fixing `src/app/viewport.rs:46`.
- The rewrite is not additive. `run_agent_turn`'s signature, `dispatch_tool_call`'s
  signature, `chat_completion`'s return type, `Message`'s shape and
  `AgentPanelMsg`'s name all change, and roughly 30 existing unit tests move with
  them. See §Where the existing code resists.
- Two undo behaviours exist: coalesced (normal) and per-action (fence aborted).
  Both are correct; the transcript makes it visible which happened.

**Committed to.**

- Positional indices as the agent's entity handle, with the renumbering stated
  in the prompt and the outcomes, until a stable-id ADR replaces them.
- The step budget as a `Settings` field clamped at the read site.
- Real-endpoint behaviour is validated by a human, not by CI: `mockito` covers
  every code path, and **"first real prompt against OpenRouter with the user's
  own key"** joins LCV-089's existing manual smoke checklist.

## The 121→123 window — a sequencing hazard, now closed

*(Added by amendment (1), rewritten by amendment (2), closed by amendment (4)
on 2026-09-13. Kept as a dated record rather than deleted: the hazard was real,
it was tracked, and it is over. **Nothing in this section is a live warning, and
nothing in it bars a release.**)*

This decision was delivered by LCV-121 (transport speaks tool calls), LCV-122
(the bridge) and LCV-123 (the turn runs against the real document). While those
three were in flight, `main` carried a silent defect. This section existed to
make sure no release was cut on top of it, and it is preserved so that the next
person tempted to ship a capability one demand ahead of the thing that makes it
honest can see what it cost.

### What the window was

`panel.rs::submit` dispatched tool calls against a throwaway
`Document::default()` / `History::default()` pair. From LCV-121 (`d584f4d`)
that throwaway was **live**: `run_agent_turn` built `tools::tool_definitions()`
once per turn, put a `tools` key on every request, and dispatched the
`tool_calls` array that came back. So the agent created, moved and deleted real
geometry in a `Document` nobody could see, and then reported the work as done —
a *worse* product than the one that shipped, and a silent one. `main` compiled,
linted and tested green throughout, because every test handed `run_agent_turn`
the very document it mutated.

### Why it was allowed to open

The original mitigation was that **LCV-121 and LCV-122 would deliver the
capability without enabling it** — leave the live send path alone, prove the new
code against `mockito` only, and let LCV-123 flip the switch in one commit.
**LCV-121's demand overrode that deliberately and on the record**: its
acceptance criteria 9 and 11 required `run_agent_turn` to forward the model and
the step budget through the live path, its §Risks named dispatch into the
throwaway document as the expected consequence, and its §Notes marked the
`_tool_defs` placeholder as the thing being upgraded. The trade was defensible
— a `tools`-less send path would have meant building and testing a second send
path only to delete it two demands later — but it spent the mitigation, and what
was left was a deadline: **no tag and no release until LCV-123 landed.**

LCV-121 did not widen the window beyond that. `panel.rs::submit` still built the
same throwaway pair it always had; nothing else about the panel's dispatch
changed.

### What closed it

**LCV-123 landed and was approved at `af5ef82`.** The deadline is discharged and
releases are unblocked as far as this ADR is concerned.

The throwaway is not merely unused — it is gone, and three independent checks
hold it gone:

1. **`panel.rs::submit` was deleted outright.** `src/agent/panel.rs` renders and
   reports: it constructs no document, spawns no thread and owns no channel. The
   turn runs on the UI thread through `src/app/agent_turn.rs::start_turn`,
   exactly as §D1..§D4 require.

2. **`src/agent/` names no document type at all.**
   `AGENT_DOCUMENT_EXCEPTIONS` in `tests/lcv122_source_scans.rs` is
   `const AGENT_DOCUMENT_EXCEPTIONS: [&str; 0] = [];`, and the test asserts it
   empty **at runtime**, bound through a `&[&str]` slice so the assertion is a
   real check rather than a compile-time tautology the optimiser folds away.
   Re-opening that list needs an ADR; `src/agent/mod.rs`'s module doc says so.

3. **Both layers were shown to be non-vacuous by mutation.** The LCV-123
   reviewer put `use crate::document::{Document, History};` back into
   `src/agent/panel.rs`, and the scan failed by name. They then re-opened the
   exception list to `["panel.rs"]` to smuggle the import past it, and the scan
   failed again — on a second, independent assertion. `panel.rs` additionally
   carries its own in-file scan, `the_panel_renders_and_reports`, which first
   checks each of its five needles against a witness string spelling out what
   the old `submit` did, so a misspelt needle fails loudly instead of passing
   vacuously.

The release-hazard banners this section spawned are cleared: `PLAN.md` keeps its
append-only log entry as written, and `docs/product/backlog.md`'s banner was
removed at `73effdb`. This section was the last one standing, and it is now
history.

## Alternatives considered

- **`Arc<Mutex<Document>>` shared with the thread** — puts a lock in the frame
  loop, lets a background thread mutate outside `App::commit`, and breaks
  AGENTS.md §"State and mutation" outright. Rejected, as the lead proposed.
- **Batch `Vec<AgentAction>` replayed at turn end** — every tool result becomes
  a prediction and multi-step index arithmetic needs a shadow document.
  Rejected; see §Context.
- **Entity-vector snapshot sent to the thread at turn start** — stale by
  construction at exactly the moment it is consulted, and it reintroduces the
  document clone the same proposal rules out. Rejected; rendezvous reads are
  fresher and cheaper.
- **`AgentAction` as a serde enum deserialized straight from the tool-call
  arguments** — free parsing, but it throws away `tools.rs`'s finite-number,
  positive-radius and non-negative-integer checks and its per-field error
  messages, which is most of the value of that file. Rejected; `tools.rs` keeps
  hand-rolled validation and produces `AgentAction`.
- **Block the canvas for the duration of a turn** — honest, and hostile. A
  five-second modal every time someone asks a question is not a CAD tool.
- **One undo entry per action** — safe and trivially ordered, but twenty
  `Ctrl+Z` presses to reverse one sentence. Kept only as the fence-aborted
  fallback.
- **Defer application and compose one command at turn end** — would need
  `History::push_done` (record without running) and puts the agent's entries on
  the stack *after* a user commit that happened *before* them, so undoing the
  composite would reverse stale indices. Rejected: this is the corruption the
  ADR exists to prevent, arriving by a different door.
- **Second long-lived reply channel on `App`** — one more field, one more thing
  to clear on every exit path, no gain over a `Sender` carried in the message.
- **`tokio`** — AGENTS.md was already corrected on this. `reqwest::blocking` on
  a `std::thread` is the whole requirement.

## Deferred to their own ADRs

- **Stable entity IDs.** The real fix for D5. It bumps `SCHEMA_VERSION`, and
  touches the autosave envelope, SVG import, `Selection`, every `Command`'s
  capture strategy and every index-based test. Too large to hide inside this
  milestone, and the fence makes it non-urgent.
- **Secret storage for the API key** (OS keyring / `secret-service`). D10 ships
  plaintext by decision and says so in the UI.

## Revisit criteria

- A turn needs to outlive the frame loop (background batch, queued prompts) —
  the rendezvous assumes frames are running; revisit here first.
- More than one turn may be in flight at once — the single `agent_rx` field and
  the single fence both assume one.
- Stable IDs land — D4's fence stays useful but D5 is superseded.
- `Document` acquires `Clone` for any reason — re-read D1 before using it here.
