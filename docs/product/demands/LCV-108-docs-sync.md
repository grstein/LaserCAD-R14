# LCV-108 — Docs sync: CHANGELOG, README status, AGENTS.md reality, backlog consistency

- **Status**: Ready
- **Phase**: 10
- **Depends on**: LCV-100, LCV-101, LCV-102, LCV-103, LCV-104, LCV-105, LCV-106, LCV-107 (lands last in Marco 0)
- **Suggested agent**: demand-manager
- **Suggested model**: sonnet

## Problem

The repository's documentation currently tells a story the repository cannot
back up, and every one of those claims is load-bearing for somebody:

- `CHANGELOG.md` carries a `## [0.1.0] - 2025-06-15` header. There is no
  `v0.1.0` tag (`git tag` is empty), no remote, and the commit history is dated
  May 2026. A user downloading an artifact would read a release note for a
  release that does not exist, and the CI `release` job extracts its GitHub
  release body from that very section.
- That section stops at LCV-040 and LCV-071. Everything an operator actually
  uses — the drawing and modify tools, snaps, ortho, SVG export and import,
  settings, autosave, recent files, native dialogs, menubar, toolbar, status
  bar, command line, undo/redo shortcuts, the agent harness and all packaging —
  shipped and is undocumented.
- `README.md:9` says "Pre-alpha. The scaffold is in", written when the scaffold
  really was all there was. The app now draws, edits and exports.
- `AGENTS.md` — the file every agent reads before acting — contains three
  statements that are false against the code. Agents act on them:
  it promises async work "lives on a `tokio` runtime owned by `App`"
  (line 106) when the agent turn actually runs on a `std::thread` with
  `reqwest::blocking` and an `mpsc` channel, and `tokio` is not even a
  dependency; it lists `src/agent/classifier.rs` in the kernel purity rule
  (line 71) and in the module tree (line 54) when that file does not exist; and
  its SVG export checklist predates the Y-axis inversion that LCV-100 landed.
- The three status stores disagree. `docs/product/backlog.md`,
  `.claude/backlog.json` and the demand files' `Status:` lines carry different
  titles for the same ids (LCV-054, LCV-075, LCV-080 shipped something other
  than what their row says), which is how the OffsetTool accident stayed
  invisible for months.

Marco 0 exists to make "Done" true. This demand is the last step: it makes the
paperwork match the software.

## Scope

`demand-manager`-owned documents only.

### 1. `CHANGELOG.md`

- Delete the `## [0.1.0] - 2025-06-15` header. Its entries fold into
  `## [Unreleased]`. No versioned section exists until a tag exists.
- Add the missing entries so that **every** demand with `status: Done` in
  `.claude/backlog.json` is referenced at least once. Group them Keep-a-Changelog
  style under `[Unreleased]`: `### Added`, `### Changed`, `### Fixed`,
  `### Removed`. Coverage must include, at minimum:
  - Tools and editing: LCV-041..053 (pointer plumbing, Select, Line, Polyline,
    Rect, Circle, Arc, Text, Move, Trim, Extend, Delete, ortho lock).
  - Text: LCV-055 (Hershey font + layout).
  - File I/O: LCV-056 (SVG export, LaserGRBL presets), LCV-057 (SVG import),
    LCV-058..062 (settings, autosave, recent files, native dialogs, file
    actions).
  - UI chrome: LCV-065..070, LCV-075 (undo/redo shortcuts + status flash).
  - Agent harness: LCV-076..080.
  - Packaging and CI: LCV-085..088, LCV-090..092.
  - Marco 0: LCV-100..107 — mostly `### Fixed` (SVG Y-axis, settings load,
    autosave firing at 800 ms, keyboard routing, tool reachability, CI Windows
    packaging) and `### Removed` (OffsetTool).
- Scaffold-only demands (LCV-001..006) may be covered by a single line that
  names their ids.
- Nothing in the file may claim a release date or a published artifact.

### 2. `README.md`

- Line 9: `Pre-alpha.` → `Alpha (0.1.0 pending).` The rest of the sentence is
  rewritten to describe what works today (drawing and modify tools, snaps,
  ortho, undo/redo, command line, SVG export/import, autosave, native dialogs,
  optional agent harness on Linux) and to state that no release has been tagged
  yet.

### 3. `AGENTS.md`

- §"Event flow", the async paragraph: replace the `tokio` claim with the
  shipped design — the agent turn runs on a `std::thread`
  (`src/agent/panel.rs:141`) using a blocking `reqwest::blocking::Client`
  (`src/agent/transport.rs:94`); the result returns through a
  `std::sync::mpsc::Receiver` polled each frame, which then calls
  `ctx.request_repaint()`. `tokio` is not a dependency of this crate.
- §"Purity rule" and §"Module tree": drop `src/agent/classifier.rs` — the file
  does not exist. The purity list becomes `src/geometry/*`, `src/document/*`,
  `src/io/svg/*`, `src/text/*`. The module-tree comment for `agent/` drops
  "classifier".
