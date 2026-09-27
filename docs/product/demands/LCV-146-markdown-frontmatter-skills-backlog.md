# LCV-146 - Future Markdown and frontmatter agent skills

- **Status**: Draft
- **Phase**: 12
- **Depends on**: LCV-143
- **Suggested agent**: product-owner
- **Suggested model**: opus
- **Implementation**: -
- **Scheduling**: Deferred; not part of the current implementation queue.

## Problem

Future reusable CAD instructions should be expressible as Markdown with
frontmatter rather than repeated large prompts. There is not yet a confirmed
runtime selection/trust contract or a first implemented skill consumer.

## Scope

Record the intended artifact format and future product questions only.
Frontmatter carries metadata; the Markdown body carries model instructions.

## Out of scope

Current loader, discovery, parser dependency, execution hooks, automatic
activation, installed skill files, UI manager, tool permissions or runtime
extension points built speculatively.

## Acceptance criteria

1. This demand stays explicitly deferred and outside the current execution queue.
2. A future refinement identifies a concrete CAD task that benefits beyond the editable system prompt.
3. Before Ready, define metadata, discovery, explicit selection, prompt precedence, trust and size limits.
4. Skill text cannot grant tools, filesystem access or capture permission denied by the harness.
5. This planning registration creates no loader, new dependency, executable hook or current skill tool.

## Expected tests

- Current documentation review confirms deferred scheduling and no runtime changes.
- Future refinement must specify malformed-frontmatter, instruction-selection, precedence, resource-bound and permission tests before implementation.

## Open questions

- Which first user task needs a reusable skill?
- Required metadata beyond candidate `name` and `description`.
- Approved discovery location and explicit enablement/selection.
- Prompt precedence, untrusted-content handling and size limits.

## Notes

These are application-agent skills, not repository development-agent
definitions. LCV-143 provides editable instructions now; this demand must not
pre-build a plugin system.
