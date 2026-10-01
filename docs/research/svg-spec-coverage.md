# SVG specification coverage — LaserCAD v2

Research note, 2026-09-29. It maps the features of the official SVG specification to what LaserCAD
exports and imports today. It also records the conformance target the user set, and the Draft
specs (LCV-170..179) that close the gap. This note is orientation, not a contract: the SVG export
contract stays in `AGENTS.md` §SVG export, and each change goes through its spec.

## 1. Sources and target

**Specifications consulted** (links in §7):

- **SVG 2**, W3C Candidate Recommendation of 4 October 2018. This is still the latest published
  version.
  - The Editor's Draft (`svg2-draft`) is maintained as a near-living standard.
  - A maintenance CR snapshot, with "no new feature", is awaiting W3C verification
    (w3c/transitions#834).
  - Chapters used: Conformance, Rendering Model, Document Structure, Styling, Coordinate Systems,
    Paths, Basic Shapes, Text, Embedded Content, Painting, Paint Servers, Implementation Notes,
    Changes from SVG 1.1.
- **SVG 1.1 Second Edition**, W3C Recommendation of 16 August 2011, for the features SVG 2 removed.
- SVG 2 delegates clipping and masking to **CSS Masking** and filters to **Filter Effects**.

**Conformance target** (the user's decision, 2026-09-29):

| Side | SVG 2 class | Meaning for LaserCAD |
|---|---|---|
| Export | *Conforming SVG Generator* | Every file written is a conforming SVG stand-alone file (well-formed XML, SVG namespace, valid content model and attribute syntax). |
| Import | *Conforming SVG Interpreter*, **secure static** processing mode | Parses any conforming SVG file; no script, animation, interaction or external fetch. Every piece of geometry the file would render becomes an exact LaserCAD entity. Features that only affect painting (gradients, filters, opacity, images, markers) are **reported and ignored**. Nothing is silently skipped or misread. |

The user also decided two representation questions:

- **Curves**: ellipses, elliptical arcs and Béziers become **native entities**, not approximations.
- **`<text>` on import**: converted to outlines with system fonts. On export there is still no
  live text; Hershey single-stroke lines remain the laser-correct output.

Legend used below: ✅ supported · ◐ partial · ❌ missing · ⛔ out of scope for the target.

## 2. LaserCAD's SVG profile today

- **Document model**: `Entity::{Line, Circle, Arc}` only (`document/entity.rs`), in millimetres,
  world Y-up.
  - Layers (LCV-156, ADR 0012): name, color, Output flag; each entity belongs to one layer.
- **Export**: `io/svg/export.rs::export_svg`, `encode_entity`, and `io/svg/layers.rs::open_group`.
  The SVG is written by hand.
  - Root: `<svg xmlns width="Wmm" height="Hmm" viewBox="0 0 W H" fill="none">`.
  - One `<g data-layer stroke="#rrggbb" stroke-width="0.1" data-output[ data-current]>` per layer.
  - Entities: `<line>`, `<circle>`, and circular arcs as `<path d="M … A r r 0 large sweep …"/>`.
  - Numbers use `{:.4}` mm, and Y is mirrored via `util::flip_y`.
- **Import**: `io/svg/import.rs::import_svg`, `Walk::collect`, `parse_line`, `parse_circle`,
  `path_data.rs::parse_path_data` and `import/path.rs::path_entities` (LCV-172), plus `io/svg/header.rs::parse_root` (LCV-173: units, viewBox, `preserveAspectRatio`; `viewport.rs`, `matrix.rs`, `length.rs`) and `io/svg/layers.rs::LayerReader`.
  - Parsing uses `roxmltree`.
  - It reads back what the exporter writes and little else.

## 3. Coverage matrix

The owning spec is the Draft that brings the feature to the target (§6).

### 3.1 Document structure (SVG 2 ch. 5)

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| Root `<svg>`, `xmlns` | ✅ | ◐ checks the local name only | Keep. Import must check the SVG namespace. | 171 |
| `version`, `baseProfile` | ✅ not emitted (deprecated in SVG 2) | ignored | OK | — |
| `<g>` nesting | ✅ one per layer | ✅ recursive | OK | — |
| `data-*` | ✅ `data-layer`, `data-output`, `data-current` | ✅ | Plain SVG 2; LaserGRBL and Inkscape ignore them. | — |
| `id`, `class` | not emitted | ignored | `class` feeds the CSS cascade. | 175 |
| `<defs>`, `<symbol>`, `<use>` (`href`, `xlink:href`) | ⛔ not needed | ❌ Geometry inside `<defs>`/`<symbol>` is **imported as if rendered**; `<use>` is ignored. | Never-rendered content is skipped; `<use>` is expanded as a shadow tree. | 171, 178 |
| `<switch>`, conditional attributes | — | ❌ every branch is imported | Evaluate conditions; import the first passing child only. | 178 |
| `<a>` | — | ◐ treated as a group | Treat as a group (same result). | 178 |
| `<title>`, `<desc>`, `<metadata>` | ❌ | ignored | Optional; could carry a generator note. | 170 |
| Nested `<svg>` viewport | — | ✅ done by LCV-173: `x y width height viewBox preserveAspectRatio` establish a viewport; not clipped, reported as `svg (not clipped)` | Establish a new viewport and clip region. | 173 |

### 3.2 Styling (ch. 6) and painting (ch. 13)

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| Presentation attributes (`stroke`, `fill`, `stroke-width`) | ✅ | ✅ done by LCV-175: `stroke`, `fill`, `color` cascade (layer groups keep their own `stroke`) | Full cascade | 175 |
| `style="…"` attribute | — | ✅ done by LCV-175, `!important` included | Parse declarations (Inkscape puts the stroke color here). | 175 |
| `<style>` sheet, `class`/`id`/type selectors | — | ✅ done by LCV-175: type, `*`, `.class`, `#id`, compounds, lists; combinators, pseudo-classes, attribute selectors and at-rules dropped and reported | A CSS subset: selectors, specificity, `!important` (Illustrator uses `.cls-1`). | 175 |
| Inheritance, `inherit`, `currentColor` | — | ✅ done by LCV-175 (`unset`, `initial` too) | Required by the cascade | 175 |
| Color syntax | ✅ lowercase `#rrggbb` | ✅ done by LCV-175 (LCV-156 parser); an invalid color is reported | Any CSS `<color>`: keywords, `#rgb[a]`, `#rrggbb[aa]`, `rgb()`, `hsl()` | 175 (and the LCV-156 amendment request) |
| `stroke` color → layer | ✅ layer color | ✅ done by LCV-175: stray geometry goes to the first layer of its stroke (else fill) color, else a new `#rrggbb` layer | A foreign file's stroke colors map to layers (LightBurn-style). | 175 |
| `display:none`, `visibility:hidden` | — | ✅ done by LCV-175: reported as `hidden (display:none)` / `hidden (visibility)` | Not imported, but reported (hidden Inkscape layers use `display:none`). | 175 |
| `fill`, `fill-rule` | ✅ `fill="none"` | ignored | Recorded in the report only. Area fill (hatching) is a separate product decision. | 171 |
| `stroke-width` | ✅ 0.1 mm hairline | ignored | OK; the laser kerf is physical. | — |
| linecap, linejoin, miterlimit, dasharray, opacity, `paint-order`, `vector-effect`, markers | — | ignored | Reported as paint-only | 171 |

### 3.3 Coordinate systems, transformations and units (ch. 8)

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| `width`/`height` with units | ✅ `mm` | ✅ done by LCV-173: every absolute unit at 96 px = 1 in, unitless = px; `%`/`em`/`ex` on the root fall back to the viewBox size | All CSS absolute units (96 px = 1 in). `%` and `em`/`ex` resolved per spec. | 173 |
| `viewBox` | ✅ `0 0 W H` | ✅ done by LCV-173: offset and user-unit → mm scale; alone, its size is read as px | Min-x/min-y offset and user-unit → mm scale | 173 |
| `preserveAspectRatio` | — (square mapping) | ✅ done by LCV-173: nine aligns, meet/slice, `none` | Align plus meet/slice | 173 |
| `transform` (`matrix`, `translate`, `scale`, `rotate`, `skewX`, `skewY`) on any element | — | ✅ done by LCV-173: CTM composed in f64; invalid or singular reported. ✅ a circle or arc under non-uniform scale or skew imports as the exact ellipse or elliptical arc (LCV-176) | Full current transformation matrix (CTM) in double precision. Under non-uniform scale or skew a circle or arc becomes an ellipse, and a line stays a line. | 173, 176 |
| Transform on nested `<g>` | — | ✅ done by LCV-173 | Accumulated CTM | 173 |

### 3.4 Paths (ch. 9)

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| Grammar: commas, compact form (`M10,20A5…`), exponents, glued flags, implicit repeated commands | ✅ writes a subset | ✅ done by LCV-172 (`io/svg/path_data.rs`) | The full SVG 2 path-data BNF | 172 |
| `M`/`m` | ✅ `M` | ✅ done by LCV-172 | Absolute and relative | 172 |
| `L`/`l`, `H`/`h`, `V`/`v`, `Z`/`z` | — (lines are `<line>`) | ✅ done by LCV-172; zero-length segments draw nothing | Lines. `Z` closes to the subpath start; SVG 2's "segment-completing" close rule applies. | 172 |
| Multiple subpaths in one `d` | — | ✅ done by LCV-172 | Every subpath | 172 |
| `A`/`a`, circular (rx = ry; φ ignored) | ✅ | ✅ done by LCV-172, absolute and relative | OK | 172 |
| `A`, elliptical (rx ≠ ry) | ✅ `A rx ry φ` (LCV-176) | ✅ done by LCV-176: native elliptical arc, radii corrected per §F.6.6 | Native elliptical arc | 176 |
| Arc out-of-range correction (rx = 0 → line, negative r → absolute value, λ > 1 → scale radii by √λ) | — | ✅ done by LCV-172; equal endpoints omit the arc | Implementation Notes, "Correction of out-of-range radii" | 172 |
| `C`/`c`, `S`/`s`, `Q`/`q`, `T`/`t` Béziers | ✅ `C`/`Q` for Bézier entities only (LCV-177) | ✅ done by LCV-177: native cubic and quadratic entities, `S`/`T` reflection, exact under any CTM; a curve whose points all coincide is reported `path curve (degenerate)` | Native Bézier entities; the smooth-command reflection rules apply. | 172, 177 |
| Error handling | — | ✅ done by LCV-172: segments before the error are imported, `path (data error)` is reported, the file opens | Spec: "render up to (but not including) the command containing the first error". LaserCAD imports up to the error and **reports** it. | 172 |
| `pathLength` | — | — | ⛔ affects dashing and text-on-path only | — |

### 3.5 Basic shapes (ch. 10)

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| `<line>` | ✅ | ✅ done by LCV-174: a missing attribute is 0; an unparseable one skips the element, reported `line (invalid attribute)` | Missing attributes default to 0 (spec) | 174 |
| `<circle>` | ✅ | ✅ done by LCV-174: `r` missing or 0 imports nothing, unreported; `r < 0` or unparseable reported `circle (invalid attribute)`; the file still opens | `r = 0` disables rendering (not an error); `r < 0` is an error. | 174 |
| `<rect>` incl. `rx`/`ry` (and `auto`) | — (RECTANGLE writes 4 lines) | ✅ done by LCV-174: its SVG 2 §10.2 equivalent path, so 4 lines plus quarter arcs (circular, or elliptical when rx ≠ ry or under a non-similarity); `auto`, copy and clamp per spec; zero size unreported, negative reported `rect (invalid attribute)` | 4 lines; rounded corners become arcs (circular or elliptical). | 174 |
| `<polyline>`, `<polygon>` | — | ✅ done by LCV-174: one line per distinct pair, a polygon closed unless already closed; `points` read with the path-data number grammar; an odd count or bad token keeps the pairs before it, reported `<el> (data error)` | Lines; a polygon is closed. | 174 |
| `<ellipse>` | ✅ plus `rotate(a cx cy)` when turned (LCV-176) | ✅ done by LCV-176: `auto`/missing radius takes the other, `%` resolved; since LCV-174 a zero or missing radius is skipped silently and a negative or unparseable one is reported `ellipse (invalid attribute)` | Native ellipse | 174, 176 |
| Percentages in geometry | — | ✅ LCV-173/174: `line`, `circle`, `ellipse` and `rect` attributes resolve against the viewport (`rx` on X, `ry` on Y) | Resolved against the viewport | 173/174 |

### 3.6 Text (ch. 11)

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| `<text>`, `<tspan>` (`x`, `y`, `dx`, `dy`, `rotate`), font properties, `text-anchor` | ✅ no live text: TEXT/DTEXT writes Hershey strokes as `<line>` | ❌ | Outlines through system fonts, becoming Bézier entities. Missing font → reported. | 179 |
| `<textPath>`, `inline-size`, `shape-inside`, `writing-mode` | — | ❌ | Decide at /specify of LCV-179 | 179 |
| SVG fonts, `tref`, `altGlyph` | — | — | Removed in SVG 2 | — |

### 3.7 Embedded content, paint servers, rendering effects, interactivity

| Feature | Export | Import | Target / note | Spec |
|---|---|---|---|---|
| `<image>` (PNG, JPEG, SVG, `data:` URL) | — | ❌ ignored silently | Reported and ignored. LaserGRBL imports rasters separately. | 171 |
| `<foreignObject>` | — | — | ⛔ | — |
| `<linearGradient>`, `<radialGradient>`, `<pattern>` | — | ❌ geometry inside `<pattern>` is imported | Never rendered as geometry; reported | 171 |
| `<clipPath>`, `<mask>`, `<marker>` (CSS Masking / markers) | ✅ forbidden by contract | ❌ **their geometry is imported** | Never rendered; reported. Clipping applied to geometry is a possible later demand. | 171 |
| `filter` (Filter Effects) | ✅ forbidden | ignored | Reported | 171 |
| `<script>`, events, SMIL animation, `<a>` navigation | — | ignored | ⛔ The secure static mode forbids them. roxmltree never executes or fetches anything. | — |
| `.svgz` (gzip) | — | ❌ | Optional; decide at /specify of LCV-170 | 170 |

## 4. What must change in what already exists

**Import** (`src/io/svg/import.rs`, `header.rs`). The behaviors below contradict the target. Each
one becomes an acceptance criterion of the owning spec:

1. ~~`parse_path` reads relative `m`/`a` as absolute. A file from another tool opens with wrong
   geometry and no error.~~ **Done by LCV-172**: the full SVG 2 path-data grammar.
2. ~~`parse_path` ignores every token after the 11th, which drops subpaths silently.~~ **Done by
   LCV-172**: every subpath is imported.
3. ~~`Walk::collect` descends into every unknown element, so geometry inside `defs`, `symbol`,
   `clipPath`, `mask`, `marker` and `pattern` is imported as cut geometry.~~ **Done by LCV-171**:
   only `svg`, `g` and `a` are descended into; never-rendered elements import nothing and are
   reported.
4. ~~`transform` is ignored everywhere (reported since LCV-171, applied by LCV-173).~~ **Done by
   LCV-173**: the full CTM is applied to every imported element.
5. ~~Unknown elements and non-arc paths are skipped silently. The target demands an import
   report.~~ **Done by LCV-171**: `ImportedSvg::report`, shown on the command line after Open.
6. ~~`parse_circle` rejects `r = 0`, and a missing `x1`/`cx`… is an error. The spec says "not
   rendered" and "default 0".~~ **Done by LCV-174**: missing positions are 0, a zero size draws
   nothing, and an invalid shape is skipped and reported instead of failing the file
   (`SvgImportError::MalformedAttribute` is gone).
9. **LCV-171 AC 5 amended by LCV-174 AC 10**: `rect`, `polyline` and `polygon` are imported, so
   they are no longer reported as ignored elements.
7. ~~`parse_path` rejects a chord longer than `2r + EPSILON` (1e-9), where the spec scales the radii
   up. *Suspected*, to be confirmed by a test: a semicircular arc may fail to reopen after the
   `{:.4}` rounding in `encode_entity`.~~ **Done by LCV-172**: SVG 2 §F.6.6 applies to every arc.
8. ~~`header.rs::parse_bed` rejects every unit except `mm` and any viewBox not at `0 0`.~~ **Done
   by LCV-173**: `header.rs::parse_root` reads every absolute unit, offset and scaled viewBoxes.

Existing tests that pin these behaviors (for example "silently skips non-arc paths" and "rejects
`px`") are rewritten by the spec that changes the behavior, not deleted in isolation.

**Export** (`src/io/svg/export.rs`, `layers.rs`):

- It is already a conforming generator for today's entities. The audit (LCV-170) must still prove
  that on every path.
- **To verify**: `document/layer.rs::check_fields` accepts any name whose `file_key` is non-empty,
  and `layers.rs::xml_escape` escapes only `& < > "`. A layer name containing a C0 control
  character would then produce a file that is not well-formed XML 1.0, which LaserCAD itself could
  not reopen. A tab or line feed in a name would also not round-trip, because XML normalizes
  attribute values.
- The `AGENTS.md` contract line "Arcs as `<path …A…>`, never béziers" must become:
  - circular arcs stay `A`;
  - elliptical arcs use `A rx ry φ`;
  - `<ellipse>` for full ellipses;
  - Béziers only for Bézier entities.

  This changes the contract, so it needs explicit user approval at /specify of LCV-176/177.

**Kernel and neighbouring demands:**

- LCV-158 (ROTATE/MIRROR/SCALE): once LCV-176 exists, non-uniform scale of a circle or arc should
  produce an ellipse.
- LCV-160 (TRIM/EXTEND with arcs): intersections with ellipses and Béziers are follow-up demands,
  not part of LCV-176/177.
- The agent tools (`src/agent/tools.rs`, `drawing.rs`) will need the new entity kinds for
  `query_entities`.

## 5. Implementation guidance for /design

These are recommendations, not decisions.

- **Grammar parsing**: the `svgtypes` crate (pure Rust, from the resvg project) implements the
  spec grammars. It covers path data (arcs keep rx, ry, φ and the flags), transform lists,
  lengths with units, viewBox, CSS colors and style declarations. Using it replaces hand-written
  parsers, which is where today's defects live. It is a new dependency, so it needs an ADR.
- **Not `usvg` as the importer**: it normalizes every arc and shape into cubic Béziers, which
  loses the exact circular arcs the laser workflow relies on.
  - It is still valuable as a **dev-dependency test oracle**: import a corpus file with both
    tools, flatten both, and compare within a tolerance.
- **Corpus**: small conforming files drawn from the resvg test suite and the W3C SVG 1.1 test
  suite. Include Inkscape and Illustrator samples: mm/px documents, layer groups with
  `transform`, `style=` colors.
- **Text** (LCV-179): system-font discovery and glyph outlines (e.g. `fontdb` + `ttf-parser`,
  shaping via `rustybuzz`). This is a new dependency set with an ADR, and kernel purity must hold.
- **Kernel purity**: all of this stays in `src/io/svg/` and `src/geometry/`, with no `egui`.

## 6. Roadmap — SVG 2 conformance track (Draft specs)

| Spec | Title | Depends on |
|---|---|---|
| LCV-170 | SVG conformance corpus and export audit | LCV-156 |
| LCV-171 | SVG import report and never-rendered elements | LCV-170 |
| LCV-172 | Full SVG path-data grammar | LCV-171 |
| LCV-173 | SVG lengths, units, viewBox and transforms | LCV-172 |
| LCV-174 | SVG basic shapes on import | LCV-173, LCV-176 |
| LCV-175 | SVG styling cascade, CSS colors and color → layer | LCV-171, LCV-156 |
| LCV-176 | Ellipse and elliptical-arc entities | LCV-172 |
| LCV-177 | Cubic and quadratic Bézier entities | LCV-172 |
| LCV-178 | SVG structure reuse: use, defs, symbol, switch | LCV-173, LCV-175 |
| LCV-179 | SVG text import as outlines | LCV-175, LCV-177 |

Scheduling against v0.3 is the user's call (`PLAN.md` §Later).

## 7. Sources

- [SVG 2 — W3C Candidate Recommendation, 4 Oct 2018](https://www.w3.org/TR/SVG2/): the
  [Conformance](https://www.w3.org/TR/SVG2/conform.html),
  [Rendering Model](https://www.w3.org/TR/SVG2/render.html),
  [Document Structure](https://www.w3.org/TR/SVG2/struct.html),
  [Styling](https://www.w3.org/TR/SVG2/styling.html),
  [Coordinate Systems, Transformations and Units](https://www.w3.org/TR/SVG2/coords.html),
  [Paths](https://www.w3.org/TR/SVG2/paths.html),
  [Basic Shapes](https://www.w3.org/TR/SVG2/shapes.html),
  [Text](https://www.w3.org/TR/SVG2/text.html),
  [Embedded Content](https://www.w3.org/TR/SVG2/embedded.html),
  [Painting](https://www.w3.org/TR/SVG2/painting.html),
  [Paint Servers](https://www.w3.org/TR/SVG2/pservers.html),
  [Implementation Notes](https://www.w3.org/TR/SVG2/implnote.html) and
  [Changes from SVG 1.1](https://www.w3.org/TR/SVG2/changes.html) chapters.
- [SVG 1.1 Second Edition — W3C Recommendation, 16 Aug 2011](https://www.w3.org/TR/SVG11/)
- [SVG 2 publication history](https://www.w3.org/standards/history/SVG2/) ·
  [SVG WG repository and Editor's Drafts](https://github.com/w3c/svgwg) ·
  [CR snapshot update request, w3c/transitions#834](https://github.com/w3c/transitions/issues/834) ·
  [SVG WG charter 2024](https://www.w3.org/2024/04/svg-wg.html)
- LaserGRBL context:
  [sizing SVG in LaserGRBL, discussion #864](https://github.com/arkypita/LaserGRBL/discussions/864) ·
  [dimensions and fill for SVG, issue #513](https://github.com/arkypita/LaserGRBL/issues/513) ·
  [raster image import](https://lasergrbl.com/usage/raster-image-import/)
