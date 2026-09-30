# LCV-159 — Tasks

ADR 0003 amendment (5) (commit `868bb01`) admits `ToolKind::Dist`, polar input and `Tool::take_message`; no ADR task here.

- [x] T1 [AC1] [AC2] [AC3] [AC5] Test: `parse` polar table — `@50<30`, `50<30`, `@10<-45`, `@-10<45`, `@ 10 < 45`, exact `@10<90`/`<180`/`<270`, malformed `@<30`, `@10<`, `10<<5`, `<30` → `Unknown` (files: src/cmdline/parse.rs)
- [x] T2 [AC1] [AC2] [AC3] [AC5] Implement `parse_polar` and the arm in `parse` (files: src/cmdline/parse.rs)
- [x] T3 [AC1] [AC4] Test end-to-end: `l` ⏎ `0,0` ⏎ `@50<30` ⏎ draws the expected line; `@10<45` with no anchor shows the no-base-point message and changes nothing; `100<0` is an absolute point (files: tests/it/cmdline/polar_and_dist.rs, tests/it/cmdline/mod.rs)
- [x] T4 [AC6] [AC7] Test then add `Tool::take_message` (default `None`), `ToolManager::take_message`, and the drain in `poll_successor` before succession (files: src/tools/tool.rs, src/tools/manager.rs, src/app/viewport.rs)
- [x] T5 [AC6] [AC7] Test then implement `DistTool`: prompts, anchor, result string (3 decimals, angle in [0,360), no `-0.000`), successor SELECT, no doc/history mutation, Escape cancels (files: src/tools/dist.rs, src/tools/mod.rs)
- [x] T6 [AC6] `ToolKind::Dist`, aliases `dist`/`di`, `make` arm + name pin (files: src/cmdline/mod.rs, src/cmdline/parse.rs, src/tools/mod.rs)
- [x] T7 [AC6] [AC7] Test end-to-end: `di` ⏎ `0,0` ⏎ `@30,40` ⏎ shows `Distance = 50.000, Angle = 53.130°, Delta X = 30.000, Delta Y = 40.000`, then SELECT, with the undo depth unchanged; the same with two pointer clicks (files: tests/it/cmdline/polar_and_dist.rs)
- [x] T8 CHANGELOG line (files: CHANGELOG.md)
