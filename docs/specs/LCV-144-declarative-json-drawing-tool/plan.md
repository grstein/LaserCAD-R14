# LCV-144 — Plan

## Approach

First a pure move: the eight narration helpers leave `agent_apply.rs` (290 LOC) for a
new `agent_narrate.rs` (ADR 0007 amendment 7). Then a new kernel-pure `src/agent/drawing.rs`
validates the JSON by hand and turns it into `Vec<DrawingItem>` (mm, radians). `tools.rs`
adds the registration and one parse arm. The worker's dispatch closure
(`agent_worker.rs::to_action`) checks the byte cap on every tool before `serde_json`.
On the UI thread, one new `apply` arm turns the items into `Entity` values and commits the
existing `document::commands::CreateEntities` once with `History::commit_grouped`, then
narrates with the post-commit numbers. Structurally that is one step, one `Act`, one command
and one revision (ADR 0010 §1, §6). No new command, thread or dependency; `create.rs` is
unchanged (`CreateEntities` already appends in order and undoes by truncating).

## Touches

- `src/app/agent_narrate.rs` (new) — `pt`, `sweep`, `kind`, `geometry`, `describe`,
  `bed_line`, `list_entities`, `list_selection` moved verbatim as `pub(super)`; plus
  `batch_created(n, first, count, revision) -> String`, which builds both AC 8 sentences.
- `src/app/agent_apply.rs` — imports from `agent_narrate`. `Planned` gains a `Batch(Box<dyn
  Command>, usize)` arm whose sentence `apply` builds *after* `commit_grouped` (revision and
  count are post-commit). Adds `DrawingItem → Entity` (`Line::new`, `Circle::new`,
  `GeoArc::new`, the same constructors the scalar arms use).
- `src/app/mod.rs` — `mod agent_narrate;` (292 → 293).
- `src/agent/drawing.rs` (new) — `DrawingItem { Line | Circle | Arc }`, `schema() -> Value`
  (only `type/properties/required/description/enum/minItems/maxItems/items`),
  `parse(args: &Value) -> Result<Vec<DrawingItem>, ToolCallError>`: root key set exactly
  `{version, entities}`, `version` must be an integer 1, 1..=1000 items, and each item's key
  set must be exactly its type's keys plus `type`. Numbers must be finite and `ccw` a bool.
  Radius goes through `tools::validate_r`. Degrees become radians only here. Unknown key
  names are cut to 64 chars and payload values are never echoed. Names no document type.
- `src/agent/tools.rs` — `validate_r` → `pub(crate)` (shared, not copied). `ToolCallError`
  gains `DrawingRoot { field: String, reason }` →
  `create_drawing {field}: {reason}` and `DrawingItem { index, field: String, reason }` →
  `create_drawing entities[{i}].{field}: {reason}` (ADR 0010 §3 Errors). Adds the
  `create_drawing` registration last and moves the order comment. One `parse_tool_call` arm
  delegates to `drawing::parse`.
- `src/agent/bridge.rs` — `AgentAction::CreateDrawing { items: Vec<DrawingItem> }`.
- `src/agent/mod.rs` — `pub mod drawing;` + re-export `DrawingItem`.
- `src/app/agent_worker.rs` — `MAX_TOOL_ARGUMENT_BYTES: usize = 1_048_576`. `to_action`
  refuses `args.len() > cap` as `Malformed` with
  ``tool `{tool}` arguments exceed 1048576 bytes`` before parsing (§D15 path, unchanged).
- `src/agent/prompt.rs` (LCV-143/151) — the built-in prompt describes `create_drawing` and
  its argument names, so LCV-151's "every advertised tool is named" test stays green.
- `AGENTS.md` §Purity rule — `drawing.rs` joins the kernel-pure `src/agent/` bucket (the
  `lcv128_normative_enumerations` scan requires it).
- `tests/it/lcv144_drawing_batch.rs` (new, `mod` line in `tests/it/main.rs`).
- ADRs: 0010 (all sections; AC 1 is its review gate, carried by T4), 0007 §D12–§D15; no amendment.

## Risks

- LOC cap: `agent_apply.rs` 290 → ~200 after T2, ~225 after the arm. `tools.rs` 202 →
  ~230. `agent_worker.rs` 155 plus LCV-143's prompt field stays under 200. `app/mod.rs`
  293 (one `mod` line). `drawing.rs` is new, ~150. None reaches 270.
- Mutation testing: **yes** — `src/agent/` is high risk (AGENTS.md). Target `drawing.rs`
  and `to_action`'s cap comparison (`>` vs `>=`: the exact-cap test pins it).
- LCV-151 seam (its plan): its coverage test walks `tool_definitions()` and needs every
  property name in the tool's own prompt paragraph, so T9 adds a `create_drawing` paragraph
  to `DEFAULT_PROMPT` and updates its golden test.
- Serde `Value::as_f64` accepts integers and floats alike, and `version` must be checked
  with `as_u64() == Some(1)` so that `1.0` and `"1"` are refused. The unit tests pin both.
- Cancel with the `Act` still queued: `end_turn` drops the reply; covered like LCV-129.
