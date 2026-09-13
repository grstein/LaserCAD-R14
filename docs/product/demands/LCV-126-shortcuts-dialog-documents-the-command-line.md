# LCV-126 — The F1 shortcuts dialog does not mention the command line

- **Status**: Ready
- **Phase**: 11
- **Depends on**: LCV-116 (Done), LCV-111 (Done); LCV-132 **soft** — AC 4 uses its harness if it has landed and an inline collector if it has not
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`AGENTS.md` §Product Philosophy puts "prefer command line and keyboard-first
flows" first among the default answers, and LCV-110/111 built that: a parser
kernel, a 50-entry recall ring (`src/cmdline`), and two bindings that make the
command line reachable without the mouse — `ArrowUp` / `ArrowDown` walk the ring
while the field has focus (`src/app/input.rs::recall`, LCV-111 AC 23), and any
typed character that is not a tool letter seeds and focuses the field
(`src/app/input.rs`, LCV-111 AC 21). Both are real, shipped, and documented in
the gate table at `src/app/input.rs:24`.

Neither appears in the `F1` Keyboard shortcuts dialog. `SHORTCUT_GROUPS`
(`src/ui/dialogs.rs:145`) lists File, Edit, View, Modes, Drawing and Help, and
the tool letters are derived from `TOOLS` above them. There is no Command line
group. The dialog is the only in-app inventory of what the keyboard does, and
`src/ui/dialogs.rs:139` states its own contract: "Nothing aspirational goes in
this table — the dialog documents what exists." The converse failed here: an
operator who opens F1 to find out how to reach the command line concludes there
is no way, and the recall ring — an entire feature with its own kernel module —
is invisible to everyone who did not read the source.

The omission is traceable: LCV-116 AC 15 enumerated the twenty rows the dialog
must carry and left `ArrowUp` / `ArrowDown` and the typed-character seed out of
the list. The implementer built exactly the table the criterion specified and a
test now pins it row for row, so correcting the criterion's text alone changes
nothing an operator can see. This demand closes the gap in the shipped UI.

### Three things changed after this demand was written (2026-09-13)

1. **LCV-124 made the command line route to an agent.** A `:` or `/ai` prefix
   goes to the model, and with an API key configured **any line the CAD grammar
   does not recognise** goes there too. That is a real discoverability problem
   and it is **not this demand's** — see §Out of scope for why a keyboard table
   is the wrong surface for it, and where it belongs instead.
2. **LCV-131 recorded that the grammar is smaller than it looks.** The alias set
   is the frozen v1 one — `l p r c a s t e m text` plus `snap grid ortho` and
   the zoom forms (ADR 0003 §A2). `line`, `circle` and every other full AutoCAD
   word is `Unknown`. Nothing this demand writes may imply otherwise, and the
   manual smoke below was rewritten because its original steps typed `LINE`,
   which draws nothing and — with a key configured — spends money.
3. **Painted text became assertable at the pinned egui 0.29.1** (LCV-132).
   `Context::run` returns the paint list and `Shape::Text` carries the exact
   string and its position. This demand's whole value is that *an operator sees
   three rows in a dialog*, and until now nothing asserted the dialog painted
   anything — a `.take(5)` on the group loop would have shipped green. AC 4 is
   new because of it.

## Scope

- One new group in `SHORTCUT_GROUPS` covering how to reach the command line and
  how to recall a previous command.
- The two LCV-116 AC 15 tests in `src/ui/dialogs.rs` extended to cover the new
  rows, keeping the guarantees they already provide.
- One paint assertion proving the dialog actually puts the new group on the
  screen, in position, with its three rows.

## Out of scope

- **Any new or changed binding.** `recall()` and the typed-character seed are
  untouched; this demand only documents them. If a row cannot be justified by a
  `grep` in `src/ui/shortcuts.rs` or `src/app/input.rs`, it does not go in.
- **Restyling, reordering or resizing the dialog**, adding search, filtering,
  scroll-position memory, a printable cheat sheet, or a second column.
- **A tooltip, placeholder or hint on the command-line widget itself.** A
  different surface and a different demand; this one is the F1 inventory.
- **Rows for the mouse** (wheel zoom, middle-drag pan). Real bindings, but the
  dialog is titled "Keyboard shortcuts" and the two key-reader files are its
  stated source of truth. A mouse section would need its own justification.
- **A row about the agent, the `:` / `/ai` prefix, or "unrecognised lines cost
  money" (LCV-124).** Rejected here, three reasons, none of them "it isn't
  important":
  1. **It is not a keyboard binding.** The dialog's own doc comment
     (`src/ui/dialogs.rs:135-140`) says every row is verified against the only
     two files that read keys. An agent row has no needle in either, so AC 3's
     `every_documented_binding_exists_in_the_two_key_readers` — whose
     `documented == mapped` equality is the thing keeping this table honest —
     would have to be weakened to admit it. Erasing a test to make room for a
     row is a bad trade at any row.
  2. **It documents a behaviour we intend to change.** LCV-131 exists to stop
     CAD words reaching the model. A dialog row shipped now is a row edited
     twice.
  3. **The surface is wrong.** A warning about what a line will *cost* belongs
     next to the line the operator is typing, not in a keyboard inventory they
     opened once. That is a different demand — see §Notes.
