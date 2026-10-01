# LaserCAD v2 — Agent Guide (constitution)

Single source of agent rules. `CLAUDE.md` imports this file; area-specific rules
live in `.claude/rules/` and load only when matching files are touched.

LaserCAD v2 is a 2D micro-CAD for laser cutting, compatible with LaserGRBL: a
**pure-Rust, single-binary, egui** KISS clone of AutoCAD R14. Preserve precision;
reject UI or architecture growth that does not improve the CAD → LaserGRBL flow.
Linux first. All versioned artifacts are English; conversation may be any language.

## Commands

```bash
scripts/gate.sh                    # THE gate: fmt check, clippy -D warnings, tests --no-fail-fast, LOC cap, backlog check
scripts/check.sh [filter...]       # inner loop: clippy + lib/integration tests matching filter, no doctests
scripts/mutants.sh [base=main]     # cargo-mutants on src/ lines changed since base (review, high-risk specs)
scripts/agent-bench.sh <settings>  # agent eval bench against a live model (opt-in; the gate replays it offline)
cargo run                          # run the app (debug)
cargo test <substring>             # one test while iterating
scripts/backlog.sh                 # regenerate docs/product/backlog.md from spec headers
scripts/backlog.sh --next          # Planned specs whose deps are Done
git config core.hooksPath .githooks  # once per clone: enables the commit-msg guard
```

Toolchain pinned in `rust-toolchain.toml`. Native prerequisites: `docs/build-local.md`.

## Workflow — lean Spec-Driven Development

```
Draft ─/specify→ Specified ─/design→ Planned ─/implement→ Done      (Blocked / Rejected any time)
```

- **One folder per demand**: `docs/specs/LCV-NNN-slug/` with `spec.md` (what; EARS acceptance
  criteria), `plan.md` (how; files, ADRs, risks), `tasks.md` (checklist, 1–3 files per task).
  Templates and size limits: `docs/specs/_templates/`.
- **The `Status` line in `spec.md` is the only status.** `docs/product/backlog.md` is generated.
  Retired pre-SDD demands live only in git history (`git log -- docs/product/demands`).
- **Human gates**: the user approves `spec.md` (→ Specified) and `plan.md` + `tasks.md` (→ Planned).
  Product or design questions are asked *then*, never invented during implementation.
- **Agents** (`.claude/agents/`): `implementer-rust` (opus) executes `tasks.md`; `reviewer-rust`
  (fable) reviews the diff against the ACs once, blocking findings only; `architect` (opus) only
  when `/design` finds a module-boundary change or an ADR is needed. Everything else runs in the
  main session via the skills `/specify`, `/design`, `/implement`, `/next`.
- **Review budget**: one review round, one fix round, then ask the user. Mutation testing
  (`scripts/mutants.sh`) only when `plan.md` flags high risk (`src/agent/`, `src/io/svg/export.rs`,
  `History`).
- **Parallel work**: independent `Planned` specs may run in git worktrees, each with its own
  `CARGO_TARGET_DIR`.
- **Fast lane**: a change with no user-visible behavior change (refactor, test-only, docs, tooling,
  or a bug fix of ≤3 files with a regression test) skips the spec folder: commit
  `fix|refactor|test|docs|chore: …` without an LCV id, `scripts/gate.sh` green, CHANGELOG only if
  user-visible. ACs of a Done spec, the SVG export contract, ADRs and module boundaries still go
  through SDD.

## Commits

- Conventional: `feat|fix|refactor|test|docs(LCV-NNN): …`, `chore: …`. One commit per task.
- **Never** add `Co-Authored-By` or any AI trailer (enforced by `.githooks/commit-msg`).
- Never `--no-verify`, never amend unless asked, never force-push, never tag. Pushing `origin main` is allowed.

## Architecture

```
src/
├── main.rs, lib.rs   # thin entry; lib re-exports for tests
├── app/              # eframe::App, top-level state, per-frame wiring, one file per concern
├── geometry/         # kernel: vec2, line, circle, arc, intersect, snap, rect, epsilon
├── cmdline/          # kernel: command-line grammar, recall ring (ADR 0003)
├── document/         # kernel: entities, schema, commands, history, selection
├── render/           # camera, grid, bed, entities, preview, snaps
├── tools/            # Tool trait, ToolManager, one file per tool
├── io/               # settings, autosave, recent, dialogs (rfd); io/svg/ = kernel
├── ui/               # menubar, toolbar, statusbar, command line, dialogs, theme
├── agent/            # LLM agent: wire, transport, tools, bridge, loop, panel
├── text/             # kernel: Hershey font, layout
└── util/             # units, world↔SVG Y flip
```

The tree is orientation; `ls` is the inventory.

### Purity rule

- Kernel — `src/geometry/`, `src/document/`, `src/io/svg/`, `src/text/`, `src/cmdline/` — never imports `egui`, `eframe` or `rfd`.
  `src/render/raster.rs` is kernel-pure too, although the rest of `render/` imports `egui` (ADR 0011).
