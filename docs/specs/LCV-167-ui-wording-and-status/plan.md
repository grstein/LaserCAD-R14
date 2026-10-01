# LCV-167 — Plan

## Approach

Mostly string edits, each pinned by a painted-text or returned-string test. Three behaviour changes:
- `status.error` becomes a `ui/theme.rs` token that `apply_theme` writes into egui's own
  `visuals.error_fg_color`. The AI panel and the dock read `ui.visuals().error_fg_color`,
  exactly as they read `warn_fg_color` today, so `agent/` gains no import of `ui/`.
- `format_coords` right-aligns each number to 8 chars (sign, 4 digits, point, 2 decimals) and
  swaps the padding spaces for U+2007.
- The autosave badge gets a fourth state from one new `App::autosave_failed` flag. The flag is
  set when a write returns `false` *and* `autosave_path` is `Some`, so a process that doesn't
  persist is never "failed", and it is cleared by the next successful write.
Assumes LCV-184 and LCV-164 landed first (`theme.rs` tokens, `statusbar/pill.rs`, `visual_refresh.rs`).

## Touches

- `ui/menubar.rs` (`Export Layers`, `Bed Size…`, `Keyboard Shortcuts…`, `AI Settings…`),
  `menubar/object_snap.rs` (`Object Snap`), window titles in `app/bed_dialog.rs`,
  `ui/shortcuts_dialog.rs`, `app/panels.rs` (`AI Settings`); `agent/settings_ui.rs` (`Restore Default`).
- `src/agent/transport.rs` (timeout, 401), `src/app/cmdline.rs::AGENT_UNAVAILABLE`,
  `src/app/agent_apply.rs::CAPTURE_DISABLED` (`agent_capture.rs` reuses the const instead of its
  duplicate literal) — `Help > AI Settings…`.
- `src/app/agent_turn.rs::turn_label` — `AI: `; `document/commands/composite.rs` doc example.
- `src/ui/statusbar.rs::format_coords` (doctest included), `::format_autosave` (a `failed`
  argument), `::draw_statusbar` (paints the failed badge in `error_fg_color`).
- `ui/theme.rs` — `STATUS_ERROR` #ff6b6b, `TOKENS` row, `v.error_fg_color`; `agent/panel.rs::draw_chat_row`
  `error` rows read `error_fg_color` instead of `Color32::RED`.
- `src/ui/command_line.rs::draw_context_row` — feedback starting with `! ` uses `error_fg_color`.
- `app/mod.rs` — `autosave_failed: bool`; `app/autosave.rs::record_autosave_outcome` sets it.
- Tests: `tests/it/ui/wording_and_status.rs` (new); every existing test keyed on an old string
  or window id (`grep` the old text) follows in the task that changes it.
- `DESIGN.md` §3, §4, §7, §9; `CHANGELOG.md`. ADRs: none (no dependency, boundary or trait change).

## Decisions (self-approved per user goal)

- The dock has no message kinds yet (LCV-165 adds them). Until then a "dock error" is a feedback
  line starting with `! ` (today only `AGENT_UNAVAILABLE`); everything else keeps `status.warning`.
- One name: the dock's own AI lines in `app/cmdline.rs` also say AI (`! AI unavailable: …`,
  `AI prompt is empty.`, `AI is busy — …`, echo `→ AI: "…"`). Panel turn-status texts in
  `app/agent_poll.rs` and the replayed `agent/memory.rs` text keep their wording; they are
  prose, and changing them would ripple into turn-replay tests. They are left for LCV-165.
- A failed badge wins over a pending one (`×` > `●` > `○ autosaved` > `○ no autosave yet`),
  because the spec says it shows until the next write succeeds.
- Checkbox/field labels (`Export this layer`) and the disabled `No recent files` row stay
  sentence case (DESIGN.md §9: field labels, messages).
- Coordinate width is 8 chars per number. A value that doesn't fit (|v| ≥ 10000) grows the
  string, with no clamp.
- AC 3 already holds after LCV-183 (`AGENT_TOGGLE_LABEL = "AI"`, tooltip, panel heading,
  `LABEL_AI`), and AC 8 after LCV-184 (`FILL_SELECTED`, accent only as a stroke). Their tasks
  add a regression test and no code.
- LCV-071/116 have no spec folders (pre-SDD): amendments recorded in DESIGN.md + CHANGELOG.

## Test approach

- AC 1/2: open File, Edit, View, Format, Tools and Help, plus the Object Snap submenu, through
  painted pointer clicks. Every item run passes a Title Case check (each word not in
  {to, as, a, an, the, of, in, on, and, or} starts with a non-lowercase char, except `As`),
  and the exact new rows are present. Then open the Bed Size, Keyboard Shortcuts (F1) and
  AI Settings windows and find their title runs, and the `Restore Default` run.
- AC 4/5/6/9 units: returned strings; integration: `:draw` with no key → the dock line.
- AC 7/9 painted: the text colour of the panel error row, the `! ` dock line and the failed
  badge (galley job sections, as in `visual_refresh.rs::text_colours`) equals #ff6b6b. WCAG
  unit test ≥4.5:1 on `BG_PANEL`. Source scan: no `Color32::RED` under `src/` outside tests.
- AC 9 flow: `autosave_path` set to a directory (the write fails), dirty, debounce passed, one
  frame → `× autosave failed`. Then point it at a writable file, dirty again, one frame →
  `○ autosaved`. With `autosave_path = None`, a flush never shows failed.

## Risks

- LOC cap: `app/mod.rs` is 287 now, about 290 after LCV-164, and about 293 after this spec. Seam
  for the next growth: move `autosave_path`, `dirty_since`, `last_autosave_at` and
  `autosave_failed` into an `AutosaveState` in `app/autosave.rs`. `app/cmdline.rs` (271) and
  `menubar.rs` (287) only get strings changed, with no growth. `statusbar.rs` (~230) gains about 5 lines.
- Window titles are egui ids: tests that call `Id::new("Agent Settings")` or `area_rect` on a
  title must move to the new title in the same commit.
- Branch overlap: `agent-harness` (LCV-186/192) edits `agent/transport.rs`, `app/cmdline.rs`,
  `app/agent_apply.rs`, `app/agent_capture.rs` and `agent/panel.rs`. Merge conflicts there
  are string-level. Keep both sides and re-run the gate after merging.
- Mutation testing: no (strings and one flag; no `src/agent/` logic).