- **Reopening LCV-116.** It is Done and reviewed; this is a follow-on, not a
  rework.

## Acceptance criteria

1. **The group exists, in position.** `SHORTCUT_GROUPS` in `src/ui/dialogs.rs`
   gains one `ShortcutGroup` with `heading: "Command line"`, placed **after**
   `"Drawing"` and **before** `"Help"`, carrying exactly these three rows in
   this order:

   ```rust
   ("Any other character", "Start a command in the command line"),
   ("ArrowUp",             "Previous command (command line focused)"),
   ("ArrowDown",           "Next command (command line focused)"),
   ```

   No other group is added, removed, reordered or reworded.

2. **The rows are honest about their two conditions**, because both are
   conditions an operator will otherwise report as a bug:
   - the seed row says "any *other* character", because a tool letter activates
     its tool instead (the Tools rows at the top of the dialog are that list);
   - both recall rows say the command line must have focus, because
     `src/app/input.rs::process_input` runs `recall` only under
     `if app.command_line_focused`, so `ArrowUp` does nothing otherwise.

3. **Both LCV-116 AC 15 tests keep working and keep discriminating.**
   `static_rows_cover_every_documented_binding` gains the three rows at the
   matching position in its `expected` table, so deleting or reordering one
   still fails. `every_documented_binding_exists_in_the_two_key_readers` gains
   three entries in its `bindings` map — `"ArrowUp" → ["Key::ArrowUp"]`,
   `"ArrowDown" → ["Key::ArrowDown"]`, `"Any other character" →
   ["typed_chars(", "focus_command_line"]` — and its `documented == mapped`
   equality assertion and its `Key::F12` negative control both stay exactly as
   they are.

4. **The dialog paints the group, and a test says so.** A headless frame with
   `app.shortcuts_open = true` (ADR 0002 §A2: `App::default()`, never
   `App::new()`) is driven twice — the second frame is the settled one — and
   the painted text of the dialog surface is asserted to contain, **as
   consecutive visual lines in this order**: the heading `Command line`, then
   the three AC 1 rows, each as its binding and its description, and then the
   heading `Help`. A data-structure assertion over `SHORTCUT_GROUPS` cannot
   make this claim: the constant and the loop that renders it are two different
   things, and every mutation in §Expected tests item (c) leaves the constant
   untouched.

   The collector is conditional, the assertion is not:
   - **If LCV-132 has landed**, use `harness::lines_on_surface_of` and
     `harness::texts`, and scope by a marker **inside** the dialog body — the
     `Command line` heading. Never the window title `Keyboard shortcuts`: a
     `Window` title is clipped to the whole screen, so it selects every surface
     in the frame.
   - **If it has not**, write the smallest collector that does the job in
     `tests/lcv126_command_line_group.rs`, recursing into `Shape::Vec`, and
     **say so in the handover** so LCV-132's move absorbs this file too. Do not
     copy LCV-125's full apparatus into a third place.

5. **The dialog's existing contracts are untouched.** It still reads no key
   (LCV-116 AC 16), still changes no `App` state, still derives its tool rows
   from `TOOLS` (LCV-116 AC 14), and `F1` is still dispatched only in
   `src/ui/shortcuts.rs`. The three tests that pin those are unmodified.

