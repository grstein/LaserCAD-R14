# LCV-131 — The command line does not know the words that name its own commands

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-124 (the classifier), LCV-148 (the rule-4 flip; it lands first and is what makes this demand a pure grammar change)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

> **Filename note.** The slug is historical (`…-pays-a-model-for-cad-words`).
> After [LCV-148](LCV-148-unrecognised-lines-stay-local.md) lands, typing `LINE`
> pays a model nothing — it answers `Unknown command: "line"` locally and for
> free. The file is deliberately **not** renamed: cross-references from LCV-124,
> LCV-126, LCV-134, ADR 0003 and ADR 0007 all point at it. The title above is
> the current claim; the slug is a permalink.

## Problem

The v2 grammar accepts **only the single-letter aliases** `l p r c a s t e m`
plus `text` (`src/cmdline/parse.rs::tool_alias`). Every **full AutoCAD command
name** is therefore unrecognised:

```
parse("line")   -> Unknown("line")      // asserted today in rejects_malformed_input_as_unknown
parse("circle") -> Unknown("circle")
parse("erase")  -> Unknown("erase")
parse("del")    -> Unknown("del")       // asserted in the same test
```

In AutoCAD R14 the full name is the **primary** form and the letter is the
abbreviation: you type `LINE` and `L` is the shortcut, `CIRCLE` and `C`. This
product is a KISS clone of R14. So the most ordinary thing an R14 operator can
type at the command line — the command's own name — draws nothing and answers an
error. An operator cutting a part reaches for the one spelling they have used
for twenty years and the product tells them it has never heard of it.

`z` is the same shape one step smaller, and is what started this: `z` is the
standard AutoCAD zoom abbreviation, and both the LCV-124 implementer and its
reviewer independently found that the demand's expected-tests table listed `z`
among the CAD cases when `parse("z")` is `Unknown("z")`.

**What has changed since this demand was written.** Its original problem
statement was that these words cost *money*: with an API key configured,
LCV-124's rule 4 forwarded every unrecognised line to a language model, so
`LINE` was a paid round trip to a model that might answer it by drawing
something. That half is now [LCV-148](LCV-148-unrecognised-lines-stay-local.md),
which flips ADR 0007 §D9 rule 4 to `Unknown → Route::Cad, always` and lands
first. After it, a word the grammar does not know is free, local and harmless.

The remaining problem is the one that was always the more valuable half: **the
command line should know the words that name its own commands.** It is a
usability gap in the product's primary input surface, not a cost control.

## The product call

**Add the full command names to the alias table.** This is not a new feature —
it restores the *primary* spelling of every command this product already ships.
No new tool, no new behaviour, no new letter: `line` does exactly what `l` does
today.

The approved word set is **exactly** these fifteen words, mapping to existing
`ToolKind` variants:

| Word | `ToolKind` | Word | `ToolKind` |
|---|---|---|---|
| `line` | `Line` | `select` | `Select` |
| `polyline` | `Polyline` | `trim` | `Trim` |
| `pline` | `Polyline` | `extend` | `Extend` |
| `rect` | `Rect` | `move` | `Move` |
| `rectangle` | `Rect` | `text` | `Text` *(already shipped)* |
| `circle` | `Circle` | `delete` | `Delete` |
| `arc` | `Arc` | `del` | `Delete` |
| | | `erase` | `Delete` |

Fourteen new rows; `text` is already there. `pline`, `rectangle` and `del` are
in the set because they are the spellings R14 operators actually type.
`delete` / `del` / `erase` all reach the same tool: R14 says `ERASE`, the v2
tool is named Delete and the menu says Delete, so the product answers to both
vocabularies rather than picking a side.

**`offset` is not in the set, and the reason is not taste.** ADR 0003 §A2a: a
word may only be added if it maps to an **existing** variant, and there is no
`ToolKind::Offset` — the tool was rejected as LCV-054. `offset` is not excluded
by preference, it is unspellable, and a word may never be the way a new tool
enters the product.

