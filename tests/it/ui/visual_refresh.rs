//! LCV-184 — visual refresh: theme tokens, status-bar pills, dock prompt.
//!
//! Every rendering claim is read off painted shapes (`FullOutput.shapes`),
//! never off source text; the one source scan is AC 1's literal ban.
//!
//! The token values below restate DESIGN.md §3. They are private to
//! `src/ui/theme.rs` (`pub(crate)`), and its `tokens_match_design_md_chrome_rows`
//! unit test pins those constants to the same §3 rows, so the two copies
//! cannot drift apart without one of the tests failing.

use crate::harness;

use harness::paint::{Run, runs_in};
use harness::raw_input_at;
use harness::scan::{is_test_file, rs_files};
use lasercad::app::App;

const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(0x25, 0x25, 0x25);
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xd0, 0xd0, 0xd0);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0x8c, 0x8c, 0x8c);
const FILL_SELECTED: egui::Color32 = egui::Color32::from_rgb(0x00, 0x5c, 0x80);
const FILL_HOVER: egui::Color32 = egui::Color32::from_rgb(0x46, 0x46, 0x46);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x4f, 0xa3, 0xe0);
const BORDER: egui::Color32 = egui::Color32::from_rgb(0x40, 0x40, 0x40);

/// The screen every test here runs at unless it says otherwise.
const SCREEN: [f32; 2] = harness::SCREEN;

/// One painted frame: its text runs and every leaf shape, `Shape::Vec`
/// flattened.
struct Painted {
    runs: Vec<Run>,
    shapes: Vec<egui::Shape>,
}

fn ctx_and_app() -> (egui::Context, App) {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    (ctx, App::default())
}

fn paint_at(
    ctx: &egui::Context,
    app: &mut App,
    screen: [f32; 2],
    events: Vec<egui::Event>,
) -> Painted {
    let out = ctx.run(raw_input_at(screen, events), |c| app.update_ui(c));
    let runs = runs_in(&out.shapes);
    let mut shapes = Vec::new();
    let mut stack: Vec<egui::Shape> = out.shapes.into_iter().map(|c| c.shape).collect();
    while let Some(shape) = stack.pop() {
        match shape {
            egui::Shape::Vec(inner) => stack.extend(inner),
            other => shapes.push(other),
        }
    }
    Painted { runs, shapes }
}

fn paint(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Painted {
    paint_at(ctx, app, SCREEN, events)
}

/// A settled primary click at `pos`: warm-up hover, then press and release.
fn click(ctx: &egui::Context, app: &mut App, screen: [f32; 2], pos: egui::Pos2) {
    let _ = paint_at(ctx, app, screen, vec![egui::Event::PointerMoved(pos)]);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = paint_at(ctx, app, screen, vec![press(true), press(false)]);
}

/// The single run reading exactly `text`, optionally above `max_y`.
fn run<'a>(runs: &'a [Run], text: &str, max_y: f32) -> &'a Run {
    let found: Vec<&Run> = runs
        .iter()
        .filter(|r| r.text.trim() == text && r.pos.y < max_y)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "`{text}` must paint once, saw {}",
        found.len()
    );
    found[0]
}

/// Centre of a run's text box.
fn centre(run: &Run) -> egui::Pos2 {
    egui::pos2(run.pos.x + 6.0, run.pos.y + run.height / 2.0)
}

/// Every painted `RectShape`.
fn rects(shapes: &[egui::Shape]) -> Vec<&egui::epaint::RectShape> {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Rect(r) => Some(r),
            _ => None,
        })
        .collect()
}

// ── AC 1 — no colour literal in src/ui/ outside theme.rs ──────────────

/// `Color32` constructors that take channel values.
const CONSTRUCTORS: [&str; 7] = [
    concat!("Color32::", "from_rgb("),
    concat!("Color32::", "from_rgba_unmultiplied("),
    concat!("Color32::", "from_rgba_premultiplied("),
    concat!("Color32::", "from_gray("),
    concat!("Color32::", "from_black_alpha("),
    concat!("Color32::", "from_white_alpha("),
    concat!("Color32::", "from_additive_luminance("),
];

