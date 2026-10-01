# LCV-199 — Tasks

- [x] T1 [AC4] `agent/attachment.rs`: `ImageKind`, `sniff`, `check`, with unit tests (PNG, JPEG,
  GIF refused, 2 MB exactly accepted, 2 MB + 1 refused); AGENTS.md purity list
  (files: src/agent/attachment.rs, src/agent/mod.rs, AGENTS.md)
- [x] T2 [P] [AC1] `pick_image_dialog` + disarmed-panic test; extend the repo scan if it lists the
  wrappers (files: src/io/dialogs.rs, tests/it/repo/dialogs_disarmed.rs)
- [x] T3 [AC2] `ContentPart::image(mime, bytes)`, `png` delegating, JPEG data-URL unit test
  (files: src/agent/wire.rs)
- [x] T4 [AC2] [AC7] `TurnConfig.image` + redacted `Debug`; `drive_turn` builds the parts user
  message. Unit tests: image request shape, and no-image requests byte-identical
  (files: src/app/agent_worker.rs, src/app/agent_worker/tests.rs)
- [x] T5 [AC2] [AC6] `send_images` consent and capture split; `turn_record` keeps `image elided`.
  Unit tests: an attached image asks nothing, a later capture still asks, memory holds the
  placeholder (files: src/agent/loop_/images.rs, src/agent/memory.rs, src/app/agent_memory.rs)
- [x] T6 [AC4] [AC5] `agent_attach.rs` (`attach_image`, `poll_attach_request`, `take_for_send`) and
  the `AgentState` fields (files: src/app/agent_attach.rs, src/app/agent_state.rs, src/app/mod.rs)
- [ ] T7 [AC2] [AC5] `start_turn` takes the attachment: `Image:` row, clear, or refuse and restore the
  draft (files: src/app/agent_turn.rs)
- [ ] T8 [AC1] [AC3] Panel chip + `Attach image…` button + tooltip; `panels.rs` polls the request
  (files: src/agent/panel.rs, src/app/panels.rs)
- [ ] T9 [AC1]–[AC7] Integration tests, one per AC (attach through `attach_image` with a temp PNG,
  JPEG and GIF; AC5 deletes the file before `start_turn`; AC6 checks the next turn's
  `config_for` memory) (files: tests/it/agent/reference_image.rs, tests/it/agent/mod.rs)
- [ ] T10 `scripts/mutants.sh` on the diff; kill or justify survivors (files: src/agent/attachment.rs)
- [ ] T11 CHANGELOG line (files: CHANGELOG.md)
