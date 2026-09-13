# LCV-131 — Typing `LINE` sends the word "line" to a language model

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-124 (the classifier this changes the input to)
- **Suggested agent**: architect (it amends a frozen ADR decision) → implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

LCV-124's classifier rule 4 says: a line `cmdline::parse` does not recognise
goes to the agent when an API key is configured. The demand's §Risks named the
hazard — `lien`, a fat-fingered `line`, becomes a paid round trip — accepted it,
and listed four mitigations. That reasoning holds for a typo.

It does not hold for what refinement actually found. The v2 grammar accepts
**only the single-letter aliases** `l p r c a s t e m` plus `text`
(`src/cmdline/parse.rs::tool_alias`, frozen by ADR 0003 §A2 as "exactly the v1
set"). Every **full AutoCAD command name** is therefore `Unknown`:

```
parse("line")   -> Unknown("line")      // asserted today at src/cmdline/parse.rs:296
parse("circle") -> Unknown("circle")
parse("erase")  -> Unknown("erase")
parse("zoom")   -> Unknown("zoom")      // asserted at :289 — the keyword with no argument
parse("z")      -> Unknown("z")
```

In AutoCAD R14 the full name is the **primary** form and the letter is the
abbreviation: you type `LINE` and `L` is the shortcut, `ZOOM` and `Z`. This
product is a KISS clone of R14. So the most ordinary thing an R14 operator can
type at the command line — the command's own name — is, with a key configured:

- a paid model call with seconds of latency instead of a tool activation;
- handed to a model that holds `create_line` in its tool set and will very
  plausibly *draw something* in answer to the prompt `line`;
- landed on the operator's live drawing (ADR 0007 §D2), undoable but unasked
  for.

LCV-124's four mitigations all survive — the send is loud, the panel opens by
itself, one `Ctrl+Z` reverses it, the step budget caps the damage — and they are
the right answer for `lien`. They are the wrong answer for `line`, because
`line` is not a mistake. It is the operator using the product correctly.

`z` is the same shape one step smaller, and is what started this: `z` is the
standard AutoCAD zoom abbreviation, and both the LCV-124 implementer and its
reviewer independently found that the demand's expected-tests table listed `z`
among the CAD cases when it is not one.

## The product call

**The CAD grammar should own the words that name CAD commands.** Two parts, in
priority order:

1. **The full command names become aliases.** This is not a new feature — it
   restores the *primary* spelling of every command this product already ships,
   and it is the half that actually improves the command line rather than merely
   making it cheaper. Candidate set, one per existing `ToolKind`: `line`,
   `polyline`, `rect`, `circle`, `arc`, `select`, `trim`, `extend`, `move`,
   `erase` (and `delete` as its synonym, since the v2 tool is named Delete),
   `text` (already present). No new tool, no new behaviour: `line` does exactly
   what `l` does today.

2. **`z` and a malformed `zoom` stay CAD.** `z` is **reserved, not redefined**:
   in R14 `Z` opens the ZOOM command and *prompts* for an option — it does not
   zoom extents — so mapping `z` to `ZoomKind::Extents` would silently move the
   operator's view to something they did not ask for. Reserved means the line
   stays in the CAD grammar and gets a free, local answer instead of a model
   call. Same for `zoom` with no argument and `zoom sideways`, which are CAD
   syntax errors, not prompts.

What this demand does **not** claim: that rule 4 is wrong. Free-text-to-agent
stays. The escape hatches (`:` and `/ai`) stay absolute. `lien` still reaches
the model and that is still accepted.

## Scope

- `src/cmdline/parse.rs`: the alias table gains the full command names.
- `src/cmdline/`: whatever the smallest correct mechanism is for keeping `z`,
  `zoom` and `zoom <bad-arg>` inside the CAD grammar (see §Open questions —
  this is the part that needs `architect`).
- `src/agent/classifier.rs`: at most one clause, and only if §Open questions
  resolves that way. The grammar's knowledge must not be duplicated in the
  classifier.
- ADR 0003: an amendment, written by `architect`, recording that §A2's frozen
  alias set is widened and why.

## Out of scope

- **New letter abbreviations.** ADR 0003 §A2 and the `src/ui/shortcuts.rs`
  cross-check test are explicit: `EXTEND` has no letter alias, `e` is Delete not
  Extend, `d` and `x` are nothing. This demand adds **words**, not letters, and
  touches none of that.
- **A modal ZOOM prompt** (`Z` then `E`), or any other modal command prompt.
  That is a state machine the command line does not have.
- **Partial / prefix matching, fuzzy matching, or a did-you-mean suggester.**
  Exact words only.
- **Turning rule 4 off**, or adding a setting for it. LCV-124 §Risks already
  records that lever and says not to pre-build it.
- **Any new tool, or reaching `ToolKind::Extend` from the command line if it is
  not already reachable.** If `extend` as a word turns out to need wiring
  beyond the alias table, drop it from the set and say so.
- **Touching the `:` / `/ai` prefixes.** `:line` still goes to the agent.

## Acceptance criteria

*(Draft — to be completed after §Open questions is answered. The two that are
already firm:)*

1. **With a key configured, no word naming a shipped command reaches the
   agent.** A table test over the full candidate set asserts
   `classify(word, true) == Route::Cad` for every one of them, and the same for
   `z`, `zoom`, and `zoom sideways`. The same table with `agent_available =
   false` must be unchanged, so the availability flag is proved not to matter
   for these rows.
2. **`lien` still reaches the agent.** The control: LCV-124's accepted hazard is
   not quietly closed by this demand, and the test says so by name.

## Expected tests

*(To be completed at refinement. The table test above, mutation checks, and the
scan hygiene of LCV-124 §Test hygiene apply verbatim.)*

## Open questions

1. **Mechanism for part 2.** `z` and a bare `zoom` must stay CAD without being
   given a meaning they do not have. Options: (a) a pure predicate exported from
   `src/cmdline/` — `is_reserved_word(&str)` or `grammar_owns_first_token(&str)`
   — that the classifier consults, keeping grammar knowledge in the grammar at
   the cost of one clause in `classify`; (b) a new `CommandInput` variant
   (`Incomplete`/`Usage`) so `parse` answers "mine, but wrong" instead of
   `Unknown`, which is more honest and more expensive, and changes an ADR 0003
   public type; (c) do part 1 only and leave `z` and `zoom` as accepted risks.
   **`architect` decides**; the product requirement is only that the grammar's
   knowledge lives in one place.
2. **Does the operator get a better message than `Unknown command: "zoom"`?**
   Product view: out of scope for now, because it needs option (b). Confirm.
3. **Is `delete` wanted alongside `erase`?** R14 says `ERASE`; the v2 tool is
   named Delete and the menu says Delete. Product view: ship both, they cost one
   line each. Confirm with the user.

## Notes

- Origin: `implementer-rust` and `reviewer-rust` independently, 2026-09-13,
  during LCV-124 — both found that its expected-tests table listed `"z"` among
  the CAD rows when `parse("z")` is `Unknown("z")`. `product-owner` widened it
  after finding that every full command name is in the same position.
  `src/agent/classifier.rs::tests` already documents the `z` case correctly, by
  name, and `src/cmdline/parse.rs:296` already asserts `parse("line")` is
  `Unknown` — the behaviour is known and pinned; what was missing is the
  judgement that it is wrong.
- LCV-124's §Risks carries the interim record of this hazard with `z`, `zoom`
  and `line` named, so the behaviour is documented while this demand is
  unscheduled.
- Normative: [ADR 0003](../../adr/0003-command-line-input-contract.md) §A2 (the
  frozen alias set — this demand amends it),
  [ADR 0007](../../adr/0007-agent-turn-mutates-the-live-document.md) §D9
  (routing precedence; rule 4 is unchanged), `docs/product/README.md` principle 1
  ("prefer command line and keyboard-first flows"), LCV-110, LCV-111, LCV-124.
- Priority: **not urgent, but higher than it looks.** Nothing is wedged and
  nothing is lost — the cost is a surprise model call and a possible unwanted
  entity, both one `Ctrl+Z` away. But it fires on the most ordinary keystroke
  sequence in the product, and it fires for the operator who knows AutoCAD best.