/// egui's named colours (`TRANSPARENT` is "no colour", not a colour).
const NAMED: [&str; 20] = [
    "BLACK",
    "DARK_GRAY",
    "GRAY",
    "LIGHT_GRAY",
    "WHITE",
    "BROWN",
    "DARK_RED",
    "RED",
    "LIGHT_RED",
    "YELLOW",
    "ORANGE",
    "LIGHT_YELLOW",
    "KHAKI",
    "DARK_GREEN",
    "GREEN",
    "LIGHT_GREEN",
    "DARK_BLUE",
    "BLUE",
    "LIGHT_BLUE",
    "GOLD",
];

/// Colour literals on the code lines of `body`: a constructor whose first
/// argument starts with a digit, or a named `Color32` constant.
fn literals(body: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for line in body.lines().filter(|l| !l.trim_start().starts_with("//")) {
        for ctor in CONSTRUCTORS {
            for (at, _) in line.match_indices(ctor) {
                let arg = line[at + ctor.len()..].trim_start();
                if arg.starts_with(|c: char| c.is_ascii_digit()) {
                    hits.push(line.trim().to_owned());
                }
            }
        }
        let prefix = concat!("Color32", "::");
        for (at, _) in line.match_indices(prefix) {
            let rest = &line[at + prefix.len()..];
            let ident: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if NAMED.contains(&ident.as_str()) {
                hits.push(line.trim().to_owned());
            }
        }
    }
    hits
}

/// AC 1 — no file in `src/ui/` but `theme.rs` builds a `Color32` from a
/// literal; `theme.rs` itself is the positive control.
#[test]
fn ac1_no_colour_literal_in_ui_outside_theme() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    files.retain(|p| !is_test_file(p));
    assert!(files.len() >= 15, "only {} files scanned", files.len());

    assert_eq!(
        literals("x(Color32::from_gray(7))").len(),
        1,
        "ctor control"
    );
    assert_eq!(literals("x(Color32::RED)").len(), 1, "named control");
    assert!(literals("x(Color32::from_rgb(r, g, b))").is_empty());

    let mut theme_hits = 0;
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let body = src.split("\n#[cfg(test)]").next().unwrap_or_default();
        let hits = literals(body);
        if path.ends_with("theme.rs") {
            theme_hits = hits.len();
        } else {
            assert!(hits.is_empty(), "{}: {hits:?}", path.display());
        }
    }
    assert!(
        theme_hits >= 9,
        "positive control: theme.rs holds the tokens"
    );
}

// ── AC 2 / AC 3 — painted menu frame, no shadow, no accent fill ───────

/// AC 2 — the open File menu's frame is 1 pt `border`, 4 pt corners, and no
/// shape in the frame is blurred (a shadow).
#[test]
fn ac2_file_menu_frame_is_flat_with_one_border() {
    let (ctx, mut app) = ctx_and_app();
    let first = paint(&ctx, &mut app, Vec::new());
    let file = run(&first.runs, "File", 30.0);
    click(&ctx, &mut app, SCREEN, centre(file));
    // egui fades a popup in over a few frames; let it settle.
    for _ in 0..30 {
        let _ = paint(&ctx, &mut app, Vec::new());
    }
    let open = paint(&ctx, &mut app, Vec::new());
    let item = run(&open.runs, "Export Layers", SCREEN[1]).pos;

    // The menu frame: the smallest panel-filled rect around a menu item.
    let frame = rects(&open.shapes)
        .into_iter()
        .filter(|r| r.fill == BG_PANEL && r.rect.contains(item))
        .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
        .expect("the menu frame must paint");
    assert!(frame.rect.width() < 400.0, "a menu, not a panel: {frame:?}");
    assert_eq!(frame.stroke, egui::Stroke::new(1.0_f32, BORDER));
    assert_eq!(frame.rounding, egui::Rounding::same(4.0));
    let blurred = rects(&open.shapes)
        .into_iter()
        .filter(|r| r.blur_width > 0.0)
        .count();
    assert_eq!(blurred, 0, "no shadow may paint");
}

