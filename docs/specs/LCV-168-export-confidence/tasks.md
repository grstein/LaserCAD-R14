# LCV-168 — Tasks

- [x] T1 [AC2] Test: `outside_bed` unit tests — inside, on the edge (within EPSILON), past each
  side by 2·EPSILON, an arc whose bulge crosses the top edge; `outside_bed_count` with a layer
  filter (files: src/document/bed.rs)
- [x] T2 [AC2] `outside_bed` + `Document::outside_bed_count`, kernel-pure; `mod bed;` and the
  re-export (files: src/document/bed.rs, src/document/mod.rs)
- [x] T3 [AC1] Test: Save into a tempdir with in-bed geometry → Info
  `Saved <name> (400 × 400 mm)`; a 297.5 mm bed prints `297.5` (files:
  tests/it/app/document_title_and_file_feedback.rs)
- [x] T4 [AC1] Extract `write_mother`; add `announce_saved` + `outside_bed_phrase`; call it on
  Save and Save As success; update any Save test that expected empty feedback (files:
  src/io/file_actions.rs, src/io/file_actions/tests.rs)
- [x] T5 [AC1] Test: Save As success path — `write_mother` then `announce_saved` on a tempdir
  path gives the Info line; the source-scan test pins that both actions call both helpers
  (files: src/io/file_actions/tests.rs)
- [ ] T6 [AC2] Test: Save with one and with two out-of-bed entities → Warning with
  ` — 1 entity outside the bed` / ` — 2 entities outside the bed`; an Output-off layer's
  entity still counts (files: tests/it/app/document_title_and_file_feedback.rs)
- [ ] T7 [AC2] Warning severity + suffix in `announce_saved` (files: src/io/file_actions.rs)
- [ ] T8 [AC3] Test: Export Layers with an out-of-bed entity on an exported layer → Warning
  `Exported layers: … — 1 entity outside the bed`; one on an Output-off layer only → Info, no
  suffix (files: tests/it/io_svg/export_layers.rs)
- [ ] T9 [AC3] Suffix and severity in `action_export_layers`, counting only the plan's layers
  (files: src/io/export_layers.rs)
- [ ] T10 [AC4] Test: with out-of-bed geometry, the saved mother equals `export_svg(&doc)` byte
  for byte and every layer file equals its `layer_exports` text (files:
  tests/it/io_svg/export_layers.rs)
- [ ] T11 [AC5] Test: Save to a path in a missing folder, Export Layers with the mother in a
  missing folder, and Save As cancelled (disarmed dialog) each leave a sentinel
  `command_feedback` unchanged and write no `Saved`/warning text; the failures set
  `error_message` (files: tests/it/app/document_title_and_file_feedback.rs,
  tests/it/io_svg/export_layers.rs)
- [ ] T12 [AC6] Test: an autosave flush with an out-of-bed entity leaves a sentinel
  `command_feedback` unchanged (files: tests/it/app/autosave_dirty.rs)
- [ ] T13 [AC7] DESIGN.md §7 (dock messages: `Saved …` Info, out-of-bed Warning) and §9 (the
  ` — n entities outside the bed` suffix pattern) (files: DESIGN.md)
- [ ] T14 CHANGELOG line: Save confirms the file and warns about geometry outside the bed
  (files: CHANGELOG.md)
