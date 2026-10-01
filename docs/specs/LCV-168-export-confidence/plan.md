# LCV-168 — Plan

## Approach

Feedback is added after the existing writes, which stay as they are. The SVG export contract is
untouched: `export_svg`, `export_layer_svg` and `layer_exports` are not edited, and no file byte
changes (AC 4).
- **Out-of-bed count (kernel).** A new `src/document/bed.rs` holds
  `outside_bed(entity, bed_mm) -> bool`: the exact `Entity::bbox` leaves `[0, w] × [0, h]` by
  more than `EPSILON` on any side. It also holds `Document::outside_bed_count(&self, on: impl
  Fn(LayerId) -> bool) -> usize`, which counts through the public `entity_layer`. `state.rs` is
  at 277 implementation lines, so nothing is added there.
- **Save.** `action_save` and `action_save_as` share one private helper,
  `file_actions.rs::write_mother(app, &path) -> bool`: export, `fs::write`, and on failure the
  same `error_message` as today. After their existing success bookkeeping, both call
  `announce_saved(app, &path)`. It builds `Saved <name> (<w> × <h> mm)` with `{}` on the `f64`
  sizes, so they print as `400` and `297.5`. With n ≥ 1 it appends ` — n entities outside the
  bed` and uses `Severity::Warning`; otherwise it uses `Severity::Info`, through LCV-165's
  `App::say`.
- **Export Layers.** `action_export_layers` counts only entities whose layer is in the plan
  (Output on, has entities), and does so after every write has succeeded. With n ≥ 1 its
  existing `Exported layers: …` line gets the same ` — n entities outside the bed` suffix as a
  Warning.
- **Failures and autosave.** Every failure and cancel path returns before `announce_saved`, as
  today (AC 5). Autosave goes through `app/autosave.rs`, never through `file_actions`, so AC 6
  holds by construction and is pinned by a test.

## Touches

- `src/document/bed.rs` (new, kernel-pure), plus `document/mod.rs` (`mod bed;` and the
  `outside_bed` re-export).
- `src/io/file_actions.rs::action_save`, `::action_save_as`, plus new `write_mother` and
  `announce_saved` (205 → about 235 lines), and `outside_bed_phrase(n)`, which gives
  `1 entity` / `n entities`, `pub(crate)` for export_layers.
- `src/io/export_layers.rs::action_export_layers`: add the suffix and the severity.
- Tests: `src/document/bed.rs` unit tests, `src/io/file_actions/tests.rs` (Save As success via
  `write_mother` + `announce_saved`, because the dialog is disarmed under ADR 0005),
  `tests/it/app/document_title_and_file_feedback.rs` (Save flows),
  `tests/it/io_svg/export_layers.rs` (export flows), `tests/it/app/autosave_dirty.rs` (AC 6).
- `DESIGN.md` §7 (dock messages) and §9 (message pattern); `CHANGELOG.md`.
- ADRs: none. No export contract change, no module boundary change.

## Decisions (self-approved per user goal)

- The Export Layers warning shares one line with the file list, using the same ` — ` separator
  as Save (`Exported layers: a-Cut.svg — 2 entities outside the bed`). The dock shows one line.
- The Save count includes every entity, including those on Output-off layers, because the mother
  file holds them all. The export count is limited to the layers it wrote.
- The file name is `path.file_name()` as written (after `.svg` is enforced), not the full path.
- Touching the edge within `EPSILON` is inside the bed. Arc bboxes already include their
  cardinal extremes, so a bulge past the edge counts.
- LCV-190 (v0.6, `document/check.rs` off-bed finding) should reuse `document::outside_bed`,
  so the two never disagree. This goes in the LCV-190 plan when it is implemented.

## Test approach

Kernel units cover the predicate (inside, on the edge, past it by 2·EPSILON on each side, an arc
bulge) and the layer filter. Flow tests assert `command_feedback` and its severity after Save
into a tempdir (Info with no out-of-bed entities, Warning with n = 1 and n = 2). A byte test
compares the written mother and layer files against `export_svg` / `layer_exports` for a
drawing with out-of-bed geometry (AC 4). The failure tests use a current file inside a missing
folder: `command_feedback` stays at a sentinel and `error_message` is set. The cancel test runs
Save As with the disarmed dialog. The autosave test flushes one autosave with an out-of-bed
entity, and the sentinel stays.

## Risks

- Order: LCV-165 must land first (`App::say`, `Severity`). If it has not landed, stop; do not
  invent a second severity path.
- LOC cap: `file_actions.rs` 205 → about 235 and `export_layers.rs` 61 → about 70. There is no
  seam near 270. `state.rs` (277) must not grow, which is why the count lives in `bed.rs`.
- Existing Save tests may assert that `command_feedback` stays empty on success. Update them in
  the task that adds the message.
- Mutation testing: no (`export.rs` untouched; no `src/agent/`, no `History`).
