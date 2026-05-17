# Product Owner Agent — Operating Instructions

The `product-owner` agent (defined in `.claude/agents/product-owner.md`) refines `Draft` demands into `Ready` demands so that `implementer-rust` can ship them without asking product questions.

This file defines the demand format, the state machine, and the Ready bar.

## Demand format

Each demand is one Markdown file under `docs/product/demands/` named `LCV-NNN-<kebab-title>.md`. The file starts with metadata, then the body. The `Status:` and `Implementation:` lines are written by `demand-manager`; everything else is written by `product-owner`.

```markdown
# LCV-NNN — <Title>

- **Status**: Draft
- **Phase**: <0..9>
- **Depends on**: <LCV-IDs or "none">
- **Suggested agent**: implementer-rust (or architect if design-heavy)
- **Suggested model**: sonnet | opus
- **Implementation**: <agent + task #, filled when In Progress; commit hash appended when Done>

## Problem

<one paragraph: the user pain or capability gap. Tie to laser-cutting workflows.>

## Scope

<bulleted; what the demand covers — and only that>

## Out of scope

<bulleted; what this demand explicitly does NOT do. Prevents creep.>

## Acceptance criteria

<numbered list; each item objective and testable>

## Expected tests

<bulleted; unit / integration / manual smoke. At least one per acceptance criterion.>

## Open questions

<bulleted; questions for the user. Empty when Ready.>

## Notes

<links to ADRs, v1 reference code paths, gotchas>
```

## State machine

```
Draft → Needs Refinement → Ready → In Progress → Done
                                              ↘ Rejected
                                              ↘ Blocked (transient)
```

- **Draft**: created, body empty or stub. Awaiting refinement.
- **Needs Refinement**: `product-owner` is shaping it; open questions remain.
- **Ready**: refined, acceptance criteria objective and testable, no open questions, deps known. `implementer-rust` can claim.
- **In Progress**: `implementer-rust` is working on it. `Implementation:` line names the agent + task.
- **Done**: shipped, tested, committed. `Implementation:` line carries the commit hash. `CHANGELOG.md` updated if user-visible.
- **Rejected**: not shipping. Reason recorded in the body.
- **Blocked**: in-flight but waiting on a dependency, a user answer, or an upstream tool. Reason recorded.

State transitions are made by `demand-manager` only. Other agents file a `TaskCreate` requesting the transition.

## The Ready bar

A demand can be marked `Ready` if and only if all of the following are true:

1. **Problem is tied to a user outcome** (e.g., "the user cannot snap to a circle center while drawing a tangent line").
2. **Scope is bounded.** Every line under "Scope" is a single observable behavior. No "while we're at it" items.
3. **Out of scope is explicit** when scope creep is plausible.
4. **Acceptance criteria are objective and testable.** "Looks nice" is not a criterion; "When the cursor is within 8 px of an endpoint, a square marker is rendered at that endpoint" is.
5. **Expected tests** are listed, with at least one test per acceptance criterion. Unit / integration / manual smoke are all allowed; manual must explicitly say "manual" and describe the steps.
6. **Open questions is empty.**
7. **Depends-on demands are all `Done` or `Ready`** (so the chain is clear). A demand may sit `Ready` while its dependencies finish.

## Anti-patterns the `product-owner` rejects

- **Decorative UI work** without a user-outcome tie ("make the toolbar prettier").
- **"For the future"** features (e.g., "expose a hook for plugins") with no current consumer.
- **Vague acceptance criteria** ("snap should feel responsive").
- **Scope creep within one demand** ("Add Circle tool — also fix Line tool while we're there").
- **Speculative APIs** ("add a `Tool::serialize()` method in case we want to save tool state someday").

When in doubt, **shrink or reject**.

## Splitting a demand

If a demand grows past one well-named title, split it. Procedure:

1. `product-owner` proposes the split in the demand body under a `## Split proposal` section.
2. `product-owner` files a TaskCreate for `demand-manager` to:
   - Create the child demands as `Draft`.
   - Mark the parent `Superseded by LCV-NNN, LCV-NNN+1, ...` and move it to a "Superseded" section in `backlog.md`.
3. `product-owner` refines each child demand normally.

## Tying back to PLAN.md

The `project-manager` mirrors demand status into the PLAN.md table. `demand-manager` is the source of truth for the demand file and `backlog.md`; the PM keeps PLAN.md in sync.
