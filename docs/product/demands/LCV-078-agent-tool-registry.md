# LCV-078 — Agent tool registry (CAD actions exposed to LLM)

- **Status**: Ready
- **Phase**: 7
- **Depends on**: LCV-023 (done), LCV-024 (done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

The agent harness (Phase 7) must translate LLM intent into geometry: when the
operator types `/ai draw a 40 mm circle at origin`, the multi-turn loop
(LCV-079) calls the LLM with a list of available CAD tools and receives back a
`tool_call` JSON object. Without a tool registry that (a) advertises the
available tools in OpenAI function-calling schema format and (b) dispatches
incoming `tool_call` payloads to the correct `Command` + `History::commit`
invocation, Phase 7 cannot close the loop between LLM reply and drawing state
change.

The five operations the LLM needs are exactly the five already implemented:
create line/circle/arc (LCV-023), delete entity, move entity (LCV-024).
No new geometry; just the bridge between JSON and existing commands.

User outcome: when the multi-turn loop (LCV-079) receives a `create_circle`
tool call from the LLM, calling `dispatch_tool_call("create_circle", args, doc,
history)` places a new circle in the document via the history stack and returns
a human-readable confirmation string — the same undo-safe path a human operator
uses when clicking with `CircleTool`.

## Scope

### 1 — New file `src/agent/tools.rs`

Pure Rust; **MUST NOT** import `egui`, `eframe`, or `rfd`. The file stays
≤ 300 LOC. If the inline JSON literals and dispatch arms together push against
the cap, extract a private `fn get_f64` (and `fn get_index`) helper (see §4
below) to compress repeated field-parsing lines before considering a file
split.

### 2 — `ToolCallError`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolCallError {
    /// The tool name is not one of the five registered tools.
    #[error("unknown tool: `{0}`")]
    UnknownTool(String),

    /// A required argument is absent from the JSON object.
    #[error("tool `{tool}` missing required argument `{field}`")]
    MissingField { tool: &'static str, field: &'static str },

    /// An argument is present but has an invalid type or value.
    #[error("tool `{tool}` argument `{field}` is invalid: {reason}")]
    InvalidArg {
        tool: &'static str,
        field: &'static str,
        reason: String,
    },
}
```

`thiserror` (v2) is already in `Cargo.toml`; no dependency change.

### 3 — `pub fn tool_definitions() -> serde_json::Value`

Returns `serde_json::Value::Array` with exactly **5** elements, each following
the OpenAI function-calling schema:

```json
{
  "type": "function",
  "function": {
    "name": "<tool_name>",
    "description": "<one sentence>",
    "parameters": {
      "type": "object",
      "properties": { ... },
      "required": [ ... ]
    }
  }
}
```

The five tools in array order:

| # | name | parameters (JSON `"type"`) | `"required"` |
|---|---|---|---|
| 0 | `create_line` | `x1`, `y1`, `x2`, `y2` — all `"number"` (mm) | all four |
| 1 | `create_circle` | `cx`, `cy` — `"number"` (mm), `r` — `"number"` (mm, > 0) | all three |
| 2 | `create_arc` | `cx`, `cy` — `"number"` (mm), `r` — `"number"` (mm, > 0), `start_deg` — `"number"` (degrees), `end_deg` — `"number"` (degrees), `ccw` — `"boolean"` | all six |
| 3 | `delete_entity` | `index` — `"integer"` (zero-based) | `index` |
| 4 | `move_entity` | `index` — `"integer"` (zero-based), `dx` — `"number"` (mm), `dy` — `"number"` (mm) | all three |

Descriptions (exact text, so tool-call tests can verify them):
- `create_line`: `"Create a straight line segment between two endpoints in mm."`
- `create_circle`: `"Create a full circle with a given center and radius in mm."`
- `create_arc`: `"Create a circular arc. start_deg and end_deg are in degrees (0 = +X axis). ccw=true means counter-clockwise sweep."`
- `delete_entity`: `"Delete the entity at the given zero-based document index."`
- `move_entity`: `"Translate a single entity by (dx, dy) mm."`

Use `serde_json::json!` macro for the entire literal; no external JSON file.
`serde_json` is already in `Cargo.toml`.

### 4 — `pub fn dispatch_tool_call`

```rust
pub fn dispatch_tool_call(
    name: &str,
    args: &serde_json::Value,
    doc: &mut crate::document::Document,
    history: &mut crate::document::History,
) -> Result<String, ToolCallError>
```

**Field-parsing helpers (private, inside `tools.rs`)**

To stay within 300 LOC, define two private helpers at the top of the function
section:

```rust
fn get_f64(
    args: &serde_json::Value,
    tool: &'static str,
    field: &'static str,
) -> Result<f64, ToolCallError> {
    args.get(field)
        .and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field })
}

