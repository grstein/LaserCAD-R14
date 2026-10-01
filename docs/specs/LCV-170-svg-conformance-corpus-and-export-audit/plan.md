# LCV-170 — Plan

## Approach

Three test-side pieces and one kernel check:

1. **Corpus** (`tests/fixtures/svg/`, runner `tests/it/io_svg/corpus.rs`). The runner reads the
   directory at run time and pairs each `<name>.svg` with `<name>.expected`. It parses the
   expectation with a small line format (`corpus/expected.rs`), imports with `import_svg` +
   `into_document`, and compares. It collects every failure and panics once, naming each file.
   The same function runs on a temporary directory to prove AC 3.
2. **Export audit** (`tests/it/io_svg/export_audit.rs`). Builds the audit set with the public
   `Document` API and checks each `export_svg`/`export_layer_svg` output with `roxmltree`
   (already a dependency): namespace, allowed elements and attributes, number syntax, `d` shape,
   and the reopen round trip.
3. **Golden bytes** (AC 10). Before the kernel change, the audit test pins today's exact output
   for one mixed document as a `const` string.
4. **Kernel**: `document/layer.rs::check_fields` refuses any `char::is_control` character with the
   new `LayerError::ControlChar`. Every path already goes through it: dialog
   (`check_edit_layer`), `AddLayer`, `Document::from_parts`, and import (`LayerReader::enter`).

`.expected` format, one record per line, `#` comments, numbers are world mm, angles in degrees
(converted to radians and compared modulo 2π within 1e-9):

```
bed 300 180
layer "Cut" #ff0000 output=1 current=1      # listed in document order; index = position
line 0 x1 y1 x2 y2                           # first field = layer index
circle 0 cx cy r
arc 1 cx cy r start_deg end_deg ccw|cw
error MalformedLayer                         # alone: import must fail with this variant
```

## Touches

- `src/document/layer.rs::LayerError` — new variant `ControlChar(String)`. Its `Display` is
  `Layer name "…" has a control character.`, with the name shown through `escape_debug`.
- `src/document/layer.rs::check_fields` — control-character test before the `name_key` test.
- `tests/it/io_svg/{corpus.rs, corpus/expected.rs, export_audit.rs, mod.rs}` — new.
- `tests/fixtures/svg/*.svg|.expected` — five pairs (four seeds + one control-character error).
- `tests/it/document/layers.rs`, `tests/it/app/layers_dialog.rs`, `tests/it/agent/layers.rs` —
  AC 9 cases.
- `src/io/svg/export.rs`, `layers.rs`, `import.rs` — **not touched**.
- ADRs: none.

## Export contract changes

None. `export.rs` is unchanged. The only output difference is that a document can no longer hold
a control-character layer name. AC 10 pins today's bytes.

## Decisions (self-approved per user goal)

- The corpus runner lists the directory at run time, so adding a fixture needs no code change. An
  `.expected` with no `.svg` is also reported.
- Expected geometry is written by hand from the SVG text (mirror by hand), never by running
  `import_svg`. This keeps the oracle independent (spec: "against SVG 2").
- Arc angles are compared modulo 2π, because the kernel's angle normalisation is not part of SVG.
- Command line: `layer`/`la` only opens the Layers dialog, and there is no typed layer name, so
  the command-line case of AC 9 is covered by one test: open the dialog by typing, then apply.
- Agent: the agent cannot create or rename layers (ADR 0012 §6). Its test asserts that a creation
  call naming a control-character layer is refused and leaves the document unchanged. It does
  not check the message text, which LCV-192 rewords on the agent branch.
- Import: the fixture uses `&#9;`. A raw tab would be normalised to a space by XML attribute
  rules, and a raw C0 character is an XML parse error. C1 characters (`&#x85;`) are also covered
  in the unit test.
- Only `is_control` is refused, as the AC says; noncharacters (U+FFFE/U+FFFF) wait for LCV-171.

## Risks

- LOC cap: `layer.rs` 142 → ~150. `state.rs` (276) and `import.rs` (276) are not touched.
  Integration test modules are exempt.
- Mutation testing: no. `export.rs` is unchanged and the kernel change is a single predicate,
  pinned by unit tests.
- An old autosave or mother file with a tab in a layer name now fails to open through
  `from_parts`. Accepted: tabs were already normalised to spaces on import, so none exist in
  practice.
- If an audit test finds a real non-conformance, fixing it changes `export.rs`, which is outside
  this plan. Stop, record it under "Export contract changes", and ask before continuing.
- Parallel branches (`ui`, `agent-harness`) edit `tests/it/app/` and `tests/it/agent/`. The AC 9
  tests only append new functions, to keep rebases trivial.

- Review addendum: T9 also touched `src/app/agent_apply.rs::target_layer` (the agent name lookup folds `"Cut\u{7}"` onto `Cut` via `name_key`).
