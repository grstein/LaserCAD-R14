# LCV-171 — Plan

## Approach

The walk in `import.rs::Walk::collect` gains one decision per child element. The decision is based
on the namespace and the local name, and has five outcomes:

| Outcome | Elements (SVG namespace unless noted) | Report |
|---|---|---|
| import | `line`, `circle`, `path` | properties (AC 7); a `path` not imported → `path (unsupported data)` |
| descend | `svg`, `g`, `a` | properties (AC 7), then children |
| never rendered | `defs symbol clipPath mask marker pattern linearGradient radialGradient filter` | name, iff it has an element child |
| silent | `title desc metadata`; any element outside the SVG namespace | nothing |
| other | every other SVG element | name |

A private `Report` builder's `note(label)` bumps an existing label or appends a new one
(first-occurrence order, AC 8); it becomes `ImportedSvg::report: Vec<(String, usize)>`.

AC 1 is one check in `import_svg`: `root.tag_name().namespace() == Some(SVG_NS)`.

Open and Open Recent share a new `file_actions.rs::open_content` (import, install, feedback
line; AC 9), which also removes today's duplicated body.

## Touches

- `src/io/svg/import.rs` — `SVG_NS`, the namespace check, the `report` field, and the module doc
  ("silently skips" becomes the table above). `Walk` moves out, as described below.
- `src/io/svg/import/walk.rs` (new) — `Walk`, `collect`, and the element classification. It calls
  the parent's private `parse_*` functions through `super::`.
- `src/io/svg/import/report.rs` (new) — `Report`, `REPORTED_PROPERTIES`, `note_properties(node)`
  (attributes and `style` declarations; `fill:none` is exempt), and `style_decls(node)`.
- `src/io/svg/layers.rs::own_stroke` — reuses `report::style_decls` (no behaviour change).
- `src/io/file_actions.rs` — `open_content(app, path, &content)`, used by `action_open` and
  `action_open_path`. It builds `Ignored: <n> <label>, …` or clears the feedback line.
- `src/io/file_actions/tests.rs` — the `action_open` source scan now looks for the
  `open_content(` call. The body assertions move to a runtime test through `action_open_path`.
- `src/io/svg/import/tests.rs` — `non_arc_path_silently_skipped` and
  `unknown_elements_silently_skipped` are rewritten to assert the report (spec: rewritten, not
  deleted). `no_svg_root_returns_error` gains the `xmlns`-less and foreign-namespace roots.
- `tests/it/io_svg/{import_report.rs, corpus.rs, corpus/expected.rs, mod.rs}` and
  `tests/fixtures/svg/inkscape-defs.{svg,expected}`.
- `docs/research/svg-spec-coverage.md` §4, `CHANGELOG.md`. `src/app/`, `src/ui/`: not touched.
- ADRs: none. This is import behaviour inside `io/svg/`; ADR 0012 §4 layer reading is unchanged.

## Export contract changes

None. `export.rs` and `layers.rs::open_group` are unchanged. AC 10 only proves that export output
produces an empty report: the root `fill="none"` is exempt, and `stroke`/`stroke-width` are not
reported properties.

## Decisions (self-approved per user goal)

- A property is reported whenever it is present, whatever its value, except `fill:none`. Neutral
  values such as `opacity:1` and `display:inline` are reported too, following AC 7 literally,
  until LCV-175 applies them.
- `style` declarations are split on `;` and `:` like `own_stroke`, property names are compared
  ASCII case-insensitively, and `!important` is ignored. Full CSS parsing is LCV-175.
- Properties are read only on elements that are imported or descended into, so a `transform` on
  a skipped `<image>` adds `image` to the report but not `transform`.
- A `<path>` without `d` counts as `path (unsupported data)`. A `MalformedPath` inside `M…A…`
  data is still an error; LCV-172 replaces that grammar.
- The label is the local name as written (`clipPath`, `foreignObject`), with no namespace prefix.
- Feedback format: `Ignored: 2 image, 1 transform`, a count and a label per entry, with no
  pluralisation (the label is a name, not a noun).
- `.expected` report lines are `ignored <count> <label…>`, with the count first so labels can
  contain spaces. An `.expected` file with no `ignored` lines expects an empty report, so the
  LCV-170 seeds (LaserCAD exports, and `inkscape-mm` whose `metadata` and `sodipodi:*` are
  silent) need no edit.

## Risks

- LOC cap: `import.rs` is at 276. T1 first moves `Walk` into `import/walk.rs` (~40 lines out),
  and the classification and report code go in the new files, so `import.rs` ends at ~245.
  `walk.rs` should be ~110 and `report.rs` ~90. `state.rs` (276) is not touched.
- Mutation testing: no (no `export.rs`, `agent/` or `History`); each outcome row is asserted.
- Rebase: the `ui` branch (LCV-165) replaces raw `command_feedback` writes with `App::say(Severity,
  …)`, and LCV-168 adds `announce_saved` to `file_actions.rs`. When `svg` is rebased,
  `open_content` should call `say` with `Warning` for a non-empty report and `Info`/clear for an
  empty one, and the conflict is resolved inside `file_actions.rs` only.
- Files without `xmlns` now fail to open (AC 1); every exporter and fixture declares it (grep).
