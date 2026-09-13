# LCV-126 — The F1 shortcuts dialog does not mention the command line

- **Status**: _(to be set by `demand-manager`)_
- **Phase**: 11
- **Depends on**: LCV-116 (Done), LCV-111 (Done)
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

## Scope

- One new group in `SHORTCUT_GROUPS` covering how to reach the command line and
  how to recall a previous command.
- The two LCV-116 AC 15 tests in `src/ui/dialogs.rs` extended to cover the new
  rows, keeping the guarantees they already provide.

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

4. **The dialog's existing contracts are untouched.** It still reads no key
   (LCV-116 AC 16), still changes no `App` state, still derives its tool rows
   from `TOOLS` (LCV-116 AC 14), and `F1` is still dispatched only in
   `src/ui/shortcuts.rs`. The three tests that pin those are unmodified.

5. **Caps and gates.** `src/ui/dialogs.rs` is at 237 of the 300
   implementation-LOC cap; measure with the ADR 0004 `awk` recipe, never
   `wc -l`. Three rows and a heading must not push it past 270 — if it does,
   flag the seam to `architect` rather than splitting on sight.
   `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`
   and `cargo test --all` exit 0.

## Expected tests

- **Unit (AC 1, 3)** in `src/ui/dialogs.rs`: the two extended tests above. Prove
  they still discriminate by deleting the `ArrowUp` row and confirming
  `static_rows_cover_every_documented_binding` fails by name, and by adding a
  fabricated `("ArrowLeft", …)` row and confirming
  `every_documented_binding_exists_in_the_two_key_readers` fails by name.
- **Unit (AC 2)**: the exact strings are pinned by AC 1's table, so no extra
  test is needed for the wording — but a row whose binding string exceeds the
  dialog's 20-character monospace column would wrap ugly. `"Any other
  character"` is 19; if a later edit makes one longer, shorten the binding, do
  not drop the row.
- **[manual] smoke**: `cargo run`; press `F1` → a **Command line** section sits
  between Drawing and Help with the three rows. Close it; press `X` (not a tool
  letter, not a shortcut) → the command line takes focus with `X` in it; press
  Escape, type `LINE`, Enter, draw a line, Escape; click into the command line
  and press `ArrowUp` → `LINE` comes back. Every row in the dialog now
  corresponds to something the operator just did.

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
- Related: LCV-111 (the command line and the recall ring), LCV-116 (the dialog),
  [ADR 0003](../../adr/0003-command-line-input-contract.md) (the command-line
  input contract), [ADR 0002](../../adr/0002-headless-input-tests-and-dirty-tracking.md)
  §A6 (the two-reader rule the dialog must not join).