/// AC 3 — with SNAP on, LINE active and the editor focused, nothing paints
/// an `accent` fill; the active rail button's `fill.selected` is the control.
#[test]
fn ac3_no_shape_is_filled_with_accent() {
    let (ctx, mut app) = ctx_and_app();
    app.snap_enabled = true;
    harness::submit_command(&ctx, &mut app, "line");
    harness::type_command(&ctx, &mut app, "1");
    let _ = paint(&ctx, &mut app, Vec::new());
    let painted = paint(&ctx, &mut app, Vec::new());
    assert_eq!(app.tool_manager.active_tool_name(), "LINE");
    assert!(app.command_line_focused, "the editor holds focus");

    // A glyph's own ink is foreground, not a fill: the LINE icon's 2.5 pt
    // vertex markers are painted in the icon colour, which is `accent` on the
    // active button (AC 3, AC 8). A fill is a surface wider than that.
    let fills: Vec<egui::Color32> = painted
        .shapes
        .iter()
        .filter(|s| s.visual_bounding_rect().size().max_elem() > 4.0)
        .filter_map(|s| match s {
            egui::Shape::Rect(r) => Some(r.fill),
            egui::Shape::Circle(c) => Some(c.fill),
            egui::Shape::Path(p) => Some(p.fill),
            _ => None,
        })
        .collect();
    assert!(fills.contains(&FILL_SELECTED), "positive control");
    assert!(!fills.contains(&ACCENT), "accent is never a fill");
}

// ── AC 4 — mode pills ──────────────────────────────────────────────────

/// The one text shape reading exactly `text` in the bottom `band` points of
/// a `screen`-tall window (the status bar is the last row).
fn bottom_text<'a>(
    shapes: &'a [egui::Shape],
    text: &str,
    screen: [f32; 2],
    band: f32,
) -> &'a egui::epaint::TextShape {
    let found: Vec<&egui::epaint::TextShape> = shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) => Some(t),
            _ => None,
        })
        .filter(|t| t.galley.text().trim() == text && t.pos.y > screen[1] - band)
        .collect();
    assert_eq!(found.len(), 1, "`{text}` must paint once near the bottom");
    found[0]
}

/// The colours a text shape paints its sections in (egui resolves a
/// `PLACEHOLDER` section to the shape's `fallback_color`).
fn text_colours(t: &egui::epaint::TextShape) -> Vec<egui::Color32> {
    if let Some(c) = t.override_text_color {
        return vec![c];
    }
    t.galley
        .job
        .sections
        .iter()
        .map(|s| match s.format.color {
            egui::Color32::PLACEHOLDER => t.fallback_color,
            c => c,
        })
        .collect()
}

/// The smallest rect shape enclosing text shape `t`.
fn rect_around<'a>(
    shapes: &'a [egui::Shape],
    t: &egui::epaint::TextShape,
) -> &'a egui::epaint::RectShape {
    let text = egui::Rect::from_min_size(t.pos, t.galley.size());
    rects(shapes)
        .into_iter()
        .filter(|r| r.rect.contains_rect(text.shrink(0.5)))
        .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
        .expect("a rect must enclose the text")
}

/// AC 4 — SNAP on: `fill.selected` pill, `accent` text. GRID off: no fill,
/// a 1 pt `border` outline, `text.muted` text.
#[test]
fn ac4_on_and_off_pills_paint_their_state() {
    let (ctx, mut app) = ctx_and_app();
    app.snap_enabled = true;
    app.grid_enabled = false;
    let _ = paint(&ctx, &mut app, Vec::new());
    let painted = paint(&ctx, &mut app, Vec::new());

    let snap = bottom_text(&painted.shapes, "SNAP", SCREEN, 40.0);
    let pill = rect_around(&painted.shapes, snap);
    assert_eq!(pill.fill, FILL_SELECTED, "SNAP on: {pill:?}");
    assert_eq!(text_colours(snap), vec![ACCENT]);

    let grid = bottom_text(&painted.shapes, "GRID", SCREEN, 40.0);
    let pill = rect_around(&painted.shapes, grid);
    assert!(pill.rect.height() < 30.0, "a pill, not the panel: {pill:?}");
    assert_eq!(pill.fill.a(), 0, "GRID off has no fill: {pill:?}");
    assert_eq!(pill.stroke, egui::Stroke::new(1.0_f32, BORDER));
    assert_eq!(text_colours(grid), vec![TEXT_MUTED]);
}

