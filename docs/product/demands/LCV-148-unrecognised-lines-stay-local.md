# LCV-148 — An unrecognised line answers locally and never reaches the model

- **Status**: Done
- **Phase**: 12
- **Depends on**: LCV-124 (the classifier, the precedence table and the one arm this inverts)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: 421d730, 4caac01, e5d33d7

## Problem

With an API key configured, any line `cmdline::parse` does not recognise is
currently sent to a language model as a prompt. LCV-124 shipped that on purpose
and named the hazard — `lien`, a fat-fingered `line`, becomes a paid round trip
— and accepted it for typos. Refinement then found it fires on much more than
typos: the grammar knows only the single-letter aliases plus `text`, so `zoom`,
`z` and every full AutoCAD command name is `Unknown` today and goes to the
model. The operator is mid-part with their hands on the keyboard; one slip
costs a network call, seconds of latency, and a model that holds `create_line`
in its tool set and may well answer the prompt `lien` by drawing something on
the live bed (ADR 0007 §D2) that nobody asked for.

The user has closed the question. [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
amendment (5) rewrote §D9 rule 4 to `Unknown → Route::Cad, **always**`, for the
stated reason that *"free-form-to-agent is a v1 behaviour this product does not
carry forward"*. **The code has not caught up.** `src/agent/classifier.rs::classify`
still carries

```rust
CommandInput::Unknown(text) if agent_available => Route::Agent(text),
_ => Route::Cad,
```

so the shipped binary contradicts the ADR that governs it, and `CHANGELOG.md`
tells operators the old rule in the present tense. This demand closes that gap
and nothing else. The way to reach the model stays exactly where it was: an
operator who wants the model says so with `:` or `/ai`.

## Scope

- **One arm in `src/agent/classifier.rs::classify`.** A line with no `:` /
  `/ai` prefix is `Route::Cad`, whatever the settings say.
- The doc comments in `src/agent/classifier.rs` that describe the old rule: the
  module header, `classify`'s precedence list (rules 3 and 4), `Route::Cad` and
  `Route::Agent`.
- The stale prose in `src/app/cmdline.rs` — **comments only, no behaviour**:
  `submit`'s numbered order-of-business item 5, the `// LCV-124 — where does
  this line go?` block inside `submit`, `to_agent`'s doc comment ("bare free
  text now costs a network call"), and the doc comment on
  `submit_parses_the_line_exactly_once`.
- Inverting and re-pinning the tests that assert the old rule, named in
  §Acceptance criteria.
- A correction to the `CHANGELOG.md` `[Unreleased]` clause LCV-124 wrote, plus
  the bounded scan that keeps it corrected.

## Out of scope

- **ADR 0007 §D11, `src/app/agent_poll.rs`, and `agent_turn::arm_turn`.** §D9a
  says this out loud: rule 4 decides whether a *caller* ever reaches
  `start_turn`; deleting a caller cannot add a writer of `agent_busy`, and
  `tests/lcv129_agent_timeout_and_cancel.rs`'s single-writer scan pins file
  paths, not call graphs, so it stays green and non-vacuous across the flip.
  **A change to this demand that touches `src/app/agent_poll.rs` or `arm_turn`
  has scope creep in it and is a review blocker.**
- **Widening the CAD grammar.** `line`, `rect`, `erase` and the rest of the
  word set are [LCV-131](LCV-131-command-line-pays-a-model-for-cad-words.md),
  which depends on this demand. Nothing here touches `src/cmdline/`.
- **A setting, a toggle or an "advanced mode" that turns the old behaviour back
  on.** The decision is a decision, not a preference. One arm, no boolean.
- **The `:` / `/ai` prefixes**, their spelling, their case-insensitivity, their
  word-boundary rule, the empty-prompt refusal, the `! Agent unavailable: …`
  message, the `→ agent: "…"` echo, the panel auto-open, the busy refusal and
  the recall ring. All LCV-124, all unchanged.
- **A new purity rule.** The classifier stops importing `crate::cmdline`
  because the import becomes unused, **not** because it is newly forbidden the
  grammar. No scan may be added that forbids `crate::cmdline` in
  `src/agent/classifier.rs`; see AC 5.
- **Any change to `src/agent/` other than `classifier.rs`**, and any change to
  the transport, the loop, the tool registry or the panel.

## Acceptance criteria

1. **No unprefixed line reaches the agent, whatever the settings say.**
   `classify(line, true) == classify(line, false) == Route::Cad` for every line
   that carries no `:` / `/ai` prefix. The table asserts at least: `lien`, `z`,
   `zoom`, `zoom sideways`, `/aim`, `é`, `x`, `d`, `1,2,3`, `nan`, `Foo`, plus
   the rule-3 rows already pinned (`l`, `snap`, `ze`, `zoom in`, `50,25`,
   `@10,0`, `37.5`, `""`). No line in this table produces `Route::Agent` or
   `Route::Unavailable` under either availability.

2. **`classify` keeps its arity, its signature and its purity.** It remains
   `pub fn classify(raw: &str, agent_available: bool) -> Route`, pinned by a
   coercion to `fn(&str, bool) -> Route` so the parameter cannot be "simplified
   away" without failing the build. `agent_available` is still load-bearing: it
   is what separates `Route::Agent` from `Route::Unavailable` on a **prefixed**
   line, and the table in AC 3 proves it by moving exactly those rows and no
   others.

3. **The escape hatches stay absolute (§D9a rule 2).** With a key configured
   `:line`, `/ai line`, `/AI line`, `:50,25`, `:snap`, `:@10,0` and `:l` are
   `Route::Agent(rest)`; without one each is `Route::Unavailable`. The four
   empty-prompt spellings (`:`, `:   `, `/ai`, `/ai   `) stay `Route::Agent("")`
   under both availabilities. `:line` in particular is asserted by name: it goes
   to the model even though `line` is on its way to parsing as CAD (LCV-131).

4. **Raw-input mode is untouched (§D9a rule 2, precedence rule 1).**
   `tests/lcv124_command_line_routing.rs::raw_input_wins_over_the_agent_prefix`,
   `…::raw_input_wins_for_the_slash_ai_prefix_too` and
   `tests/lcv112.rs::raw_input_is_not_parsed` are green **unmodified**. A key is
   configured in the first two on purpose; do not weaken them.

5. **The classifier's grammar import has a decided end state, not a
   clippy-driven one.** Removing the arm leaves `parse` and `CommandInput`
   unused by the implementation section. The decided end state:
   - `classify` no longer calls `parse`, and the implementation section of
     `src/agent/classifier.rs` no longer imports `crate::cmdline`. This is
     correct and is the point: after the flip the classifier decides on the
     **prefix alone**, so it needs no grammar knowledge at all, which is a
     stronger form of ADR 0003 §A2a's "nothing outside `src/cmdline/` may hold a
     second copy of the alias table" than the call it replaces.
   - The `use crate::cmdline::{…}` moves **into** the `#[cfg(test)] mod tests`
     block, which still needs it: `z_is_not_a_zoom_word` is the record of what
     the grammar does with `z`, `ze` and `zoom in`, and it stays (its doc
     comment is corrected — `z` is no longer a "rule 4 row", it is a CAD row
     like every other unrecognised line).
   - `classifier_is_kernel_pure` keeps its shape and its discrimination: its
     positive controls are **re-derived** from the post-flip implementation
     section (`crate::cmdline` and `CommandInput` are no longer in it and must
     not be asserted as present), at least two controls remain, and the
     forbidden list is unchanged — `crate::cmdline` is **not** added to it.

6. **The behaviour LCV-124 pinned by name inverts and is re-pinned in place
   (§D9a rule 3).** `tests/lcv124_command_line_routing.rs::a_typo_reaches_the_agent_only_when_a_key_is_configured`
   is **inverted, never deleted**: it keeps its file and its two-app shape (one
   `App::default()`, one `app_with_a_key()`), and asserts that **both** answer
   `lien` identically — the same local `Unknown command: "lien"` feedback, and
   for each app `!agent_busy`, `agent_rx.is_none()`, `agent_chat.is_empty()`,
   `!agent_panel_open`, and zero entities on the document. Its name must state
   the new claim (`a_typo_never_reaches_the_agent_even_with_a_key_configured` or
   equivalent) and its doc comment must say that it is the flip's witness. A
   diff that removes this test without an equivalent replacement in the same
   file is a review blocker: it is the only assertion in the suite that bare
   text does not reach the model.

7. **The inline classifier tests that encode the old rule are inverted, not
   loosened.** In `src/agent/classifier.rs::tests`:
   `the_precedence_table_holds_for_both_availabilities` keeps its
   `moved_with_availability` counter — the count becomes the number of prefixed
   rows and nothing else, and the assertion message names them;
   `slash_ai_requires_a_boundary` asserts `classify("/aim", true) == Route::Cad`;
   `a_multibyte_line_is_classified_without_panicking` asserts
   `classify("é", true) == Route::Cad` and still cannot panic;
   `a_prefixed_cad_command_is_a_prompt` is unchanged.

8. **No source or doc comment in the crate still claims that an unrecognised
   line can reach the model.** Specifically corrected: `src/agent/classifier.rs`'s
   module header, `classify`'s rules 3 and 4 (which must read as ADR 0007 §D9
   reads after amendment (5)), `Route::Cad`'s doc (drop *"when no API key is
   configured"* — the message is now unconditional) and `Route::Agent`'s doc
   (state that a `:` / `/ai` prefix is now its only producer); and in
   `src/app/cmdline.rs`, `submit`'s order-of-business item 5, the `// LCV-124`
   comment block inside `submit`, `to_agent`'s doc, and
   `submit_parses_the_line_exactly_once`'s doc.

