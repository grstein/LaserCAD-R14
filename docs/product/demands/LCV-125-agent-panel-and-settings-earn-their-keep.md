# LCV-125 — The panel shows what the agent did to the drawing

- **Status**: Ready
- **Phase**: 12
- **Depends on**: LCV-121, LCV-123
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

Two gaps open the moment LCV-123 lands, and both cost the operator something
concrete.

**The transcript is unreadable.** LCV-123 records every agent action, every
outcome, every refusal and the end-of-turn undo-shape note into `agent_chat`,
but `src/agent/panel.rs` renders four role values and falls through to a plain
grey `ui.label` for everything else. So `Deleted entity 1 (circle, center
(10.000, 10.000) mm, r = 5.000 mm). Indices 2..3 are now 1..2.` — the exact
sentence ADR 0007 §D5 requires *because positional indices shift underneath the
model* — arrives looking identical to chit-chat. That sentence is the disclosure
mechanism for the one hazard this milestone ships with. If the operator cannot
pick it out of the column, the disclosure does not exist, and a wrong
`delete_entity` is invisible until they notice a missing circle three operations
later. The same applies to the undo-shape note: whether one `Ctrl+Z` takes the
whole turn back or four presses are needed is the single most useful thing on
the panel after a bad answer, and today it is one grey line among many.

**The two settings LCV-121 added are unreachable.** `Settings::agent_model` and
`Settings::agent_step_budget` exist and are honoured, but
`src/agent/settings_ui.rs` draws only Endpoint URL and API Key. Changing the
model means hand-editing `~/.config/lasercad/settings.json` — and the model is
the field most likely to need changing, because the default
`anthropic/claude-sonnet-4.6` is an OpenRouter id and any other endpoint wants a
different one. Meanwhile the API key sits in that same JSON file **in plain
text** (ADR 0007 §D10 ships it that way on purpose) and the dialog masks the
field with bullets, which quietly implies the opposite. The UI must say out
loud what it actually does with the key.

## Scope

- `src/agent/panel.rs`: render the agent transcript by role — the action /
  outcome rows, refusals, errors and the end-of-turn note each visually
  distinct from assistant prose.
- `src/agent/settings_ui.rs`: a Model field, a step-budget slider bounded by
  `AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX`, and a plain-language warning
  that the API key is stored in clear text.
- **One absorbed pre-existing fix** (`architect` ruling, 2026-09-13, written in
  here by `product-owner` before implementation starts): `src/agent/mod.rs`
  declares `pub mod settings_ui;` with no `pub use` companion, and
  `src/app/panels.rs` therefore calls
  `crate::agent::settings_ui::draw_agent_settings` — a deep-path import from
  outside the module, which AGENTS.md §Module tree forbids. It dates to
  `0d1d52b` (LCV-105) and is not this demand's debt, but this demand already has
  both files open and the fix is two lines. See AC 13.

## Out of scope

- **Changing what LCV-123 records.** The seam is: **LCV-123 produces the rows,
  this demand renders them.** The role vocabulary and every sentence are
  LCV-123's — `user` (its AC 3), `tool` and `refused` (its AC 23), `assistant`
  and `error` (its AC 7), `note` (its AC 11) — and their order in `agent_chat`
  is fixed there too (its AC 23). If a sentence is wrong, or a row is missing,
  that is an LCV-123 defect, not a new string here.
- **A new `App` field or a new transcript type.** `agent_chat:
  Vec<(String, String)>` is the carrier. Do not introduce a parallel store.
- **Markdown rendering, syntax highlighting, streaming/typewriter output,
  avatars, timestamps, message bubbles, copy buttons, a resizable splitter, or
  a settings search box.** Decorative; rejected.
- **Editing, re-running or deleting a transcript row.**
- **Persisting the chat across sessions**, or adding it to the autosave
  envelope.
- **Secret storage for the API key** (OS keyring / `secret-service`). Deferred
  to its own ADR; this demand ships the warning, not the fix.
- **A model picker, a model list fetched from the endpoint, or validation of the
  model string.** A free-text field. The endpoint is the authority on what ids
  exist and it already reports a bad one through LCV-121's status mapping.
- **Spawning anything from `panel.rs`.** AGENTS.md: *"`panel.rs` renders and
  reports. It never spawns a thread and never constructs a `Document` or a
  `History`."*
