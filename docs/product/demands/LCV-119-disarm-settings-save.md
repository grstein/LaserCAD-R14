# LCV-119 — Real user paths are injected: `cargo test` stops writing the developer's config and data directories

- **Status**: Done
- **Phase**: 11
- **Depends on**: none (normative design: [ADR 0006](../../adr/0006-real-user-paths-are-injected.md); precedent: LCV-118, ADR 0005)
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: implementer-rust — 2d81a14 (implementation), e90c3e0 (rework, reviewer-approved), 0e9c9c4 (doc follow-up)

## Problem

**`cargo test` deletes the developer's crash-recovery file today, on every run.**
Reproduced by the architect on this tree at `32334ec`:

```
$ printf '{"probe":"…"}' > ~/.local/share/lasercad/autosave.json
$ cargo test --lib new_document_seeds_bed_from_settings   # 1 passed
$ ls ~/.local/share/lasercad/autosave.json
ls: cannot access '…/autosave.json': No such file or directory
```

`new_document_seeds_bed_from_settings` is an existing, green, entirely
innocuous-looking library test. It calls `action_new`, which ends in
`clear_autosave()` (`src/io/file_actions.rs:53`), which resolves
`directories::ProjectDirs::from("", "", "lasercad")` and unlinks the real file.
An operator who crashes mid-job after running the test suite has no recovery
file to come back to. Nothing warns, nothing fails, and the test passes.

The sibling hazard has not fired yet, but it is one careless test away and it
is worse when it does. `Settings::save()` (`src/io/settings.rs:162`) resolves
the real config directory the same way and writes it with no guard, and **five**
production call sites reach it: `src/io/file_actions.rs:103`, `:172`, `:213`,
`src/app/panels.rs:83` and `src/app/bed_dialog.rs:65`. A test that drives one of
them overwrites `~/.config/lasercad/settings.json` — stored API key, recent
files, default bed — with the `Settings::default()` a test `App` carries. A
disarmed dialog hangs or panics immediately; this succeeds silently and
surfaces days later as a vanished API key.

That hazard has already cost this milestone real coverage. LCV-114 AC 10 and
LCV-115 AC 9 are both paid for with bounded source scans
(`src/io/file_actions.rs::both_open_paths_adopt_the_file_bed_and_leave_the_seed_alone`
and `…_adopt_the_file_preset`), whose own doc comments name the settings write
as the reason a behavioural test could not be written. Three implementers in a
row have avoided the settings hazard by discipline — which is the argument
ADR 0005 already rejected once, and which, as the autosave repro above shows,
has already silently failed for the sibling path.

[ADR 0006](../../adr/0006-real-user-paths-are-injected.md) decides the shape and
is normative for this demand: **a real per-user filesystem location is resolved
exactly once, at boot, and carried as data on `App`. No function below boot
resolves one.** Two points of that decision are load-bearing here and must not
be re-litigated during implementation:

- **The scope is both paths, not settings alone.** `action_open_path` ends in
  `settings.save(); clear_autosave();` and all five file actions call
  `clear_autosave()`. Guarding only settings leaves those functions exactly as
  untestable as they are today, and leaves `cargo test` deleting recovery files.
