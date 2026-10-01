# LCV-178 — Tasks

- [x] T1 [AC11] Test: `conditions.rs` unit tests — `requiredExtensions=""` passes, `"x"` fails;
      `systemLanguage` `en`, `en-US`, `fr, en-GB`, ` EN ` pass; `fr`, `english`, `""` fail;
      `requiredFeatures="x"` ignored (files: src/io/svg/import/conditions.rs, src/io/svg/import.rs)
- [ ] T2 [AC11] `conditions::passes` (files: src/io/svg/import/conditions.rs)
- [ ] T3 [AC10] [AC11] Test: a `<line systemLanguage="fr">` is not imported and notes
      `line (conditions)`; a `<g requiredExtensions="x">` hides its subtree; a `<switch>` with a
      failing `foreignObject`, a passing `<g>` and a trailing `<line>` imports only the `<g>` and
      notes `switch (branch skipped)` ×2; a `<switch transform>` moves its child
      (files: tests/it/io_svg/switch.rs, tests/it/io_svg/mod.rs)
- [ ] T4 [AC10] [AC11] Walk: conditions gate on every element, `Kind::Switch` arm, `switch`
      out of `Other` (files: src/io/svg/import/walk.rs)
- [ ] T5 [AC2] [AC7] [AC8] Test: `reuse.rs` unit tests — `Index` first-id-wins; `href` beats
      `xlink:href`; `xlink:href` alone resolves; missing href, `#nope`, `other.svg#a`,
      `http://x/#a`, `#` are unresolved; self, ancestor and stacked-use cycles are detected
      (files: src/io/svg/import/reuse.rs, src/io/svg/import.rs)
- [ ] T6 [AC2] [AC7] [AC8] `Index::build`, `target`, `is_cycle` (files: src/io/svg/import/reuse.rs)
- [ ] T7 [AC1] [AC3] Test: `instance_ctx` — `transform="rotate(90)" x="10"` composes
      transform then translate; a `symbol viewBox="0 0 10 10"` into `width="20" height="40"`
      with default `xMidYMid meet`, with `none`, with no width (symbol's, then 100%)
      (files: src/io/svg/import/reuse.rs)
- [ ] T8 [AC1] [AC3] `instance_ctx` (files: src/io/svg/import/reuse.rs)
- [ ] T9 [AC1]–[AC8] Test: end-to-end through `import_svg` — `<use>` of a `<line>` in `<defs>`
      at `x/y`; of a `<g>`; of a `symbol`; `href` over `xlink:href`; stroke inherited from the
      `<use>` but a `.cls` rule and own `stroke` on the target win; the instance lands on the
      `<use>`'s `<g data-layer>` and, stray, on its color layer; `use`→`use`→line recursion;
      a↔b cycle notes `use (cycle)`; `#nope` notes `use (unresolved)`
      (files: tests/it/io_svg/reuse.rs, tests/it/io_svg/mod.rs)
- [ ] T10 [AC1]–[AC8] Walk `use` arm, `uses` stack, symbol-children walk
      (files: src/io/svg/import/walk.rs)
- [ ] T11 [AC9] Test: 33 nested uses fail with `LimitExceeded` naming `depth 32`; 32 pass; a
      six-level ×10 fan-out (10⁶ lines) fails naming `100000` within the test's normal run time;
      100 000 plain lines without `use` still import (files: tests/it/io_svg/reuse.rs)
- [ ] T12 [AC9] `SvgImportError::LimitExceeded`, depth check, `instanced` budget
      (files: src/io/svg/import.rs, src/io/svg/import/walk.rs)
- [ ] T13 [AC9] Test: `action_open_path` on a depth-33 file leaves the document, title and
      entities unchanged and surfaces the error (files: tests/it/app/document_title_and_file_feedback.rs)
- [ ] T14 [AC12] Test: two `<use>` of one `<line>` give two entities with distinct ids;
      committing `MoveEntities` on one leaves the other's geometry bit-identical
      (files: tests/it/io_svg/reuse.rs)
- [ ] T15 [AC1] [AC3] [AC10] Corpus pairs `inkscape-clones` and `icon-symbols` with hand-written
      `.expected` (files: tests/fixtures/svg/)
- [ ] T16 Coverage §3 `<use>`/`<switch>` rows → ✅; CHANGELOG line (files:
      docs/research/svg-spec-coverage.md, CHANGELOG.md)