- `src/agent/`: only `panel.rs` and `settings_ui.rs` may import `egui`; none may import `eframe`/`rfd`.
  `classifier.rs`, `wire.rs`, `transport.rs`, `tools.rs`, `tools/args.rs`, `tools/schema.rs`, `tools/transform.rs`, `tools/capture.rs`, `tools/measure.rs`, `drawing/*.rs` (`keys.rs`, `items.rs`, `items/array.rs`, `expand.rs`, `schema.rs`), `bridge.rs`, `bridge/action.rs`, `bridge/action/ops.rs`, `bridge/action/measure.rs`, `loop_.rs`, `loop_/images.rs`, `loop_/batch.rs`, `loop_/verify.rs`, `settings_ui/copy.rs`, `prompt.rs`,
  `memory.rs`, `repeat.rs`, `metrics.rs`, `attachment.rs`, `drawing.rs` and `mod.rs` are kernel-pure. Only `transport.rs` imports `reqwest`; only `wire.rs` uses `base64` (ADR 0011). `panel.rs` never spawns a thread
  and never builds a `Document`/`History` (ADR 0007 §D8).

### Invariants (review blockers)

- **Millimeters** canonical everywhere except `render/camera` (pixels). **Radians** in kernel/state; degrees only in UI.
- **All entity mutation** is a `Command` committed via `App::commit` into `History` (depth 200). Tools never touch `Document` directly.
- **The agent mutates like everything else**: the worker thread holds no `Document`/`History`; `Document` stays `!Clone` (ADR 0007).
- **`rfd` only in `src/io/dialogs.rs`**; dialogs are disarmed outside `crate::run()` (ADR 0005). No test sends Ctrl+O/S.
- **User paths** resolved once in `App::new()` and injected; `App::default()` persists nothing (ADR 0006).
- **≤300 implementation LOC per `.rs`** (lines before the column-0 `#[cfg(test)]`), checked by `scripts/loc-cap.sh` (ADR 0004). At 270+, note the seam in `plan.md`.
- **Repaint** only on the three guarded sites (`.claude/rules/repaint-ui.md`).
- No `unwrap`/`expect` in library code without a documented invariant; no `unsafe` without an ADR; no `tokio`. (enforced by `[lints]` in `Cargo.toml`; exceptions need `#[expect(.., reason)]`)
- `pub` on `mod.rs` re-exports only; doc comments on `pub` items.
- One test per acceptance criterion. Rendering ACs are proven on painted shapes (`tests/harness/paint.rs`), not source scans.
- Cite symbols (`file.rs::fn`), never line numbers, in this file and in `.claude/rules/`.

### SVG export (LaserGRBL compatibility)

Contract of `src/io/svg/export.rs`; changing it needs explicit user confirmation.

- `xmlns` on root `<svg>`; `width`/`height` are the document's bed size in mm (`Document::bed_mm`);
  `viewBox="0 0 <bed_width> <bed_height>"` without units. Import reads the header and layers back.
- Y mirrored: `y_svg = flip_y(y_world, bed_height)` via `crate::util::flip_y`, using the document's
  own bed height, never a constant. X, radii and stroke widths untouched.
- `fill="none"`; no live text; no `filter`/`mask`/`clipPath`.
- One `<g>` per layer, in layer order, empty layers included:
  `<g data-layer="<name>" stroke="#rrggbb" stroke-width="0.1" data-output="1|0">`, plus
  `data-current="1"` on the current layer only; name XML-escaped; no `id` (ADR 0012).
- `File > Export layers` writes `<mother>-<file_key>.svg` beside the mother for each layer with
  Output on and entities: the same header and only that layer's `<g>` (no `data-current`).
- Arcs as `<path d="M sx sy A r r 0 large sweep ex ey"/>`, never béziers; the mirror inverts
  `sweep` (`sweep = 0` for a CCW world arc), `large` unchanged.
- Full ellipses as `<ellipse cx cy rx ry/>`, plus `transform="rotate(a cx cy)"` only when `a` does
  not print as `0.000000`; elliptical arcs as `<path d="M sx sy A rx ry φ large sweep ex ey"/>`,
  never béziers. `a = φ = −rotation` in degrees, normalized into (−180, 180], written `{:.6}`;
  `sweep = 0` for a CCW world span, `large = 1` iff the parametric sweep > π (ADR 0015).
- Bézier entities, and only they, as `C`/`Q` paths: `<path d="M x0 y0 C x1 y1 x2 y2 x3 y3"/>`
  (cubic), `<path d="M x0 y0 Q x1 y1 x2 y2"/>` (quadratic), every `y` mirrored (ADR 0016).

## Product philosophy

Command line and keyboard first · plain inspectable SVG · small composable tools ·
deterministic geometry over visual convenience · reject features rather than carry accidental complexity.
UI directives: `DESIGN.md`.

## ADRs (`docs/adr/`)

- 0001 pure Rust + egui · 0002 headless input tests, dirty tracking, paint harness · 0003 command-line contract
- 0004 measuring the 300-LOC cap · 0005 native dialogs disarmed by default · 0006 user paths injected
- 0007 agent turn mutates the live document (fence, flat group, budgets) · 0008 tests run `--no-fail-fast`
- 0009 dialog body capped at 426pt · 0010 declarative drawing batch tool · 0011 canvas observation raster
- 0012 document layers, mother SVG with one `<g>` per layer, one export file per layer
- 0013 tools describe canvas feedback as styled marks (`Tool::feedback`)
- 0014 stable entity ids `e<N>` beside indices, for the app run, never persisted
- 0015 ellipse entity: parametric span, `<ellipse>` / `A rx ry φ` export, exact import under any CTM
- 0016 Bézier entities (cubic, quadratic): `C`/`Q` export for them only, tight bbox, Wang polyline
- 0017 SVG `<text>` imports as system-font outlines (`fontdb` + `ttf-parser`), deterministic

A reversed ADR gets a `**Superseded**` header; ADR text keeps its original line citations.
Product principles: `docs/product/README.md`. Roadmap: `PLAN.md`. User-visible changes: `CHANGELOG.md`.
