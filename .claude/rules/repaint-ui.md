---
paths:
  - "src/app/**"
  - "src/ui/**"
  - "src/render/**"
  - "src/io/dialogs.rs"
---
# UI, repaint and dialogs

- **Repaint policy**: every `ctx.request_repaint*` is conditional. Exactly three sites exist:
  `src/app/mod.rs` (`if self.agent.busy`), `src/app/autosave.rs::schedule_flush_repaint`
  (guarded on `dirty_since`, load-bearing for debounced autosave) and `src/app/viewport.rs`
  (`viewport_is_live`: hovered, dragged, live preview — three terms). Pinned by
  `every_repaint_request_in_src_is_conditional`. A fourth site or a dropped guard is a blocker.
- Frame order in `update`: input → commands → viewport paint → chrome.
- `rfd` only in `src/io/dialogs.rs`, disarmed until `crate::run()` arms it (ADR 0005).
- An `egui::Window` body with a `ScrollArea` is capped at 426pt regardless of screen (ADR 0009):
  assert headroom, derive the expected row set from the table, prefer a measured sizing call.
- The autosave dirty signal is `History::revision()` (ADR 0002).
