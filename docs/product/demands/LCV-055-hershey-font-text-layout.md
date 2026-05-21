# LCV-055 — Hershey font data + text layout

- **Status**: Ready
- **Phase**: 5
- **Depends on**: LCV-011 (Done), LCV-020
- **Suggested agent**: implementer-rust
- **Suggested model**: sonnet
- **Implementation**: —

## Problem

LaserCAD v2 targets laser engraving: the machine follows a tool path, it does not fill or rasterize. That makes TrueType and OpenType fonts unsuitable — they define outlines (filled regions), not strokes. The TEXT command (LCV-048) needs text that is already a collection of `Line` segments so the laser can trace each stroke directly without any rasterization step. Hershey fonts, published 1967 by Dr. A. V. Hershey and in the public domain, are single-stroke typefaces defined as integer (x, y) coordinate sequences. They embed cleanly as compile-time `static` data in a pure-Rust library, require no font files at runtime, and produce exactly the same `Line` geometry type already used for all other entities. Without this demand, the TEXT command (LCV-048) has no font data and cannot produce any output.

## Scope

- **`src/text/hershey.rs`** — kernel module containing:
  - `pub struct Glyph` with fields:
    - `pub left_bearing: i8` — signed Hershey-grid column of the glyph's left edge.
    - `pub right_bearing: i8` — signed Hershey-grid column of the glyph's right edge.
    - `pub strokes: &'static [&'static [(i8, i8)]]` — each inner slice is one pen-down stroke (sequence of connected points); a new stroke represents a pen-up/pen-down transition.
  - Derives: `Copy, Clone, Debug`.
  - `pub const HERSHEY_CAP_HEIGHT_UNITS: f64` — the measured cap height of capital letters in the embedded Simplex Roman data (the Y value, in CAD Y-up world after negating raw Hershey coordinates, of the topmost stroke point across all uppercase letters; the implementer measures this from the actual data; nominally `16.0` for Simplex Roman).
  - `pub fn glyph(c: char) -> Option<&'static Glyph>` — looks up a character in the embedded table; returns `Some(...)` for all printable ASCII (U+0020–U+007E), `None` otherwise.
  - `mod hershey_data;` (private, data file).
  - No `egui`, `eframe`, `rfd` imports. LOC ≤ 300 (logic only; see Notes on data file).

- **`src/text/hershey_data.rs`** — data-only file:
  - Declares `pub(super) static SIMPLEX_ROMAN: [Option<Glyph>; 95]` (index `i` covers ASCII codepoint `32 + i`, i.e. index 0 = Space U+0020, index 94 = `~` U+007E).
  - Every entry is `Some(...)`. The data is the public-domain Hershey Simplex Roman glyph set, hardcoded as `static` literals.
  - The file contains **only** static data declarations, no logic, no tests. It is explicitly exempt from the 300-LOC hard cap (see Notes).

- **`src/text/layout.rs`** — layout engine:
  - `pub fn layout_text(text: &str, origin: Vec2, height_mm: f64, spacing_factor: f64) -> Vec<Entity>`:
    - Iterates over each `char` in `text`.
    - For each char, calls `glyph(c)`:
      - **Supported char** (`Some(g)`): emits `Entity::Line(...)` for each consecutive point pair within each stroke, translated and scaled to world mm coordinates (see coordinate mapping below). Advances the cursor by the glyph's advance width + gap.
      - **Unsupported char** (`None`): emits no strokes; advances the cursor by `0.6 * height_mm` (a fixed em-width fallback).
    - Returns the flat `Vec<Entity>` of all strokes for all characters in order.
  - **Coordinate mapping** (applied per stroke point `(px, py: i8)`):
    - `scale = height_mm / HERSHEY_CAP_HEIGHT_UNITS`
    - Glyph's local X origin is at `cursor_x - glyph.left_bearing as f64 * scale` (cursor tracks the glyph's left edge).
    - World point: `Vec2 { x: cursor_x + (px - left_bearing) as f64 * scale, y: origin.y + (-(py as f64)) * scale }` — the negation of `py` flips Hershey Y-down into CAD Y-up so text ascends in the positive-Y direction from `origin.y` (the baseline).
    - **Skip degenerate segments**: only emit `Entity::Line` when the two world endpoints differ (i.e., the mapped `p1 != p2` within `EPSILON`). A stroke with only one point emits nothing.
  - **Advance width per glyph**: `(glyph.right_bearing - glyph.left_bearing) as f64 * scale + spacing_factor * height_mm`.
  - `cursor_x` starts at `origin.x` and advances after each character.
  - Empty `text` returns an empty `Vec`.
  - No `egui`, `eframe`, `rfd` imports. LOC ≤ 300.