- **A pathless process writes nothing, and does not panic.** ADR 0005 chose to
  panic because its `None` was a *valid production outcome* ("the user cancelled
  the dialog"), so a quiet fallback would have been indistinguishable from
  correct behaviour and would have turned a hang into a false green. That
  ambiguity does not exist for a write: whether the write happened is directly
  observable by the test that asked for it — point the path at a tempdir and
  read the bytes back — so an absent write is a failed assertion, never a green
  one. Two further reasons point the same way. All five production call sites
  already swallow write failures (`let _ =`, `.ok()`), because an unwritable
  config directory must not abort a CAD session mid-job, so a panic would be
  *louder than the real failure it guards*. And a panic would make `action_new`
  and `action_open_path` panic in tests, which would force the very source scans
  this demand exists to retire to stay exactly where they are.

## Scope

- `directories::ProjectDirs` confined to `src/io/settings.rs` and
  `src/io/autosave.rs`, each behind one `pub(crate)` resolver, called only from
  `App::new()`.
- Two new `App` fields, `settings_path` and `autosave_path`, both
  `Option<PathBuf>`, both `None` in `App::default()`.
- The five unguarded wrappers deleted in favour of the path-injected siblings
  that already exist, reached through three `App` methods in one new file
  `src/app/persist.rs`.
- Every existing call site rewired to those three methods, with no change to
  what is persisted, when, or in what format.
- The two source scans bought with lost coverage replaced by real tempdir-backed
  behavioural tests, keeping their names.

## Out of scope

- **Anything to do with `rfd` or ADR 0005.** `action_open` and `action_save_as`
  still open a native dialog and remain unreachable from a test. No new
  `AtomicBool`, no arming, no disarming. ADR 0006 §"Relationship to ADR 0005"
  is the rule: native OS surface → arm-by-main; filesystem location → inject the
  path.
- **Converting any source scan other than the two named.**
  `both_save_paths_export_in_the_session_preset` and the `App::new` boot scan
  stay as they are; a later demand may convert them.
- **Making `App::new` callable from tests.** ADR 0002 §A2 stands unchanged:
  `App::new` is boot-only, `App::default` is the test constructor.
- **Changing *what* is persisted.** No new `Settings` field, no autosave
  envelope change, no schema bump, no change to the recent-files list or the
  atomic-rename write strategy.
- **Changing autosave timing.** The 800 ms debounce, `sync_dirty`,
  `mark_clean` and the repaint schedule are untouched. The repaint defect is
  LCV-120's.
- **A `SettingsStore` trait, an injected store object, or a `Paths` struct.**
  Three layers where one `Option<PathBuf>` does. ADR 0006 rejected all three;
  the `Paths` struct is explicitly deferred to a fourth path field appearing.
- **A `--config-dir` CLI flag, an `$XDG_CONFIG_HOME` override, or any
  environment-variable hook.** Nothing in this demand is operator-visible.
- **A portable / no-persistence mode as a product feature.** `None` means "this
  process does not persist"; it is a test affordance, not a menu item.

## Acceptance criteria

1. **Resolution is confined to two files.**
   `grep -rn "ProjectDirs" src/` matches only `src/io/settings.rs` and
   `src/io/autosave.rs`, each inside one `pub(crate) fn` returning
   `Option<PathBuf>` (the existing private `platform_path()`, promoted). A third
   occurrence anywhere in `src/` is a review blocker.

2. **Resolution happens once, at boot.** Those two resolvers are called from
   exactly one place: `App::new()` in `src/app/init.rs`. A bounded source scan
   over `App::new`'s body — the technique that file already uses for the bed
   seed, haystack stopped at the test module — asserts both calls are present,
   with a positive control alongside. If `App::new` ever stops filling a field
   the app silently stops persisting, so this scan is the only guard against
   that failure mode.

3. **The two fields.** `App` (in `src/app/mod.rs`) gains
   `pub settings_path: Option<PathBuf>` and `pub autosave_path: Option<PathBuf>`,
   each with a one-or-two-line doc comment saying `None` means this process does
   not persist. `App::default()` (in `src/app/init.rs`) sets both to `None`;
   `App::new()` sets both from the resolvers. Unit test:
   `app_default_has_no_persistence_paths`.

4. **The five unguarded wrappers are gone.** `Settings::load`, `Settings::save`,
   `crate::io::save_autosave`, `crate::io::load_autosave` and
   `crate::io::clear_autosave` no longer exist. `load_from`, `save_to`,
   `save_autosave_to`, `load_autosave_from` and a new
   `pub(crate) fn clear_autosave_at(path: &Path)` are the only persistence API
   in `src/io/`. `src/io/mod.rs`'s `pub use autosave::{…}` line goes with them.
   The now-unreachable `SettingsError::NoPlatformPath` and
   `AutosaveError::NoPlatformPath` variants are deleted too — an error variant
   describing a failure that can no longer occur is a documentation lie.

5. **One persistence surface, one file.** New file `src/app/persist.rs` holds
   exactly three `impl App` methods and nothing else:
   `persist_settings(&self)`, `write_autosave(&self) -> bool`,
   `clear_autosave(&self)`. Each returns the "nothing happened" value
   immediately when its path field is `None`. Besides `App::new`,
   `src/app/persist.rs` is the **only** reader of the two fields:
   `grep -rn "settings_path\|autosave_path" src/` matches only
   `src/app/mod.rs` (the declarations), `src/app/init.rs` (the two
   constructors), `src/app/persist.rs`, and `#[cfg(test)]` modules.

6. **A pathless `App` writes nothing and does not panic.** Test
   `an_injected_path_persists_and_none_persists_nothing`, one tempdir, both
   halves in one test so neither can be read without the other:
   - with `settings_path`/`autosave_path` pointing into the tempdir,
     `persist_settings()` produces a settings file whose parsed contents equal
     `app.settings`, and `write_autosave()` returns `true` and produces an
     autosave file that `load_autosave_from` reads back to the same document —
     the positive control;
   - with the tempdir emptied and both fields `None`, all three methods run,
     `write_autosave()` returns `false`, and `std::fs::read_dir(tempdir)` yields
     **zero** entries.

7. **`clear_autosave` removes an injected file and tolerates its absence.**
   Test: seed a file at `autosave_path`, call `App::clear_autosave`, assert it
   is gone; call it a second time and assert no panic and no error surfaced.

8. **The five settings-write call sites go through `persist_settings`.**
   `src/io/file_actions.rs:103`, `:172`, `:213`, `src/app/panels.rs:83` and
   `src/app/bed_dialog.rs:65` all call `app.persist_settings()`.
   `grep -rn "settings\.save()\|\.settings\.save\b" src/` returns nothing.
   Behaviour at each site is unchanged, including that a failed write is
   swallowed and the session continues.

9. **The five `clear_autosave()` call sites go through `App::clear_autosave`.**
   `src/io/file_actions.rs:53`, `:104`, `:131`, `:173`, `:214`.
   `grep -rn "crate::io::clear_autosave\|io::clear_autosave" src/` returns
   nothing.

10. **The autosave flush goes through `write_autosave`.**
    `src/app/autosave.rs::flush_if_due` reads
    `let wrote = app.write_autosave();` then `record_autosave_outcome(app, wrote);`
    (two statements — a single expression borrows `app` mutably twice). The
    existing bounded source scan in that file, which pins the literal
    `crate::io::save_autosave(&app.document).is_ok()`, is **re-pinned to the new
    literal with its positive control kept**, so the seam still cannot be
    rewired to a hard-coded `true`. `record_autosave_outcome`'s doc comment,
    which today says "there is no path-injected seam on
    `crate::io::save_autosave`", is corrected — after this demand there is one.

11. **Boot behaviour is unchanged.** `App::new()` still loads settings from the
    resolved config path, still recovers an autosaved document from the resolved
    data path when one is present and schema-compatible, still seeds a blank
    document's bed from `settings.default_bed_mm` otherwise (LCV-114 AC 11), and
    still performs **no write of its own**. A `None` path at boot degrades to
    defaults, never to a panic.

12. **Payback 1 — LCV-114 AC 10 becomes behavioural.**
    `both_open_paths_adopt_the_file_bed_and_leave_the_seed_alone` keeps its name
    and gains a real test for the half that is now testable: write an SVG whose
    header declares a bed that is neither the default nor
    `settings.default_bed_mm` into a tempdir; build an `App` with both path
    fields pointing into that tempdir and a distinct `settings.default_bed_mm`;
    call `action_open_path`; assert `app.document.bed_mm` equals the file's bed
    and `app.settings.default_bed_mm` is untouched; assert the settings file now
    exists on disk with the opened path at `recent_files[0]`; assert the seeded
    autosave file in the tempdir is gone. The `action_open` half **stays a
    bounded source scan** — ADR 0005 keeps its native dialog unreachable — and
    its doc comment says that and only that. The retired justification (the
    settings write) must not survive in the comment.

13. **Payback 2 — LCV-115 AC 9 becomes behavioural.**
    `both_open_paths_adopt_the_file_preset`, same split and same name: export a
    document with `Preset::Mark` into the tempdir, `action_open_path` it, assert
    `app.export_preset == Preset::Mark`; the `action_open` half stays a scan for
    the ADR 0005 reason alone.

14. **The defect is closed, and shown to be closed.** **[manual]**, run once by
    the implementer and recorded in the commit message:

    ```
    printf '{"probe":"lcv119"}' > ~/.local/share/lasercad/autosave.json
    cp ~/.config/lasercad/settings.json /tmp/settings.before   # if one exists
    cargo test --all
    cat ~/.local/share/lasercad/autosave.json                  # still the probe
    diff /tmp/settings.before ~/.config/lasercad/settings.json # no difference
    ```

    Both files must be byte-identical afterwards. This is the criterion the
    demand exists for; every structural criterion above is a means to it.

15. **Docs that become false are corrected in the same commit.**
    - `AGENTS.md` §Implementation Rules: the whole **"Pending LCV-119"**
      sentence (line 167, "…until it lands, `Settings::save()` and
      `clear_autosave()` reach the developer's real … so no new test may drive
      `action_new` / `action_open` / `action_open_path` / `action_save_as`")
      is deleted. What replaces it, if anything, is one clause: a test drives
      those actions with its path fields pointing at a tempdir, or leaves them
      `None` and nothing is written. The `action_open` / `action_save_as` ban
      survives for the ADR 0005 reason, which the preceding bullet already
      states.
    - `tests/harness/mod.rs` rule 2 ("Never let the autosave debounce elapse. A
      fired autosave writes to the user's real data directory.") is rewritten:
      an `App` with `autosave_path: None` writes nothing at all, and a test that
      wants a real flush points the field at a tempdir it owns.
    - The module headers of `src/io/settings.rs` and `src/io/autosave.rs`, and
      the `App::default` / `App::new` doc comments in `src/app/init.rs`, all
      describe the deleted wrappers today. Each is brought in line.
    - `tests/skeleton.rs:23` witnesses the `io` module with
      `io::load_autosave as fn() -> Option<document::Document>`, which stops
      compiling. Pick another genuinely public `io` item (`io::Preset`,
      `io::recent_files`) — do not make a `pub(crate)` helper `pub` to keep the
      line.

16. **Caps, purity and gates.** `src/app/mod.rs` is at **255** of the 300
    implementation-LOC cap; measure with the ADR 0004 `awk` recipe, never
    `wc -l`. Two fields plus tight doc comments must not push it past 270 — if
    it does, **flag the seam to `architect` and stop; do not split on sight**
    (ADR 0004). `src/app/persist.rs` stays well under the cap; it holds three
    small methods. `src/app/persist.rs` MUST NOT import `eframe` or `rfd`, and
    the kernel purity rule is untouched. `cargo fmt --all -- --check`,
    `cargo clippy --all-targets -- -D warnings` and `cargo test --all` all exit
    0.

## Expected tests

- **Unit (AC 1, 2, 5)** in `src/app/init.rs` and `src/app/persist.rs`: the
  `App::new` bounded source scan pinning both resolver calls, with a positive
  control; a scan (or a `grep`-shaped test) proving `persist.rs` and `init.rs`
  are the only readers of the two fields.
- **Unit (AC 3)** in `src/app/mod.rs`'s test module, next to the field
  declarations: `app_default_has_no_persistence_paths`.
- **Unit (AC 6, 7)** in `src/app/persist.rs`:
  `an_injected_path_persists_and_none_persists_nothing` (both halves, one
  tempdir, the `read_dir` count as the absence assertion);
  `clear_autosave_removes_the_injected_file_and_tolerates_a_missing_one`.
- **Unit (AC 4, 8, 9)**: the three greps as static checks, each with a positive
  control so the scan is shown to be able to fail (the recurring bug class in
  this project is a source scan that matches its own arguments — bound the
  haystack to the implementation section).
- **Unit (AC 10)** in `src/app/autosave.rs`: the existing scan, re-pinned, with
  its positive control intact. Prove it discriminates by temporarily rewiring
  `flush_if_due` to a hard-coded `true` and confirming it fails by name.
- **Unit (AC 12, 13)** in `src/io/file_actions.rs`: the two renamed-in-place
  tests, each now half behavioural (tempdir) and half source scan.
- **Unit (AC 11)**: `new_document_seeds_bed_from_settings` and the other
  `action_new` tests keep passing with both fields `None`, and — this is the
  point — provably touch no file. Add
  `action_new_clears_only_the_injected_autosave_file`: seed a file at
  `autosave_path`, call `action_new`, assert it is gone; then with
  `autosave_path: None` assert the tempdir stays empty.
- **Integration**: the existing suites must pass unchanged. If any of them was
  relying on a wrapper that is now deleted, fix the witness, not the wrapper.
- **Mutation checks the reviewer will run, so run them first**: (a) make
  `App::new` forget `autosave_path` → AC 2's scan must fail by name; (b) make
  `persist_settings` write unconditionally to a hard-coded path → AC 6's
  `read_dir` assertion must fail; (c) delete the `clear_autosave` call from
  `action_new` → AC 12's tempdir assertion must fail.
- **[manual] (AC 14)**: the probe-file reproduction above, before and after.
- **[manual] smoke**: `cargo run`; change the agent endpoint in
  `Help > Agent settings` and close the window; change the bed in
  `File > Bed size…`; quit and relaunch — both survive. Draw a line, wait ~1 s,
  kill the process, relaunch — the line is recovered. `File > New` then quit and
  relaunch — nothing is recovered. This is the only way to observe that
  `App::new` really did fill both fields; no automated test may call `App::new`.

## Risks

- **The failure mode if `App::new` forgets a field is silent.** Preferences
  simply stop persisting and no test notices, because no test may call
  `App::new`. AC 2's scan and the manual smoke are the whole guard. That is
  still strictly milder than ADR 0005's failure mode (a panic on the operator's
  first `File > Open`), which is why ADR 0006 accepted it.
- **`src/app/panels.rs:83` holds `&mut app.settings` across the Agent Settings
  window closure** (`let settings = &mut app.settings;` at line 71, used inside
  `.show(ctx, …)`). `app.persist_settings()` needs `&App`, so the call has to
  move past the end of that borrow — restructure the tail of that function, do
  not clone the settings to dodge it.
- **`record_autosave_outcome(app, app.write_autosave())` does not compile**:
  `app` is borrowed mutably twice in one expression. Bind `wrote` first (AC 10
  spells the two statements out). Do not "fix" this by changing
  `record_autosave_outcome`'s signature — its split exists for a tested reason.
- **`tests/skeleton.rs` breaks at compile time**, not at assert time, and the
  tempting fix (widening a `pub(crate)` helper to `pub`) would reopen the hole
  from the other side. AC 15 forbids it.
- **Scan self-matching.** Three of this demand's criteria are source scans and
  this project has shipped three self-matching scans already. Bound every
  haystack to the implementation section and give every absence assertion a
  neighbouring presence assertion over the same slice.

## Notes

- Normative design: [ADR 0006](../../adr/0006-real-user-paths-are-injected.md).
  Where this demand and the ADR appear to differ, the ADR wins and the
  difference is a bug in this file — except for the one point recorded below,
  which is a difference between the ADR's prose and the tree.
- **Known tension, resolved here and worth re-reading before starting.**
  ADR 0006 §Consequences says the two source scans "can be paid back". They can
  be paid back *by half*. Both scans cover `action_open` **and**
  `action_open_path` — that is what "both open paths" means in their names.
  After this demand, `action_open_path` is behaviourally testable, but
  `action_open` still opens an `rfd` dialog that ADR 0005 deliberately keeps
  unreachable from any test, so its half must remain a bounded source scan
  forever, or until `rfd` itself is injected (which ADR 0006 explicitly rules
  out). AC 12 and AC 13 encode that split. Do not delete the scan halves; do not
  invent a way to call `action_open` from a test.
- `src/io/settings.rs` already ships `load_from` / `save_to` with their own
  tests (lines 306–475), and `src/io/autosave.rs` already ships
  `save_autosave_to` / `load_autosave_from` (lines 125, 156). This demand builds
  on that existing seam and adds exactly one new function to it,
  `clear_autosave_at`.
- The three `App` method names are ADR 0006's: `persist_settings`,
  `write_autosave`, `clear_autosave`. `App::clear_autosave` does not collide
  with `crate::io::clear_autosave`, because AC 4 deletes the latter.
- Related: [ADR 0005](../../adr/0005-native-dialogs-disarmed-by-default.md) and
  LCV-118 (the complementary mechanism, for native OS surfaces);
  [ADR 0002](../../adr/0002-headless-input-tests-and-dirty-tracking.md) §A2 (the
  two-constructor contract this decision makes structural) and §A4 rule 2 (the
  harness rule AC 15 rewrites); LCV-114 AC 10 and LCV-115 AC 9 (the coverage
  being repaid); LCV-120 (the other defect in `App::update_ui`'s frame; the two
  do not touch the same lines).
