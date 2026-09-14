# ADR 0009 — Dialog content is capped at 420pt, and dialog growth is asserted, not assumed

- **Status**: Accepted
- **Date**: 2026-09-13
- **Deciders**: architect

## Context

`Window::new` in egui 0.29.1 bakes `.default_size([340.0, 420.0])`
(`egui-0.29.1/src/containers/window.rs:64`), and `Resize::default_height`'s own
doc comment says that when the contents are a `ScrollArea` this decides the
**maximum** size (`egui-0.29.1/src/containers/resize.rs:98-104`). Every in-app
dialog in this repo is a `Window` with `.resizable(false)`; the keyboard
shortcuts dialog is the only one whose body is rendered from a table
(`SHORTCUT_GROUPS` + `tool_rows`), so it is the only one that can outgrow the
cap without anybody editing a layout.

It did. LCV-126 added the `Command line` group, proved by paint assertion that
it rendered, and shipped; five of eight groups were nonetheless below the fold.
LCV-133 fixed that with a two-column layout and no sizing call.

**Measured at `aa36bd8`**, egui 0.29.1, `pixels_per_point = 1.0`, `App::default()`
with `shortcuts_open: true`, on a settled (second) frame, at 1280×800, 1024×600
and 800×600 — the three numbers are identical at all three sizes, because the cap
is independent of screen size:

- dialog body clip height: **426.00pt**
- one shortcut row: **20.91pt**; one heading: **23.83pt**; inter-section spacer: 6pt
- shipped content (36 items = 8 headings + 28 rows): deepest column ends
  **8.00pt** above the clip bottom. The other column has 32.00pt.

Eight points is 38% of one row. What that buys, measured the same way
(*runs painted / runs the content requires*):

| content | left slack | right slack | runs | what the operator sees |
|---|---|---|---|---|
| shipped | +8.00 | +32.00 | 64 / 64 | correct |
| +1 tool | **−13.00** | +32.00 | 66 / 66 | a row sliced by the clip edge |
| +2 tools | −13.00 | +32.00 | 66 / **68** | a row gone |
| +3 tools | +17.00 | −21.00 | 68 / 68 | `Help` painted outside the clip |
| +4 rows on `Help` | +8.00 | +20.00 | 67 / **72** | `View` gone |
| +1 group of 4 rows | +8.00 | −10.00 | 68 / **73** | `View` gone |

**The first three of those pass every test LCV-133 shipped.** AC 1/AC 2 assert
eight hand-typed headings and the `F1` / `This dialog` row; a sliced or culled
*tool* row is none of those. Only AC 3's derived run count sees it, and only from
the second added row on.

The same measurement refutes a second assumption. LCV-133 §Expected tests probes
AC 5 by adding a synthetic ninth group of four rows and re-running the AC 1 test
unchanged. Enumerating all ten order-preserving whole-section cut points of that
mutated 9-section / 41-item set, by rendering each one rather than by counting
items:

| cut | left/right items | runs / 73 | AC 1 |
|---|---|---|---|
| 3 | 19 / 22 | **65** | **passes** |
| 4 (what the shipped rule picks) | 22 / 19 | 68 | fails — `View` painted 0 times |
| every other cut (0,1,2,5,6,7,8,9) | — | 35..62 | fails |

Cut 3 passes the probe **while culling all four rows of the group the probe just
added** — 65 of 73 runs. No cut paints 73. The probe can therefore be satisfied
by a dialog exhibiting the exact defect LCV-126 and LCV-133 exist to fix, and the
difference between "passes" and "fails" is which side of a two-way tie the
balancing rule breaks — an accident, not a property.

## Decision

1. **Treat 426pt of body as a hard cap on any `Window` containing a
   `ScrollArea`.** It does not grow with the screen, the monitor, or the window.
   A dialog body that can grow from a table is measured against it; it is never
   assumed to fit because it fits today.

2. **A paint test that claims a dialog shows *all* of a table derives its
   expected set from that table.** A hand-typed expected list can only see what
   it was told about. `expected_run_count` in
   `tests/lcv133_shortcuts_dialog_fits.rs` is the shape to copy; the `HEADINGS`
   const in the same file is the shape that missed cut 3.

3. **A growth probe asserts on the content it added.** Re-running a test whose
   expectations are fixed proves nothing about growth — it proves the fixed
   expectations still hold, which is a different claim and a weaker one. A
   mutation that adds a row or a group must be accompanied by the derived count
   that can see it.

