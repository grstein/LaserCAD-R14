# ADR 0007 — The agent turn mutates the live document through a fenced rendezvous, never a snapshot

- **Status**: Accepted
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
if index 7 is now **out of range** the existing `get_index` check refuses it
loudly; if it is still in range but is now **a different entity**, nothing
anywhere notices and the drawing is silently corrupted. No amount of care inside
the thread can see that. It has to be fenced from outside.

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

`src/app/agent_turn.rs` records `expected = history.revision()` when the turn
starts, and before applying each `Act`:

- if `history.revision() != expected`, the user has committed something since
  the model last looked. The action is **refused** with a message that says so
  (`AgentOutcome::Refused`), and the turn ends after that result reaches the
  model. The already-applied actions stay applied and stay undoable.
- otherwise apply, then `expected = history.revision()` — which is
  `expected + 1`.

This is what closes the move-entity-7 hole. The fence proves that no mutation
the model does not know about happened between the model's read and its write,
so "in range but now a different entity" cannot occur while the fence holds.
Out-of-range is still caught where it already is, in `get_index`.

The fence is deliberately conservative: `SelectionCommand` bumps the revision
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
  agent_turn.rs    spawn the thread, hold the fence, coalesce at turn end.
  agent_apply.rs   AgentAction -> Box<dyn Command> -> App::commit; outcomes.
  agent_poll.rs    drain AgentEvent until Empty; dispatch; answer.
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
4. `Unknown` → `Route::Agent` when `agent_available`, else `Route::Cad` so the
   existing `Unknown command: "…"` feedback is produced unchanged.

`agent_available` is "a non-empty API key is configured". Without one, a
prefixed line answers `! Agent unavailable: set the API key in Help > Agent
settings` and nothing is spawned.

**Open product question, not decided here:** whether bare unprefixed free text
(rule 4) should reach the agent at all, or only prefixed text should. Today a
typo answers `Unknown command: "lien"`; under rule 4 with a key configured it
would be posted to an LLM. That is a product call for `product-owner` to make in
the LCV-124 body. The architecture supports either: it is one boolean in
`classify`.

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
