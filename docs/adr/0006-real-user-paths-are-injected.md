# ADR 0006 — Real user paths are resolved at boot and injected, never resolved inside product logic

- **Status**: Accepted
- **Date**: 2026-09-13
- **Deciders**: architect

## Context

`Settings::save()` (`src/io/settings.rs`) resolves
`directories::ProjectDirs::from("", "", "lasercad")` and writes the developer's
real `~/.config/lasercad/settings.json`, API key included, with no guard of any
kind. `action_open`, `action_open_path` and `action_save_as`
(`src/io/file_actions.rs:103`, `:172`, `:213`) all end in
`let _ = app.settings.save();`. Any test that drives one of those functions
overwrites that file with the `Settings::default()` a test `App` carries.

The same shape as [ADR 0005](0005-native-dialogs-disarmed-by-default.md), and
harder to see: a disarmed dialog hangs or panics immediately, a settings write
succeeds silently and surfaces days later as a vanished API key.

Three implementers have avoided it by discipline, each by writing a bounded
source scan instead of a behavioural test
(`src/io/file_actions.rs::both_open_paths_adopt_the_file_bed_and_leave_the_seed_alone`
and `…_adopt_the_file_preset`, whose own doc comments name the settings write as
the reason). That is two acceptance criteria — LCV-114 AC 10 and LCV-115 AC 9 —
paying for safety with coverage. "Implementers will be careful" is the argument
ADR 0005 already rejected once.

### The discipline has already failed, for the sibling path

`src/io/autosave.rs` has the identical shape, and **all five** file actions call
`clear_autosave()` — including `action_new`, which an existing library test
drives directly. Measured on this tree at `32334ec`:

```
$ printf '{"probe":"…"}' > ~/.local/share/lasercad/autosave.json
$ cargo test --lib new_document_seeds_bed_from_settings   # 1 passed
$ ls ~/.local/share/lasercad/autosave.json
ls: cannot access … : No such file or directory
```

`cargo test` deletes the developer's crash-recovery file today, on every run.
The settings overwrite is the same hazard one careless test away. The two paths
must therefore be fixed together: `action_open_path` ends in
`settings.save(); clear_autosave();`, so guarding only settings still leaves it
untestable.

### The finding that decides the shape

LCV-118's review recorded a structural defect in ADR 0005's mechanism: arming is
process-wide and there is deliberately no disarm, so **no automated test can
ever observe the armed path** — arming poisons the shared `AtomicBool` for the
rest of the test binary. That verification had to be pushed onto a human
smoke-test checklist (LCV-089).

Transplanting that mechanism here inherits the blind spot *and* defeats the
goal. A `Settings::save()` that panics when disarmed makes `action_open_path`
and `action_new` panic in tests; the source scans this decision exists to
retire would have to stay, and the one existing behavioural test that calls
`action_new` would have to be deleted.

## Decision

**A real per-user filesystem location is resolved exactly once, at boot, and
carried as data on `App`. No function below boot resolves one.**

Shape for `implementer-rust` (this ADR ships no code):

1. `directories::ProjectDirs` may appear in exactly two files —
   `src/io/settings.rs` and `src/io/autosave.rs` — each behind one
   `pub(crate) fn` returning `Option<PathBuf>`. Those two functions may be
   called from exactly one place: `App::new()`.
2. `App` gains `pub settings_path: Option<PathBuf>` and
   `pub autosave_path: Option<PathBuf>`. `App::new()` (boot-only, ADR 0002 §A2)
   fills both. `App::default()` (the test constructor, which touches no
   filesystem) leaves both `None`.
3. The unguarded wrappers `Settings::load`, `Settings::save`, `save_autosave`,
   `load_autosave` and `clear_autosave` are **deleted**. The path-injected
   siblings that already exist — `load_from`, `save_to`, `save_autosave_to`,
   `load_autosave_from`, plus a new `clear_autosave_at(path)` — become the only
   API, and every call site goes through a small `App` method
   (`persist_settings`, `write_autosave`, `clear_autosave`) living in one new
   file, `src/app/persist.rs`. That file is then the only reader of the two
   path fields besides `App::new`, so the invariant is one `grep` wide.
4. `None` means "this process does not persist". The write is a **no-op**; it
   does not panic.
5. A test that wants persistence sets the field to a path under a temp
   directory and asserts the bytes on disk. Real behavioural tests of
   `action_new` / `action_open_path` / `action_save_as` become possible, and the
   two source scans above are replaced by them.