fn get_index(
    args: &serde_json::Value,
    tool: &'static str,
    doc_len: usize,
) -> Result<usize, ToolCallError> {
    let raw = args
        .get("index")
        .and_then(|v| v.as_f64())
        .ok_or(ToolCallError::MissingField { tool, field: "index" })?;
    if raw < 0.0 || raw.fract() != 0.0 || !raw.is_finite() {
        return Err(ToolCallError::InvalidArg {
            tool,
            field: "index",
            reason: format!("{raw} is not a non-negative integer"),
        });
    }
    let idx = raw as usize;
    if idx >= doc_len {
        return Err(ToolCallError::InvalidArg {
            tool,
            field: "index",
            reason: format!("index {idx} is out of range (document has {doc_len} entities)"),
        });
    }
    Ok(idx)
}
```

`get_f64` is intentionally permissive about "missing vs wrong type" — an absent
field and a `null` both return `MissingField`. If a field is present as a
string, `as_f64()` returns `None`, which also yields `MissingField`. This is
acceptable for MVP; a stricter variant can distinguish the two cases in a later
demand if LLM output quality requires it.

`get_index` uses `as_f64()` so it accepts both integer JSON values (`3`) and
float JSON values that happen to be whole numbers (`3.0`), which LLMs sometimes
emit despite the schema declaring `"integer"`.

**Dispatch arms** (`match name`):

- `"create_line"`:
  1. Parse `x1`, `y1`, `x2`, `y2` via `get_f64`.
  2. Construct `CreateLine::new(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))`.
  3. Call `history.commit(Box::new(cmd), doc)`.
  4. Return `Ok(format!("Line created: ({x1:.3}, {y1:.3}) → ({x2:.3}, {y2:.3}) mm."))`.

- `"create_circle"`:
  1. Parse `cx`, `cy`, `r` via `get_f64`.
  2. Validate `r > 0.0 && r.is_finite()`; if not, return `Err(ToolCallError::InvalidArg { tool: "create_circle", field: "r", reason: format!("{r} is not a positive finite number") })`.
  3. Construct `CreateCircle::new(Circle::new(Vec2::new(cx, cy), r))`.
  4. `history.commit(Box::new(cmd), doc)`.
  5. Return `Ok(format!("Circle created: center ({cx:.3}, {cy:.3}) mm, r = {r:.3} mm."))`.

- `"create_arc"`:
  1. Parse `cx`, `cy`, `r`, `start_deg`, `end_deg` via `get_f64`.
  2. Parse `ccw` via `args.get("ccw").and_then(|v| v.as_bool()).ok_or(ToolCallError::MissingField { tool: "create_arc", field: "ccw" })`.
  3. Validate `r > 0.0 && r.is_finite()`.
  4. Convert: `let start_rad = start_deg.to_radians(); let end_rad = end_deg.to_radians();`.
  5. Construct `CreateArc::new(Arc::new(Vec2::new(cx, cy), r, start_rad, end_rad, ccw))`.
  6. `history.commit(Box::new(cmd), doc)`.
  7. `let dir = if ccw { "ccw" } else { "cw" };`
  8. Return `Ok(format!("Arc created: center ({cx:.3}, {cy:.3}) mm, r = {r:.3} mm, {start_deg:.1}°→{end_deg:.1}° {dir}."))`.

- `"delete_entity"`:
  1. Parse `idx` via `get_index(args, "delete_entity", doc.entities.len())`.
  2. Construct `DeleteEntities::new(vec![idx])`.
  3. `history.commit(Box::new(cmd), doc)`.
  4. Return `Ok(format!("Entity {idx} deleted."))`.

- `"move_entity"`:
  1. Parse `dx`, `dy` via `get_f64`.
  2. Parse `idx` via `get_index(args, "move_entity", doc.entities.len())`.
  3. Construct `MoveEntities::new(vec![idx], Vec2::new(dx, dy))`.
  4. `history.commit(Box::new(cmd), doc)`.
  5. Return `Ok(format!("Entity {idx} moved by ({dx:.3}, {dy:.3}) mm."))`.

- `_` (anything else):
  Return `Err(ToolCallError::UnknownTool(name.to_owned()))`.

### 5 — `src/agent/mod.rs` update

Add after the existing `pub const MODULE`:

```rust
pub mod tools;
pub use tools::{dispatch_tool_call, tool_definitions, ToolCallError};
```

### 6 — Kernel-purity

`grep -nE '^use (egui|eframe|rfd)' src/agent/tools.rs` must return no matches.
`src/agent/tools.rs` may import only from `crate::document`, `crate::geometry`,
`serde_json`, and `thiserror`.

## Out of scope

- HTTP transport (`reqwest` / `tokio`) — owned by **LCV-077**.
- Multi-turn loop that calls `tool_definitions()` to build the API payload and
  calls `dispatch_tool_call` on each response — owned by **LCV-079**.
- Command-line wiring of the `:` / `/ai` prefix — owned by **LCV-080**.
- Any tool beyond the five specified: no `trim_entity`, no `create_rect`,
  no `create_polyline`, no `set_layer`. The registry is strictly the minimal
  set that covers LCV-023 and LCV-024.
- Batch tool calls (multiple mutations in one dispatch invocation).
- Validation of geometric intent (degenerate lines, zero-sweep arcs). The
  existing commands do not validate; the registry mirrors that contract. The
  LLM is responsible for sending sensible geometry.
- Tool schema versioning or a dynamic registry. The five tools are hardcoded.
- Any change to existing `Command` implementations, `Document`, or `History`.

## Acceptance criteria

1. `src/agent/tools.rs` exists. `src/agent/mod.rs` declares `pub mod tools;`
   and re-exports `dispatch_tool_call`, `tool_definitions`, `ToolCallError`.

2. `ToolCallError` is a `thiserror::Error` with exactly three variants:
   `UnknownTool(String)`, `MissingField { tool: &'static str, field: &'static str }`,
   `InvalidArg { tool: &'static str, field: &'static str, reason: String }`.
   All three implement `std::error::Error` via `thiserror`.

3. `tool_definitions()` returns a `serde_json::Value::Array` of length 5. Each
   element satisfies:
   - `element["type"] == "function"`.
   - `element["function"]["name"]` is one of the five names in the table above
     (in order: `create_line`, `create_circle`, `create_arc`, `delete_entity`,
     `move_entity`).
   - `element["function"]["parameters"]["type"] == "object"`.

4. `tool_definitions()` element `0` (`create_line`) has `"required"` equal to
   `["x1", "y1", "x2", "y2"]`; element `1` (`create_circle`) has `"required"`
   `["cx", "cy", "r"]`; element `2` (`create_arc`) has `"required"`
   `["cx", "cy", "r", "start_deg", "end_deg", "ccw"]`; element `3`
   (`delete_entity`) has `"required"` `["index"]`; element `4` (`move_entity`)
   has `"required"` `["index", "dx", "dy"]`.

5. `tool_definitions()` element `2` (`create_arc`) has `"ccw"` property with
   `"type": "boolean"`.

6. **dispatch create_line**: starting from `Document::default()` and
   `History::new()`, calling
   `dispatch_tool_call("create_line", &json!({"x1":0.0,"y1":0.0,"x2":10.0,"y2":0.0}), &mut doc, &mut history)`
   returns `Ok(s)` where `s` contains `"Line created"`, and
   `doc.entities.len() == 1`, and `doc.entities[0] == Entity::Line(Line::new(Vec2::new(0.0,0.0), Vec2::new(10.0,0.0)))`.

7. **dispatch create_circle**: calling with
   `json!({"cx":5.0,"cy":5.0,"r":3.0})` returns `Ok(s)` where `s` contains
   `"Circle created"`, and `doc.entities` contains the corresponding circle.
   Calling with `json!({"cx":0.0,"cy":0.0,"r":-1.0})` returns
   `Err(ToolCallError::InvalidArg { field: "r", .. })`.

8. **dispatch create_arc — degree→radian conversion**: calling with
   `json!({"cx":0.0,"cy":0.0,"r":1.0,"start_deg":0.0,"end_deg":90.0,"ccw":true})`
   returns `Ok(s)` and commits an `Arc` whose `start_angle` is `0.0` and
   `end_angle` is within `EPSILON` of `std::f64::consts::FRAC_PI_2`.
   The return string contains `"Arc created"`, `"0.0°→90.0°"`, and `"ccw"`.

9. **dispatch delete_entity**: with a document containing one entity at index 0,
   calling with `json!({"index":0})` returns `Ok(s)` containing `"Entity 0 deleted"`,
   and `doc.entities.is_empty()`. Calling `history.undo(&mut doc)` restores the
   entity (the commit went through the history stack). Calling with
   `json!({"index":5})` on a document with fewer than 6 entities returns
   `Err(ToolCallError::InvalidArg { field: "index", .. })`.

10. **dispatch move_entity**: with a document containing one line at index 0
    from `(0,0)` to `(10,0)`, calling with `json!({"index":0,"dx":3.0,"dy":4.0})`
    returns `Ok(s)` containing `"Entity 0 moved"`, and the line endpoints are
    within `EPSILON` of `(3,4)` and `(13,4)`. Calling `history.undo(&mut doc)`
    restores the original endpoints.

11. **UnknownTool**: `dispatch_tool_call("frobnicate", &json!({}), &mut doc, &mut history)`
    returns `Err(ToolCallError::UnknownTool(s))` where `s == "frobnicate"`.

12. **MissingField**: `dispatch_tool_call("create_line", &json!({"x1":0.0}), ...)`
    returns `Err(ToolCallError::MissingField { tool: "create_line", .. })`.
    The document is unchanged and no command was committed to history.

13. All mutations reach the document exclusively through `history.commit` — the
    command count in `history.undo_stack` (observable via `history.undo(&mut doc)`
    succeeding) equals the number of successful `dispatch_tool_call` calls.

14. `ToolCallError` implements `Display` (via `thiserror`) — all three variants
    format to a non-empty string.

15. Kernel-purity: `grep -nE '^use (egui|eframe|rfd)' src/agent/tools.rs` returns
    no matches.

16. Size: `wc -l src/agent/tools.rs` reports `<= 300`.

17. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo test --all` all exit 0.

