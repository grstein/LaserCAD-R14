# LCV-124 — The command line can reach the agent

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-123
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

Once LCV-123 lands, the agent can actually draw on the operator's bed — but the
only way to reach it is to open the side panel with the mouse and type into a
text field. That contradicts the first line of AGENTS.md §Product Philosophy
("prefer command line and keyboard-first flows") and it contradicts
`PLAN.md`'s LCV-080 row, which has always said the command line wires `:` and
`/ai` to the agent. An operator whose hands are on the keyboard drawing a part
should be able to type `:add a 5 mm fillet radius circle at 10,10` on the same
line they type `L` and `@20,0`, without reaching for the mouse.

`src/agent/classifier.rs` is named in `PLAN.md`'s module tree and in ADR 0007
§D8's file table and does not exist. `src/app/cmdline.rs::submit` currently ends
at `CommandInput::Unknown(text) => app.command_feedback = format!("Unknown
command: \"{text}\"")`, which is where the routing decision belongs.

## Scope

- New `src/agent/classifier.rs` — `classify(raw: &str, agent_available: bool)
  -> Route`, pure, with the ADR 0007 §D9 precedence.
- `src/app/cmdline.rs::submit` calls it, after the raw-input early return and
  after the recall-ring push.
- The `! Agent unavailable: set the API key in Help > Agent settings` path.
- Making an agent send **unmistakable** in the command line and in the panel,
  because bare free text now costs a network call.

## Out of scope

- **Changing `cmdline::parse`'s grammar.** The classifier calls the parser; it
  does not reimplement or extend it. No new alias, no new toggle, no new prefix
  beyond `:` and `/ai`.
- **A setting that turns bare-free-text routing on or off.** The decision is
  made below and is not configurable in this milestone. §Risks records the
  revisit path: it is one boolean in `classify` and needs no re-architecture.
- **Anything about the turn itself** — the fence, coalescing, the transcript
  rows, the budget. All LCV-123.
- **Panel styling, the settings fields, the plaintext-key warning.** LCV-125.
  This demand and LCV-125 are independent and may run in parallel.
- **Multi-line prompts, prompt history separate from the recall ring, a
  `/help`, or any second prefix.**
- **Routing while a turn is in flight.** One turn at a time (ADR 0007 §Revisit
  criteria); AC 10 refuses rather than queues.

## Acceptance criteria

1. **`src/agent/classifier.rs` is pure and takes availability as data.**
   ```rust
   pub enum Route { Cad, Agent(String), Unavailable }
   pub fn classify(raw: &str, agent_available: bool) -> Route
   ```
   Its implementation section imports none of `egui`, `eframe`, `rfd`,
   `reqwest`, `crate::app`, `crate::document`. Bounded scan with a positive
   control.

2. **Precedence, exactly ADR 0007 §D9, highest first.** Each rule is one test:
   1. **Raw-input mode wins over everything** — enforced at the call site, not
      in the classifier: `src/app/cmdline.rs::submit` already returns early when
      `tool_manager.wants_raw_input()`, and `classify` is called **after** that
      check. With `TextTool` active and awaiting its string, `:hello` and
      `/ai hello` are forwarded to the tool verbatim, no turn is spawned, and
      nothing is pushed to the recall ring.
   2. **`:` or `/ai` → `Route::Agent(rest)`, absolutely** — even when `rest`
      would parse as a CAD command. `:line`, `:50,25`, `:snap` and `:@10,0` all
      route to the agent. This is the escape hatch and nothing overrides it.
   3. **`cmdline::parse(raw)` returning anything other than `Unknown` →
      `Route::Cad`.** Tool aliases, toggles, zoom, absolute points, relative
      points, bare distances and a bare Enter (`CommandInput::Empty`) all still
      behave exactly as they do today.
   4. **`Unknown` → `Route::Agent(trimmed)` when `agent_available`, else
      `Route::Cad`** so `Unknown command: "lien"` is produced unchanged.

3. **Prefix handling is precise.** The prefix is matched on the **trimmed**
   line. `:` takes everything after the first colon; `/ai` takes everything
   after the literal `/ai` and requires a following space or end-of-line (so
   `/aim` is not a prefix). Both are matched **case-insensitively** (`/AI`
   works). The remainder is trimmed before it becomes the prompt.

4. **An empty prompt is refused, not sent.** `:`, `:   `, `/ai` and `/ai   `
   produce `Route::Agent("")`; `submit` sends nothing, spawns nothing, and sets
   `command_feedback` to `Agent prompt is empty.` In particular a bare `:` must
   not fall through to `CommandInput::Empty`, which would send `Enter` to the
   active tool and finish a polyline.