**`z` and a bare `zoom` need no mechanism at all.** They stay `Unknown`, and
after LCV-148 that is a free, local `Unknown command: "z"` under every setting.
`z` is deliberately **not** mapped to `ZoomKind::Extents`: in R14 `Z` opens the
ZOOM command and *prompts* for an option, so mapping it would silently move the
operator's view to something they did not ask for. Reserved by inaction is the
correct and cheapest answer. See §Open questions 1, which this dissolved.

## Scope

- `src/cmdline/parse.rs::tool_alias`: fourteen new rows, and the rustdoc on
  `tool_alias` updated to list the word axis and to keep saying that the letter
  axis is closed.
- `src/cmdline/parse.rs::tests`: the rows that move from
  `rejects_malformed_input_as_unknown` into `parses_tool_aliases`, and the new
  table and case-insensitivity assertions.
- One integration test that the words reach the **tool**, not only the parser.
- `CHANGELOG.md` `[Unreleased]` — written by `demand-manager`, not by the
  implementer.

That is the whole change. **No ADR is needed**: ADR 0003 amendment (2) already
landed §A2a, which states the word axis is open and that a row is a row.

## Out of scope

- **New letter abbreviations.** ADR 0003 §A2a: the letter axis is **closed** —
  `l p r c a s t e m` is the set and it does not grow, because a letter is a
  scarce one-keystroke resource shared with the bare tool-activation keys.
  **`e` is Delete, not Extend**, a v2 decision that deliberately departs from
  v1 (where `e` *is* `extend` — §A2's contrary claim is the factual error §A2a
  corrects; the reassignment stands either way). `d` and `x` stay unbound.
  `src/ui/shortcuts.rs::alias_table_agrees_with_tool_keys` is the test that
  fails if this is violated. **This demand adds words, not letters.**
- **Turning ADR 0007 §D9 rule 4 off.** That is
  [LCV-148](LCV-148-unrecognised-lines-stay-local.md)'s entire job and it lands
  **before** this demand. Nothing here touches `src/agent/classifier.rs` — see
  AC 9.
- **Any mechanism for holding specific words back from the model** — no
  `is_reserved_word` predicate, no `CommandInput::Incomplete` / `Usage` variant,
  no new classifier clause. ADR 0007 §D9a forbids it in these terms: it would be
  deleted by LCV-148, which now precedes this demand anyway.
- **Documenting the new words in the F1 shortcuts dialog.** ADR 0003 §A2a is
  explicit: `SHORTCUT_GROUPS` is a table of **key bindings**, `src/ui/dialogs.rs`
  is at 299 of 300 implementation LOC, and its body clears the clip by 8.00pt
  while one row costs 21.00pt (LCV-134) — one alias row overflows the dialog. A
  discoverable command vocabulary is its own demand, lands after LCV-134, and
  decides its own presentation. Until then the words are documented in
  `CHANGELOG.md` and in the rustdoc on `tool_alias`.
- **A modal ZOOM prompt** (`Z` then `E`), or any other modal command prompt.
  That is a state machine the command line does not have.
- **Partial / prefix matching, fuzzy matching, or a did-you-mean suggester.**
  Exact words only.
- **A better error message for `zoom` or `z`.** See §Open questions 2 — closed,
  rejected for this demand.
- **Any new tool.** And no word may be added that does not name an existing
  `ToolKind` / `ToggleKind` / `ZoomKind` variant.
- **Touching the `:` / `/ai` prefixes.** `:line` still goes to the model, which
  is precedence rule 2 and the thing this demand could plausibly break — AC 8.

## Acceptance criteria

1. **Every word in the set activates exactly the tool the table above names.**
   A table test asserts `parse(word) == CommandInput::Tool(kind)` for all
   fifteen rows, with the `kind` written out per row — not merely "is a
   `Tool(_)`", which a swapped mapping would survive.