- §"Module tree": drop the `util/` line if LCV-105 removed `src/util/`.
- §"SVG export (LaserGRBL compatibility)": **verify** that the checklist states
  the world-Y-up → SVG-Y-down inversion rule as implemented. LCV-100 AC#19 owns
  that edit and authorises it; this demand only closes the gap if LCV-100 left
  the section stale. Do not restate the rule differently from
  `src/io/svg/export.rs`.

### 4. Status-store consistency

- For every demand id, the `Status:` line in the demand file, the section it
  appears under in `docs/product/backlog.md`, and the `status` field in
  `.claude/backlog.json` agree.
- Fix the substantive title mismatches, taking the demand file's `# LCV-NNN — …`
  heading as truth and keeping a `note` that records what the id was originally
  planned for:
  - LCV-054 — "Snap integration into all drawing tools" → the OffsetTool
    reality; status becomes `Rejected` (removed by LCV-106), moved to the
    Rejected section in `backlog.md`.
  - LCV-075 — "Agent classifier (regex routing)" → "Undo/Redo keyboard shortcuts
    (Ctrl+Z / Ctrl+Y) + status bar flash".
  - LCV-080 — "Command-line wires ':' / '/ai' prefixes to agent" → "Agent panel
    — chat UI for AI assistant".
  - LCV-077 — drop "tokio" from the title ("HTTP transport (reqwest + tokio)").
- LCV-100..108 end at `Done` in all three stores.
- LCV-089 stays `Blocked` with its reason ("awaiting git remote") intact.

## Out of scope

- **`PLAN.md`** — single-writer `project-manager`. This demand files the sync
  request; it does not edit the table.
- **Any file under `src/`, `tests/`, `Cargo.toml`, `.github/`** — code and CI
  are `implementer-rust` scope. The stale module doc inside `src/lib.rs` is
  handled by LCV-105, not here.
- **Tagging a release, creating a remote, publishing a GitHub release** —
  LCV-089, blocked on the user.
- **Rewriting demand bodies.** Only `Status:` / `Implementation:` lines, backlog
  rows and the three shared docs change.
- **Restructuring the CHANGELOG format** or adopting a different convention.
  Keep a Changelog stays.
- **Back-dating or inventing dates.** Entries reference LCV ids and commits, not
  invented release days.

## Acceptance criteria

1. `grep -n "^## \[" CHANGELOG.md` outputs exactly one line: `## [Unreleased]`.

2. `grep -n "2025" CHANGELOG.md` returns no match (the fictional date is gone
   and no other 2025 date is introduced).

3. Every demand with `"status": "Done"` in `.claude/backlog.json` appears at
   least once in `CHANGELOG.md`:
   ```
   python3 -c "
   import json
   ids=[d['id'] for d in json.load(open('.claude/backlog.json'))['demands'] if d['status']=='Done']
   text=open('CHANGELOG.md').read()
   print([i for i in ids if i not in text])"
   ```
   prints `[]`.

4. `CHANGELOG.md` `[Unreleased]` contains entries for LCV-100..107, including a
   `### Removed` line for OffsetTool that names LCV-106.

5. `CHANGELOG.md` contains no claim of a published release, downloadable
   artifact or release date.

6. `README.md` line 9 starts with `Alpha (0.1.0 pending).`, and
   `grep -n "Pre-alpha" README.md` returns no match.

7. `README.md` states that no release has been tagged yet.

8. `grep -n "tokio" AGENTS.md` returns at most one line, and that line states
   that `tokio` is **not** used; `grep -n "tokio" Cargo.toml` returns no match
   (the corrected doc matches the manifest).

9. `grep -rn "classifier" AGENTS.md` returns no match, and the purity rule lists
   exactly `src/geometry/*`, `src/document/*`, `src/io/svg/*`, `src/text/*`.

10. The AGENTS.md module tree matches `ls src/`: every directory listed exists
    and every top-level module under `src/` is listed (in particular `util/` is
    listed if and only if `src/util/` exists).

11. The AGENTS.md SVG export checklist states the world-Y-up → SVG-Y-down
    inversion rule and does not contradict `src/io/svg/export.rs` as landed by
    LCV-100 (normally already true — LCV-100 AC#19 owns that edit; this is the
    verification pass). The other LaserGRBL rules (xmlns, mm width/height, `fill="none"`,
    one `<g>` per preset colour with `#ff0000` / `#0000ff` / `#00aa00`,
    `stroke-width="0.1"`, arcs as `A` path commands) are unchanged.