9. **`src/app/cmdline.rs` changes comments only.** `submit`'s control flow,
   `agent_available`, `to_agent`, `echo`, `ECHO_CHARS`, `AGENT_UNAVAILABLE`,
   `AGENT_EMPTY_PROMPT` and `AGENT_BUSY` are byte-identical.
   `submit_parses_the_line_exactly_once` stays green and non-vacuous: `submit`
   still calls `classify(raw` exactly once and `parse(` exactly once.

10. **The CHANGELOG stops telling operators the old rule.** The `[Unreleased]`
    LCV-124 bullet's final clause — *"With a key configured, a line the CAD
    grammar does not recognise is sent to the model, which is a paid round trip:
    note that the grammar today knows the single-letter tool aliases plus
    `text`, so a command spelled out in full — `line`, `circle` — is not
    recognised locally and goes to the model instead of drawing."* — is replaced
    by a clause that states the shipped rule: a line the grammar does not
    recognise answers `Unknown command: "…"` locally and makes no network call
    whatever is configured, and `:` / `/ai` are the only way to the model from
    the command line. **`CHANGELOG.md` is `demand-manager`'s file**
    (`AGENTS.md` §Subagent Suite): `implementer-rust` does not edit it, it hands
    the wording over. See §Notes for the ordering this forces.

