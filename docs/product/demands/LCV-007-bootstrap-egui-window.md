# LCV-007 — Bootstrap egui window — title "LaserCAD v2 — bootstrap"

- **Status**: Done
- **Phase**: 0
- **Depends on**: LCV-001, LCV-002
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: d94e038 — feat(LCV-007): freeze bootstrap window contract via APP_TITLE / DEFAULT_WINDOW_SIZE

## Problem

Every later phase — geometry kernel, document model, render, tools, I/O, UI chrome, agent harness — assumes that `cargo run` opens *some* native window so the implementer can observe behavior. Phase 1..2 demands have no UI surface of their own; they need a stable, minimal "is the app alive?" shell to render against. Without a frozen contract for the bootstrap window (title, size, entry-point shape, panic discipline), later demands like LCV-030 (`eframe::App` impl + central panel) cannot extend the window without risking a silent rename of the title constant or a re-shuffling of the `main` → `lib::run` → `App` wiring. This demand pins that contract.

The bootstrap shell already exists in the LCV-001 scaffold (`src/main.rs` calls `lasercad::run()` which opens an eframe window titled "LaserCAD v2 — bootstrap"). LCV-007 is therefore largely **retroactive lock-in**: it freezes what the scaffold delivered and adds the small extras needed to make the contract observable (named constants and a unit test).

## Scope

