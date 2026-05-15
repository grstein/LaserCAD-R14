# LCD-008 - Adopt English-Only Convention in CLAUDE.md and Agent Surface

Status: Ready
Priority: P2
Owner: Product Owner
Implementation: Unassigned

## Problem

`CLAUDE.md` is the entry point for every coding agent that touches this repository. It is itself in pt-BR and contains the line "Documentação de produto está em pt-BR; código e comentários em inglês ou pt-BR conforme arquivo", which sanctions a mixed-language repository. Without changing this rule, the work done in LCD-006 and LCD-007 will be slowly reversed by future contributions and agent edits.

The PO and the user already agreed in conversation that the language of versioned artifacts must be English; human conversation with the PO can stay in pt-BR.

## Smallest Useful Outcome

`CLAUDE.md` is in English and states one unambiguous rule: every versioned artifact (code, comments, documentation, commit messages, file names) is English-only. Human conversation outside the repository is unaffected. Cross-references to deleted specs are removed.

## Scope

In:

- Translate `CLAUDE.md` in its entirety into English while keeping every existing rule, ordered step, and file reference intact.
- Replace the line "Documentação de produto está em pt-BR; código e comentários em inglês ou pt-BR conforme arquivo" with a clear English-only convention for versioned artifacts. Suggested wording: "All versioned artifacts (source, comments, documentation, commit messages, file names) are written in English. Conversation with the product owner can happen in any language; the repository remains English-only."
- Remove the reference to `specs/_conventions/state-contract.md` from `CLAUDE.md` and point readers at `src/app/state.ts` and `src/app/event-bus.ts` as the new sources of truth for state and event bus contracts.
- Before LCD-007 deletes `specs/_conventions/state-contract.md`, audit it for any rule that is not already captured in `CLAUDE.md` (Estado e mutação section). If anything is missing, lift it into `CLAUDE.md` before deletion. Most rules already exist there; this is a check, not a transplant.
- Add a single line to `docs/product/README.md` (under "Non-Negotiables" or "Sources Of Truth") stating that the repository is English-only. Optional but cheap.

Out:

- Translating commit history.
- Translating pt-BR in closed GitHub issues, discussions, or external comms.
- Adding an ESLint rule, a pre-commit hook, or any automated language linter.
- Editing source files. Implementation agents touching `CLAUDE.md` should not touch `src/`.
- Creating a CONTRIBUTING.md or expanded contributor guide.

## Acceptance Criteria

- `CLAUDE.md` reads as English with no pt-BR sentences remaining.
- The English-only rule appears explicitly near the top of `CLAUDE.md`, framed as a project-wide convention for versioned artifacts.
- No reference to `specs/_conventions/state-contract.md` (or any other path inside `specs/`) survives in `CLAUDE.md`.
- `docs/product/README.md` either (a) gains the English-only line, or (b) is left untouched if the implementer judges CLAUDE.md sufficient. Either choice is acceptable as long as the project documents the rule somewhere readable.
- `npm run typecheck`, `npm run lint`, and `npm test` pass unchanged (zero `.ts` files were edited).
- Manual review confirms `AGENTS.md`, `docs/product/product-owner-agent.md`, and `docs/product/README.md` do not contradict the new rule.

## Expected Tests

- No code tests added.
- Manual smoke:
  - Open `CLAUDE.md` and read top to bottom; confirm every rule from the prior pt-BR version is present in English.
  - Grep for `pt-BR|ção|ões|você` in `CLAUDE.md`; result must be empty.
  - Confirm `AGENTS.md` still aligns: implementation agents read CLAUDE.md and produce English artifacts.

## Notes For Implementation Agents

- Relevant docs/files:
  - `CLAUDE.md`
  - `docs/product/README.md`
  - `AGENTS.md`
  - `docs/product/product-owner-agent.md`
  - `src/app/state.ts` and `src/app/event-bus.ts` (link targets that replace the deleted spec)
- Risks:
  - Order with LCD-007: if LCD-008 runs first, leave the `specs/_conventions/state-contract.md` reference in CLAUDE.md alone until LCD-007 does the deletion in the same PR; otherwise the file still exists and the link is valid. Cleanest option is to land LCD-007 and LCD-008 close together or have LCD-008 own the CLAUDE.md reference removal entirely.
  - Translation fidelity: the existing CLAUDE.md captures architectural rules (kernel purity, unit contract, state contract, bootstrap order, runtime split, SVG export rules). Translate each rule faithfully; do not reinterpret or shrink them.
- Product decisions:
  - English-only applies to versioned artifacts only. Conversation with the PO in pt-BR is encouraged and unaffected.
  - No automated language linting. Reviewer judgment plus CLAUDE.md is the enforcement surface.

