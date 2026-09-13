# LCV-117 — Hershey glyph table: 22 alphanumeric glyphs (plus `?`) render below the baseline

- **Status**: Draft
- **Phase**: 11
- **Depends on**: none (the data shipped with LCV-055, Done)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet

## Problem

Engraved text sags. `src/text/layout.rs` places the caller's `origin` **on** the
text baseline and maps glyph data with `py = origin.y + (−hy) * scale`, where
`scale = height_mm / CAP_HEIGHT_HERSHEY` and `CAP_HEIGHT_HERSHEY` is 9. So any
stored datum with `hy > 0` lands **below** the baseline in world space. The
module header of `src/text/hershey_data.rs` states the convention the data is
supposed to obey — baseline at `y = 0`, cap tops at `y = −9`, descenders only
on `g j p q y` — and **22 of the 62 alphanumeric glyphs violate it**.

`'O'` is stored as one stroke running
`(0,−9) (−4,−8) (−5,−5) (−5,−4) (−5,5) (−4,8) (0,9) (4,8) (5,5) (5,−4) (5,−5) (4,−8) (0,−9)`:
symmetric about `y = 0`. Ask for 10 mm text and you get a 20 mm `O`, half of it
hanging under the line, next to an `H` that is correctly 10 mm and flush. Every
word containing one of the 22 sags — on canvas, in the exported SVG, and
therefore in LaserGRBL, where the operator is burning the result into material.
`TEXT` is the tool that just shipped (LCV-112), so this is the first thing a new
user will engrave.

Measured y-extents today (second number = how far below the baseline the glyph
reaches). **Overshoot with no descender at all** — these must end at `y = 0`:
`0` `6` `8` `O` `Q` at `[−9..9]`; `5` `9` at `[−9..6]`; `t` at `[−9..6]`;
`c` `e` `o` `s` at `[−6..6]`; `b` `d` at `[−9..5]`; `u` at `[−5..4]`;
`a` at `[−6..3]`; `S` at `[−9..3]`. **Real descenders, far deeper than the
documented `≈ +4`**: `g` `[−6..10]`, `p` `[−5..9]`, `q` `[−5..9]`, `j` `[−8..6]`,
`y` `[−5..6]`. The other 40 alphanumerics — `1 2 3 4 7 A B C D E F G H I J K L M
N P R T U V W X Y Z f h i k l m n r v w x z` — are correct and flush at `y = 0`.

This is a **data defect, not a layout defect**. The mapping in `layout.rs` and
`CAP_HEIGHT_HERSHEY = 9` are right; the numbers in `hershey_data.rs` are wrong.

### Root cause (diagnosis, verified against the data)

The broken glyphs — or, in several cases, the broken *stroke inside* an
otherwise-correct glyph — were authored with the origin at the **vertical centre
of the glyph body** instead of on the baseline. The tell is `'b'`:

```rust
// 98 'b'
(11, &[
    &[(-5, -9), (-5, 0)],                    // stem: cap-top to baseline — CORRECT
    &[(-5, -2), (-3, -5), (0, -5), (3, -4), (5, -2),
      (5, 2), (3, 4), (0, 5), (-3, 4), (-5, 2)],   // bowl: spans −5..+5 — centred on the baseline
]),
```

`'a'` is the same shape of bug (stem `(5,−6)→(5,0)` correct, bowl `−5..+3`
centred). `'0'`, `'6'`, `'8'`, `'O'`, `'c'`, `'e'`, `'o'`, `'s'` are whole
glyphs drawn symmetric about `y = 0`. This is why the repair is per-glyph and
in some cases per-stroke.

### Why it survived since LCV-055

Every vertical-metrics test in the module uses `'H'`, and `'H'` is one of the 40
correct glyphs: `layout::h_cap_height_matches_height_mm`,
`layout::baseline_at_origin_y`, `layout::y_negation_places_strokes_above_baseline`
and `hershey::h_cap_height_calibration`. Two of them do check the bottom of the
glyph — they simply never look at any glyph but `'H'`. **That** is the coverage
gap this demand closes: the table has no whole-table contract.

