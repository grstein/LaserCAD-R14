# LCV-138 - Native document title and honest file feedback

- **Status**: Ready
- **Phase**: 11
- **Depends on**: LCV-113, LCV-119, LCV-136
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: -

## Problem

The native window title is set once, at boot, to the constant `APP_TITLE`
(`src/lib.rs`, `ViewportBuilder::with_title(APP_TITLE)`) and never updated
again. It never names the current drawing and never signals unsaved changes,
even though `App::has_unsaved_changes()` and `App::current_file` (both from
LCV-113/LCV-062) already carry everything needed to compute it.

Two smaller, related gaps in file identity feedback:

- A document recovered from the crash-safety autosave at boot
  (`App::new()`, `src/app/init.rs`) is indistinguishable from a normal
  session once running: nothing on `App` records that it happened, so
  nothing in the UI can say so. An operator who sees their drawing reappear
  after a crash has no way to be told "this came back from a safety copy, it
  is not saved to any file" — the exact distinction the codebase base
  already protects at the model level (`saved_revision` vs. `dirty_since`,
  `src/app/file_ops.rs`'s module doc) but never surfaces.
- File > Open Recent (`src/ui/menubar.rs::recent_submenu`) shows only each
  entry's basename (`Path::file_name()`), with no tooltip. Two files with the
  same name in different directories — already stored as distinct, full-path
  entries in `Settings::recent_files` (`push_recent_file` dedupes on the full
  path) — render as two identical, unlabelled menu rows.

## Scope

- A native title that derives from `current_file`, `has_unsaved_changes()`
  and `APP_TITLE`, updated live.
- A recovery indicator, distinct from the existing autosave badge
  (`src/ui/statusbar.rs::format_autosave`), that explains the
  autosave-vs-save distinction on hover.
- Distinguishable Open Recent entries when basenames collide, plus full-path
  tooltips.
- A regression test locking in the already-correct (but currently untested)
  behavior of a failed Open/Open Recent: the drawing, history and filename
  are untouched and `app.error_message` is set.

## Out of scope

Changing saved-revision semantics, content-based dirty detection (e.g.
diffing entities instead of using `saved_revision`), a save dashboard, a new
file lifecycle, or persisting title state across restarts (the title is
recomputed every frame from existing fields; nothing new is written to
`Settings` or the autosave envelope).

## Acceptance criteria

1. A new pure function (e.g. `App::display_title(&self) -> String`) returns
   `"Untitled.svg"` when `current_file` is `None`, or the file name only
   (via `Path::file_name()`, never the full path) when it is `Some`;
   prefixed with `"*"` iff `has_unsaved_changes()` is true; suffixed with
   `" - "` followed by `APP_TITLE` — e.g. `"*drawing.svg - LaserCAD v2"`,
   `"Untitled.svg - LaserCAD v2"`.
2. Every frame, `App::update_ui` computes `display_title()` and compares it
   against a new `last_title: Option<String>` cached on `App`; only on a
   difference does it call
   `ctx.send_viewport_cmd(egui::ViewportCommand::Title(...))` and update the
   cache. This path adds no `ctx.request_repaint*` call, so
   `src/app/viewport.rs::every_repaint_request_in_src_is_conditional`'s
   three-site count is unaffected. A title change following a dialog
   confirmation or an agent turn's completion is visible on the very next
   frame, with no extra plumbing beyond this per-frame comparison.
3. A successful Save, Open, Open Recent or Save As clears the `*` (already
   true through `mark_saved`); a cancelled native dialog or a failed
   Save/Open/Open-Recent (I/O or parse error) leaves `current_file` and
   `has_unsaved_changes()` exactly as they were, so `display_title()` is
   unchanged. An autosave flush never clears the `*` (already true and
   already regression-tested by
   `src/app/file_ops.rs::autosave_flush_does_not_clear_unsaved_changes`;
   this demand must not weaken that).
4. `App` gains a `recovered_from_autosave: bool` field: `false` on
   `App::default()`, set to `true` only inside `App::new()`'s recovery
   branch (alongside `app.document = recovered`), and cleared back to
   `false` by `App::mark_saved()` — the one call every successful Save,
   Save As, Open, Open Recent and New already makes
   (`src/io/file_actions.rs`) — and nowhere else. The label's whole point is
   honest file feedback, so it must not keep naming a state that has
   stopped being true: once the recovered drawing is actually written to a
   file, or the operator replaces it entirely via New/Open(/Recent), the
   document `mark_saved()` just certified is no longer "recovered and
   unsaved". An autosave flush (`mark_clean()` alone, with no
   `mark_saved()`) leaves the flag set, matching AC 3's guarantee that
   autosave never clears the unsaved marker either — a crash-safety write
   is not the operator saving. While true, the status bar shows one label
   distinct from the existing autosave badge — its text names recovery
   explicitly (e.g. "Recovered (not saved)") — carrying
   `.on_hover_text(...)` that explains the drawing was restored from a
   crash-safety copy and the file on disk, if any, is untouched until the
   operator saves. Rendering the label itself still never calls
   `mark_saved`/`mark_clean` and never touches `dirty_since` or
   `last_autosave_at` — it only reads a flag that `App::mark_saved()`, not
   the status bar, is responsible for clearing.
