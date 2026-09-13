# LCV-121 — The transport speaks tool calls

- **Status**: Done
- **Phase**: 12
- **Depends on**: none (first demand of Marco 2; blocks LCV-122, LCV-123, LCV-125)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust — d584f4d (single commit, no rework). Reviewed, APPROVED with no blocking findings: the reviewer independently re-ran the gates (`cargo fmt --all -- --check` clean, `cargo clippy --all-targets -- -D warnings` clean, `cargo test --all` 893 lib tests / 0 failed / 1 ignored across 26 targets plus 6 doc-tests) and a 47-mutation pass (42 killed, 5 survivors — two equivalent mutants, three non-blocking carried items routed to LCV-123 and recorded in `.claude/backlog.json`). CI run 34758445630 green on ubuntu-24.04, windows-2022 and macos-15. The LCV-121 → LCV-123 release hazard in §Risks is now live: do not tag until LCV-123 is `Done`.

## Problem

`CHANGELOG.md` records the Marco 2 gap in one sentence: *"The agent can read and
narrate the drawing but does not yet modify it end-to-end from chat."* The first
reason it cannot is mechanical and sits below the agent loop:
**`src/agent/transport.rs` cannot carry a tool call in either direction.**
`chat_completion` returns a `String`, so a response whose `content` is `null`
and whose `tool_calls` array is populated deserialises into
`TransportError::MissingContent` and the turn dies. The request body
(`ChatRequest { model, messages }`) has no `tools` field, so the model is never
told the CAD tools exist. `transport::Message { role, content }` has no
`tool_call_id` and no `tool_calls`, so `src/agent/loop_.rs`'s `send_fn` filters
out every message whose `content` is `None` and silently drops the `tool`-role
results the loop just built — the conversation the model sees has holes in it.
`loop_.rs` then hardcodes `"gpt-4o"` as the model id while the default endpoint
is OpenRouter, where `gpt-4o` is not a valid id: the shipped default
configuration cannot complete a single request.

The same two files also declare the same wire shapes twice. `loop_.rs` publishes
`ToolCall` / `ToolCallFunction` / `AssistantMessage` / `Choice` / `ChatResponse`
as hand-rolled non-serde structs; `transport.rs` privately declares its own
`ChatResponse` / `Choice` / `ChoiceMessage` as serde types. Neither can be used
by the other, and both files have to grow to carry tool calls — `loop_.rs` is at
178 of the 300 implementation-LOC cap and `transport.rs` at 116. Collapsing both
sets into one `src/agent/wire.rs` is what pays for the growth.

For a laser-cutting operator none of this is visible yet: this demand ships no
new UI and no new workflow. It is the wire that LCV-122 and LCV-123 need before
"draw a 20 mm square at the origin" can put four lines on the bed.

## Scope

- New `src/agent/wire.rs` — the single declaration site for the OpenAI-compatible
  chat + tool-call serde types. No `reqwest`, no `egui`.
- `src/agent/transport.rs`: send `tools` in the request body; deserialise
  `tool_calls` from the response; return the assistant message instead of a
  `String`; map 401 / 429 / 5xx onto readable errors.
- `src/agent/loop_.rs`: use `wire`'s types instead of its own; take the model id
  and the step budget as parameters; stop dropping messages whose `content` is
  `None`.
- Retire `MAX_TOOL_CALLS_PER_TURN`; add `AGENT_STEP_BUDGET_DEFAULT` / `_MIN` /
  `_MAX` and `clamp_step_budget` to `loop_.rs`.
- `src/io/settings.rs`: add `agent_model: String` and `agent_step_budget: u8`,
  each with its own `#[serde(default = …)]`.
- Rewrite the existing agent tests against the new signatures, and add the
  mockito coverage the new paths need.

## Out of scope

- **Any change to what the user sees.** No settings-UI field (LCV-125), no
  command-line prefix (LCV-124), no panel change (LCV-125).
- **`AgentAction`, `bridge.rs`, `parse_tool_call`, `agent_apply.rs`,
  `CompositeCommand`, `coalesce_last`, the revision fence.** All LCV-122.