- **Update `src/text/mod.rs`** — replace the stub body with:
  - `mod hershey;`, `mod layout;`.
  - `pub use hershey::{Glyph, glyph, HERSHEY_CAP_HEIGHT_UNITS};`.
  - `pub use layout::layout_text;`.
  - Module-level doc comment referencing the kernel purity rule.

## Out of scope

- **Multi-line text**: `layout_text` lays out a single line starting at `origin`. Newline characters (`\n`) are treated as unsupported (no strokes, advance by em-width). Multi-line layout is a future demand.
- **Right-to-left, bidirectional, or CJK text**: only left-to-right ASCII.
- **Font variants**: only Simplex Roman. No bold, italic, duplex, complex, Gothic, script, or cyrillic variants. One font; add others when there is a concrete user demand.
- **Kerning pairs**: fixed advance-width spacing only. No per-pair kerning table.
- **TextTool wiring** (LCV-048): this demand ships only the kernel library. The UI tool that reads user input and calls `layout_text` is out of scope here.
- **SVG text elements**: text in v2 is always exploded to `Line` entities before SVG export. No `<text>` in the emitted SVG (AGENTS.md §SVG export: "no live text").
- **Runtime font loading**: everything is compile-time `static` data.
- **Variable height per glyph**: a single `height_mm` applies uniformly to all characters in the call.
- **Vertical text layout**.
- **A `TextEntity` variant in `Entity`**: text is fully decomposed to `Entity::Line` by `layout_text`; no new `Entity` variant is introduced.

## Acceptance criteria

1. `src/text/hershey.rs` exists and defines `pub struct Glyph { pub left_bearing: i8, pub right_bearing: i8, pub strokes: &'static [&'static [(i8, i8)]] }` with `#[derive(Copy, Clone, Debug)]`.

2. `pub const HERSHEY_CAP_HEIGHT_UNITS: f64` is declared in `hershey.rs` and equals the measured cap height of capital letters in the embedded data. When `layout_text("H", Vec2::new(0.0, 0.0), 10.0, 0.0)` is called, every returned `Entity::Line` endpoint satisfies `y >= -EPSILON` and `y <= 10.0 + EPSILON`, and the maximum Y coordinate across all endpoints is `>= 9.5` (cap top lands within 5% of `height_mm`).

3. `pub fn glyph(c: char) -> Option<&'static Glyph>` returns `Some(...)` for every character in the range U+0020–U+007E (printable ASCII), and `None` for U+0000 (NUL) and U+00FF (ÿ).

4. `src/text/hershey_data.rs` exists and declares `pub(super) static SIMPLEX_ROMAN: [Option<Glyph>; 95]` where every element is `Some(...)`. The data covers printable ASCII 32–126 sourced from the public-domain Hershey dataset (Simplex Roman).

5. `src/text/layout.rs` exists and exports `pub fn layout_text(text: &str, origin: Vec2, height_mm: f64, spacing_factor: f64) -> Vec<Entity>`.

6. `layout_text("", origin, 10.0, 0.2)` returns an empty `Vec<Entity>`.

7. `layout_text("I", Vec2::new(0.0, 0.0), 10.0, 0.0)` returns at least one `Entity::Line` (the capital 'I' glyph has strokes in Simplex Roman).

8. Every element in the return value of any `layout_text` call is `Entity::Line(_)`. No `Entity::Circle` or `Entity::Arc` variants appear.

9. No returned `Entity::Line` is degenerate: for every element `Entity::Line(l)`, `l.length() > EPSILON`.

10. Y-axis convention — text is upright in CAD Y-up space: for `layout_text("A", Vec2::new(0.0, 0.0), 10.0, 0.0)`, all endpoint Y values are in `[−EPSILON, 10.0 + EPSILON]` (baseline at `origin.y = 0`, cap at `origin.y + height_mm = 10.0`), and no endpoint Y is negative beyond `EPSILON` (capital 'A' has no descender).

