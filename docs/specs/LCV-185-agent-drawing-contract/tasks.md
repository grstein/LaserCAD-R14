# LCV-185 — Tasks

- [ ] T1 [AC1] [AC2] [AC3] [AC4] Test: null foreign key accepted, non-null foreign key refused with the new reason, unpublished key keeps `unknown key`, one all-properties item per type draws one entity (files: tests/it/agent/drawing_batch.rs)
- [ ] T2 [AC1] [AC2] [AC3] [AC4] Three-way key check in `item`; `<keys>` from the type's list (files: src/agent/drawing.rs)
- [ ] T3 [AC5] [AC6] Test + code: schema item properties == union of the key lists; no `oneOf`/`anyOf`/`allOf`/`const`/`additionalProperties` anywhere in the schema (files: src/agent/drawing.rs, tests/it/agent/drawing_batch.rs)
- [ ] T4 [AC7] Test: prompt's create_drawing paragraph says other types' keys may be omitted or null; update `DEFAULT_PROMPT` (files: tests/it/agent/default_prompt.rs, src/agent/prompt.rs)
- [ ] T5 Amend ADR 0010 §2 wording on foreign keys (files: docs/adr/0010-declarative-drawing-batch-tool.md)
- [ ] T6 CHANGELOG line (files: CHANGELOG.md)