12. For every file in `docs/product/demands/`, its `- **Status**:` value equals
    the `status` field of the matching id in `.claude/backlog.json`:
    ```
    python3 -c "
    import json,re,glob,os
    st={d['id']:d['status'] for d in json.load(open('.claude/backlog.json'))['demands']}
    bad=[]
    for f in glob.glob('docs/product/demands/LCV-*.md'):
        i=os.path.basename(f)[:7]
        m=re.search(r'^- \*\*Status\*\*: (\S+)', open(f).read(), re.M)
        if m and st.get(i) and m.group(1)!=st[i]: bad.append((i,m.group(1),st[i]))
    print(bad)"
    ```
    prints `[]`.

13. Every id in `.claude/backlog.json` appears in exactly one section of
    `docs/product/backlog.md`, and that section matches its status.

14. LCV-054 is `Rejected` in all three stores, with a reason naming LCV-106, and
    its title matches the demand file heading (OffsetTool).

15. LCV-075, LCV-077 and LCV-080 titles in `backlog.md` and `.claude/backlog.json`
    match their demand-file headings, each carrying a `note` that records the
    originally-planned scope.

16. LCV-100..108 are `Done` in all three stores; LCV-089 is still `Blocked` with
    its reason intact.

17. `.claude/backlog.json` is valid JSON
    (`python3 -c "import json;json.load(open('.claude/backlog.json'))"` exits 0).

18. No file under `src/`, `tests/`, `.github/`, and not `Cargo.toml` or
    `PLAN.md`, is modified by this demand (`git diff --name-only` shows only
    `CHANGELOG.md`, `README.md`, `AGENTS.md`, `docs/product/backlog.md`,
    `.claude/backlog.json` and demand files).

19. `cargo test --all` still exits 0 (docs-only change; regression guard).

## Expected tests

- **(AC 1, 2, 4, 5)**: `grep` checks on `CHANGELOG.md` plus a read-through of
  `[Unreleased]`.
- **(AC 3)**: the Python coverage snippet above; expected output `[]`.
- **(AC 6, 7)**: `grep` on `README.md`.
- **(AC 8, 9, 10, 11)**: `grep` on `AGENTS.md` and `Cargo.toml`; `ls src/`
  compared against the module tree; a read of the SVG checklist against
  `src/io/svg/export.rs`.
- **(AC 12)**: the Python status-consistency snippet; expected output `[]`.
- **(AC 13, 14, 15, 16)**: a read of `docs/product/backlog.md` section by
  section against `.claude/backlog.json`.
- **(AC 17)**: `json.load` parse check.
- **(AC 18)**: `git diff --name-only` before committing.
- **(AC 19)**: `cargo test --all`.
- **Manual**: read `[Unreleased]` top to bottom as a first-time user. Every
  entry must describe an observable behaviour (what the operator can now do),
  not an internal refactor, and must name its LCV id.

## Risks

- **Landing early.** If this demand runs before LCV-100..107 are `Done`, it
  documents behaviour that is not yet in the tree — exactly the failure it
  exists to fix. It is last in Marco 0 by construction.
- **CHANGELOG entries drifting into commit-log paraphrase.** The file is
  user-facing. Entries such as "refactored app.rs" do not belong; LCV-105's
  user-visible piece is "Edit > Select All is now undoable", not the split.
- **PLAN.md going stale in the other direction.** After this demand, PLAN.md is
  the only store not synchronised. File the request to `project-manager` in the
  same pass so Marco 0 does not close with a fourth disagreeing table.
- **Re-introducing a version header.** `## [0.1.0]` may only be created by
  whoever cuts the tag (LCV-089), with the real date, moving entries down from
  `[Unreleased]`.
- **AGENTS.md corrections changing agent behaviour.** Removing the `tokio`
  sentence means a future demand will not reach for a runtime that is not
  there — that is the point, but reviewers should expect the change to show up
  in later design decisions.

## Open questions

*(none — demand is Ready)*

## Notes

- Verified at authoring time: `git tag` is empty, `git remote -v` is empty,
  `Cargo.toml` has no `tokio` dependency, `src/agent/` contains
  `loop_.rs mod.rs panel.rs settings_ui.rs tools.rs transport.rs` and no
  `classifier.rs`, and `CHANGELOG.md` references only LCV-007..040 and LCV-071.
- `.claude/backlog.json` is the live status board per its own `description`
  field; `backlog.md` and the demand `Status:` lines mirror it. When two
  disagree, the demand file heading decides the *title* and the shipped code
  decides the *status*.
- AGENTS.md §"Documentation Hygiene" already requires exactly this: "Sync the
  CHANGELOG. When a `Done` demand changes user-visible behavior,
  `demand-manager` updates `CHANGELOG.md` under `[Unreleased]`." This demand
  pays off the accumulated debt and returns the repo to that steady state.
- The CI `release` job extracts its GitHub release body from the CHANGELOG
  section matching the tag (`.github/workflows/ci.yml`, "Extract CHANGELOG
  section for this tag"), so the header the tag expects must exist *at tag
  time* — created by LCV-089, not here.