### Corrections to the defect report that spawned this demand

The report is accurate on the numbers — the 22 glyphs, every extent, and the
`'O'` coordinates all reproduce exactly. Four claims in it do not survive
checking, and the acceptance criteria below are shaped by the corrections:

1. **"No affine map repairs `'O'`" is false.** `y' = (y − 9) / 2` maps `'O'` to
   top `(0,−9)`, bottom `(0,0)`, shoulders at `±8.5`/`±0.5` and vertical sides
   running `−7 … −2` — i.e. the widest extent is centred at `−4.5`, exactly where
   a 9-unit bowl wants it. The report reads the two side points `−7` and `−6.5`
   as "the widest points" and misses the third at `−2`. The real obstacle to a
   mechanical transform is different and is stated in the notes: the data is
   `i8`, so every squash factor other than a clean `1/2` lands on fractions
   (`'8'`: 6 of 11 distinct y values; `'S'`: 4 of 8), and rounding flattens
   shoulders. A per-glyph affine squash is therefore a legitimate *starting
   point* that must be cleaned up by eye — not a forbidden approach, and not a
   sufficient one.
2. **`'Q'` is not a plain overshoot.** It carries a genuine second stroke — the
   tail, `(1,3) → (5,7)` — on top of the corrupt bowl. Clipping the bowl to the
   baseline without re-anchoring the tail turns `Q` into `O`. See decision 3.
3. **The header's "x-height at `y = −6`" is not what the correct glyphs do.**
   `m n r v w x z` are all correct and top out at `−5`, not `−6`, while
   `a c e o s` (broken) top out at `−6`. The font genuinely uses both. An
   invariant that demands `−6` would fail on seven *correct* glyphs and force a
   restyle of the 40. See decision 5.
4. **`'?'` — confirmed corrupt, and pulled in.** `[−9..5]`, dot at `(0,4)–(0,5)`,
   while the structurally identical `'!'` puts its dot at `(0,0)–(0,−1)`. That is
   the same authoring bug, not a design choice. It is the only punctuation mark
   in scope. See decision 4.

## Scope

- Repair the y coordinates of **23 glyph entries** in `src/text/hershey_data.rs`:
  the 22 alphanumerics `0 5 6 8 9 O Q S a b c d e g j o p q s t u y` plus `?`.
- Update the `hershey_data.rs` module header so the documented convention and
  the data agree exactly.
- Add a **class-based vertical-metrics contract test** covering all 95 glyphs
  (ASCII 32–126) in `src/text/hershey.rs`, and mm-space regression tests in
  `src/text/layout.rs`.

## Out of scope

- **Replacing the table with authoritative Hershey Simplex data, or any other
  font.** Decision 1.
- **Touching the 40 correct alphanumerics or the 32 correct punctuation
  entries.** They stay byte-identical.
- **Advance widths.** Not one of the 95 changes. Horizontal layout is correct
  today and must not move.
