# LCV-172 — Plan

## Approach

Split path import into three stages, and give each one its own kernel file under `src/io/svg/`:

1. **Lexer** (`path_data/lexer.rs`): reads a command letter, a number or a flag from a `&str`
   cursor. It skips commas and whitespace, and follows the SVG 2 number grammar: sign, digits,
   `.`, exponent. Each span is parsed with `str::parse::<f64>`, and a non-finite result is an
   error. A flag is exactly one `0` or `1` character, so glued flags work.
2. **Resolver** (`path_data.rs::parse_path_data(d) -> PathData`): the command loop, in SVG
   coordinates. It tracks the current point and the subpath start, resolves relative commands,
   repeats implicitly (extra `M`/`m` pairs become `L`/`l`), and emits absolute segments:
   `Line(a, b)`, `Arc { from, to, rx, ry, large, sweep }` (φ is dropped here, because only
   circular arcs are imported), or `Skipped { label, to }` for `C S Q T`. A segment is emitted
   only once all its arguments have parsed, so at a syntax error `PathData` keeps every earlier
   segment and sets `error: true` (AC 8).
3. **Convert** (`import/path.rs::path_entities`): turns segments into world entities with
   `flip_y`, drops zero-length lines, and applies §F.6.6 and the circular test to arcs. The arc
   centre rebuild moves here unchanged from `import.rs::parse_path`, which keeps AC 9 exact. It
   returns the entities and the report labels, and `walk.rs` pushes them, several per `<path>`.

## Touches

- `src/io/svg/path_data.rs` (new, ~170) and `src/io/svg/path_data/lexer.rs` (new, ~90). Both are
  kernel-pure, and `mod.rs` declares them private.
- `src/io/svg/import/path.rs` (new, ~110): `path_entities(&PathData, bed_h) -> (Vec<Entity>,
  Vec<&'static str>)`, plus the arc rebuild moved from `import.rs`.
- `src/io/svg/import.rs`: `parse_path` and `tok_f64` are removed, and the `MalformedPath` variant
  is removed (nothing produces it any more). The module doc changes from "`M…A…` only" to the full
  grammar.
- `src/io/svg/import/walk.rs` (from LCV-171): the `path` arm calls `parse_path_data` +
  `path_entities`, pushes every entity on the current layer, and notes each label. An error notes
  `path (data error)`. A `<path>` with no `d` still reports `path (unsupported data)` (LCV-171).
- `src/io/svg/import/tests.rs`: rewrites `path_with_non_numeric_a_command_returns_malformed_path`
  (now: nothing is imported and `path (data error)` is reported),
  `arc_zero_radius_returns_malformed_path` (now: a line) and `non_arc_path_silently_skipped` (now:
  a line). The same rewrite applies to `tests/it/io_svg/import.rs` AC 14/15. `MalformedPath` is
  also removed from `tests/it/io_svg/corpus/expected.rs`'s kind list.
- `tests/it/io_svg/{path_grammar.rs, mod.rs}`, `tests/it/io_svg/import_fuzz.rs` (arbitrary `d`
  never panics), and six fixture pairs in `tests/fixtures/svg/`.
- `docs/research/svg-spec-coverage.md`, `CHANGELOG.md`. No `src/app/`/`src/ui/` edits.
- ADRs: none (hand-written, no new dependency, as decided in the spec).

## Export contract changes

None. `export.rs` is not touched. AC 9 is proven by the existing `roundtrip_props.rs` and the
`import_arc_golden_paths_*` tests, which stay unchanged and green.

## Béziers and ellipses until LCV-176/177

`C S Q T` and an `A` with `|rx| ≠ |ry|` import nothing, advance the current point to their
endpoint, and add one report note per segment (`path C`, `path S`, `path Q`, `path T`,
`path elliptical arc`). They are never approximated. `Skipped` is the hook that LCV-176/177 will
replace.

## Decisions (self-approved per user goal)

- A syntax error keeps every segment parsed before it, implicit repetitions included. This is
  SVG 2's "render up to the last correct segment", and a finer reading of AC 8's "command".
- A lowercase skipped command reports under its uppercase label: `c` counts as `path C`.
- "Equal endpoints", "zero length", "`rx = 0`" and the `|rx| = |ry|` test all use `EPSILON`,
  measured after `flip_y`, as today's `chord < EPSILON` does.
- `d` that is empty or whitespace only imports nothing and is not an error. `d` that doesn't start
  with `M`/`m` is a data error. A lone `M x y` imports and reports nothing.
- A number after `Z`/`z` with no command letter is a data error, per the SVG 2 grammar.
- A circular arc with `λ > 1` keeps today's `r = chord / 2`, which is the same as scaling by `√λ`
  when `rx = ry`.

## Risks

- LOC cap: after LCV-171, `import.rs` is at about 245 and drops to about 195. `walk.rs` goes from
  about 110 to about 120. The new files stay well under 270. If `path_data.rs` passes 200, the
  segment enum moves to `path_data/segment.rs`.
- Mutation testing: no (not `export.rs`, `agent/` or `History`). The lexer's edge cases each get
  a unit test, and the fuzz test covers panics.
- AC 9 drift: the arc rebuild moves byte for byte into `import/path.rs`, and T7's test pins the
  golden paths before the move.
- Rebase: only `svg`-branch files are touched, so no `ui`/`agent-harness` conflicts.