2. **The words are case-insensitive, like every other alias.** `LINE`, `Line`
   and `line` all parse to `Tool(Line)`; the same for at least `ERASE`, `Del`
   and `RECTANGLE`. Surrounding whitespace is still trimmed (`"  circle  "`).

3. **No new letters, and `e` is still Delete.** `parse("x")` and `parse("d")`
   stay `Unknown`, `parse("e")` stays `Tool(Delete)`, and
   `src/ui/shortcuts.rs::alias_table_agrees_with_tool_keys` is green
   **unmodified**.

4. **No new word shadows a toggle, a zoom form, a coordinate or a distance.**
   `tool_alias` is consulted **first** in `parse`, so a careless row could
   silently steal an existing line. Asserted unchanged: `snap`, `grid`, `ortho`
   → `Toggle(_)`; `ze`, `zoom in`, `zoom out`, `zoom extents` → `Zoom(_)`;
   `50,25` → `Point`; `@10,0` → `Relative`; `37.5` → `Distance`; `""` →
   `Empty`; and `zoom`, `zoom sideways`, `z`, `1,2,3`, `nan` → `Unknown` with
   the payload unchanged.

5. **The words reach the tool, not only the parser.** An integration test
   submits each word through the real command line
   (`harness::submit_command`) and asserts
   `app.tool_manager.active_tool_name()` is the expected tool and that
   `app.command_feedback` carries no `Unknown command` text. At minimum:
   `LINE` → `LINE`, `erase` / `del` / `delete` → the Delete tool, `extend` →
   the Extend tool, `pline` → the Polyline tool. A parser-only change fails
   this.

6. **`extend` costs one alias row and no wiring.** Verified at refinement:
   `ToolKind::Extend` exists and `src/tools/mod.rs::make` already maps it to
   `ExtendTool`, so the word is reachable with a table row alone. **If that
   turns out to be wrong** — if making `extend` work needs anything beyond the
   alias table — drop `extend` from the set, say so in the commit message, and
   hand the reason to `product-owner`. Do not add wiring to make a word work.

7. **Raw input still wins over the grammar.** `tests/lcv112.rs::raw_input_is_not_parsed`
   is green **unmodified**: typing `line` at TEXT's string prompt still commits
   the word "line" as geometry and does not switch tools. This is ADR 0007 §D9
   rule 1 and it becomes more load-bearing the moment `line` means something.

8. **The escape hatch still wins over the grammar.** `:line` is still sent to
   the model when a key is configured, even though `line` now parses as CAD.
   `src/agent/classifier.rs::tests::a_prefixed_cad_command_is_a_prompt` is green
   **unmodified** — it already carries `line` as a row, and after this demand it
   becomes the honest control it always claimed to be: `line` is CAD, `:line` is
   a prompt. This is the LCV-124 control that replaces the old, now-inverted
   "`lien` still reaches the agent".

9. **`src/agent/classifier.rs` is not modified.** The grammar's knowledge lives
   in one place (ADR 0003 §A2a). `git diff --stat` for the shipped change names
   only `src/cmdline/parse.rs`, the new/edited test file(s), `CHANGELOG.md` and
   the docs.

10. **The assertions this demand inverts are moved, not deleted.**
    `src/cmdline/parse.rs::tests::rejects_malformed_input_as_unknown` currently
    asserts `parse("line")` and `parse("del")` are `Unknown`; both rows move into
    the AC 1 table with their new verdicts, and the `x`, `d`, `zoom`,
    `zoom sideways` rows stay where they are as the controls of AC 3 and AC 4.
    No `Unknown` assertion is silently dropped.

11. **The vocabulary is documented where §A2a says it is documented.** The
    rustdoc on `tool_alias` lists the word rows and keeps the sentence that the
    letter axis is closed and `e` is Delete; `demand-manager` adds one
    user-visible `[Unreleased]` clause naming the words an operator can now
    type. No F1 dialog change (see §Out of scope).

