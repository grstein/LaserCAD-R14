# LCV-184 — Plan

## Approach

All chrome colours become named `pub(crate) const`s in `ui/theme.rs`, one per DESIGN.md §3 token
(`BG_CANVAS` keeps its public alias `CANVAS_BG`), listed once in a `TOKENS: [(&str, Color32)]`
table that a unit test checks against the §3 rows. `apply_theme` stops relying on egui defaults:
it sets shadows to `Shadow::NONE`, `window_stroke` 1 pt `border`, rounding 3 pt on every widget
state and 4 pt on windows and menus, inactive/hovered/active fills, `selection.bg_fill =
FILL_SELECTED`, `selection.stroke = 1 pt ACCENT` (the selected foreground, never a fill),
`warn_fg_color = STATUS_WARNING`, widget `fg_stroke` colours `TEXT_PRIMARY`, and the
noninteractive `bg_stroke` (egui's separator line) 1 pt `border`. Most of AC 5 and all of AC 8
fall out of that: `ui.separator()` and `icons.rs::square_button` already read those visuals.
Three small widgets change: a `mode_pill` in the status bar (painted rect + text, click sense),
a coloured prompt (`prompt_spans` → `LayoutJob`) in the dock, and a 1 pt frame around the editor
row whose stroke is `ACCENT` while the editor holds focus.

## Touches

- `src/ui/theme.rs` — token constants, `TOKENS`, `apply_theme` rewrite; unit tests.
- `src/ui/statusbar/pill.rs` (new) — `mode_pill(ui, on, label) -> Response`: body-font text,
  `PILL_PADDING` named constant, on = `FILL_SELECTED` fill + `ACCENT`
  text, off = no fill + 1 pt `BORDER` outline + `TEXT_MUTED` text, hovered off = `FILL_HOVER`.
- `src/ui/statusbar.rs::draw_statusbar` — `mode_pill` replaces `selectable_label`; coordinates as
  `RichText::monospace()`. `src/ui/statusbar/tests.rs` source-scan needles follow the new call.
- `src/ui/command_line/prompt.rs` (new) — pure `prompt_spans(&str) -> Vec<(PromptPart, &str)>`
  (`Verb`, `Request`, `Option`) with unit tests.
- `src/ui/command_line.rs` — prompt painted from a `LayoutJob` (still `Label::truncate`);
  editor row wrapped in `egui::Frame` with 1 pt stroke, `TextEdit::frame(false)`.
- `src/ui/icons.rs::square_button` — only if the AC 8 test shows a gap (expected: none).
- Tests: `tests/it/ui/visual_refresh.rs` (new) + `tests/it/ui/mod.rs`.
- `DESIGN.md` §3 (token rows, in the theme task), §1, §5, §7; `CHANGELOG.md`.
- ADRs: none — no dependency, no module boundary, no trait or kernel change.

## Decisions (self-approved per user goal)

- New §3 tokens: `border` #404040 (gray 64), `fill.widget` #3c3c3c, `fill.hover` #464646,
  `fill.active` #373737 — egui's dark values, now named, so the refresh is flat, not a repaint.
- `selection.stroke` = `ACCENT`: a selected rail icon or pill text is accent, as AC 3 allows.
- Prompt verb = the leading run of ≥2 ASCII capitals ending at a space or `:` (today's prompts
  use one space, e.g. `LINE Specify first point:`, and `TRIM: Click…`); every `[…]` and `<…>`
  span after it is an option; `Command:` and any prompt without a verb is all `text.primary`.
- Editor focus colour is read from `ui.memory(|m| m.has_focus(editor_id()))` before the frame
  is painted (one-frame lag is invisible; tests run one extra frame).
- AC 1's scope is `src/ui/`: `agent/panel.rs::TOOL_COLOR` and `Color32::RED` stay (agent/, and
  `status.error` is LCV-167); `app/viewport/paint.rs` gray 64 is canvas (LCV-164).
  `layers_dialog.rs` builds its swatch from layer data, not a literal — allowed.
- Rail inactive buttons stay flat (no fill); hover/pressed/selected use the AC 2 fills.

## Test approach

- AC 1: unit test `TOKENS` ↔ `include_str!("../../DESIGN.md")` §3 rows; integration source scan
  (tests/harness/scan.rs, `concat!` needles, positive control in `theme.rs`) for `Color32`
  literal constructors (digit first argument) and named colour constants in `src/ui/`.
- AC 2 / AC 3: `ctx.style().visuals` read back after `apply_theme`; painted: open File menu →
  its frame `RectShape` has 1 pt `BORDER` stroke, rounding 4, no shape with `blur_width > 0`;
  a full frame (SNAP on, LINE active, editor focused) paints no `Shape::Rect` filled `ACCENT`.
- AC 4–8: painted `FullOutput.shapes` — rect/text colours found by position under the
  run's text (`SNAP`, `X:`, prompt verb) or the editor rect (`ctx.read_response`); text colour
  and font family from the `TextShape` galley's job sections.
- AC 9: three sizes × modes all on/all off: menubar titles inside the window, status segments
  contained (≤56 pt), dock ≤64 pt, rail centres inside the rail with no scroll.

## Risks

- LOC cap: `statusbar.rs` 224 → pill in its own file; `command_line.rs` 170 → parser in its own
  file; `theme.rs` ~35 → ~120. No file nears 270.
- `src/ui/statusbar/tests.rs` scans `draw_statusbar` source for `.selectable_label(…)` and the
  segment order; T6 rewrites those needles in the same commit.
- `TextEdit::frame(false)` moves the editor rect by the frame margin; LCV-139 context-row tests
  (`tests/it/cmdline/context_row.rs`) must stay green — T12 runs them.
- Pill padding can widen the status bar at 800 pt; AC 9 test pins it, `PILL_PADDING` is the knob.
- Mutation testing: no.
