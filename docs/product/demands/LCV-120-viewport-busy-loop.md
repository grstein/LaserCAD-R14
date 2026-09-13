# LCV-120 — The viewport repaints unconditionally every frame

- **Status**: Draft
- **Phase**: 11
- **Depends on**: none (interacts with LCV-105, LCV-116)
- **Suggested agent**: architect
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`src/app/viewport.rs:46` calls `ctx.request_repaint()` unconditionally, inside
`draw()`, under the comment "Always repaint so cursor-coords and smooth camera
motion stay live." (`src/app/viewport.rs:45`). `git blame` attributes the line
to `0d1d52b`, which is LCV-105. Because `viewport::draw` runs once per frame
from `App::update_ui` (`src/app/mod.rs:228`) with no condition around it, egui
never idles: the reviewer drove the real `App::update_ui` through
`egui::Context::run` on a clean, idle app (no pointer movement, no dirty
document) and measured `FullOutput`'s `repaint_delay` at zero nanoseconds on
consecutive frames.

The consequence is that the application holds one CPU core at 100 percent for
the entire life of every session, whether or not anything is happening on
screen — a battery defect on a laptop, and a fan-noise/heat defect on any
machine.

A second consequence is why this surfaced now rather than earlier: the
blanket repaint silently masks the mechanism LCV-116 AC 9 was written to add.
`autosave::schedule_flush_repaint` (`src/app/autosave.rs:84`, called from
`src/app/mod.rs:237`) is correct and independently tested at its own
granularity — it requests a bounded follow-up repaint only while a write is
pending — but with `viewport::draw`'s unconditional repaint already firing
every frame regardless, `schedule_flush_repaint` is not currently what keeps
autosave alive on an idle app; the blanket repaint is. **Record this so
`schedule_flush_repaint` is not deleted as dead code when this defect is
fixed** — it becomes load-bearing the moment `viewport::draw`'s unconditional
call is narrowed, and removing it at that point would silently reopen the
exact defect LCV-116 AC 9 fixed (a document left dirty and unsaved
indefinitely once the app goes idle).

## Scope

*(deferred to `product-owner` during refinement, once the design question
below is settled)*

## Out of scope

*(deferred to `product-owner` during refinement)*

## Acceptance criteria

*(deferred to `product-owner` during refinement)*

## Expected tests

*(deferred to `product-owner` during refinement)*

## Open questions

- **What actually needs a live repaint?** This is `architect`'s and
  `product-owner`'s design question, not `demand-manager`'s. The comment at
  `src/app/viewport.rs:45` names two real requirements — cursor coordinates
  in the status bar, and smooth camera motion (pan/zoom animation, if any) —
  and both need to keep working. The fix has to identify the actual triggering
  conditions (pointer moved, camera mid-animation, drag in progress, dirty
  document pending autosave, agent busy, etc.) and request a repaint only
  when one holds, rather than every frame unconditionally.
- Does `viewport::draw` need to be told more about frame-level state (e.g.
  "is the pointer over the canvas and did it move since last frame") than it
  currently tracks, and where should that state live given `src/app/input.rs`
  is the single keyboard gate and `viewport.rs` explicitly must not read keys
  or import `eframe`/`rfd`?
- How does the fix compose with `agent_busy` (`src/app/mod.rs:223-225`) and
  `schedule_flush_repaint` (`src/app/mod.rs:237`), which already request
  repaints conditionally and must keep doing so unchanged?

## Notes

- Origin: the LCV-116 review, which measured `repaint_delay == 0` on
  consecutive frames of a clean, idle app driven through
  `egui::Context::run`.
- `git blame src/app/viewport.rs` attributes line 46 to `0d1d52b` (LCV-105).
  LCV-105's own demand did not set out to introduce a busy loop; this is a
  pre-existing line that LCV-116's review happened to measure.
- **Do not delete `autosave::schedule_flush_repaint` as part of this fix.** It
  is correct, tested, and currently redundant only because of the defect this
  demand exists to remove. See §Problem.
- Related: LCV-102 (autosave dirty tracking), LCV-116 AC 9/10 (the repaint
  scheduling this defect currently masks).