- **Removing `&mut Document` / `&mut History` from `run_agent_turn`, deleting
  the throwaway `Document` in `panel.rs::submit`, `app/agent_turn.rs`,
  `app/agent_poll.rs`, `query_entities`, `query_selection`.** All LCV-123. The
  throwaway document stays exactly where it is and a reviewer must not flag it
  here.
- **Clamping the step budget inside `io/settings.rs`.** ADR 0007 §D7 puts the
  clamp at the read site. `io/settings.rs` must not import `crate::agent` — that
  edge inverts the layering.
- **Streaming, retries, connection pooling, a timeout setting, `tokio`.** The
  transport stays one blocking POST per call on a fresh
  `reqwest::blocking::Client`.
- **Upgrading `egui`.** Pinned at 0.29.1.
- **Secret storage for the API key.** Plaintext by decision (ADR 0007 §D10);
  the UI warning is LCV-125.
- **Any test that reaches a real endpoint.** mockito only — see §Expected tests.

## Acceptance criteria

1. **`src/agent/wire.rs` exists and is the only declaration site for the wire
   types.** It declares, with `serde` derives: `ToolCallFunction { name,
   arguments }`, `ToolCall { id, function }` (plus the `"type": "function"`
   field the API requires on the request side), `ChatMessage { role, content:
   Option<String>, tool_calls: Option<Vec<ToolCall>>, tool_call_id:
   Option<String> }` with the four constructors `system` / `user` /
   `assistant_with_tool_calls` / `tool_result`, `AssistantMessage { content:
   Option<String>, tool_calls: Option<Vec<ToolCall>> }`, `Choice { message }`,
   and `ChatResponse { choices }`. Every `Option` field carries
   `#[serde(skip_serializing_if = "Option::is_none")]` so a plain user turn
   still serialises as `{"role":"user","content":"…"}`.

2. **The duplicates are gone.** A bounded source scan (implementation section
   only, needles built with `concat!`) over `src/agent/loop_.rs` and
   `src/agent/transport.rs` finds no `struct ChatResponse`, `struct Choice`,
   `struct ChoiceMessage`, `struct AssistantMessage`, `struct ToolCall` or
   `struct ToolCallFunction` in either file, and finds `use crate::agent::wire`
   (or `use super::wire`) in both. `transport::Message` no longer exists
   anywhere in `src/`.

3. **`wire.rs` is kernel-pure.** Its implementation section contains none of
   `reqwest`, `egui`, `eframe`, `rfd`, or `crate::document`. Bounded scan with a
   positive control.

4. **The request carries the tools.** `chat_completion` takes
   `tools: &serde_json::Value` and serialises it as the body's `tools` field
   when it is a non-empty array, and omits the field entirely when the value is
   `Null` or an empty array. A mockito test asserts on the received body in both
   shapes.

5. **The signature change.**
   ```rust
   pub fn chat_completion(
       endpoint: &str,
       api_key: &str,
       model: &str,
       messages: &[wire::ChatMessage],
       tools: &serde_json::Value,
   ) -> Result<wire::AssistantMessage, TransportError>
   ```
   A response with `"content": null` and a populated `tool_calls` array returns
   `Ok(AssistantMessage { content: None, tool_calls: Some(v) })` with `v.len()`,
   the tool `name` and the raw `arguments` string preserved verbatim.
   `TransportError::MissingContent` is returned only when `choices` is empty or
   when `choices[0].message` has **neither** `content` nor a non-empty
   `tool_calls` — a response with tool calls and no content is a success.

6. **The conversation is sent losslessly.** `loop_.rs`'s `send_fn` passes the
   `ChatMessage` slice straight through: no `filter_map`, no
   `content.clone()?`. A `tool`-role message reaches the wire with its
   `tool_call_id` and its content; an assistant message reaches the wire with
   its `tool_calls`. A mockito test captures a second-round request body and
   asserts the `assistant`-with-`tool_calls` turn and the `tool` turn are both
   present, in order, with the id matching.