5. **`agent_available` has one definition.**
   `!app.settings.agent_api_key.trim().is_empty()`, read at the call site and
   passed into `classify`. A whitespace-only key is not a key.

6. **Without a key, a prefixed line says so and does nothing else.**
   `classify(":draw a square", false)` is `Route::Unavailable`, and `submit`
   sets `command_feedback` to exactly
   `! Agent unavailable: set the API key in Help > Agent settings`
   — verbatim, including the leading `! ` — spawns no thread, opens no dialog,
   and leaves `agent_chat` untouched.

7. **Without a key, unprefixed nonsense still answers the way it does today.**
   `classify("lien", false)` is `Route::Cad`, and `submit` produces
   `Unknown command: "lien"` byte-for-byte as it does now. This is the
   regression guard on the existing behaviour.

8. **A send is unmistakable.** When `submit` routes to the agent it does all
   three of:
   - sets `command_feedback` to `→ agent: "<prompt>"`, with the prompt
     truncated to 60 characters and `…` appended when truncated;
   - opens the agent panel (`app.agent_panel_open = true`) if it was closed, so
     the prompt, the spinner and the reply are visible where they happen;
   - records the prompt in `agent_chat` as the `user` row (this already happens
     inside `arm_turn`, LCV-123 AC 3 — assert it, do not duplicate it).
   This is the mitigation for the typo hazard in §Risks and it is not optional.

9. **The recall ring is unchanged.** An agent-routed line is pushed to
   `app.command_history` exactly like any other submitted line, before dispatch,
   so `ArrowUp` recalls a mistyped prompt for editing. The blank-line cursor
   reset (`fix(LCV-110)`) is untouched.

10. **A second turn is refused, not queued.** Submitting an agent-routed line
    while `agent_busy` is true sets `command_feedback` to
    `Agent is busy — wait for the current turn to finish.`, spawns nothing, and
    leaves the in-flight turn alone.

11. **The classifier does not parse twice in two places.** `classify` calls
    `crate::cmdline::parse`; `src/app/cmdline.rs` does not call `parse` a second
    time for the same line. Bounded scan: exactly one `parse(` call site in
    `submit`.

12. **Purity, caps, gates.** `src/agent/classifier.rs` and
    `src/app/cmdline.rs` are each at or under 300 implementation LOC (ADR 0004's
    `awk` recipe, never `wc -l`), and the numbers are reported. `src/cmdline/`
    stays free of `egui` / `eframe` / `rfd`. `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings`, `cargo test --all` exit 0.

## Expected tests

- **Unit / AC 1** — bounded purity scan over `classifier.rs` with a positive
  control.
- **Unit / AC 2** — a table test, one row per precedence rule and at least
  these cases: `":line"`, `"/ai line"`, `"/AI line"`, `":50,25"`, `":snap"`,
  `"l"`, `"snap"`, `"z"`, `"50,25"`, `"@10,0"`, `"37.5"`, `""`, `"lien"` — each
  asserted with `agent_available` both `true` and `false`, so the table is 26
  assertions and the availability flag is proved to matter.
- **Integration / AC 2 rule 1** — with `TextTool` active and awaiting its
  string, `submit(app, ":hello")` puts `:hello` into the text being drawn, does
  not set `agent_busy`, and does not push to the recall ring. ADR 0002 §A2:
  `App::default()`, never `App::new()`.
- **Unit / AC 3** — `"/aim"` is **not** a prefix (it reaches rule 3/4 as
  `Unknown`); `"  :  draw  "` yields the prompt `draw`; `":a:b"` yields `a:b`.
- **Unit / AC 4** — the four empty-prompt spellings; plus an integration
  assertion that a bare `:` does **not** reach the active tool (a polyline with
  two points is not finished by it).
- **Unit / AC 5** — `" "` as a key is not availability; `"sk-test"` is.
- **Integration / AC 6** — the exact feedback string (needle built with
  `concat!`), `agent_busy` false, `agent_rx` none, `agent_chat` empty.
- **Integration / AC 7** — `Unknown command: "lien"` unchanged, asserted
  against the existing LCV-111 test rather than a new copy of the literal.
- **Integration / AC 8** — all three effects, including the 60-character
  truncation with the `…`, and `agent_panel_open` flipping from false to true.
  The turn itself is driven with an injected channel (LCV-123's `arm_turn`
  seam), **not** by spawning a thread and not over HTTP.
- **Integration / AC 9** — after an agent-routed submit, `ArrowUp` recalls the
  line.
