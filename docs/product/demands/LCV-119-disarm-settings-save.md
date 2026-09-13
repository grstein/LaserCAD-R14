# LCV-119 — Disarm Settings persistence outside the app binary

- **Status**: Draft
- **Phase**: 11
- **Depends on**: none (related precedent: LCV-118, ADR 0005)
- **Suggested agent**: architect
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

`Settings::save()` (`src/io/settings.rs:162`) resolves the real platform
config directory through `directories::ProjectDirs::from("", "", "lasercad")`
(`src/io/settings.rs:240`, `platform_path()`) with no test-mode guard of any
kind, and writes to it via `save_to` (`src/io/settings.rs:205`). `action_open`
and `action_open_path` in `src/io/file_actions.rs` (lines 103 and 172) both end
with an unconditional `let _ = app.settings.save();`; a third call site sits at
line 213. A test that calls either function — directly, or by driving a code
path that reaches it, exactly the shape LCV-118 was written to guard against —
would silently overwrite the developer's own `~/.config/lasercad/settings.json`
(stored API key included) with a test `Settings::default()`.

The hazard predates LCV-114 and dates to LCV-062 (`New / Open / Save / Save As
/ Exit actions`), which is where `action_open` was first written. Three
separate implementers have since avoided it by discipline alone: `settings.rs`
already documents a path-injected `save_to(settings, path)` used by its own
tests, and `src/io/file_actions.rs` carries source-scan tests (around lines
280–346) asserting that no test calls `action_open` / `action_open_path`
directly, rather than a real test that exercises them safely. That is coverage
bought by a hand-written scan instead of a real test, on two acceptance
criteria this milestone — the same trade LCV-118's own review called out as
already having cost real coverage once.

This is the same shape as the problem [ADR 0005](../../adr/0005-native-dialogs-disarmed-by-default.md)
solved for native file dialogs — a real OS/filesystem side effect reachable
from product logic that a test can trip into — but it is worse to detect: a
disarmed native dialog panics loudly and immediately; an unguarded
`settings.save()` writes silently and only surfaces later, on the developer's
next real run of the app, as a vanished API key or a reset recent-files list.
ADR 0005's own premise was that discipline alone had already failed once for
the dialogs; here it has not failed yet only because every implementer so far
has been careful enough to write a scan instead of a test.

## Scope

*(deferred to `product-owner` during refinement, once the design question
below is settled)*

## Out of scope

*(deferred to `product-owner` during refinement)*

## Acceptance criteria

*(deferred to `product-owner` during refinement)*

## Expected tests

*(deferred to `product-owner` during refinement)*

## Open questions

- **Does this extend ADR 0005, or does it need its own ADR?** ADR 0005 disarms
  a `static AtomicBool` at the `rfd` boundary, armed once by `crate::run()`.
  Settings persistence is a different boundary (`std::fs` + `directories`, not
  `rfd`) with a different shape of hazard (silent overwrite of real user state,
  not a blocking hang), so the same mechanism may not transplant cleanly —
  for instance, `Settings::save()` already has a path-injected sibling
  (`save_to`) that a real test can call instead of the guarded one, which
  `rfd`'s wrappers do not have an equivalent of. `architect` must be consulted
  before refinement proceeds; do not let `product-owner` or `implementer-rust`
  improvise the mechanism.
- Should the guard live on `Settings::save()` itself (mirroring ADR 0005's
  choice to guard at the `rfd` boundary rather than on `App`), or is the
  existing `save_to(settings, path)` path-injection already the correct shape,
  with the real gap being that `action_open` / `action_open_path` /
  `action_new` should be routed to take an explicit path in tests the way
  `action_open_path` already does for opening?

## Notes

- Origin: the LCV-114 review, which flagged `let _ = app.settings.save();` at
  `src/io/file_actions.rs:103` and `:172` (a third call site at `:213`) as
  reachable from a test with no guard.
- Related: [ADR 0005](../../adr/0005-native-dialogs-disarmed-by-default.md)
  (native dialogs disarmed by default) and LCV-118 (which implemented it) are
  the precedent for the general shape of the fix, not necessarily its
  mechanism — see Open questions.
- `src/io/settings.rs` already separates `Settings::save()` (real platform
  path) from `pub(crate) fn save_to(settings: &Settings, path: &Path)`
  (path-injected, used by `settings.rs`'s own tests at lines 332–363). Any fix
  should build on that existing seam rather than duplicate it.