7. **Readable status errors.** `TransportError` gains three variants, and
   `chat_completion` maps onto them before the generic case:
   - **401** → `Unauthorized`, whose `Display` reads
     `Authentication failed (HTTP 401): the API key is missing, invalid, or not accepted by this endpoint. Check Help > Agent settings.`
   - **429** → `RateLimited`, whose `Display` reads
     `Rate limited (HTTP 429): the endpoint asked you to slow down. Wait a moment and try again.`
   - **500..=599** → `ServerError { status, body }`, whose `Display` reads
     `The endpoint failed (HTTP {status}): {body}` .
   - Every other non-2xx keeps the existing `Http { status, body }`.
   `body` stays truncated to the first 1024 characters, as today.

8. **The API key never appears in an error.** No variant of `TransportError`
   carries or formats the key, and the request is never logged. A test builds
   each of the four non-2xx paths with a recognisable dummy key
   (`"sk-test-DO-NOT-LEAK"`), formats the error with `to_string()`, and asserts
   the key substring is absent. The needle is built with `concat!` so the test
   cannot match its own literal.

9. **The model id comes from the caller.** The literal `"gpt-4o"` appears
   nowhere in `src/` (bounded scan over every `src/**/*.rs` implementation
   section, needle built with `concat!`). `run_agent_turn` takes `model: &str`
   and forwards it unchanged to `chat_completion`; a mockito test asserts the
   body's `model` field equals the string that was passed in.

10. **The step budget replaces the constant.** `MAX_TOOL_CALLS_PER_TURN` appears
    nowhere in `src/` (bounded scan, `concat!` needle). `src/agent/loop_.rs`
    declares `pub const AGENT_STEP_BUDGET_DEFAULT: u8 = 12;`,
    `AGENT_STEP_BUDGET_MIN: u8 = 1;`, `AGENT_STEP_BUDGET_MAX: u8 = 32;` and
    `pub fn clamp_step_budget(v: u8) -> u8` returning
    `v.clamp(MIN, MAX)` — `0 → 1`, `12 → 12`, `200 → 32`.

11. **The budget is a parameter and the error says the real number.**
    `agent_loop` and `run_agent_turn` take `step_budget: u8`. The guard still
    fires **before** dispatching any call of a batch that would cross the
    budget. `AgentError::IterationLimitExceeded` carries the budget that was in
    force (`IterationLimitExceeded(u8)`) and its `Display` names that number,
    not a constant: `step budget exceeded (7 tool calls per turn)` for a budget
    of 7. A unit test with budget `1` and a stubbed two-call response asserts
    the error and that **zero** dispatches happened.

12. **`Settings` gains two fields, defaulted field-by-field.**
    `agent_model: String` with `#[serde(default = "default_agent_model")]`
    returning `"anthropic/claude-sonnet-4.6"`, and `agent_step_budget: u8` with
    `#[serde(default = "default_agent_step_budget")]` returning
    `AGENT_STEP_BUDGET_DEFAULT`'s value written as a literal `12` (the function
    must not import `crate::agent`). `Settings::default()` sets both.
    A settings JSON that predates this demand (no `agent_model`, no
    `agent_step_budget`) loads with both defaults and every other field intact.

13. **`io/settings.rs` does not import the agent.** A bounded scan over its
    implementation section finds no `crate::agent`, and the value is stored
    verbatim: a file containing `"agent_step_budget": 200` loads as `200`.
    Clamping is the reader's job (LCV-123).

14. **The temporary read site.** Until LCV-123 creates `src/app/agent_turn.rs`,
    `src/agent/panel.rs::submit` is the one place that reads the two new
    settings, passing `app.settings.agent_model.clone()` and
    `crate::agent::loop_::clamp_step_budget(app.settings.agent_step_budget)` into
    `run_agent_turn`. Nothing else in this demand changes `panel.rs`: the
    throwaway `Document::default()` / `History::default()` stay until LCV-123
    deletes them.