11. X-advance and `spacing_factor`: let `w0 = layout_text("A", origin, 10.0, 0.0)` and `ws = layout_text("A", origin, 10.0, 0.2)`. The maximum X of `ws` strokes equals the maximum X of `w0` strokes (a single character's internal strokes are unaffected by `spacing_factor`). For a two-character string: the minimum X coordinate of `B` strokes in `layout_text("AB", origin, 10.0, 0.0)` is strictly greater than the minimum X of `B` strokes in the same layout with `spacing_factor = 0.0` than when `spacing_factor = 0.2` — i.e., `spacing_factor > 0` increases inter-character gap.

12. Space character (U+0020) produces no strokes: `layout_text(" ", origin, 10.0, 0.0)` returns an empty `Vec<Entity>`. The cursor still advances: the X-span of `layout_text(" A", origin, 10.0, 0.0)` is strictly greater than the X-span of `layout_text("A", origin, 10.0, 0.0)`.

13. Unsupported character: `layout_text("\x01", origin, 10.0, 0.0)` returns an empty `Vec<Entity>` and does not panic.

14. Full printable ASCII round-trip: `layout_text(" !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_\`abcdefghijklmnopqrstuvwxyz{|}~", Vec2::new(0.0, 0.0), 5.0, 0.2)` returns a non-empty `Vec<Entity>` and does not panic.

15. `src/text/mod.rs` re-exports `Glyph`, `glyph`, `HERSHEY_CAP_HEIGHT_UNITS`, and `layout_text` such that `use lasercad::text::{Glyph, glyph, HERSHEY_CAP_HEIGHT_UNITS, layout_text};` resolves from outside the module.

16. Kernel purity: `grep -nE '^use (egui|eframe|rfd)' src/text/hershey.rs src/text/layout.rs src/text/mod.rs` returns no matches.

17. Logic-file LOC: `wc -l src/text/hershey.rs` ≤ 300; `wc -l src/text/layout.rs` ≤ 300; `wc -l src/text/mod.rs` ≤ 50.

18. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all` all exit 0.

## Expected tests

All unit tests live in `#[cfg(test)] mod tests` blocks inside the respective `.rs` files. No new `tests/` integration file is required for this demand.

- **Unit (AC 1)**: test `glyph_struct_fields_accessible` — constructs a `Glyph { left_bearing: -5, right_bearing: 5, strokes: &[] }` via struct literal, asserts fields are readable (proves fields are `pub`).
- **Unit (AC 2, 10)**: test `layout_capital_H_y_bounds` — calls `layout_text("H", Vec2::new(0.0, 0.0), 10.0, 0.0)`, iterates all `Entity::Line` endpoints, asserts `y >= -EPSILON`, `y <= 10.0 + EPSILON`, and `y_max >= 9.5`.
- **Unit (AC 3)**: test `glyph_lookup_printable_ascii_all_some` — loops `' '..='~'` and asserts each returns `Some(_)`.
- **Unit (AC 3)**: test `glyph_lookup_nul_is_none` — asserts `glyph('\0').is_none()`.
- **Unit (AC 3)**: test `glyph_lookup_non_ascii_is_none` — asserts `glyph('ÿ').is_none()`.
- **Unit (AC 4)**: test `simplex_roman_table_has_95_entries_all_some` — directly accesses `hershey::hershey_data::SIMPLEX_ROMAN` (via `pub(crate)` or through the `glyph` function), asserts `len() == 95` and all elements are `Some`.
- **Unit (AC 6)**: test `layout_empty_string_returns_empty` — `assert!(layout_text("", Vec2::default(), 10.0, 0.2).is_empty())`.
- **Unit (AC 7)**: test `layout_capital_I_has_strokes` — `assert!(!layout_text("I", Vec2::default(), 10.0, 0.0).is_empty())`.
- **Unit (AC 8)**: test `layout_returns_only_line_entities` — calls `layout_text("Hello", …)`, asserts every returned `Entity` matches `Entity::Line(_)`.
- **Unit (AC 9)**: test `layout_no_degenerate_lines` — calls `layout_text` on the full printable ASCII string (AC 14), asserts every `Entity::Line(l)` satisfies `l.length() > EPSILON`.
- **Unit (AC 11)**: test `layout_spacing_factor_increases_inter_char_gap` — calls `layout_text("AB", origin, 10.0, 0.0)` and `layout_text("AB", origin, 10.0, 0.5)`, asserts the X-span of the second result is strictly greater.
- **Unit (AC 12)**: test `layout_space_produces_no_strokes` — `assert!(layout_text(" ", Vec2::default(), 10.0, 0.0).is_empty())`.
- **Unit (AC 12)**: test `layout_space_before_A_wider_than_A_alone` — asserts `x_max(layout_text(" A", …)) > x_max(layout_text("A", …))`.
- **Unit (AC 13)**: test `layout_unsupported_char_no_panic_and_empty` — `assert!(layout_text("\x01", Vec2::default(), 10.0, 0.0).is_empty())`.
- **Unit (AC 14)**: test `layout_full_printable_ascii_no_panic` — runs the full-range string, asserts `!is_empty()`.
- **Static check (AC 16)**: documented in the build gate; the `grep` command is run as part of CI validation.
- **Build gate (AC 18)**: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --all` exits 0.

## Open questions

(none)

## Notes

- **Data file LOC exemption**: `hershey_data.rs` will contain several hundred lines of static array literals for 95 glyphs. This file contains **no logic, no functions, and no tests** — it is a data table. The AGENTS.md 300-LOC cap exists to limit logic complexity; a pure data file does not carry that complexity. The exemption is granted here by the product-owner. The implementer must document this exemption with a comment at the top of `hershey_data.rs`: `// Data-only file — exempt from the 300-LOC cap per LCV-055.`

- **Hershey data source**: The public-domain Hershey typefaces (1967, U.S. Naval Weapons Laboratory) are redistributable without restriction. A convenient, well-formatted Rust-usable transcription is included in the GNU `plotutils` package (`hershey.c`) and in several Rust crates (e.g., the unmaintained `hershey-fonts` crate). The implementer should use any public-domain source and verify character coverage (ASCII 32–126). The data is hardcoded, **not** loaded from a crate dependency, to keep the kernel dependency-free for text layout.

- **Hershey coordinate convention**: In the original Hershey data, Y increases downward (screen convention). Baseline is at `y = 0`; capital tops are at negative Y (e.g., `y = -16` for Simplex Roman). Descenders are at positive Y (e.g., `y = +7`). In CAD (Y-up), the negation `-(py)` maps cap tops to `+16` (above baseline) and descenders to `−7` (below baseline). The `HERSHEY_CAP_HEIGHT_UNITS` constant should equal the absolute value of the most-negative raw Y coordinate across all capital-letter strokes in the embedded data.

- **Pen-up encoding in raw Hershey data**: The original Hershey dataset encodes pen-up moves as a special coordinate pair (commonly `(-999, -999)` or the ASCII characters `' R'` in the raw text format). The implementer pre-processes the raw data into the `strokes: &'static [&'static [(i8, i8)]]` format (one sub-slice per pen-down stroke, pen-up boundaries split sub-slices). The raw sentinel values must not appear in the embedded static data.

- **`spacing_factor` defaults**: The `layout_text` function does not have a default argument. The caller (TextTool, LCV-048) is responsible for passing `0.2` as the conventional default. This demand documents `0.2` as the recommended value when calling from tools.

- **Dependency on LCV-020**: `layout_text` returns `Vec<Entity>`, so it imports `crate::document::Entity`. LCV-020 ships `Entity`; if LCV-020 is not yet `Done` when LCV-055 is claimed, the implementer may stub the return type as `Vec<crate::geometry::Line>` temporarily, but the final merge must use `Vec<Entity>`.

- **`src/text/mod.rs` stub**: the file already exists with a placeholder comment `// Submodule arrives with demand LCV-055.` and a single `pub const MODULE`. The implementer replaces the full content.

- **TextTool consumer**: LCV-048 (TextTool) depends on this demand. It calls `layout_text(user_input, anchor_point, text_height, 0.2)` and wraps the result in a `CreateEntities` command. That wiring is out of scope here.

- **v1 reference**: v1 had no shipped TEXT command; the Hershey font integration is a new capability introduced in v2.