6. **Caps and gates.** `src/ui/dialogs.rs` is at 237 of the 300
   implementation-LOC cap; measure with the ADR 0004 `awk` recipe, never
   `wc -l`. Three rows and a heading must not push it past 270 — if it does,
   flag the seam to `architect` rather than splitting on sight (237 re-measured
   with that recipe on 2026-09-13 — still accurate).
   `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
   and `cargo test --all --no-fail-fast` exit 0. **`--no-fail-fast` is not
   optional** (ADR 0008): without it one red unit test skips every integration
   binary, including the one this demand adds.

## Expected tests

- **Unit (AC 1, 3)** in `src/ui/dialogs.rs`: the two extended tests above, each
  shown to discriminate — delete the `ArrowUp` row and confirm
  `static_rows_cover_every_documented_binding` fails by name; add a fabricated
  `("ArrowLeft", …)` row and confirm
  `every_documented_binding_exists_in_the_two_key_readers` fails by name.
- **Unit (AC 2)**: the exact strings are pinned by AC 1's table, so no extra
  test is needed for the wording — but a binding string longer than the
  dialog's 20-character monospace column wraps badly. `"Any other character"`
  is 19; if a later edit makes one longer, shorten the binding, do not drop the
  row.
- **Integration (AC 4)** in `tests/lcv126_command_line_group.rs`: the painted
  assertion, on the second frame. It must also carry a **positive control** —
  the frame painted more than ten runs — so that a dialog which rendered
  nothing at all cannot read as "the rows are absent, as expected".
- **Mutations the implementer applies, measures and reverts**, reporting the
  failing test name and assertion message for each. The first two are the
  existing discrimination checks; (c) is the set AC 4 exists for, and **every
  one of (c) leaves `SHORTCUT_GROUPS` untouched, so items 1–3 stay green**:
  (a) delete the `ArrowUp` row from `SHORTCUT_GROUPS`;
  (b) add a fabricated `("ArrowLeft", …)` row;
  (c) in `shortcuts_dialog`: `for group in SHORTCUT_GROUPS.iter().take(5)`
      (the last group never renders); `.rev()` on the group loop (headings come
      out backwards); and `ui.heading(group.heading)` without the row loop
      (headings only, no rows).
  A mutation that survives is a finding to report, not a criterion to relax.
- **[manual] smoke** — one pass, and it must not touch the agent:
  1. `cargo run`. Press `F1`. A **Command line** section sits between Drawing
     and Help with the three rows, and the window is titled
     `Keyboard shortcuts`. Close it.
  2. Press `G`. `G` is not a tool letter and not a shortcut, so the command
     line takes focus with `g` in it — this is the first row, demonstrated.
     (Do **not** use `X`: it is the Trim tool's letter. The tool letters are
     `L P R C A D M T X E`.)
  3. Continue typing `rid` so the field reads `grid`, press Enter. The GRID
     indicator in the status bar flips — the command was recognised, locally,
     with no network call.
  4. Click into the command line and press `ArrowUp`. `grid` comes back. Press
     `ArrowDown`; it walks forward. Both recall rows, demonstrated.
  5. Every row in the dialog now corresponds to something the operator just
     did.

  **Do not type `LINE`, `CIRCLE` or any other full command word in this smoke.**
  The grammar does not know them (ADR 0003 §A2, LCV-131), so nothing is drawn —
  and with an API key configured, LCV-124 forwards the line to the model, which
  turns a manual smoke into a billable request. `grid` is chosen because it is
  in the grammar, its effect is visible in one glance at the status bar, and it
  draws nothing to undo.

## Test hygiene (mandatory)

- Bound every source scan to the implementation section (bare `#[cfg(test)]` at
  column 0) and build every needle with `concat!`. The two extended tests in
  `src/ui/dialogs.rs` read the key-reader files; keep them bounded the way
  LCV-116 left them. Canonical correct example: `guard_is_runtime_not_cfg` at
  `src/io/dialogs.rs:179-194`.
- Rebuild compared paths from `components()` joined with `/` — never
  `Path::display()`. This has broken CI twice.
- **AC 4 asserts no absolute coordinate.** Lines are asserted by their text and
  their order, never by the `y` they landed on: font metrics and theme spacing
  are not this demand's contract.
- No test reaches a real endpoint, and no test in this demand touches the agent
  at all.

## Open questions

None. The one judgement call — whether the dialog should warn that an
unrecognised line reaches a paid model — is decided in §Out of scope, with the
surface it belongs on named in §Notes. This demand is Ready when
`demand-manager` flips it.

## Notes

- Origin: `product-owner` review of LCV-116 AC 15 (2026-09-13). That criterion
  listed twenty rows and omitted these; the implementer built the table it
  specified. LCV-116 AC 15 now carries a correction note pointing here. LCV-116
  itself stays `Done` — it shipped what it asked for.
- `src/app/input.rs:16-25` is the gate table and the authoritative list of every
  key route in the app. It is worth re-reading against `SHORTCUT_GROUPS`
  whenever either changes: it is the only place the two readers' full contract
  is written down in one view, and this demand exists because the dialog drifted
  from it.
- **Where the agent warning belongs, since it does not belong here.** The
  operator-facing problem LCV-124 created is "I typed something and it cost me
  money". The two candidate surfaces are a hint under the command-line field
  and a line in the agent panel; both are a new demand, and both are worth less
  once LCV-131 lands, because LCV-131's whole purpose is that CAD words stop
  reaching the model. Sequence it after LCV-131, not before, or it documents a
  behaviour that is about to change.
- **`X` is the Trim tool's letter**, which the original manual smoke got wrong
  (it used `X` as an example of a character that is *not* a tool letter). The
  full set is `L P R C A D M T X E`, derived from `TOOLS` in
  `src/ui/toolbar.rs`. The smoke now uses `G`.
- Related: LCV-111 (the command line and the recall ring), LCV-116 (the dialog),
  LCV-124 (agent routing — see §Out of scope), LCV-131 (the grammar is the
  frozen v1 alias set), LCV-132 (the paint-list harness AC 4 prefers),
  [ADR 0003](../../adr/0003-command-line-input-contract.md) (the command-line
  input contract), [ADR 0002](../../adr/0002-headless-input-tests-and-dirty-tracking.md)
  §A6 (the two-reader rule the dialog must not join).