**No-op, not panic — the ADR 0005 reasoning inverts here.** ADR 0005 chose
panic because the alternative return, `None`, is a *valid* production outcome
("the user cancelled"), so a quiet fallback would be indistinguishable from
correct behaviour and would turn a hang into a false green. That ambiguity does
not exist for a write: whether the write happened is directly observable by the
test that asked for it — point the path at a tempdir and read the file back; an
absent write is a failed assertion, never a green one. Two further reasons
point the same way: the production contract for a failed settings write is
already "swallow and continue" at all five call sites (`let _ =`, `.ok()`) —
an unwritable config directory must not abort a CAD session mid-job — so a
panic would be louder than the real failure it guards; and a panic would make
the very functions this decision exists to open up untestable.

**Relationship to ADR 0005: complementary, not superseding.** ADR 0005 stands
unchanged for `rfd`. The rule that tells a future demand which mechanism to
reach for:

- **A blocking native OS surface** (`rfd`, a message box, an OS print dialog)
  → arm-by-main + panic, ADR 0005. There is nothing to inject; the thing you
  would inject is a native modal a test can never let run.
- **A filesystem location** → inject the path, this ADR. A path is data;
  injecting it costs one field and buys real tests.

Neither is a licence for the other's shape: no new global `AtomicBool` for a
filesystem side effect, no path injection to make `rfd` "testable".

## Consequences

**Easier**

- The file actions become behaviourally testable end to end, against a tempdir.
  Two source scans bought with coverage (LCV-114 AC 10, LCV-115 AC 9) can be
  paid back.
- Both sides are observable in the same test binary, in any order, with no
  global state: injected path → file appears; `None` → no file. ADR 0005's
  blind spot is not inherited.
- `cargo test --all` stops touching `~/.config/lasercad` and
  `~/.local/share/lasercad` at all.
- Mutation testing of `src/io/file_actions.rs` becomes safe, which is how both
  this hazard and ADR 0005's were found.

**Harder / costs**

- Two more `App` fields. `src/app/mod.rs` is at 255 of the 300 implementation
  LOC cap (ADR 0004); keep the two doc comments tight, and if it crosses 270
  name the seam to `architect` rather than splitting on sight.
- A boot ordering invariant: `App::new` must fill both fields or the app
  silently stops persisting. Guarded by a bounded source scan over `App::new`,
  the technique `src/app/init.rs` already uses for the bed seed. The failure
  mode if it is ever wrong is "preferences do not persist", which is strictly
  milder than ADR 0005's "panic on the operator's first File > Open".
- `src/app/autosave.rs`'s existing scan pins the literal
  `crate::io::save_autosave(&app.document).is_ok()`; it must be re-pinned to
  the new call, keeping its positive control.
- `src/app/panels.rs:83` holds `&mut app.settings` across the Agent Settings
  window closure; the persist call has to move past the end of that borrow.

**Committed to**

- `ProjectDirs` in two files, called from one. A third occurrence is a review
  blocker.
- Anything new that writes to a real user location gets a path field on `App`
  and a method in `src/app/persist.rs` — not a global flag, not a fresh
  `ProjectDirs` call.
- `App::default()` remains the constructor that touches no filesystem
  (ADR 0002 §A2). This decision makes that property structural rather than
  documentary.

## Alternatives considered

- **ADR 0005's `AtomicBool`, armed by `crate::run()`, panicking when
  disarmed** — inherits LCV-118's blind spot (nothing can observe the armed
  path) and makes `action_new` / `action_open_path` panic in tests, which is
  the opposite of this demand's goal. The option that looks consistent and is
  not.
- **The same flag with a silent no-op instead of a panic** — same blind spot,
  and `cargo test` stays one stray `arm()` call away from real writes, with no
  way to test the armed side.
- **Keep `Settings::save()` and rely on the audit** — the status quo, already
  rejected by ADR 0005 and *already failing* for `clear_autosave` (see
  §Context). It has cost two acceptance criteria their coverage.
- **`#[cfg(test)]` guard** — dead on arrival for the reason ADR 0005 records:
  integration tests link the library with `cfg(test)` off.
- **Override `$XDG_CONFIG_HOME` / `$HOME` in tests** — process-global mutable
  state shared by parallel test threads, and it does not exist on Windows,
  where CI runs.
- **A `SettingsStore` trait / injected store object** — three layers where one
  `Option<PathBuf>` does. Rejected against `AGENTS.md` §Product Philosophy.
- **Fix settings only, leave autosave** — `action_open_path` ends in
  `settings.save(); clear_autosave();`. Half the fix leaves the function just
  as untestable and leaves `cargo test` deleting recovery files.

## Revisit criteria

- A second boot path appears (CLI export, headless mode): it fills the two
  fields itself, or sets them `None` and deliberately does not persist.
- A library consumer outside this repo appears: it supplies its own paths,
  which is now the only way in. There is no such consumer today.
- `App` outgrows the two fields (a third and fourth path): fold them into one
  small `Paths` struct, still owned by `App`, still filled only at boot.