- `src/main.rs` is the binary entry point. Its `main` function returns `eframe::Result<()>` and delegates to a single library call (`lasercad::run()` or equivalent re-export). `main` itself contains no business logic and no `unwrap()` / `expect()`.
- `src/lib.rs` exposes `pub fn run() -> eframe::Result<()>` which constructs `eframe::NativeOptions` and calls `eframe::run_native`.
- The window title is defined as `pub const APP_TITLE: &str = "LaserCAD v2 — bootstrap";` in `src/lib.rs` (or `src/app.rs`, whichever the implementer picks — but exactly one location, re-exported via `lasercad::APP_TITLE`). The string literal `"LaserCAD v2 — bootstrap"` MUST NOT appear anywhere in `src/` other than at the constant's definition and in the unit test that asserts its value.
- The default window inner size is defined as `pub const DEFAULT_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];` in the same file as `APP_TITLE`. Units are screen pixels (`eframe`'s `with_inner_size` argument), explicitly NOT millimeters — this is render-surface pixels, not a CAD world quantity.
- The `eframe::ViewportBuilder` passed to `eframe::run_native` is configured with `.with_title(APP_TITLE)` and `.with_inner_size(DEFAULT_WINDOW_SIZE)`. The implementer may keep the existing `.with_min_inner_size([800.0, 600.0])` from the LCV-001 scaffold.
- The window closes on the OS close button without panicking.
- A unit test in `src/lib.rs` (under `#[cfg(test)] mod tests`) asserts `APP_TITLE == "LaserCAD v2 — bootstrap"` and that `DEFAULT_WINDOW_SIZE == [1280.0, 800.0]`.

## Out of scope

- Any UI chrome: menubar, toolbar, statusbar, command line, dialogs (Phase 6 — LCV-065..071).
- The full `eframe::App` impl with central panel and pointer input — owned by **LCV-030** and **LCV-032**.
- Camera / viewport / grid / bed / entity rendering — Phase 3 (LCV-030..038).
- Settings persistence and window-size restoration — owned by **LCV-058**.
- File dialogs, recent files, autosave — Phase 5 (LCV-058..062).
- Logging: do **not** add `tracing`, `log`, `env_logger`, or any other logging crate as part of LCV-007. The LCV-001 dependency lock allows only `eframe` and `egui`; introducing a logger requires its own demand.
- `argv` parsing, CLI flags, environment-variable configuration.
- Icon / `.desktop` entry / window-class branding (Phase 8 — LCV-088).
- Custom panic hooks; `App` panics during `update` are handled by `eframe`'s default behavior.

## Acceptance criteria

1. `src/main.rs` defines `fn main() -> eframe::Result<()>` whose body is a single call into the library (`lasercad::run()` or an equivalent single-function re-export). `main` does not contain `unwrap()` or `expect()`.
2. `src/lib.rs` (or `src/app.rs` — pick one and stay consistent) declares `pub const APP_TITLE: &str = "LaserCAD v2 — bootstrap";` exactly once. `grep -rnF '"LaserCAD v2 — bootstrap"' src/` returns exactly two matches: the constant definition and the unit test referenced in criterion 6.
3. The same file declares `pub const DEFAULT_WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];`. The literal pair `[1280.0, 800.0]` does not appear elsewhere in `src/`.
4. `lasercad::run()` configures `eframe::NativeOptions` such that the `ViewportBuilder` uses `APP_TITLE` for the window title and `DEFAULT_WINDOW_SIZE` for the inner size. (Both are referenced by name, not as inline literals.)
5. Running `cargo run` on Linux opens a native window within 5 seconds. The window's title bar text equals exactly `LaserCAD v2 — bootstrap`. The window remains responsive (does not freeze) and closes cleanly on the OS close button without panicking or printing a stack trace. (Manual smoke — see Expected tests.)
6. A unit test located at `src/lib.rs` (or `src/app.rs`, matching the constants' location) under `#[cfg(test)] mod tests` asserts both `APP_TITLE == "LaserCAD v2 — bootstrap"` and `DEFAULT_WINDOW_SIZE == [1280.0, 800.0]`. `cargo test --all` includes this test and it passes.
7. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0 against the resulting tree. (Inherits the LCV-002 gate.)
8. No `unwrap()` or `expect()` is introduced in `src/main.rs` or in the body of `lasercad::run()`. The `?` operator on `eframe::Result<()>` is the only error propagation path.

## Expected tests

- **Unit test (criterion 2, 3, 6)**: a single `#[test]` named `bootstrap_window_contract` (or similar) in the constants' module that asserts:
  ```rust
  assert_eq!(APP_TITLE, "LaserCAD v2 — bootstrap");
  assert_eq!(DEFAULT_WINDOW_SIZE, [1280.0_f32, 800.0_f32]);
  ```
  Run via `cargo test --all`.
- **Static check (criterion 2)**: `grep -rnF '"LaserCAD v2 — bootstrap"' src/` returns exactly two matches.
- **Static check (criterion 3)**: `grep -rnF '[1280.0, 800.0]' src/` returns exactly one match (the constant definition).
- **Static check (criterion 1, 8)**: `grep -nE '\b(unwrap|expect)\s*\(' src/main.rs` returns no matches; same grep on the body of `lasercad::run()` in `src/lib.rs` returns no matches outside `#[cfg(test)]`.
- **Build gate (criterion 7)**: run `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all`; exit code must be 0.
- **Manual smoke (criterion 5)**: from the repository root run `cargo run`. Within 5 seconds, a window opens with the title bar text exactly `LaserCAD v2 — bootstrap`. The window is responsive (the close button reacts immediately). Click the OS close button; the process exits with status 0 and prints no stack trace to stderr. Record the result in the demand's `Implementation:` line when the implementer completes the demand.

## Open questions

(none)

## Notes

- The LCV-001 scaffold already opens a window with the correct title at `src/lib.rs::run`. LCV-007 promotes the magic strings/numbers to named `pub const` items and adds the unit test that locks the title contract. The behavior is otherwise unchanged.
- Future demands referencing `APP_TITLE`: LCV-030 (`eframe::App` impl) and LCV-067 (statusbar) may both want to display or compare the title. By making it a `pub const`, those demands import the name rather than copying the literal.
- The 1280×800 default is a round number that fits comfortably on a 14"/15" laptop screen at 100% scale. It is not derived from any CAD-world quantity; window size is a UI-surface concern, not a millimeter concern (units: pixels, see Scope).
- `eframe::Result<()>` is `Result<(), eframe::Error>` — the standard `?`-propagation path. The implementer must not wrap it in `anyhow` or any other error crate (would require a new dep, blocked by LCV-001's dependency lock).
- Phase 3 (LCV-030 onward) will replace the placeholder centered label in `src/app.rs` with the real central panel. The contract LCV-007 freezes is the *window*, not the *contents*; LCV-030 may freely modify the `App::update` body.