## Expected tests

All in `#[cfg(test)] mod tests` inside `src/agent/tools.rs`.

- **`tool_definitions_array_length_is_five`** (AC 3): assert array length == 5.
- **`tool_definitions_names_in_order`** (AC 3): assert the five `function.name`
  strings in index order.
- **`tool_definitions_types_are_function`** (AC 3): assert each `type` is
  `"function"`.
- **`tool_definitions_required_fields`** (AC 4): assert `required` arrays for
  all five tools.
- **`tool_definitions_arc_ccw_is_boolean`** (AC 5): assert
  `defs[2]["function"]["parameters"]["properties"]["ccw"]["type"] == "boolean"`.
- **`dispatch_create_line_happy_path`** (AC 6): full round-trip including entity
  content check.
- **`dispatch_create_circle_happy_path`** (AC 7): entity check.
- **`dispatch_create_circle_negative_radius`** (AC 7): `InvalidArg` on `r = -1`.
- **`dispatch_create_arc_degree_to_radian_conversion`** (AC 8): checks
  `end_angle ≈ FRAC_PI_2` and return string contains `"0.0°→90.0°"` and `"ccw"`.
- **`dispatch_create_arc_clockwise_label`** (AC 8): `ccw: false` → return
  string contains `"cw"`.
- **`dispatch_delete_entity_happy_path`** (AC 9): removes entity; undo restores.
- **`dispatch_delete_entity_out_of_range`** (AC 9): `InvalidArg` for `index 5`
  on a 1-entity document.