11. **The correction is checked, not asserted.** A test bounds its haystack to
    the `## [Unreleased]` section of `CHANGELOG.md` (from that heading to the
    next `\n## ` at column 0) and fails if the stale phrase is still there. The
    needle is built with `concat!`, split inside a word. Two positive controls
    run the **same** search over the **same** slice: the slice is non-empty and
    names `LCV-124`, and it is shorter than the whole file. The demand is not
    `Done` while this test is red.

12. **Gates and caps.** `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and
    `cargo test --all --no-fail-fast` (ADR 0008 — the flag is not optional) all
    exit 0, and the implementation-LOC of `src/agent/classifier.rs` is measured
    with ADR 0004's `awk` recipe and reported. `git diff --stat` for the shipped
    change names only: `src/agent/classifier.rs`, `src/app/cmdline.rs`,
    `tests/lcv124_command_line_routing.rs`, the new CHANGELOG-scan test file,
    and the docs. `src/app/agent_poll.rs`, `src/app/agent_turn.rs`,
    `src/agent/loop_.rs`, `src/agent/transport.rs` and `src/cmdline/` are **not**
    in it.

## Expected tests

- **Unit / AC 1** — the precedence table in `src/agent/classifier.rs::tests`,
  extended with the rows above, every row asserted with `agent_available` both
  `true` and `false`.
- **Unit / AC 2** — the coercion pin (`let f: fn(&str, bool) -> Route =
  classify;` then assert through `f`), and the `moved_with_availability` counter
  of AC 7, which is what proves the parameter still does something.
- **Unit / AC 3** — the prefixed rows both ways, including `:line` and `:l`, and
  the four empty-prompt spellings.
- **Integration / AC 4** — the three named raw-input tests, run unmodified.
- **Unit / AC 5** — `classifier_is_kernel_pure` with re-derived positive
  controls; the implementer states in the commit message which controls changed
  and why, and shows the scan still discriminates (a deliberate temporary
  `use egui;` in the implementation section must turn it red).
- **Integration / AC 6** — the inverted, renamed `lien` test. Drive it with
  `lasercad::app::submit` as it is driven today; no HTTP, no thread.
- **Unit / AC 7** — the four named inline tests.
- **Unit / AC 9** — `submit_parses_the_line_exactly_once`, unchanged and green.
- **Unit / AC 11** — the bounded `[Unreleased]` scan with its two positive
  controls.
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result by name**:
  (a) restore the deleted arm → AC 1's table and AC 6's integration test both
  fail by name;
  (b) make `classify` ignore `agent_available` and always answer
  `Route::Agent(prompt)` on a prefixed line → AC 3's "without a key" rows fail;
  (c) drop the `agent_available` parameter entirely → AC 2 fails to compile,
  which is the intended failure;
  (d) move the `classify` call above `submit`'s `wants_raw_input` early return →
  AC 4's TextTool tests fail by name (this is LCV-124 mutation (a), re-run
  because it is the one that would leak a private string);
  (e) revert the CHANGELOG clause → AC 11's scan fails, and its positive
  controls do not.
- **[manual] smoke** — configure a real API key. Type `lien` and confirm the
  command line answers `Unknown command: "lien"`, the agent panel does **not**
  open, no spinner appears and nothing is drawn. Type `zoom` and `z` and confirm
  the same. Then type `:draw a 20 mm square at 10,10` and confirm the model is
  still reachable and still draws. Finally clear the key and confirm `:hello`
  answers `! Agent unavailable: set the API key in Help > Agent settings`.

## Test hygiene (mandatory)

Copied verbatim from LCV-124, which is the demand this one inverts:

- **Bound every source scan** to the implementation section (slice at the offset
  of the bare `#[cfg(test)]` at column 0) and **build every needle with
  `concat!`**. Canonical correct example: `guard_is_runtime_not_cfg` at
  `src/io/dialogs.rs:179-194`. Every scan must be **shown to discriminate**.