4. **The table-driven dialog carries a headroom assertion, not a comment.** Two
   assertions, both cheap: *every* non-empty run in the dialog body is fully
   inside the body clip rect (not just the headings and not just one row), and
   the deepest column's bottom clears the clip bottom by at least one row height.
   A slack floor named in points is what makes the failure arrive on the commit
   that adds the row, instead of on the bug report that follows it.

5. **When the cap binds, the pre-decided remedy is a sizing call on top of the
   columns — not a third column.** Both were measured at the pin, two columns
   held constant:

   - `.default_height(ctx.screen_rect().height() - 80.0)`: real content gets
     145/169pt of slack at 1280×800 and 45/69pt at 600-high screens; +3 tools
     → 70/70 runs painted and contained; +1 group of 4 rows → **73/73** at all
     three sizes. It absorbs every growth case in the table above.
     Caveat for whoever lands it: at 600-high screens the body clip bottom lands
     exactly on the screen edge (`clip = [104.68, 600.00]`), so that demand must
     assert on the **window rect against the screen rect**, not only on the runs.
   - A balanced third column: 1280×800 paints all 64 runs (body 1189pt wide),
     1024×600 is screen-clamped, and **800×600 paints 36 of 64** — the window is
     wider than the screen and the third column is cut off horizontally. It
     trades a vertical clip for a horizontal one at exactly the sizes the demand
     cares about. **Rejected.**

   LCV-133 §Out of scope rejected `.default_height()` *as a substitute for the
   columns*, on its own measurements, and was right: alone it leaves `Help`
   clipped at 1280×800. That is not this decision. Adding it *on top of* a
   landed two-column layout is a different configuration with different measured
   results, and it is the one to reach for. Nothing here relitigates LCV-133.

## Consequences

**Easier**

- The next person who adds a tool or a shortcut row learns about the cap from a
  red test naming the slack, in the same commit, instead of from an operator who
  cannot find a binding.
- Rules 2 and 3 are mechanical. "Is the expected set derived from the same table
  the UI renders from?" is answerable by reading two lines, so implementer and
  reviewer cannot disagree about whether a growth probe is real.
- Decision 5 means the demand that finally crosses the cap executes a measured
  configuration instead of re-deriving one against a deadline — the same posture
  as ADR 0004 rule 4.

**Harder / committed to**

- Paint tests get slightly more expensive to write: a derived expected set is a
  few lines more than a literal, and a slack floor is a number that has to be
  measured once.
- A named slack floor will fail on an egui upgrade that changes row metrics.
  That is the point; the failure is one number in one test, and the alternative
  is a cap nobody notices moving.
- We are committed to the shortcuts dialog staying inside 426pt **until**
  decision 5's sizing call lands. Until then, the dialog is one tool away from
  a sliced row. That limit is documented rather than fixed because fixing it is
  a sizing change that LCV-133 deliberately did not carry, and stacking it in
  after the fact is how a dialog acquires a permanent mystery (LCV-133 §Out of
  scope's own reasoning, which still holds).

## Alternatives considered

- **Force the balancing rule's tie-break to prefer the earlier cut**, so
  LCV-133's AC 5 probe goes green. Rejected outright: cut 3 is green *because*
  it hides the four rows it was handed. Choosing a rule to make a probe pass
  while the dialog silently drops content is worse than the red probe.
- **A third column.** Measured, rejected — 36 of 64 runs at 800×600 (above).
- **`.max_size()` / `.resizable(true)` / removing the `ScrollArea`.** Rejected
  by LCV-133 §Out of scope on its own measurements; nothing here changes that.
  The `ScrollArea` stays as the safety net for a screen shorter than the
  content.
- **Shorter rows** (dropping `shortcut_row`'s `{binding:<20}` padding). Buys
  width. The constraint is height.
- **Collapsible groups, search, a printable sheet.** Product scope, refused
  twice already (LCV-126, LCV-133 §Out of scope). This dialog is a static
  inventory an operator reads for four seconds.

## Revisit criteria

- egui is unpinned from 0.29.1 → re-measure the 426pt body and the row/heading
  heights; every number in this ADR is a snapshot of the pin.
- A second dialog acquires a table-driven body → rules 2, 3 and 4 bind it too,
  with no new decision.
- Decision 5's sizing call lands → the "committed to" paragraph above is spent;
  record the shipped configuration and its measured slack in the demand.