- **Upgrading `egui`.** Pinned at 0.29.1.
- **A tree-wide regression scan for deep-path imports.** Proposed by `architect`
  on the premise that "the tree goes from one hit to zero", which would have
  made this the cheapest moment it will ever be. `product-owner` measured it and
  the premise does not hold: outside the `#[cfg(test)]` sections there are
  **13** cross-module deep-path `use`/path expressions in 12 files, not one —
  `src/app/init.rs` (5), `src/app/persist.rs` (2), `src/io/svg/export.rs` (2),
  `src/io/svg/import.rs`, `src/io/autosave.rs`, `src/text/layout.rs`,
  `src/tools/trim.rs`, `src/app/mod.rs`, `src/app/input.rs`,
  `src/agent/bridge.rs`, `src/app/panels.rs` and **`src/agent/settings_ui.rs`
  itself**, which does `use crate::io::settings::Settings;`. A scan shipped
  today would carry a twelve-entry grandfather list, which is the weakest
  possible shape for this repository's weakest test form. Reproduce with:

  ```bash
  for f in $(grep -rlE "crate::[a-z_0-9]+::[a-z_0-9]+::" src/ --include="*.rs"); do
    end=$(awk '/^#\[cfg\(test\)\]/{print NR-1; f=1; exit} END{if(!f) print NR}' "$f")
    own=$(echo "$f" | cut -d/ -f2)
    head -n "$end" "$f" | grep -noE "crate::[a-z_0-9]+::[a-z_0-9]+::[A-Za-z_0-9]*" |
      while IFS=: read -r ln hit; do
        [ "$(echo "$hit" | cut -d: -f3)" != "$own" ] && echo "$f:$ln: $hit"
      done
  done
  ```

  Whether the other twelve are violations or legitimate (some target items their
  parent `mod.rs` does not re-export) is a module-boundary call and belongs to
  `architect`, in its own demand. Do **not** smuggle a scan with an exception
  list into this one.

## Acceptance criteria

1. **The role vocabulary is closed and documented.** `panel.rs` matches on
   exactly the six roles LCV-123 and LCV-124 write — `user`, `assistant`,
   `error`, `tool`, `refused`, `note` — each with its own arm, plus the existing
   `_ =>` plain-label fallback which stays as the safety net. A doc comment on
   the render function lists the six and names the writer of each: every one is
   produced by LCV-123 (AC 3, AC 7, AC 11, AC 23), and `user` is additionally
   produced by LCV-124 when the prompt arrives from the command line.

2. **Action and outcome rows are distinguishable at a glance.** A `tool` row
   renders in `egui::TextStyle::Monospace` and is prefixed with `▸ `. It is not
   the same colour as assistant prose. Two consecutive `tool` rows are
   visually separable (each on its own line; no run-on).

3. **A refusal looks like a refusal.** A `refused` row renders in a warning
   colour distinct from both the normal text colour and the `error` red already
   used, and is prefixed with `⚠ `. The refusal text itself is LCV-123's and is
   rendered verbatim.

4. **The undo-shape note is the most findable thing after a turn.** A `note` row
   renders in `egui::TextStyle::Small` with a leading separator line, so the end
   of each turn is visually bounded. The operator can tell, without scrolling
   back, whether the turn is one `Ctrl+Z` or several.

5. **Order and scrolling are unchanged.** Rows render in `agent_chat` order,
   interleaved with assistant text; the scroll area keeps
   `.stick_to_bottom(true)`; long rows **wrap** rather than widening the panel
   or being truncated. A 300-character outcome does not push the panel wider.

6. **The key is never rendered.** A test sets a recognisable dummy key
   (`"sk-test-DO-NOT-LEAK"`), drives a turn through LCV-123's `arm_turn` seam,
   and asserts the substring (needle built with `concat!`) appears in no
   `agent_chat` content. The panel never reads `settings.agent_api_key` at all —
   bounded scan over `panel.rs`.

7. **`panel.rs` still only renders and reports.** Bounded scan: no
   `thread::spawn`, no `mpsc::channel`, no `Document`, no `History`. This
   re-asserts LCV-123 AC 2 so a panel demand cannot quietly reopen the defect.

8. **The settings form gains a Model field.** Labelled `Model`, a plain
   single-line `TextEdit` (not password-masked) bound to
   `settings.agent_model`, minimum width 320 logical pixels like its two
   siblings, with hint text `anthropic/claude-sonnet-4.6`. It sits between
   Endpoint URL and API Key.

9. **The settings form gains a step-budget slider.** `egui::Slider` bound to
   `settings.agent_step_budget`, labelled `Steps per turn`, with its range taken
   from `crate::agent::loop_::AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX`
   **referenced by name, not as the literals `1` and `32`** — a bounded scan
   asserts both constant names appear in `settings_ui.rs`. A one-line
   explanation sits under it: `How many tool calls one prompt may make. More
   steps means a bigger drawing per prompt, and more API calls.`

10. **The plaintext warning is unmissable and always visible.** Directly below
    the API Key row, rendered in a warning colour (not a tooltip, not on hover,
    not behind a collapsing header), the exact sentence:
    `The API key is stored in plain text in settings.json. Anyone who can read that file can read your key.`
    A bounded scan pins the sentence.