- **Unifying the x-height.** The `−6` / `−5` split stays as it is. Decision 5.
- **The 17 baseline-crossing punctuation marks** `# $ ( ) , / ; < > @ [ \ ] _ { | }`.
  Most of them legitimately cross the baseline (`,` `;` `_` `( ) [ ] { }` `$`
  `/` `\` `@`). `{ | }` at `[−9..9]` are 18 units tall and probably carry the
  same authoring bug, but a brace that spans the line is a defensible design and
  nobody is engraving braces. Left alone deliberately, not by oversight; file a
  follow-up demand if they ever bite.
- **`src/text/layout.rs` mapping semantics and `CAP_HEIGHT_HERSHEY = 9`.** They
  are correct. Only the test module in `layout.rs` changes.
- **New glyphs, kerning, ligatures, per-pair spacing, a font-metrics API
  (`text_extents` / bounding box / `line_height`), or any new `pub` item.** No
  consumer.
- **Migrating already-drawn text.** `TEXT` commits plain `Entity::Line`s, so
  drawings made before this fix keep their sagging letters; the operator re-types
  the text to pick up the repair. No migration, no document-schema change.
- **UI, tools, SVG export/import.** No file outside `src/text/` changes.

## Product decisions (do not re-open these)

1. **Re-author the 23 in place; do not import an authoritative Hershey Simplex
   table.** This table is not a mechanical port of the real font: it uses cap
   height 9 (real Hershey uses a taller em and cap 12) and hand-chosen advances
   (`H` 13, `A` 11, `I` 8, `W` 15). Importing the real data would change the
   metrics of all 95 glyphs, break every existing layout test, and change the
   look of text users have already engraved. Surgical repair keeps the 40 good
   glyphs byte-identical and gives the implementer a style reference that is
   already in the file: caps are 10–12 units wide and 9 tall, lowercase bodies
   are 10 wide and 5–6 tall.
2. **The five descenders bottom out at exactly `y = +4`.** The data is at fault,
   not the header: a descender as deep as the cap is tall (`g` at +10) is wrong,
   and +4 is 44 % of cap height, which is in the normal typographic band. The
   `≈` in the header goes away and the data obeys the number, so the invariant
   test can assert equality instead of a fuzzy bound. All five bottom at the same
   depth — `p q g y j` must align with each other.
3. **`Q`'s tail terminates on the baseline (`max y == 0`); it does not become a
   sixth descender.** One exception class in the contract, not two. A tail that
   leaves the bowl at the lower right and stops on the baseline is unmistakable
   against `O`, which is all the tail has to do. AC 14 keeps the tail from being
   quietly deleted.
4. **`?` is in scope; the rest of punctuation is not.** It is one glyph, two
   strokes, and demonstrably the same bug (correction 4 above). The line is drawn
   there and nowhere else.
5. **The x-height split (`−6` for `a c e g o s`, `−5` for `m n p q r u v w x y z`)
   is preserved.** Each repaired glyph keeps the top it has today (AC 11);
   unifying the x-height would mean editing correct glyphs, which is a restyle,
   not a defect fix.
6. **The acceptance gate is a class-based invariant over the whole table, not
   golden coordinates.** Golden per-glyph coordinates would be brittle, would
   freeze the repair's exact choices, and would not stop the next bad glyph. A
   contract keyed on character class bites on all 23 at once, documents the
   convention in executable form, and cannot regress.
7. **The repair method is the implementer's choice** — hand re-authoring, a
   per-glyph affine squash followed by cleanup, or a mix. The gate is the
   invariant test **plus** the visual smoke. A glyph that passes the invariant
   and looks wrong has not been fixed; if a glyph cannot be made to look right
   within these constraints, stop and ask rather than shipping it.

## Acceptance criteria

### A. The vertical-metrics contract (the headline test)

1. A new unit test in `src/text/hershey.rs` — name it `vertical_metrics_contract`
   or equivalent — iterates **every glyph in `GLYPHS`** (ASCII 32–126) and
   asserts criteria 2–9. It is driven by character class and by one explicit
   exemption list, never by per-glyph expected coordinates. Its failure message
   names the offending character and its measured `[min y .. max y]`.
2. **Universal bound**: no point of any glyph has `y < −9` or `y > 9`. (Both hold
   today for the whole table except `g` at +10.)
3. **Caps `A`–`Z`, digits `0`–`9`, and lowercase ascenders `b d f h k l t`**:
   `min y == −9` exactly.
4. **`i` and `j`**: `min y == −8` (the dot).
5. **Remaining lowercase `a c e g m n o p q r s u v w x y z`**: `−6 ≤ min y ≤ −5`.
   Both values occur in the correct 40 and the split is deliberate (decision 5).
6. **Every alphanumeric except `g j p q y`**: `max y == 0` — the glyph ends flush
   on the baseline. This includes `Q` (decision 3).
7. **`g j p q y`**: `max y == 4` exactly.
8. **`?`**: `min y == −9` and `max y == 0`.
9. **Punctuation exemption list**: exactly the 17 characters
   `# $ ( ) , / ; < > @ [ \ ] _ { | }` are exempt from criterion 6. The list is a
   `const` in the test with a one-line comment saying these legitimately cross
   the baseline and are out of LCV-117's scope. They are still bound by
   criterion 2. No other character may appear in the list; every printable ASCII
   character not in it is checked.

### B. The repair

10. Exactly 23 entries change in `src/text/hershey_data.rs`:
    `0 5 6 8 9 O Q S a b c d e g j o p q s t u y ?`. The remaining 72 entries are
    byte-identical, verifiable with
    `git diff -U0 src/text/hershey_data.rs` — every changed hunk falls inside one
    of the 23 named entries or in the module header.
11. **Tops are frozen.** Each repaired glyph's `min y` after the repair equals
    its value today, per the table in the notes (`−9` for
    `0 5 6 8 9 O Q S b d t ?`, `−6` for `a c e g o s`, `−8` for `j`, `−5` for
    `p q u y`).
12. **Widths are frozen.** Each repaired glyph's `min x` and `max x` are
    unchanged (table in the notes), and `advance_width(ch)` is unchanged for all
    95 glyphs. x coordinates may move only *inside* that envelope, and only where
    re-drawing a bowl genuinely requires it — this is a y-coordinate repair, not
    a restyle.
13. **Stroke structure is frozen.** Each repaired glyph has the same number of
    pen-down strokes as today (table in the notes). No stroke is deleted — in
    particular, "fixing" `p`, `q`, `g`, `j` or `y` by dropping the descender
    stroke, or `Q` by dropping the tail, fails this criterion.
14. **`Q` stays distinguishable from `O`.** After the repair `Q` has ≥ 2 strokes,
    every one of its points satisfies `y ≤ 0`, and its tail stroke contains a
    point with `x ≥ 4` and `y ≥ −3` — the tail still leaves the bowl at the
    lower right.
15. **Degenerate data is not introduced.** Every stroke in the 23 repaired
    glyphs still has ≥ 2 points, and no stroke consists entirely of one repeated
    point (which `layout_text` would drop, silently erasing part of a glyph).

### C. Documentation and blast radius

16. The `src/text/hershey_data.rs` module header is updated to state the contract
    the data now satisfies: baseline at `y = 0`; caps, digits and ascenders top
    at `y = −9`; lowercase bodies top at `y = −6` **or** `y = −5` (both occur);
    the `i`/`j` dot at `y = −8`; descenders `g j p q y` reaching **exactly**
    `y = +4` (the `≈` is gone); and one line noting that some punctuation crosses
    the baseline, with the authoritative list living in the contract test. The
    `LOC cap **exempt**` line stays — the file remains pure data literals plus a
    `#[cfg(test)]` module.
17. `CAP_HEIGHT_HERSHEY` is still `9.0`; `layout_text`'s mapping is still
    `py = origin.y + (−hy) * scale`; `glyph_strokes` and `advance_width` are
    unchanged. `git diff --stat` touches only `src/text/hershey_data.rs`,
    `src/text/hershey.rs` and `src/text/layout.rs`, and in the latter two only
    inside `#[cfg(test)]`.

### D. The user-visible outcome, in millimetres

18. For every character in the 62 alphanumerics except `g j p q y`, and for `?`,
    `layout_text(&ch.to_string(), Vec2::new(0.0, 0.0), 10.0, 1.0)` emits only
    endpoints with `−EPSILON ≤ y ≤ 10.0 + EPSILON`.
19. For each of `g j p q y`, the same call's minimum endpoint y equals
    `−10.0 * 4.0 / 9.0` (≈ `−4.4444`) within `EPSILON`, and the maximum is
    ≤ `10.0 + EPSILON`. Millimetre canonicity holds: the depth is
    `height_mm * 4 / 9`, scaling with the requested height and with nothing else.
20. For every cap `A`–`Z` and digit `0`–`9` laid out alone at `height_mm = 10.0`
    from `origin = (0,0)`: maximum endpoint y `== 10.0` and minimum `== 0.0`,
    both within `EPSILON`. Caps and digits are exactly cap-height tall and sit on
    the line.
21. `layout_text("HO", Vec2::new(0.0, 0.0), 10.0, 1.0)` yields max y `== 10.0`
    and min y `== 0.0` within `EPSILON` — the reported symptom, dead.
22. The four existing `'H'` tests
    (`h_cap_height_matches_height_mm`, `baseline_at_origin_y`,
    `y_negation_places_strokes_above_baseline`, `h_cap_height_calibration`)
    still pass unmodified.

### E. Hygiene

23. Kernel purity holds: `grep -rn "egui\|eframe\|rfd" src/text/` returns
    nothing. No new `pub` item anywhere in `src/text/`.
24. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings` and
    `cargo test --all` all exit 0.

## Expected tests

**Quality bar (Marco 1).** Every criterion is covered by an automated test or
tagged **[manual]**. This is a pure-kernel demand: no `App`, no egui, no
harness. All tests are `#[cfg(test)]` unit tests inside `src/text/`.

- **AC 1–9 — `vertical_metrics_contract`** in `src/text/hershey.rs`. One loop
  over ASCII 32–126, a `match` on character class, and the 17-character
  `BASELINE_CROSSING_PUNCTUATION` const. Assertion messages carry the character
  and its measured extent. This single test fails on all 23 broken glyphs today;
  run it before the repair to confirm it bites (list the 23 failures in the
  commit message or the PR body).
- **AC 9 (the list itself)** — `exemption_list_is_exactly_the_documented_17`:
  asserts `BASELINE_CROSSING_PUNCTUATION.len() == 17`, that it is sorted/unique,
  and that none of its members is alphanumeric or `?`. Adding a character to the
  list to silence a future failure then fails this test.
- **AC 11/12/13 — `repaired_glyph_envelopes_are_frozen`** in
  `src/text/hershey.rs`: a `const` table of the 23 `(char, top, min_x, max_x,
  stroke_count, advance)` tuples from the notes, asserted against the data.
- **AC 12 (advances) — `every_advance_width_is_unchanged`**: a `const` table of
  all 95 advance widths, asserted. Cheap, and it makes "the repair must not move
  text horizontally" executable.
- **AC 14 — `q_keeps_a_baseline_tail_and_differs_from_o`**: `Q` has ≥ 2 strokes,
  all points `y ≤ 0`, some tail point with `x ≥ 4 && y ≥ −3`, and
  `glyph_strokes('Q') != glyph_strokes('O')`.
- **AC 15 — `no_repaired_stroke_is_degenerate`**: every stroke of the 23 has
  ≥ 2 points and at least 2 distinct points.
- **AC 2 (regression floor) — `no_glyph_exceeds_the_vertical_bounds`**: covered
  by the contract test; keep it as its own assertion block so the failure message
  distinguishes "out of the table's range" from "wrong class".
- **AC 18 — `every_non_descender_sits_on_or_above_the_baseline`** in
  `src/text/layout.rs`: loops the 57 non-descender alphanumerics plus `?`.
- **AC 19 — `descenders_reach_exactly_four_ninths_below_the_baseline`** in
  `src/text/layout.rs`: the five glyphs, at 10 mm and again at 3.5 mm, asserting
  the depth scales as `height_mm * 4 / 9`.
- **AC 20 — `caps_and_digits_are_exactly_cap_height_and_flush`** in
  `src/text/layout.rs`: 36 characters, top `== height_mm`, bottom `== 0`.
- **AC 21 — `h_and_o_share_both_extremes`** in `src/text/layout.rs`: the
  named-symptom test, `"HO"` at 10 mm.
- **AC 22** — the four existing tests are not edited; `cargo test --all` covers
  them.
- **AC 10/16/17 — static checks**, run by the implementer and recorded in the
  commit: `git diff -U0 src/text/hershey_data.rs` reviewed hunk by hunk against
  the list of 23; `git diff --stat` showing exactly three files;
  `grep -n "LOC cap" src/text/hershey_data.rs` still matching; `grep -n "9.0"
  src/text/hershey.rs` showing `CAP_HEIGHT_HERSHEY` untouched.
- **AC 23 — static checks**: the purity grep, plus
  `git diff src/text/hershey.rs src/text/layout.rs` showing no change outside
  `#[cfg(test)]`.
- **AC 24**: the three commands.
- **[manual] visual smoke — mandatory, this is a font repair and the invariant
  test cannot see ugly.** `cargo run`, then `TEXT` at 10 mm, one string per line:
  `ABCDEFGHIJKLMNOPQRSTUVWXYZ`, `abcdefghijklmnopqrstuvwxyz`,
  `0123456789`, `Quiz? Gypsy jabs.`. Check, at a readable zoom:
  (a) every capital and digit starts and stops on the same two horizontal lines —
  no letter is taller, shorter or lower than its neighbours;
  (b) `O` and `Q` are round, not squat or lens-shaped, and `Q` is instantly
  distinguishable from `O`;
  (c) `8`, `S`, `s`, `a`, `e`, `g` still read as themselves — rounding during the
  repair must not have flattened a shoulder into a corner;
  (d) `g j p q y` all descend to the same depth, and the descender is clearly
  shallower than the cap height;
  (e) `?` sits between `z` and `!` in height with its dot just above the line.
  Then `File > Save As…` (v2 saves the document *as* SVG — there is no separate
  export item), open the written file, and confirm the same in the exported
  geometry. The export path is untouched by this demand, so this is a sanity
  check, not a hypothesis.

## Open questions

None.

## Risks

- **Aesthetic regression passing a green test.** The invariant pins the metrics,
  not the shape. `8`, `S`, `s` and `g` are the ones to watch: a naive squash
  rounds their curve shoulders onto integers and can produce a straight segment
  where the eye expects a bowl. The mandatory visual smoke is the counterweight;
  decision 7 says stop and ask rather than ship a passing-but-ugly glyph.
- **Per-stroke, not per-glyph.** `a`, `b`, `d`, `p`, `q`, `t` mix a correct
  stroke with a corrupt one (see the root-cause section). Transforming the whole
  glyph moves the stem too and breaks the ascender.
- **`i8` rounding.** Squash factors other than `1/2` land on fractions for most
  glyphs (`'S'` 0.75, `'a'` 0.667, `'b'`/`'d'`/`'?'` 0.643, `'g'` 0.625,
  `'y'` 0.818, `'j'` 0.857). Rounding can also collapse two adjacent points into
  one — harmless (layout drops degenerate segments) as long as AC 15 holds.
- **Nothing else consumes these coordinates — verified.** The only consumers are
  `src/tools/text.rs` (calls `layout_text` for preview and commit) and three
  tests: `tests/lcv100_svg_orientation.rs` uses `'F'` (untouched),
  `tests/lcv112.rs` compares `layout_text` output against `layout_text` output
  (shape-insensitive), and `tests/skeleton.rs` only names
  `CAP_HEIGHT_HERSHEY`. No test anywhere hardcodes a glyph's coordinates or an
  entity count derived from one. `src/io/svg/` never mentions text: glyph strokes
  reach it as ordinary `Entity::Line`s.
- **Old drawings are not migrated** (out of scope, and worth saying out loud in
  the CHANGELOG entry): text already committed is plain line geometry and keeps
  its sag until the operator re-types it.

## Notes

- **The 23 entries, with the envelope to preserve.** `top` and the x-extent are
  today's values and must survive the repair (AC 11, 12); `strokes` and `adv`
  likewise (AC 12, 13). `bottom → target` is the only thing that moves.

  | ch | today `[min y..max y]` | target `[min y..max y]` | x-extent | strokes | adv |
  |---|---|---|---|---|---|
  | `0` | `[−9..9]` | `[−9..0]` | `−5..5` | 1 | 11 |
  | `5` | `[−9..6]` | `[−9..0]` | `−5..5` | 1 | 11 |
  | `6` | `[−9..9]` | `[−9..0]` | `−5..5` | 1 | 11 |
  | `8` | `[−9..9]` | `[−9..0]` | `−5..5` | 2 | 11 |
  | `9` | `[−9..6]` | `[−9..0]` | `−5..5` | 2 | 11 |
  | `O` | `[−9..9]` | `[−9..0]` | `−5..5` | 1 | 12 |
  | `Q` | `[−9..9]` | `[−9..0]` | `−5..5` | 2 | 12 |
  | `S` | `[−9..3]` | `[−9..0]` | `−5..5` | 1 | 10 |
  | `a` | `[−6..3]` | `[−6..0]` | `−4..5` | 2 | 11 |
  | `b` | `[−9..5]` | `[−9..0]` | `−5..5` | 2 | 11 |
  | `c` | `[−6..6]` | `[−6..0]` | `−5..5` | 1 | 10 |
  | `d` | `[−9..5]` | `[−9..0]` | `−5..5` | 2 | 11 |
  | `e` | `[−6..6]` | `[−6..0]` | `−5..5` | 1 | 11 |
  | `g` | `[−6..10]` | `[−6..4]` | `−5..5` | 2 | 11 |
  | `j` | `[−8..6]` | `[−8..4]` | `−1..2` | 2 | 6 |
  | `o` | `[−6..6]` | `[−6..0]` | `−5..5` | 1 | 11 |
  | `p` | `[−5..9]` | `[−5..4]` | `−5..5` | 2 | 11 |
  | `q` | `[−5..9]` | `[−5..4]` | `−5..5` | 2 | 11 |
  | `s` | `[−6..6]` | `[−6..0]` | `−5..5` | 1 | 9 |
  | `t` | `[−9..6]` | `[−9..0]` | `−3..3` | 2 | 8 |
  | `u` | `[−5..4]` | `[−5..0]` | `−5..5` | 2 | 11 |
  | `y` | `[−5..6]` | `[−5..4]` | `−5..5` | 2 | 11 |
  | `?` | `[−9..5]` | `[−9..0]` | `−4..5` | 2 | 9 |

- **Style reference for the repair**, read off the 40 correct glyphs: caps are
  9 tall, and most are 10 wide (`±5`) — `H M N T` are 12 (`±6`), `W` is 14
  (`±7`), `J` is 7 and `I` is 6; lowercase bodies are 5–6 tall and 10 wide.
  `C` and `G` are the closest correct relatives of `O`/`Q`, `n`/`v` of
  `o`/`c`/`e`, and `!` of `?`.
- **Easy wins.** Some of the 23 need only their descending stroke moved, not a
  redraw: `y`'s tail is `(−2,4) (−4,6) (−5,6)` and wants `(−2,2) (−4,4) (−5,4)`;
  `b`/`d`/`a` need only their bowl stroke re-fitted between the stem's top and
  the baseline; `p`/`q` need the stem shortened to `+4` and the bowl fitted into
  `[−5..0]`. `0 6 8 O c e o s` are the true redraws.
- **v1 reference**: none. The Hershey data entered v2 with LCV-055; v1 had no
  text tool, so there is no prior art to diff against.
- `src/text/hershey_data.rs` is LOC-cap exempt (976 lines of data literals) and
  stays exempt; adding a `#[cfg(test)]` module to it is not required — the
  contract test belongs in `src/text/hershey.rs`, which already owns
  `h_cap_height_calibration` and the accessors, keeping `hershey_data.rs` pure
  data as its header claims.
- **`CHANGELOG.md`**: `demand-manager` writes the entry, under **Fixed**, when
  this lands. It should say that previously drawn text is not migrated.