/// AC 4 — a click on a pill flips its own flag and no other.
#[test]
fn ac4_a_click_on_each_pill_flips_only_its_flag() {
    for (label, index) in [("SNAP", 0), ("GRID", 1), ("ORTHO", 2)] {
        let (ctx, mut app) = ctx_and_app();
        let painted = paint(&ctx, &mut app, Vec::new());
        let text = bottom_text(&painted.shapes, label, SCREEN, 40.0);
        let at = egui::Rect::from_min_size(text.pos, text.galley.size()).center();
        let flags = |a: &App| [a.snap_enabled, a.grid_enabled, a.ortho_enabled];
        let before = flags(&app);
        click(&ctx, &mut app, SCREEN, at);
        let after = flags(&app);
        for i in 0..3 {
            assert_eq!(after[i] != before[i], i == index, "{label}: {after:?}");
        }
    }
}

// ── AC 5 — separators and monospace coordinates ───────────────────────

/// A panel's outer rect, as egui stored it last frame.
fn panel_rect(ctx: &egui::Context, id: &str) -> egui::Rect {
    egui::containers::panel::PanelState::load(ctx, egui::Id::new(id))
        .unwrap_or_else(|| panic!("panel `{id}` must have stored its state"))
        .rect
}

/// The text shapes inside `area`, left to right.
fn texts_in(shapes: &[egui::Shape], area: egui::Rect) -> Vec<&egui::epaint::TextShape> {
    let mut found: Vec<&egui::epaint::TextShape> = shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Text(t) if area.contains(t.pos) => Some(t),
            _ => None,
        })
        .filter(|t| !t.galley.text().trim().is_empty())
        .collect();
    found.sort_by(|a, b| a.pos.x.total_cmp(&b.pos.x));
    found
}

/// Vertical 1 pt `border` line segments inside `area`, by x.
fn vertical_rules(shapes: &[egui::Shape], area: egui::Rect) -> Vec<f32> {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::LineSegment { points, stroke }
                if points[0].x == points[1].x
                    && area.contains(points[0])
                    && stroke.width == 1.0
                    && matches!(stroke.color, egui::epaint::ColorMode::Solid(c) if c == BORDER) =>
            {
                Some(points[0].x)
            }
            _ => None,
        })
        .collect()
}

/// AC 5 — a 1 pt vertical rule between every pair of adjacent status-bar
/// segments, in every mode state.
#[test]
fn ac5_a_rule_separates_every_adjacent_pair_of_segments() {
    for on in [true, false] {
        let (ctx, mut app) = ctx_and_app();
        (app.snap_enabled, app.grid_enabled, app.ortho_enabled) = (on, on, on);
        let _ = paint(&ctx, &mut app, Vec::new());
        let painted = paint(&ctx, &mut app, Vec::new());
        let bar = panel_rect(&ctx, "statusbar");
        let segments = texts_in(&painted.shapes, bar);
        assert!(segments.len() >= 8, "eight segments: {}", segments.len());
        let rules = vertical_rules(&painted.shapes, bar);
        for pair in segments.windows(2) {
            let left = pair[0].pos.x + pair[0].galley.size().x;
            let right = pair[1].pos.x;
            let between = rules.iter().filter(|x| **x > left && **x < right).count();
            assert_eq!(
                between,
                1,
                "one rule between {:?} and {:?}",
                pair[0].galley.text(),
                pair[1].galley.text()
            );
        }
    }
}

