# LCV-143 — Plan

## Approach

Move the hardcoded prompt out of `loop_.rs` into a new kernel-pure `src/agent/prompt.rs`
(`DEFAULT_PROMPT` + `resolve(Option<&str>) -> &str`, no trim, no fallback for `""`).
`Settings` stores a plain `Option<String>`; `turn_config` resolves it once into a new
`TurnConfig::system_prompt: String` (ADR 0007 §D13, the field that amendment already
reserves for LCV-143), and `drive_turn` seeds the system message from it. The editor is
stateless: each frame it copies the effective text into a local buffer, and only a real
`changed()` writes `Some(buffer)`, so opening the dialog never creates an override.
`Restore default` sets `None` and redraws from the built-in text in the same frame.
Persistence stays on the LCV-141 Done/× path; no new transaction, thread or channel.

## Touches

- `src/agent/prompt.rs` (new) — `DEFAULT_PROMPT` (the spec's text, exact), `resolve`;
  the index-contract test moves here from `loop_.rs` and is rewritten for the new text.
- `src/agent/mod.rs` — `pub mod prompt; pub use prompt::DEFAULT_PROMPT;` (callers use `agent::prompt::resolve`)
  (one re-export line); drop the `AGENT_SYSTEM_PROMPT` re-export.
- `src/agent/loop_.rs` — delete `AGENT_SYSTEM_PROMPT` and its test (190 → ~175 LOC).
- `src/io/settings.rs::Settings` — `#[serde(default)] agent_system_prompt: Option<String>`;
  `Default` sets `None`. No `crate::agent` import.
- `src/io/settings_store.rs` (new, ADR 0007 §D8 seam) — `platform_path`, `load_from`,
  `save_to`, `sibling_with_suffix`, `.bak` logic and their tests; `settings.rs`
  re-exports them `pub(crate)` so callers (`app/init.rs`, `app/persist.rs`, `io/*`) are untouched.
- `src/app/agent_worker.rs::TurnConfig` — `system_prompt: String`; manual `Debug` prints
  `system_prompt_len` only. `drive_turn` takes `system: &str` (5 params) instead of
  reading the constant.
- `src/app/agent_turn.rs::turn_config` — `system_prompt: prompt::resolve(settings.agent_system_prompt.as_deref()).to_owned()`.
- `src/agent/settings_ui.rs::draw_agent_settings` — "System prompt" label, `TextEdit::multiline`
  inside a height-capped inner `ScrollArea` (so a long prompt cannot push Done off the
  426pt body), `Restore default` button; doc says five fields.
- `AGENTS.md` §Purity rule — add `prompt.rs` to the kernel-pure list
  (`tests/it/lcv128_normative_enumerations.rs` fails otherwise).
- ADRs: none new. ADR 0007 §D13/§D8 already sanction the field and the seam.

## Seams for later specs

- LCV-151 rewrites only `DEFAULT_PROMPT` and the golden test in `prompt.rs`.
- LCV-144 (ADR 0010): the default already says "If create_drawing is advertised"; the
  prompt stays static — no flag reaches `resolve`.
- LCV-145 (ADR 0011): adds `TurnConfig::vision: bool` beside `system_prompt` and
  `tool_definitions(vision)`; the manual `Debug` gains one field. The prompt is still
  resolved once in `turn_config`, never re-read by the worker.

## Risks

- LOC cap: `io/settings.rs` 266 → ~274 with the field, so the §D8 seam runs first
  (T1) and drops it to ~200. `settings_ui.rs` 174 → ~200, `agent_worker.rs` 155 → ~162,
  `agent_turn.rs` 232 → ~234, `prompt.rs` ~60. None near 270 after the change.
- Mutation testing: yes — `src/agent/` is high-risk. Scope `cargo mutants` to
  `src/agent/prompt.rs` and `src/app/agent_worker.rs::drive_turn`/`TurnConfig::fmt`,
  `src/app/agent_turn.rs::turn_config`; T5's "first message is `config.system_prompt`"
  test must kill the mutation back to a constant.
- Existing `loop_.rs::the_system_prompt_discloses_the_index_contract` asserts
  "millimetres (mm)", "instead of retrying" and no `\n` — all false for the spec's
  text. It is replaced (not weakened) by the exact golden + index needles in `prompt.rs`.
- Egui `TextEdit` on a per-frame buffer keeps cursor state by widget id; a harness test
  types several characters across frames to prove edits accumulate verbatim.
- AC 7 scan: `src/app/mod.rs` has a `println!`/`eprintln!` today; the scan targets
  `src/agent/`, `agent_turn.rs`, `agent_worker.rs` impl sections only, with a positive
  control proving the matcher fires.