12. **Gates and caps.** `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and
    `cargo test --all --no-fail-fast` (ADR 0008) all exit 0. The implementation
    LOC of `src/cmdline/parse.rs` is measured with ADR 0004's `awk` recipe and
    reported. `src/cmdline/` stays free of `egui` / `eframe` / `rfd`.

## Expected tests

- **Unit / AC 1** — the fifteen-row table in `src/cmdline/parse.rs::tests`,
  each row naming its `ToolKind`. Fold it into `parses_tool_aliases` rather than
  starting a second alias test.
- **Unit / AC 2** — extend `aliases_are_case_insensitive` with at least `LINE`,
  `Line`, `ERASE`, `Del`, `RECTANGLE` and a whitespace-padded row.
- **Unit / AC 3** — the `x` / `d` / `e` rows, kept where they are;
  `src/ui/shortcuts.rs::alias_table_agrees_with_tool_keys` run unmodified.
- **Unit / AC 4** — the non-shadowing table: every toggle, every zoom form,
  point, relative, distance, empty, and the four `Unknown` controls, asserted
  against the same `parse` in the same test run.
- **Integration / AC 5** — a new `tests/lcv131_command_words.rs` (or a section
  of the existing command-line integration binary) driving
  `harness::submit_command` with `LINE`, `pline`, `erase`, `del`, `delete`,
  `extend`, and asserting the active tool by name plus an empty/`Unknown`-free
  `command_feedback`. ADR 0002 §A2: `App::default()`, never `App::new()`.
- **Integration / AC 7** — `tests/lcv112.rs::raw_input_is_not_parsed`, run
  unmodified.
- **Unit / AC 8** — `a_prefixed_cad_command_is_a_prompt`, run unmodified.
- **Mutation checks the implementer runs first, in a scratch `git worktree`,
  reporting each result by name**:
  (a) delete the `erase` row → AC 1 and AC 5 fail by name;
  (b) map `line` to `ToolKind::Polyline` → AC 1 fails by name (a table that only
  asserted `Tool(_)` would survive this — that is why AC 1 names the kind);
  (c) add `"x" => ToolKind::Extend` → AC 3 fails by name;
  (d) add `"snap" => ToolKind::Select` → AC 4 fails by name, proving the
  shadowing guard is real;
  (e) match the words against the raw `trimmed` instead of the lowercased
  `lower` → AC 2 fails by name;
  (f) make `parse` answer `Tool(Line)` for `line` but leave `submit`'s dispatch
  untouched in a way that never reaches `tools::make` → AC 5 fails where AC 1
  passes.
- **[manual] smoke** — with **no** API key configured (so nothing can leave the
  machine whatever happens): type `LINE`, click two points, see a line; type
  `CIRCLE`, draw one; select it, type `ERASE`, confirm it is gone and one
  `Ctrl+Z` brings it back; type `EXTEND` and confirm the Extend tool becomes
  active with its own prompt; type `zoom` and confirm it still answers
  `Unknown command: "zoom"` locally. Then configure a key and type `:line` —
  confirm it still reaches the model rather than starting the LINE tool.

## Test hygiene (mandatory)

LCV-124's §Test hygiene applies verbatim:

- **Bound every source scan** to the implementation section (slice at the offset
  of the bare `#[cfg(test)]` at column 0) and **build every needle with
  `concat!`**. Canonical correct example: `guard_is_runtime_not_cfg` at
  `src/io/dialogs.rs:179-194`. Every scan must be **shown to discriminate**.
- **No CI test may reach a real endpoint** — mockito only, and this demand
  should need no HTTP at all: drive turns through LCV-123's `arm_turn` seam.
- ADR 0002 §A2 / §A4 rule 1; ADR 0006 and ADR 0007 §D10: **no test may cause a
  settings write to a real per-user path**, and a test that injects a
  `settings_path` must not also set a real API key.