/// AC 5 — the coordinate run is laid out in egui's monospace family.
#[test]
fn ac5_coordinates_use_the_monospace_font() {
    let (ctx, mut app) = ctx_and_app();
    let _ = paint(&ctx, &mut app, Vec::new());
    let painted = paint(&ctx, &mut app, Vec::new());
    let bar = panel_rect(&ctx, "statusbar");
    let coords = texts_in(&painted.shapes, bar)
        .into_iter()
        .find(|t| t.galley.text().starts_with("X:"))
        .expect("the coordinate readout paints");
    for section in &coords.galley.job.sections {
        assert_eq!(section.format.font_id.family, egui::FontFamily::Monospace);
    }
    let tool = texts_in(&painted.shapes, bar)[1];
    assert_eq!(
        tool.galley.job.sections[0].format.font_id.family,
        egui::FontFamily::Proportional,
        "control: other segments stay proportional"
    );
}

// ── AC 6 — coloured prompt ────────────────────────────────────────────

/// The dock's painted prompt `prompt`, as `(section text, colour)` pairs.
fn prompt_sections(
    ctx: &egui::Context,
    app: &mut App,
    prompt: &str,
) -> Vec<(String, egui::Color32)> {
    assert_eq!(app.tool_manager.active_status_text(), prompt);
    let _ = paint(ctx, app, Vec::new());
    let painted = paint(ctx, app, Vec::new());
    let dock = panel_rect(ctx, "command_line");
    let shape = texts_in(&painted.shapes, dock)
        .into_iter()
        .find(|t| t.galley.text() == prompt)
        .unwrap_or_else(|| panic!("the dock must paint {prompt:?}"));
    let colours = text_colours(shape);
    let text = shape.galley.text();
    shape
        .galley
        .job
        .sections
        .iter()
        .zip(colours)
        .map(|(s, c)| (text[s.byte_range.clone()].to_owned(), c))
        .collect()
}

/// AC 6 — LINE: the verb in `accent`, the request in `text.primary`.
#[test]
fn ac6_line_prompt_paints_verb_accent_and_request_primary() {
    let (ctx, mut app) = ctx_and_app();
    harness::submit_command(&ctx, &mut app, "line");
    let sections = prompt_sections(&ctx, &mut app, "LINE  Specify first point:");
    assert_eq!(
        sections,
        vec![
            ("LINE".to_owned(), ACCENT),
            ("  Specify first point:".to_owned(), TEXT_PRIMARY),
        ]
    );
}

/// AC 6 — MIRROR's confirm: `[Yes/No]` and `<N>` in `text.muted`.
#[test]
fn ac6_mirror_confirm_paints_options_muted() {
    let (ctx, mut app) = ctx_and_app();
    let _ = paint(&ctx, &mut app, Vec::new());
    let line = lasercad::geometry::Line::new(
        lasercad::geometry::Vec2::new(10.0, 0.0),
        lasercad::geometry::Vec2::new(20.0, 5.0),
    );
    app.commit(Box::new(lasercad::document::CreateLine::new(line)));
    app.document.selection.add(0);
    for step in ["mirror", "0,0", "0,10"] {
        harness::submit_command(&ctx, &mut app, step);
    }
    let prompt = "MIRROR  Erase source objects? [Yes/No] <N>:";
    let sections = prompt_sections(&ctx, &mut app, prompt);
    let colour_of = |part: &str| {
        sections
            .iter()
            .find(|(t, _)| t == part)
            .unwrap_or_else(|| panic!("no section {part:?}: {sections:?}"))
            .1
    };
    assert_eq!(colour_of("MIRROR"), ACCENT);
    assert_eq!(colour_of("[Yes/No]"), TEXT_MUTED);
    assert_eq!(colour_of("<N>"), TEXT_MUTED);
    assert_eq!(colour_of("  Erase source objects? "), TEXT_PRIMARY);
}

/// AC 6 — the idle `Command:` prompt has no verb: all `text.primary`.
#[test]
fn ac6_command_prompt_is_all_primary() {
    let (ctx, mut app) = ctx_and_app();
    let sections = prompt_sections(&ctx, &mut app, "Command:");
    assert!(!sections.is_empty());
    for (text, colour) in sections {
        assert_eq!(colour, TEXT_PRIMARY, "{text:?}");
    }
}