5. In File > Open Recent, an entry whose basename collides with another
   entry in the same list shows enough of its parent path to disambiguate
   the two (e.g. `"parent/name.svg"`); a non-colliding entry keeps showing
   its bare basename. Every entry carries `.on_hover_text()` with its full
   path. Selecting an entry whose file is missing or fails to parse leaves
   the current drawing, history, `current_file` and the recent-files order
   unchanged, and shows `app.error_message`; the failing entry is not
   removed from the recent-files list.
6. Every persistence-touching test added by this demand points
   `settings_path`/`autosave_path` at a temporary directory it owns (ADR
   0006); none reaches the operator's real config or data directory, and
   `App::new()` itself is never called from a test (ADR 0002 §A2) — its one
   new line (setting `recovered_from_autosave`) is covered by a bounded
   source scan in `src/app/init.rs`'s existing test style, not a behavioral
   test.

## Expected tests

- AC 1: unit tests over `display_title()` for the blank/untitled, dirty,
  clean-with-path and dirty-with-path cases.
- AC 2: a real-`App` frame test inspecting `out.viewport_output[&ROOT
  ].commands` for `ViewportCommand::Title(...)` present on a frame that
  changes title-relevant state and absent on the next, otherwise-idle frame;
  a source or behavioral check that no `ctx.request_repaint*` call was added.
- AC 3: Save/Open/Open-Recent/Save-As success and failure/cancellation
  cases, each asserting the title before and after; the existing
  `autosave_flush_does_not_clear_unsaved_changes` re-run unmodified.
- AC 4: a source scan of `App::new`'s recovery branch (mirroring
  `src/app/init.rs`'s existing `boot_seeds_the_bed_only_when_no_autosave_is_recovered`
  style, since `App::new()` cannot be driven behaviorally) proving
  `recovered_from_autosave = true` is set only there; a unit test setting
  `recovered_from_autosave = true` by hand and calling `App::mark_saved()`,
  asserting the flag is now `false`; a companion test from the same
  starting state calling `App::mark_clean()` alone (the autosave-flush
  path, no `mark_saved()`), asserting the flag is still `true`; an
  egui-harness paint test (`tests/harness/paint.rs`) driving
  `App::default()` with `recovered_from_autosave` set by hand, asserting
  the distinct label and its hover text are painted, and asserting
  `dirty_since`/`last_autosave_at` are untouched by rendering it.
- AC 5: two recent-file entries sharing a basename in different directories,
  asserting the disambiguated labels via `tests/harness/paint.rs`'s painted
  text; a missing-file and a malformed-SVG fixture selected from Open
  Recent, asserting drawing/history/`current_file`/recent-list order are
  unchanged and `error_message` is set.
- AC 6: every new test's `App` construction checked for test-owned
  `settings_path`/`autosave_path`, and the `App::new` scan bounded to its
  implementation section per the existing pattern in `src/app/init.rs`.

## Open questions

None at product level. `architect` records the narrow seam before this
demand's code lands in `src/app/mod.rs` (292 implementation LOC today) or
`src/app/file_ops.rs` (281 today) — both close enough to the 300-LOC cap
that a new field and a new title-computation function likely need their own
file (`src/app/document_title.rs` is the natural home, per this file's
Notes) rather than growing either.

## Notes

Prefer `src/app/document_title.rs` for `display_title()`, `last_title` and
the frame-level title-update call; related files are `src/app/init.rs` (the
new `recovered_from_autosave` field and its one recovery-branch write),
`src/ui/statusbar.rs` (the recovery label) and `src/ui/menubar.rs`
(`recent_submenu`, for the disambiguation and tooltips). Retain `APP_TITLE`
(`src/lib.rs`) as the suffix rather than duplicating the string.

`egui::ViewportCommand::Title(String)` exists at the pinned egui 0.29.1
(`egui::viewport::ViewportCommand::Title`); no version bump needed.