This demand should need no scan at all — its claims are behavioural and are
asserted by calling `parse` and by driving the real command line. If a scan
appears in the implementation, it is answering the wrong question.

## Open questions

1. ~~**Mechanism for keeping `z`, `zoom` and `zoom sideways` inside the CAD
   grammar** — a predicate, a new `CommandInput` variant, or accepting the
   risk.~~ **Closed: LCV-148 dissolved it.** Verified against the code, not
   assumed: `zoom_form` returns `None` for `zoom` and `zoom sideways`, so both
   fall through `parse` to `Unknown`, as does `z`; once ADR 0007 §D9 rule 4 is
   `Unknown → Route::Cad, always`, every one of them — and `lien` with them —
   stays CAD with **no predicate, no new enum variant and no classifier
   clause**. Nothing is needed, so nothing is built. ADR 0007 §D9a says the same
   thing normatively and forbids building it. This removes an `architect` round
   trip and a public-type change from the plan.
2. ~~**Does the operator get a better message than `Unknown command: "zoom"`?**~~
   **Closed: no, and not in this demand.** It would need the rejected option (b)
   — a new public `CommandInput` variant, an ADR-level change under §A2a — for a
   cosmetic gain on a line that is now free and local. If operators report
   confusion about `zoom` specifically, that is its own demand.
3. ~~**Is `delete` wanted alongside `erase`?**~~ **Closed: yes, and `del` too.**
   The approved set is the fifteen words in §The product call. `offset` was
   considered and excluded — see §The product call for why it is unspellable
   rather than unwanted.

*(No open questions remain.)*

## Notes

- Origin: `implementer-rust` and `reviewer-rust` independently, 2026-09-13,
  during LCV-124 — both found that its expected-tests table listed `"z"` among
  the CAD rows when `parse("z")` is `Unknown("z")`. `product-owner` widened it
  after finding that every full command name is in the same position.
  `src/agent/classifier.rs::tests::z_is_not_a_zoom_word` documents the `z` case
  correctly by name, and `src/cmdline/parse.rs::tests::rejects_malformed_input_as_unknown`
  already asserts `parse("line")` and `parse("del")` are `Unknown` — the
  behaviour is known and pinned; what was missing is the judgement that it is
  wrong.
- **The demand was split.** The cost half — *an unrecognised line reaches a
  language model* — is [LCV-148](LCV-148-unrecognised-lines-stay-local.md),
  which lands first. This demand is the grammar half and nothing else. Because
  LCV-148 goes first, the interim window ADR 0007 §D9a describes (in which `z`
  and `zoom` still reach the model with a key configured) never opens.
- **Two claims in this demand's earlier body were true when written and are now
  false**, and are recorded here so a reader of the git history is not misled:
  *"What this demand does not claim: that rule 4 is wrong. Free-text-to-agent
  stays"*, and the citation of ADR 0007 §D9 as *"rule 4 is unchanged"*. ADR 0007
  amendment (5), 2026-09-13, changed rule 4. A third is obsolete rather than
  false: *"ADR 0003: an amendment, written by `architect`"* — amendment (2)
  landed §A2a in the same commit (`300c766`), the word axis is already open, and
  **no ADR work remains for this demand**.
- Normative: [ADR 0003](../../adr/0003-command-line-input-contract.md) §A2a (the
  two axes — variants are the contract, letters closed, words open; and where a
  new alias is documented), [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md)
  §D9 rules 1–3 and §D9a's consequence paragraph, `docs/product/README.md`
  principle 1 ("prefer command line and keyboard-first flows"), LCV-110,
  LCV-111, LCV-124, LCV-148.
- Priority: **not urgent, and smaller than it looks now.** After LCV-148 nothing
  is at stake but a wrong error message on the most ordinary keystroke sequence
  in the product — fired at the operator who knows AutoCAD best. It is fourteen
  table rows and their tests.