// ── AC 7 — editor frame ───────────────────────────────────────────────

/// The stroke of the smallest stroked rect enclosing the command editor.
fn editor_frame_stroke(ctx: &egui::Context, painted: &Painted) -> egui::Stroke {
    let editor = ctx
        .read_response(egui::Id::new("command_line_editor"))
        .expect("the editor must have painted")
        .rect;
    rects(&painted.shapes)
        .into_iter()
        .filter(|r| r.stroke.width > 0.0 && r.rect.contains_rect(editor))
        .min_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
        .expect("a stroked frame must enclose the editor")
        .stroke
}

/// AC 7 — 1 pt `border` while unfocused, `accent` once a typed character
/// has focused the editor.
#[test]
fn ac7_editor_frame_turns_accent_with_focus() {
    let (ctx, mut app) = ctx_and_app();
    // The dock settles on the second frame; `read_response` may report the
    // frame before the one painted, so paint three.
    for _ in 0..2 {
        let _ = paint(&ctx, &mut app, Vec::new());
    }
    let idle = paint(&ctx, &mut app, Vec::new());
    assert!(!app.command_line_focused);
    assert_eq!(
        editor_frame_stroke(&ctx, &idle),
        egui::Stroke::new(1.0_f32, BORDER)
    );

    harness::type_command(&ctx, &mut app, "1");
    let _ = paint(&ctx, &mut app, Vec::new());
    let focused = paint(&ctx, &mut app, Vec::new());
    assert!(app.command_line_focused, "the typed character focused it");
    assert_eq!(
        editor_frame_stroke(&ctx, &focused),
        egui::Stroke::new(1.0_f32, ACCENT)
    );
}

// ── AC 8 — rail buttons ────────────────────────────────────────────────

/// Centre of the rail button at `(column, row)`: 4 pt margin, 32 pt
/// buttons, 4 pt gaps (pinned by `tests/it/ui/icon_tool_rail.rs`).
fn rail_centre(rail: egui::Rect, column: usize, row: usize) -> egui::Pos2 {
    let step = 32.0 + 4.0;
    rail.min
        + egui::vec2(4.0 + column as f32 * step, 4.0 + row as f32 * step)
        + egui::vec2(16.0, 16.0)
}

/// The smallest rect shape filled `fill` under `at`.
fn filled_under(
    shapes: &[egui::Shape],
    fill: egui::Color32,
    at: egui::Pos2,
) -> Vec<egui::epaint::RectShape> {
    rects(shapes)
        .into_iter()
        .filter(|r| r.fill == fill && r.rect.contains(at))
        .cloned()
        .collect()
}

/// Whether any stroked shape inside `area` is drawn in `colour`.
fn stroked_in(shapes: &[egui::Shape], area: egui::Rect, colour: egui::Color32) -> bool {
    let solid = |c: &egui::epaint::ColorMode| matches!(c, egui::epaint::ColorMode::Solid(x) if *x == colour);
    shapes.iter().any(|s| match s {
        egui::Shape::LineSegment { points, stroke } => {
            area.contains(points[0]) && solid(&stroke.color)
        }
        egui::Shape::Path(p) => {
            p.points.first().is_some_and(|q| area.contains(*q)) && solid(&p.stroke.color)
        }
        egui::Shape::Circle(c) => area.contains(c.center) && c.stroke.color == colour,
        _ => false,
    })
}