- **Integration / AC 10** — the busy path.
- **Unit / AC 11, AC 12** — the single-`parse` scan and the LOC measurements.
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result**:
  (a) move the `classify` call **above** the `wants_raw_input` early return →
  AC 2 rule 1's TextTool test fails by name;
  (b) make rule 2 defer to rule 3 (so `:line` starts the LINE tool) → AC 2
  fails by name;
  (c) make rule 4 route to the agent regardless of `agent_available` → AC 7
  fails by name;
  (d) delete the `agent_panel_open = true` line → AC 8 fails by name;
  (e) let an empty prompt through → AC 4's polyline assertion fails by name.
- **[manual] smoke** — with no key configured: type `:hello` and read the
  `! Agent unavailable: …` line; type `lien` and read `Unknown command:
  "lien"`. With a key configured: type `:draw a 20 mm square at 10,10`, watch
  the panel open by itself, the prompt appear, and the square land on the bed;
  press `Ctrl+Z` once and watch the whole square go. Then type `lien` and
  confirm you can tell at a glance that it was sent to the model.

## Test hygiene (mandatory)

- **Bound every source scan** to the implementation section (slice at the offset
  of the bare `#[cfg(test)]` at column 0) and **build every needle with
  `concat!`**. Canonical correct example: `guard_is_runtime_not_cfg` at
  `src/io/dialogs.rs:179-194`. Every scan must be **shown to discriminate**.
- **No CI test may reach a real endpoint** — mockito only, and this demand
  should need no HTTP at all: drive turns through LCV-123's `arm_turn` seam.
- ADR 0002 §A2 / §A4 rule 1; ADR 0006 and ADR 0007 §D10: **no test may cause a
  settings write to a real per-user path**, and a test that injects a
  `settings_path` must not also set a real API key.

## Risks

- **The typo hazard is real, it is named, and it is accepted.** Rule 4 means
  that with a key configured, `lien` — a fat-fingered `line` — stops being a
  free local error message and becomes a paid round trip to an LLM, with
  latency and with a chance the model helpfully draws something. The team lead's
  instruction stands: CAD verbs, toggles, zoom and coordinates always win; bare
  free text reaches the agent only when a key is configured. What bounds the
  hazard:
  - every real verb, alias, toggle, zoom word, coordinate, relative offset and
    bare distance parses to something other than `Unknown`, so only genuinely
    unrecognised text can leak;
  - AC 8 makes the send loud — the command line says `→ agent: "lien"` and the
    panel opens by itself. The operator learns within one keystroke of feedback,
    not after a mysterious five-second pause;
  - the turn is one `Ctrl+Z` away from being undone (LCV-123 AC 10);
  - the step budget caps the damage of a confused model.
  **Revisit path, if the operator reports surprise calls:** flip the boolean —
  route rule 4 to `Route::Cad` unconditionally and let prefixes be the only way
  in. ADR 0007 §D9 already says the architecture supports either and that it is
  one boolean in `classify`. No ADR change and no re-architecture is needed, so
  do **not** pre-build a setting for it now.
- **`:` is a plausible prefix for a future CAD feature** (R14 has none, but
  named views or scripts might want it). If that day comes, the prefix moves and
  the classifier is the one place that changes.
- **Rule 1 is the one that would leak a private string.** If `classify` is ever
  called before the `wants_raw_input` check, the text an operator is typing into
  a TEXT entity gets posted to an LLM. Mutation (a) is the guard; it must be run
  and reported.

## Notes

- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D9 (precedence and the availability rule) and §D10 (the key never leaves the
  transport). ADR 0007 left one question open — *should bare unprefixed free
  text reach the agent at all?* — and explicitly delegated it to this demand's
  body. **It is answered here: yes, but only when a key is configured, and only
  after the parser has declined the line.** The hazard is recorded above as a
  named risk with a one-boolean revisit path.
- [ADR 0003](../../adr/0003-command-line-input-contract.md) is the command-line
  input contract this demand extends by exactly one branch.
  [LCV-112](LCV-112-text-tool-raw-input.md) decision 1 ("raw means raw") is what
  rule 1 protects.
- `src/app/cmdline.rs::submit`'s existing order of business — clear feedback,
  raw-input early return, parse, push to the ring, dispatch — is preserved. The
  routing branch replaces only the `CommandInput::Unknown` arm's behaviour and
  adds the prefix check ahead of the parse.
- LCV-125 touches `src/agent/panel.rs` and `src/agent/settings_ui.rs`; this
  demand touches `src/agent/classifier.rs` and `src/app/cmdline.rs`. They are
  disjoint and may run in parallel once LCV-123 has landed.