15. **Caps, purity, gates.** Measured with ADR 0004's `awk` recipe, never
    `wc -l`: `src/agent/wire.rs`, `src/agent/transport.rs` and
    `src/agent/loop_.rs` are each **at or under 300 implementation LOC**, and the
    measured numbers are reported. `transport.rs` remains the only file in the
    crate that imports `reqwest` (bounded scan over `src/`).
    `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
    and `cargo test --all` all exit 0.

## Expected tests

Existing tests move with the signatures. Today: **12 `#[test]`s in
`transport.rs`** (mockito-backed), **10 in `loop_.rs`**, **1 in `panel.rs`**.
Every one of them is rewritten or retired deliberately — none may be deleted
silently, and the final count is reported.

- **Unit / AC 1, AC 3** — `wire.rs`: serialising `ChatMessage::user("hi")`
  produces exactly `{"role":"user","content":"hi"}` (no `null` keys);
  `ChatMessage::tool_result(id, text)` produces `role`, `content` and
  `tool_call_id` and nothing else; `assistant_with_tool_calls` round-trips
  through `serde_json` unchanged. Bounded purity scan with a positive control.
- **Unit / AC 2** — bounded source scans over `loop_.rs` and `transport.rs`
  (haystack sliced at the offset of the bare `#[cfg(test)]` at column 0,
  needles built with `concat!`), each shown to discriminate: the test must be
  demonstrated failing when the struct is pasted back.
- **mockito / AC 4** — two tests: a call with a 5-element tools array asserts
  the received body's `tools` has 5 entries; a call with `Value::Null` asserts
  the body has no `tools` key at all.
- **mockito / AC 5 — a single tool call.** Server returns one choice with
  `content: null` and one `tool_calls` entry. Assert `content.is_none()`,
  `tool_calls` length 1, the name and the raw `arguments` string.
- **mockito / AC 5** — `choices: []` and `choices[0].message` with neither
  content nor tool calls both return `MissingContent`; a choice with tool calls
  and null content does **not**.
- **mockito / AC 6 — a multi-step turn.** Two sequenced mock responses (tool
  call, then final text) driven through `run_agent_turn`; assert the final
  string, and assert the **second** request body contains the assistant turn
  with `tool_calls` and the `tool` turn with the matching `tool_call_id`. This
  is the test that fails if the old `filter_map` survives.
- **mockito / AC 7, AC 8** — one test per status: 401, 429, 500, and one
  other-non-2xx (e.g. 418) as the control that the generic arm still exists.
  Each asserts the variant and a distinctive substring of the `Display` text,
  and asserts the dummy key is absent from `to_string()`.
- **mockito / AC 9** — the request body's `model` equals the value passed in;
  a second assertion with a different value proves the test is not matching a
  default.
- **Unit / AC 9, AC 10** — bounded scans over every `src/**/*.rs`
  implementation section for `"gpt-4o"` and `MAX_TOOL_CALLS_PER_TURN`, needles
  built with `concat!`. Each scan is shown to fail when the literal is
  reintroduced.
- **Unit / AC 10** — `clamp_step_budget` at `0`, `1`, `12`, `32`, `33`, `200`.
- **mockito / AC 11 — the cap is reached.** A server that always answers with a
  tool call, budget 2: `run_agent_turn` returns `IterationLimitExceeded(2)`,
  the `Display` contains `2`, and the dispatch stub recorded at most 2 calls.
  Plus the budget-1 / two-calls-in-one-batch unit test asserting **zero**
  dispatches.
- **Unit / AC 12, AC 13** — settings serde: a JSON string with neither new key
  parses with both defaults and a preserved `recent_files`; a JSON string with
  `"agent_step_budget": 200` parses as `200`; `Settings::default()` carries
  `anthropic/claude-sonnet-4.6` and `12`. **These tests parse strings or use a
  `tempfile` directory the test owns. No test in this demand may cause a write
  to a real per-user path** (ADR 0006, ADR 0007 §D10): build `App::default()`,
  never `App::new()` (ADR 0002 §A4 rule 2), and never set a real API key
  alongside an injected `settings_path`.