- **`dispatch_move_entity_happy_path`** (AC 10): endpoint check within `EPSILON`; undo restores.
- **`dispatch_unknown_tool`** (AC 11): checks variant and payload string.
- **`dispatch_missing_field`** (AC 12): document unchanged after error.
- **`dispatch_missing_field_no_commit`** (AC 12 / AC 13): `history.undo` returns
  `false` after a failed dispatch (nothing was committed).
- **`tool_call_error_display_non_empty`** (AC 14): all three variants display
  to a non-empty string.

## Open questions

(none)

## Notes

- **Why degrees in the schema?** LLMs reason about angles in degrees
  (AutoCAD's command line also accepts degrees). Radians are a kernel-internal
  concept. The bridge conversion (`f64::to_radians`) is the right place for
  this translation — here, at the entry point of the agent layer, not inside
  `CreateArc`.

- **Why `&'static str` for `tool` and `field` in `ToolCallError`?** Tool names
  and field names are string literals known at compile time; `&'static str`
  avoids an allocation on every error path and is consistent with the `label()`
  pattern in `Command`.

- **Why is `get_f64` intentionally permissive?** Distinguishing "field absent"
  from "field present but wrong type" requires a two-step check. For the MVP
  agent loop (LCV-079), both conditions indicate a malformed LLM output and the
  same retry strategy applies. LCV-079 can tighten error reporting if needed.

- **`get_index` accepts `3.0` from JSON.** Several popular LLMs emit
  `"index": 3.0` despite a schema declaring `"integer"`. `as_f64()` + truncation
  guard is the pragmatic defensive choice.

- **All mutations through `history.commit`.** `dispatch_tool_call` never calls
  `cmd.do_` directly. `History::commit` (LCV-026) runs `do_` internally and
  pushes to the undo stack — same path the interactive tools use. This ensures
  agent-issued mutations are Ctrl+Z-reversible.

- **LOC budget note.** The `serde_json::json!` literal for five tools and the
  five dispatch arms together occupy roughly 170–200 lines. Add the error type,
  helpers, module header, use statements, and tests and the file will be near
  the 300-LOC cap. The two private helpers (`get_f64`, `get_index`) are
  necessary to stay inside the limit; do not inline field extraction per arm.
  If the file still exceeds 300 LOC, extract `tool_definitions()` into a
  private `fn schemas() -> serde_json::Value` and inline it as the return value,
  reducing indentation overhead.

- **Dependency check.** `serde_json = "1"` and `thiserror = "2"` are already
  in `Cargo.toml`. `reqwest`, `tokio`, and `egui` must NOT be added by this
  demand.

- **`CreateEntities` re-export.** `src/document/commands/mod.rs` re-exports
  `CreateEntities` alongside `CreateLine` etc. This demand does not use
  `CreateEntities`; use the single-entity constructors `CreateLine::new`,
  `CreateCircle::new`, `CreateArc::new` from LCV-023.

- **`Arc` name collision.** `crate::geometry::Arc` shadows `std::sync::Arc`.
  The implementer must use `crate::geometry::Arc` explicitly or alias:
  `use crate::geometry::{Arc as GeoArc, Circle, Line, Vec2};`. Either is
  acceptable; the test suite already uses `use crate::geometry::Arc;` elsewhere.

- **LCV-079 consumer.** The multi-turn loop will call `tool_definitions()`
  once per conversation to build the `tools` array in the OpenAI API request,
  and will call `dispatch_tool_call(name, &args, doc, history)` for each
  `tool_call` element in the LLM response. The exact API integration is LCV-079's
  scope; this demand delivers only the two public functions and the error type.
