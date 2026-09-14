# LCV-141 - Agent panel stays within the right third

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-125, LCV-129, LCV-132
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The resizable agent side panel has a 300pt default but no proportional maximum.
Long content or remembered width can consume the drawing surface; composer
controls compete for unbounded space.

## Scope

A hard right-third width limit, bounded transcript/composer allocation and
explicit completion of the existing live-edit settings workflow.

## Out of scope

Full-screen chat, new width preferences, docking infrastructure, new agent
capabilities or an Apply/Cancel settings transaction.

## Acceptance criteria

1. The actual outer panel rectangle never exceeds the full application content width divided by three. Use application width, not remaining canvas width.
2. Enforce the ceiling on first frame, remembered oversize, drag, long content and window shrink. Default width is `min(300pt, application_width/3)`; minimum width cannot override that ceiling.
3. At 800pt application width the ceiling is approximately 266.67pt, not 300pt. An optional 240pt compact width is valid only while below the ceiling.
4. Children are bounded/wrapped so intrinsic content cannot force the panel wider. Reserve Send/Cancel and composer height before allocating transcript scrolling.
5. Input, Send, busy Cancel and close controls remain contained and non-overlapping with long/unbroken text and long transcripts. Closing the panel restores canvas space.
6. Agent Settings explains that changes apply immediately and persist on close. Done uses the same close/persist path as X. Existing key masking and plaintext-storage warning remain.
7. Settings content uses bounded scrolling when needed, including a future multiline prompt editor; completion controls remain reachable at 800x600.
8. Do not change submission, cancellation, undo, default closed state, settings persistence schema or repaint policy under this demand.

## Expected tests

- AC 1-3: real panel bounds at 800/1024/1280pt, oversize restored state, resize drag and window shrink; assert ratio numerically, not button visibility alone.
- AC 4-5: full-precision text/widget containment, busy/nonbusy and actual Send/Cancel/close clicks.
- AC 6-7: Done/X persistence through test-owned paths, masking/warning content and bounded settings scrolling.
- AC 8: existing LCV-125/129 and idle repaint regressions.

## Open questions

None at product level. The one-third ceiling is a hard requirement, not an
approximate default.

## Notes

Width belongs in `src/app/panels.rs`; child layout in
`src/agent/panel.rs`; settings content in `src/agent/settings_ui.rs`.
