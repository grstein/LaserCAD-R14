# LCV-178 — Plan

## Approach

All kernel-pure under `src/io/svg/import/`, on top of LCV-173's `Ctx`, LCV-175's `Style`/`Slot`
and LCV-174's shapes.

1. **Conditions** (`import/conditions.rs`, new, ~40): `passes(node) -> bool` — a present,
   non-empty (after trim) `requiredExtensions` fails; `systemLanguage` passes iff one of its
   comma-separated tags is `en` or starts with `en-` (ASCII case-insensitive, trimmed); an empty
   `systemLanguage` fails; `requiredFeatures` is ignored. The walk checks it on **every** element
   before its kind is acted on; a failure skips the subtree and notes `<name> (conditions)`
   (AC 11).
2. **Switch** (walk arm): `switch` moves from `Other` to its own kind. Among its direct
   SVG-namespace element children, the first that `passes` is collected as if a `<g>` child;
   every other element child notes `switch (branch skipped)` (AC 10). Non-SVG children stay
   silent (LCV-171 AC 4). The `<switch>` itself composes its transform and style like a `<g>`.
3. **Reuse** (`import/reuse.rs`, new, ~110):
   - `Index::build(root)`: one pass over `root.descendants()`, SVG-namespace elements with `id`,
     first occurrence wins.
   - `target(use, &Index) -> Result<Node, Unresolved>`: `href` (no namespace) first, else
     `xlink:href` (`http://www.w3.org/1999/xlink`) (AC 2); the trimmed value must be `#` + a
     non-empty id present in the index, otherwise `use (unresolved)` — nothing is ever fetched
     (AC 8).
   - `is_cycle(target, use, stack)`: the target is an ancestor-or-self (roxmltree `ancestors()`
     includes self) of the `<use>` or of any `<use>` on the expansion stack → `use (cycle)` (AC 7).
     This also catches `<g id="a"><use href="#a"/></g>` and mutual a↔b references.
   - `instance_ctx(use, target, &Ctx) -> Ctx`: `ctx · transform(use) · translate(x, y)` (AC 1);
     for a `symbol` (or a nested `svg`) target, additionally LCV-173's `view_box_map` of the
     target's `viewBox`/`preserveAspectRatio` into `width`/`height` of the `<use>`, else the
     target's, else 100% of the current viewport (AC 3). `x`/`y`/`width`/`height` via `attr_len`.
4. **Walk** (`use` arm): resolve → cycle check → push the `<use>` on `Walk::uses: Vec<Node>`;
   if `uses.len() > 32` return `SvgImportError::LimitExceeded("use nesting depth 32")`. The
   instance style is `Style::child(use)` and the target is collected with it as parent
   (inheritance from the `<use>`, own `style`/attributes/sheet rules — LCV-175 selectors are
   compound-only, so "as in place" needs no tree context, AC 4). The slot is the one the `<use>`
   would get (`Layer(id)` of its enclosing `<g data-layer>`, else the color rule) (AC 5). A
   `symbol` target walks its children (the symbol itself stays `NeverRendered` when met in
   place); any other target goes through the normal `collect`, so a nested `<use>` recurses
   (AC 6). Pop after.
5. **Entity budget**: `Walk::instanced` counts entities pushed while `uses` is non-empty; above
   100 000 → `LimitExceeded("100000 instanced entities")` (AC 9). Ordinary geometry is not
   capped. `import_svg` returns the error, so `action_open_path` leaves the document as is.
6. Instances are plain new entities with their own ids (AC 12); nothing links them.

## Touches

- `src/io/svg/import/{reuse.rs, conditions.rs}` (new, private `mod` in `import.rs`).
- `src/io/svg/import/walk.rs`: `Kind::{Use, Switch}`, conditions gate, `uses`, `instanced`.
- `src/io/svg/import.rs`: `SvgImportError::LimitExceeded(&'static str)` (`"import refused:
  exceeds the {0} limit"`), builds the `Index`.
- Tests: `tests/it/io_svg/{reuse.rs, switch.rs, mod.rs}`, an open-failure case in
  `tests/it/app/document_title_and_file_feedback.rs`; corpus pairs `inkscape-clones` (`use` of a
  group + `xlink:href`) and `icon-symbols` (`symbol` + `viewBox`, a `switch`).
- Docs: `docs/research/svg-spec-coverage.md` §3 rows `<use>`/`<switch>` → ✅, `CHANGELOG.md`.

## LOC seams

`walk.rs` is ~200 after LCV-173/174/175; the two arms must stay thin (≤40 lines added) — all
logic in `reuse.rs`/`conditions.rs`. If `walk.rs` passes 270, move the `use` arm into
`reuse.rs::expand(&mut Walk, …)`. `import.rs` (261) gains ≤8 lines; at 270+ move
`SvgImportError` to `import/error.rs` (re-exported).

## Export contract changes

None. `export.rs` and `layers.rs` are untouched; nothing exported uses `<use>`/`<switch>`.

## ADRs / risk

No ADR (no boundary or dependency change). Mutation testing: no (`src/io/svg/import`, not
`export.rs`). Risk: a hostile file (billion-laughs `use` fan-out) — the depth + instance caps
bound it; a test proves a 10×10×10×10×10×10 fan-out refuses fast.

## Decisions (self-approved per user goal)

- Inside a `<switch>`, every non-chosen child is `switch (branch skipped)`, not `(conditions)`.
- Duplicate ids: first in document order wins (browser behaviour).
- `width`/`height` on a `<use>` of a non-`symbol`/`svg` target are ignored (SVG 2 §5.6).