11. **`draw_agent_settings` reports every field.** It returns `true` when **any**
    of the four fields changed this frame and `false` otherwise. The existing
    close-to-persist wiring in `src/app/panels.rs::agent_settings_dialog` is
    unchanged, so a model or budget edit is written on dialog close through
    `App::persist_settings` — which is a no-op without an injected
    `settings_path` (ADR 0006).

12. **The slider cannot produce an out-of-range value**, and the clamp at the
    read site (LCV-123 AC 18) stays anyway: a settings file hand-edited to `200`
    still shows `32` on the slider after the dialog is opened and closed, and
    still runs at `32`.

13. **The deep-path import is closed, and only that one.** `src/agent/mod.rs`
    gains `pub use settings_ui::draw_agent_settings;` next to its six siblings
    (`wire`, `transport`, `bridge`, `tools`, `loop_` and `panel` each already
    have a `pub use` companion — `settings_ui` is the only one without, which is
    what makes this an omission rather than a design position), and
    `src/app/panels.rs` calls `crate::agent::draw_agent_settings(ui, settings)`.
    Two lines changed plus one added. A bounded scan over `panels.rs`'s
    implementation section asserts the needle
    `crate::agent::settings_ui::` (built with `concat!`) is **absent** and
    `crate::agent::draw_agent_settings` present. **Fix nothing else**: if the
    implementer notices another deep path anywhere in the tree, leave it and say
    so in the handover — see §Out of scope.

14. **Purity, caps, gates.** `panel.rs` and `settings_ui.rs` may import `egui`
    and MUST NOT import `eframe` or `rfd`; they remain the only two files under
    `src/agent/` that import `egui` (bounded scan over the whole `src/agent/`
    tree). Both are at or under 300 implementation LOC by ADR 0004's `awk`
    recipe, never `wc -l`, and the numbers are reported (**corrected
    2026-09-13**: `panel.rs` is **116** before this work, not the 155 this
    demand was written with — LCV-123 deleted `submit` from it — and
    `settings_ui.rs` is **56**, not 55; both re-measured with the ADR 0004
    recipe at `5452fd6` and at `96a8fb4`, which agree). `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and `cargo test --all` exit 0.

## Expected tests

`egui` rendering cannot be asserted pixel-wise at 0.29.1 (`egui_kittest` needs
≥ 0.30 and is out of scope, ADR 0002). So each rendering criterion is covered by
a **bounded source scan over the render function plus a headless frame that
proves the code path runs without panicking**, and the reviewer's eye plus the
manual smoke cover appearance. Say so in the test names; do not pretend a scan
is a rendering assertion.

- **Unit / AC 1** — a scan over `panel.rs`'s implementation section finding all
  six role literals, and a `App::default()` frame driven through
  `App::update_ui` with `agent_panel_open = true` and one row of **each** of the
  six roles plus one unknown role in `agent_chat`, asserting the frame completes
  and the fallback arm is exercised.
- **Unit / AC 2, AC 3, AC 4** — scans for `Monospace`, the `▸` prefix, the `⚠`
  prefix, a colour call distinct from the existing `Color32::RED`, `Small`, and
  the separator, each shown to discriminate (paste the literal into a comment
  and prove the scan still fails when the call is removed — bound the haystack
  to the implementation section).
- **Unit / AC 5** — a scan proving `stick_to_bottom(true)` survives and that the
  row widgets wrap; plus a headless frame with a 300-character `tool` row that
  completes.
- **Integration / AC 6** — the dummy-key absence test, and the scan proving
  `panel.rs` never reads `agent_api_key`.
- **Unit / AC 7** — the four scans, each shown to discriminate.
- **Unit / AC 8, AC 9, AC 10** — scans for the Model label and hint, the
  `Slider` with both constant names, the explanation line, and the exact warning
  sentence; plus a headless frame that draws the form into a real `egui::Ui` and
  completes.
- **Unit / AC 11** — `draw_agent_settings` returns `true` for a change in each
  of the four fields (four cases) and `false` for an untouched frame.
- **Unit / AC 12** — a `Settings` with `agent_step_budget: 200` drawn through
  the form and then read back through `clamp_step_budget` yields `32`.
- **Unit / AC 13** — the two bounded scans over `panels.rs`, each shown to
  discriminate, plus the compile itself: the old deep path stops resolving only
  if it is removed, so the positive control matters more than usual here.
- **Unit / AC 14** — the `egui` / `eframe` / `rfd` scans over `src/agent/` and
  the LOC measurements.
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result**:
  (a) delete the `refused` arm so refusals fall through to the plain label →
  AC 3's scan fails by name;
  (b) delete the `note` arm → AC 4 fails by name;
  (c) replace the constant names in the slider range with the literals `1..=32`
  → AC 9's scan fails by name;
  (d) move the warning sentence into `.on_hover_text(...)` → AC 10 fails by
  name;
  (e) make `draw_agent_settings` return `false` unconditionally → AC 11 fails
  by name;
  (f) restore the deep path in `panels.rs` → AC 13's absence scan fails by name.
- **[manual] smoke** — `cargo run`. Open `Help > Agent settings`: four rows, the
  key masked, the warning readable without hovering anything, the slider moving
  between 1 and 32. Set the budget to **1**, ask the agent for two lines, and
  confirm the step-budget error appears rather than a partial drawing. Set it
  back to 12. Ask for a 20 mm square: the tool rows appear one per action as the
  geometry lands, the note row at the end says one `Ctrl+Z`, and pressing
  `Ctrl+Z` once removes the whole square. Then click an entity in the middle of
  a turn and confirm the refusal row is impossible to miss and the note row says
  the actions stayed separate.

## Test hygiene (mandatory)

- **Bound every source scan** to the implementation section (slice at the offset
  of the bare `#[cfg(test)]` at column 0) and **build every needle with
  `concat!`**, or the scan matches its own literal. Canonical correct example:
  `guard_is_runtime_not_cfg` at `src/io/dialogs.rs:179-194`. Every scan must be
  **shown to discriminate** and the demonstration reported. A scan is the weakest
  test shape in this repo's history — this demand leans on several, so each one
  carries a higher burden of proof.