- **Unit / AC 13, AC 15** — bounded scans: `crate::agent` absent from
  `io/settings.rs`; `reqwest` present in `transport.rs` and absent from every
  other file under `src/`.
- **Unit / AC 14** — bounded scan over `panel.rs` proving `submit` passes
  `clamp_step_budget(...)` and `settings.agent_model`.
- **Mutation checks the implementer runs first, in a scratch
  `git worktree`, and reports the results of**:
  (a) drop the `tools` field from the request body → AC 4's test fails by name;
  (b) restore the `filter_map(|m| … m.content.clone()?)` in `send_fn` → AC 6's
  multi-step test fails by name;
  (c) make the 401 arm fall through to the generic `Http` arm → AC 7's 401 test
  fails by name;
  (d) change `clamp_step_budget` to the identity function → AC 10 fails by name;
  (e) hardcode `"gpt-4o"` back into `send_fn` → AC 9's scan **and** AC 9's
  mockito test both fail.
  If any mutation leaves the suite green, the test is wrong, not the mutation.
- **[manual] none.** This demand is deliberately invisible; the real-endpoint
  check belongs to LCV-123 and is recorded on LCV-089's manual smoke checklist.

## Test hygiene (mandatory, not advisory)

- **No CI test may reach a real endpoint.** Real-endpoint validation needs the
  user's own API key and is a human step (ADR 0007 §Consequences). `mockito`
  covers every code path here.
- **The recurring bug class on this project is a test that cannot fail** — it
  has shipped six times. Every source scan must (1) bound its haystack to the
  implementation section by slicing at the offset of a bare `#[cfg(test)]` at
  column 0, and (2) build its needles with `concat!` so the scan cannot match
  its own needle literal. The canonical correct example is
  `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`. Every scan must be
  **shown to discriminate** — state, in the report, the mutation that makes it
  fail.
- **Positive controls must be tight.** A control that would also pass against
  the mutant is not a control.

## Risks

- **Between this demand and LCV-123 the app is worse than it is today, and must
  not be released in that state.** Today the model is never offered the tools,
  so it cannot claim to have drawn anything. Once this lands the model *can*
  return a tool call, `run_agent_turn` will dispatch it against the throwaway
  `Document` in `panel.rs::submit`, and the chat will confidently report
  `Line created: (0.000, 0.000) → (20.000, 0.000) mm.` while the bed stays
  empty. LCV-121 → LCV-122 → LCV-123 land as one unbroken sequence; the
  `CHANGELOG` sentence about narration stays until LCV-123, and **no tag is cut
  between them**.
- **`tool_calls` ordering and ids come from the model and are not validated
  here.** A duplicated or missing `id` is the model's error; the loop echoes
  whatever it received into `tool_call_id`. Do not invent ids.
- **A `u8` budget silently wraps if anyone does arithmetic on it.** The guard
  compares `dispatched + batch_len` as `usize` against `budget as usize`.
- **`skip_serializing_if` is load-bearing.** Some OpenAI-compatible endpoints
  reject `"tool_calls": null` on a user turn. AC 1 pins it; the round-trip
  tests are what keep it pinned.

## Notes

- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D7 (budget and where it is clamped), §D8 (file responsibilities and the
  `wire.rs` deduplication), §D10 (the key). Nothing in this demand re-decides
  any of it.
- Measured before the work, with ADR 0004's `awk` recipe:
  `loop_.rs` 178, `panel.rs` 155, `tools.rs` 151, `transport.rs` 116,
  `src/app/mod.rs` 265. The cap is 300 implementation LOC.
- The current `send_fn` in `loop_.rs` already captures `tool_definitions()` into
  `_tool_defs` with a comment saying it is "forwarded to post_chat once LCV-077
  is upgraded". This demand is that upgrade; the placeholder binding goes away.
- `mockito = "1"` is already a dev-dependency (`Cargo.toml:23`). No new
  dependency is needed or wanted.
- LCV-122 and LCV-123 depend on this demand. LCV-124 and LCV-125 depend on
  LCV-123 and on this one respectively, and become parallelizable once LCV-123
  lands.