/// AC 8 — the active tool's button is `fill.selected` with 3 pt corners and
/// an `accent` icon; a hovered one is `fill.hover`, 3 pt corners.
#[test]
fn ac8_rail_buttons_use_the_theme_state_fills() {
    let (ctx, mut app) = ctx_and_app();
    harness::submit_command(&ctx, &mut app, "line");
    let _ = paint(&ctx, &mut app, Vec::new());
    let rail = panel_rect(&ctx, "toolbar");
    let line = rail_centre(rail, 0, 1);
    let circle = rail_centre(rail, 0, 4);
    let painted = paint(&ctx, &mut app, vec![egui::Event::PointerMoved(circle)]);
    let painted_hover = paint(&ctx, &mut app, Vec::new());

    let active = filled_under(&painted.shapes, FILL_SELECTED, line);
    assert_eq!(active.len(), 1, "one selected fill under LINE: {active:?}");
    assert_eq!(active[0].rounding, egui::Rounding::same(3.0));
    assert!(
        stroked_in(&painted.shapes, active[0].rect, ACCENT),
        "the active icon is drawn in accent"
    );

    let hovered = filled_under(&painted_hover.shapes, FILL_HOVER, circle);
    assert_eq!(hovered.len(), 1, "one hover fill under CIRCLE: {hovered:?}");
    assert_eq!(hovered[0].rounding, egui::Rounding::same(3.0));
    assert_eq!(
        app.tool_manager.active_tool_name(),
        "LINE",
        "hover does not click"
    );
}

// ── AC 9 — DESIGN.md §2 budgets at three sizes ────────────────────────

/// AC 9 — at 800×600, 1024×600 and 1280×800, with the modes all on and all
/// off: every menu title inside the window, every status segment inside the
/// bar and the bar ≤ 56 pt, the dock ≤ 64 pt, and all 17 rail buttons
/// inside the unscrolled rail.
#[test]
fn ac9_chrome_fits_at_three_sizes_in_both_mode_states() {
    for screen in [[800.0, 600.0], [1024.0, 600.0], [1280.0, 800.0]] {
        for on in [true, false] {
            let (ctx, mut app) = ctx_and_app();
            (app.snap_enabled, app.grid_enabled, app.ortho_enabled) = (on, on, on);
            for _ in 0..2 {
                let _ = paint_at(&ctx, &mut app, screen, Vec::new());
            }
            let painted = paint_at(&ctx, &mut app, screen, Vec::new());
            let case = format!("{screen:?} modes {on}");
            let right_edge = |t: &egui::epaint::TextShape| t.pos.x + t.galley.size().x;

            let menubar = panel_rect(&ctx, "menubar");
            let titles = texts_in(&painted.shapes, menubar);
            let names: Vec<&str> = titles.iter().map(|t| t.galley.text()).collect();
            assert_eq!(
                names,
                ["File", "Edit", "View", "Format", "Tools", "Help"],
                "{case}"
            );
            for t in &titles {
                assert!(
                    right_edge(t) <= screen[0],
                    "{case}: menu {:?}",
                    t.galley.text()
                );
            }

            let bar = panel_rect(&ctx, "statusbar");
            assert!(bar.height() <= 56.0, "{case}: bar {}", bar.height());
            let segments = texts_in(&painted.shapes, bar);
            assert!(segments.len() >= 8, "{case}: {} segments", segments.len());
            for t in segments {
                assert!(
                    t.pos.x >= bar.left() && right_edge(t) <= bar.right(),
                    "{case}: segment {:?} clipped",
                    t.galley.text()
                );
                assert!(!t.galley.elided, "{case}: {:?} elided", t.galley.text());
            }

            let dock = panel_rect(&ctx, "command_line");
            assert!(dock.height() <= 64.0, "{case}: dock {}", dock.height());

            let rail = panel_rect(&ctx, "toolbar");
            let mut buttons: Vec<egui::Pos2> = (0..7).map(|r| rail_centre(rail, 0, r)).collect();
            buttons.extend((0..9).map(|r| rail_centre(rail, 1, r)));
            let ai = texts_in(&painted.shapes, rail)
                .into_iter()
                .find(|t| t.galley.text() == "AI")
                .expect("the AI toggle paints");
            buttons.push(egui::Rect::from_min_size(ai.pos, ai.galley.size()).center());
            assert_eq!(buttons.len(), 17);
            for c in buttons {
                let button = egui::Rect::from_center_size(c, egui::Vec2::splat(32.0));
                assert!(
                    rail.contains_rect(button),
                    "{case}: button at {c:?} outside {rail:?}"
                );
            }
        }
    }
}