- **No CI test may reach a real endpoint** — mockito only, and this demand
  should need no HTTP at all: drive turns through LCV-123's `arm_turn` seam.
- ADR 0002 §A2 / §A4 rule 1; ADR 0006 and ADR 0007 §D10: **no test may cause a
  settings write to a real per-user path**, and a test that injects a
  `settings_path` must not also set a real API key.

One clause this demand adds, because its scan reads a **document** rather than
source: a needle split by `concat!` must be split **inside a word**, so no
literal in the scanning file can satisfy the scan it is checking. The haystack
is `CHANGELOG.md`, bounded to one section, never the whole file.

## Open questions

*(none)*

## Notes

- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D9 rule 4 as rewritten by amendment (5), and §D9a, which enumerates the three
  things the flip may not disturb and states that §D11 is not in scope. This
  demand is the implementation of a decision already taken; it does not reopen
  it.
- **Ordering forced by AC 10 and AC 11.** The scan is red until the CHANGELOG
  clause is corrected, and the implementer may not correct it. So:
  `implementer-rust` opens the handoff task for `demand-manager` (carrying the
  replacement wording) **before** running its final gate, and the final
  `cargo test --all --no-fail-fast` is run after that edit has landed. A red
  suite handed to review is not acceptable; neither is a scan that was weakened
  to be green against the stale text.
- **`src/app/cmdline.rs::submit` needs no code change**, which is worth saying
  out loud because it looks like it should. Its `Route::Cad => {}` arm already
  falls through to the dispatch, and the dispatch already answers
  `CommandInput::Unknown(text)` with `Unknown command: "…"`. After the flip that
  arm simply receives more lines. The only thing wrong in that file is prose.
- **One ADR sentence goes stale and is `architect`'s to fix, not this demand's.**
  ADR 0003 §A2a says *"`src/agent/classifier.rs` already obeys this by calling
  `parse` rather than re-deciding a rule of it, and its module header says so"*,
  and its preceding paragraph lists the classifier among the readers of the
  `CommandInput` variants. After AC 5 neither is true — the classifier obeys the
  rule by needing the grammar not at all, which is strictly stronger. Raise it
  with `architect` as a one-line correction to §A2a; do **not** edit the ADR from
  this demand, and do **not** keep a dead `parse` call alive to make the sentence
  true.
- LCV-124's body is left verbatim as the dated record of what shipped and why,
  including its §Risks "revisit path, if the operator reports surprise calls:
  flip the boolean". This demand is that revisit, taken for a stronger reason
  than a report of surprise: the grammar is small enough that the surprise is
  the normal case.
- The window ADR 0007 §D9a describes — in which `z` and `zoom` still reach the
  model while the grammar demand lands first — does not open, because this
  demand lands **before** LCV-131.