- ADR 0002 §A2: `App::default()`, never `App::new()`. §A4 rule 1: no test sends
  `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S`.
- ADR 0006 / ADR 0007 §D10: **no test may cause a settings write to a real
  per-user path.** Every settings test here leaves `settings_path` as `None` or
  points it at a `tempfile` directory the test owns, and never sets a real API
  key.
- **No CI test may reach a real endpoint.** No HTTP is needed in this demand at
  all: drive any turn through LCV-123's `arm_turn` seam.

## Risks

- **This demand is the one most likely to drift into decoration.** The scope is
  two things: make the §D5 disclosure legible, and make two functional settings
  reachable. Every criterion above ties to one of those. A bubble, an avatar or
  a timestamp is a rejected change, not a small one.
- **Source scans are the weakest evidence in this repo and this demand is full
  of them.** Six un-failable tests have shipped here. Every scan must be bounded
  and demonstrated; the manual smoke is not a formality for the rendering
  criteria, it is the actual acceptance.
- **`panel.rs` grows — but much less than this demand first assumed.** It is at
  **116** of 300 before this work, not 155: LCV-123 deleted `submit` from it.
  Six render arms cannot get near the cap from there, so the cap is no longer a
  live risk for this demand. The rule if it ever does bite is unchanged: split
  the per-role row renderer into a private helper in the same file before
  reaching for a new file, and flag `architect` rather than inventing a seam.
  (Note for whoever schedules LCV-129, which adds a Cancel button to the same
  thinking row: re-measure after this demand lands.)
- **A masked field plus a warning is a mixed message, deliberately.** The mask
  stops shoulder-surfing while the warning states the storage truth. Do not
  "resolve" the tension by unmasking the field or by dropping the warning.

## Notes

- Normative: [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D5 (the transcript is the disclosure mechanism for positional-index
  renumbering), §D6 (two undo behaviours exist and the transcript must show
  which happened), §D7 (the budget range lives with the loop), §D8 (`panel.rs`
  and `settings_ui.rs` are the only `egui` importers under `src/agent/`), §D10
  (plaintext key, and the UI must say so in words).
- The role values and every sentence rendered here are written by LCV-123 —
  `tool` and `refused` per action at the apply site (LCV-123 AC 23),
  `assistant` / `error` at turn end (its AC 7), `note` after that (its AC 11),
  `user` when the turn is armed (its AC 3) — and `user` again by LCV-124 when
  the prompt comes from the command line. This demand adds **no** new strings to
  `agent_chat` and appends no row of its own.
- Existing wiring that stays: `src/app/panels.rs::agent_settings_dialog`
  persists on close and scopes its borrows (LCV-119);
  `draw_agent_settings` already returns a `changed` bool with two fields —
  this demand takes it to four.
- **Measured before the work (ADR 0004 `awk`, at `96a8fb4`): `panel.rs` 116,
  `settings_ui.rs` 56.** The 155 this demand originally recorded was stale —
  LCV-123 removed `submit`. A third number, 103, circulated during refinement;
  it does not reproduce with
  `awk '/^#\[cfg\(test\)\]/{print NR-1; f=1; exit} END{if(!f) print NR}' src/agent/panel.rs`,
  which answers 116 at `5452fd6` and at `96a8fb4` alike. 116 is the number to
  use and to re-report after the work.
- **Parallelizable with LCV-124** once LCV-123 has landed: the two demands touch
  disjoint files (`panel.rs` + `settings_ui.rs` here; `classifier.rs` +
  `app/cmdline.rs` there).
