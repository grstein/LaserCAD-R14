# LCV-171 — SVG import report and never-rendered elements

- **Status**: In Progress
- **Depends on**: LCV-170
- **Implementation**: -

## Problem

`io/svg/import.rs::Walk::collect` imports `<line>`, `<circle>` and `<path d="M…A…">` wherever
they sit and descends into every other element. Geometry inside never-rendered elements (`defs`,
`clipPath`, `mask`, `marker`, `pattern`, …) becomes cut geometry, while `<image>`, `<text>`,
unsupported paths, transforms and paint-only features vanish without a word. The conformance
target (`docs/research/svg-spec-coverage.md` §1) forbids silent loss and silent gain.

## Stories

- As an operator, I want Open to tell me what it ignored, so I never cut a drawing that silently
  lost parts.
- As an operator, I want content the file would not display to stay out of my drawing.

## Acceptance criteria

1. IF the root element is not `svg` in namespace `http://www.w3.org/2000/svg` (a missing `xmlns`
   included) THEN THE SYSTEM SHALL refuse the file with `NoSvgRoot`.
2. THE SYSTEM SHALL descend only into `svg`, `g` and `a` elements in the SVG namespace.
3. WHEN import meets `defs`, `symbol`, `clipPath`, `mask`, `marker`, `pattern`,
   `linearGradient`, `radialGradient` or `filter` THE SYSTEM SHALL import nothing inside it, and
   SHALL add the element name to the report iff it has an element child.
4. WHEN import meets `title`, `desc`, `metadata` or an element outside the SVG namespace THE
   SYSTEM SHALL skip it and its subtree without a report entry.
5. WHEN import meets any other SVG element it does not turn into entities (e.g. `image`, `text`,
   `use`, `switch`, `rect`, `style`, `script`, `foreignObject`) THE SYSTEM SHALL skip its subtree
   and add the element name to the report.
6. WHEN a `<path>`'s data is not turned into an entity THE SYSTEM SHALL add `path (unsupported
   data)` to the report instead of skipping it silently.
7. WHEN an element that is imported or descended into carries `transform`, `fill` (other than
   `none`), `clip-path`, `mask`, `filter`, `marker-start|mid|end`, `stroke-dasharray`, `opacity`,
   `display` or `visibility`, as an attribute or a `style` declaration, THE SYSTEM SHALL add that
   property name to the report.
8. THE SYSTEM SHALL expose the report on `ImportedSvg` as entries `(label, count)` in order of
   first occurrence, one entry per label.
9. WHEN Open or Open Recent succeeds with a non-empty report THE SYSTEM SHALL set the command-line
   feedback to `Ignored: <count> <label>, …` in report order; with an empty report it SHALL
   clear the feedback.
10. WHEN a file written by `export_svg` or `export_layer_svg` is opened THE SYSTEM SHALL produce an
    empty report (the root's `fill="none"` is not reported).
11. WHEN the LCV-170 corpus is opened THE SYSTEM SHALL match its `.expected` files, which gain a
    report section, and an Inkscape-style fixture with `<defs>` geometry proves AC 3.

## Out of scope

- Supporting the reported features (LCV-172..179); applying `display`/`visibility` (LCV-175).
- A report dialog; per-element positions in the report.

## Open questions

- None. Decided (self-approved per user goal): the report shows on the command-line feedback
  line (no dialog); a root outside the SVG namespace, `xmlns`-less included, is refused as not
  SVG; `transform` is reported until LCV-173 applies it; tests pinning "silently skips" behaviour
  are rewritten, not deleted.
