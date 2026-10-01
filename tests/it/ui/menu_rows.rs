//! LCV-166 AC 1–4 — every menu row is `icon slot | label | shortcut`.
//!
//! Menus are opened by real pointer input: a click on a menubar title, a
//! hover on a nested submenu row (egui 0.29.1 opens nested menus on hover).
//! Assertions read the painted frame: text shapes carry their galley, so the
//! label's first glyph x (after the slot's leading space) and the shortcut's
//! right edge are measured, not inferred. A row's *icon slot* is the band left
//! of that first glyph, one row high; a non-text shape whose visual bounding
//! rect lies wholly inside it is "painted in the slot" — a hover highlight
//! spans the whole row and never fits. Only shapes painted after the row's
//! own label count: the popup sits on a later layer than the tool rail it
//! overlaps, and a row paints its icon after its label.

use crate::harness;

use harness::raw_input;
use lasercad::app::App;

/// One painted text shape: its string, its galley rect on screen, and the x
/// of its first glyph (the galley origin plus any leading space), plus its
/// paint order in the frame.
#[derive(Clone, Debug)]
pub(crate) struct Text {
    pub(crate) order: usize,
    pub(crate) text: String,
    pub(crate) rect: egui::Rect,
    pub(crate) label_x: f32,
}

/// What one frame painted: every text shape and the visual bounds of every
/// other leaf shape, each with its paint order.
pub(crate) struct Painted {
    pub(crate) texts: Vec<Text>,
    pub(crate) marks: Vec<(usize, egui::Rect)>,
}

fn collect(order: usize, shape: &egui::Shape, out: &mut Painted) {
    match shape {
        egui::Shape::Vec(inner) => inner.iter().for_each(|s| collect(order, s, out)),
        egui::Shape::Text(t) => {
            let first = t
                .galley
                .rows
                .first()
                .and_then(|r| r.glyphs.first())
                .map_or(0.0, |g| g.pos.x);
            if !t.galley.text().trim().is_empty() {
                out.texts.push(Text {
                    order,
                    text: t.galley.text().to_owned(),
                    rect: t.galley.rect.translate(t.pos.to_vec2()),
                    label_x: t.pos.x + first,
                });
            }
        }
        egui::Shape::Noop => {}
        other => out.marks.push((order, other.visual_bounding_rect())),
    }
}

/// Drive one frame with `events` and return what it painted.
pub(crate) fn paint_frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) -> Painted {
    let out = ctx.run(raw_input(events), |c| app.update_ui(c));
    let mut painted = Painted {
        texts: Vec::new(),
        marks: Vec::new(),
    };
    for (order, clipped) in out.shapes.iter().enumerate() {
        collect(order, &clipped.shape, &mut painted);
    }
    painted
}

/// A few idle frames (popups fade in, areas are placed a frame late); the
/// last one's paint.
pub(crate) fn settle(ctx: &egui::Context, app: &mut App) -> Painted {
    for _ in 0..5 {
        let _ = paint_frame(ctx, app, Vec::new());
    }
    paint_frame(ctx, app, Vec::new())
}

/// The one painted text reading exactly `label` below the menubar, or among
/// several the one whose galley starts nearest `column_x`.
pub(crate) fn find<'a>(painted: &'a Painted, label: &str, column_x: Option<f32>) -> &'a Text {
    let hits: Vec<&Text> = painted
        .texts
        .iter()
        .filter(|t| t.text.trim() == label)
        .collect();
    match (hits.len(), column_x) {
        (0, _) => panic!("`{label}` is not painted"),
        (1, _) => hits[0],
        (_, Some(x)) => hits
            .into_iter()
            .min_by(|a, b| {
                let da = (a.rect.min.x - x).abs();
                let db = (b.rect.min.x - x).abs();
                da.total_cmp(&db)
            })
            .unwrap_or_else(|| panic!("`{label}`")),
        (n, None) => panic!("`{label}` is painted {n} times"),
    }
}

fn centre(t: &Text) -> egui::Pos2 {
    egui::pos2(t.rect.min.x + 4.0, t.rect.center().y)
}

/// Hover then click at `pos`.
pub(crate) fn click(ctx: &egui::Context, app: &mut App, pos: egui::Pos2) {
    let _ = paint_frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = paint_frame(ctx, app, vec![press(true), press(false)]);
}

/// A fresh context at one point per pixel.
pub(crate) fn ctx() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    ctx
}

/// Click menubar title `title` and return the settled frame.
pub(crate) fn open_menu(ctx: &egui::Context, app: &mut App, title: &str) -> Painted {
    let closed = settle(ctx, app);
    let at = centre(find(&closed, title, None));
    click(ctx, app, at);
    settle(ctx, app)
}

/// Hover the submenu row `row` of the open menu and return the settled frame.
pub(crate) fn hover_submenu(
    ctx: &egui::Context,
    app: &mut App,
    open: &Painted,
    row: &str,
) -> Painted {
    let at = centre(find(open, row, None));
    let _ = paint_frame(ctx, app, vec![egui::Event::PointerMoved(at)]);
    settle(ctx, app)
}

/// The rows of one menu, in `labels` order, located on the menu's column.
pub(crate) fn rows<'a>(painted: &'a Painted, labels: &[&str]) -> Vec<&'a Text> {
    let first = find(painted, labels[0], None);
    let column = first.rect.min.x;
    labels
        .iter()
        .map(|l| find(painted, l, Some(column)))
        .collect()
}

/// The shortcut run on `row`'s line, right of its label, if any.
pub(crate) fn shortcut_of<'a>(painted: &'a Painted, row: &Text) -> Option<&'a Text> {
    painted.texts.iter().find(|t| {
        (t.rect.center().y - row.rect.center().y).abs() < 3.0
            && t.rect.min.x > row.rect.max.x
            && t.rect.min.x < row.rect.min.x + 400.0
    })
}

/// The icon slot of `row`: from just left of its galley to just left of its
/// first glyph, one icon high about the row's centre.
pub(crate) fn slot(row: &Text) -> egui::Rect {
    let cy = row.rect.center().y;
    egui::Rect::from_min_max(
        egui::pos2(row.rect.min.x - 3.0, cy - 10.0),
        egui::pos2(row.label_x - 0.5, cy + 10.0),
    )
}

/// Whether any non-text shape painted after `row`'s label lies wholly
/// inside its icon slot.
pub(crate) fn slot_painted(painted: &Painted, row: &Text) -> bool {
    let s = slot(row);
    s.width() > 4.0
        && painted
            .marks
            .iter()
            .any(|(order, m)| *order > row.order && s.contains_rect(*m))
}

/// AC 1 — `rows` of one menu: no tab or hand-drawn arrow in a label, one
/// label x, the expected shortcut beside each, and every shortcut's right
/// edge shared and right of every label.
fn assert_menu(painted: &Painted, spec: &[(&str, Option<&str>)]) {
    let labels: Vec<&str> = spec.iter().map(|(l, _)| *l).collect();
    let found = rows(painted, &labels);
    let x0 = found[0].label_x;
    let mut edges = Vec::new();
    for ((label, key), row) in spec.iter().zip(&found) {
        assert!(!row.text.contains('\t'), "`{label}` carries a tab");
        assert!(!row.text.contains('\u{25b6}'), "`{label}` carries ▶");
        assert!(
            (row.label_x - x0).abs() < 0.5,
            "`{label}` label x {} differs from {x0}",
            row.label_x
        );
        if let Some(key) = key {
            let sc = shortcut_of(painted, row)
                .unwrap_or_else(|| panic!("`{label}` has no shortcut run"));
            assert_eq!(sc.text.trim(), *key, "`{label}` shortcut");
            edges.push(sc.rect.max.x);
        }
    }
    let label_right = found.iter().map(|r| r.rect.max.x).fold(f32::MIN, f32::max);
    for e in &edges {
        assert!((e - edges[0]).abs() <= 1.0, "shortcut edges {edges:?}");
        assert!(*e > label_right, "shortcut edge {e} left of a label");
    }
}

fn assert_slots(painted: &Painted, labels: &[&str], expect: bool) {
    for row in rows(painted, labels) {
        assert_eq!(
            slot_painted(painted, row),
            expect,
            "`{}` slot painted should be {expect}",
            row.text
        );
    }
}

/// AC 1 / AC 2 / AC 4 — File: shortcuts in one column, icons for New, Open
/// and Save, empty slots for Save As and Export Layers, `Open Recent` bare.
#[test]
fn file_menu_rows() {
    let ctx = ctx();
    let mut app = App::default();
    let file = open_menu(&ctx, &mut app, "File");
    assert_menu(
        &file,
        &[
            ("New", Some("Ctrl+N")),
            ("Open…", Some("Ctrl+O")),
            ("Open Recent", None),
            ("Save", Some("Ctrl+S")),
            ("Save As…", Some("Ctrl+Shift+S")),
            ("Export Layers", None),
            ("Bed Size…", None),
            ("Exit", None),
        ],
    );
    assert_slots(&file, &["New", "Open…", "Save"], true);
    assert_slots(&file, &["Save As…", "Export Layers"], false);
    let recent = hover_submenu(&ctx, &mut app, &file, "Open Recent");
    assert_menu(&recent, &[("No recent files", None)]);
}

/// AC 1 / AC 2 — Edit: Undo and Redo carry icons and their keys.
#[test]
fn edit_menu_rows() {
    let ctx = ctx();
    let mut app = App::default();
    let edit = open_menu(&ctx, &mut app, "Edit");
    assert_menu(
        &edit,
        &[
            ("Undo", Some("Ctrl+Z")),
            ("Redo", Some("Ctrl+Y")),
            ("Select All", None),
        ],
    );
    assert_slots(&edit, &["Undo", "Redo"], true);
}

/// AC 1 / AC 2 / AC 3 / AC 4 — View: zoom icons; Grid and Snap checked in
/// the slot, Ortho off and empty, F7 / F3 / F8 in the shortcut column;
/// `Object Snap` lines up and carries no hand-drawn arrow.
#[test]
fn view_menu_rows_and_check_marks() {
    let ctx = ctx();
    let mut app = App::default();
    assert!(app.grid_enabled && app.snap_enabled && !app.ortho_enabled);
    let view = open_menu(&ctx, &mut app, "View");
    assert_menu(
        &view,
        &[
            ("Zoom In", None),
            ("Zoom Out", None),
            ("Fit to Bed", None),
            ("Grid", Some("F7")),
            ("Snap", Some("F3")),
            ("Object Snap", None),
            ("Ortho", Some("F8")),
        ],
    );
    assert_slots(&view, &["Zoom In", "Zoom Out", "Fit to Bed"], true);
    assert_slots(&view, &["Grid", "Snap"], true);
    assert_slots(&view, &["Ortho"], false);

    // The marks follow the flags.
    let ctx = self::ctx();
    let mut app = App::default();
    app.grid_enabled = false;
    app.snap_enabled = false;
    app.ortho_enabled = true;
    let view = open_menu(&ctx, &mut app, "View");
    assert_slots(&view, &["Grid", "Snap"], false);
    assert_slots(&view, &["Ortho"], true);
}

/// AC 1 / AC 3 — View > Object Snap: one label column, a check mark in the
/// slot of every kind that is on and none for Nearest (off by default).
#[test]
fn object_snap_rows_carry_check_marks() {
    let ctx = ctx();
    let mut app = App::default();
    assert!(!app.settings.object_snaps.nearest);
    let view = open_menu(&ctx, &mut app, "View");
    let sub = hover_submenu(&ctx, &mut app, &view, "Object Snap");
    let on = ["Endpoint", "Midpoint", "Center", "Intersection"];
    let spec: Vec<(&str, Option<&str>)> = on
        .iter()
        .chain(["Quadrant", "Perpendicular", "Tangent", "Nearest"].iter())
        .map(|l| (*l, None))
        .collect();
    assert_menu(&sub, &spec);
    assert_slots(&sub, &on, true);
    assert_slots(&sub, &["Nearest"], false);
}

/// AC 1 / AC 2 — Tools: every row carries its rail icon and, when it has
/// one, its key in the shortcut column.
#[test]
fn tools_menu_rows() {
    let ctx = ctx();
    let mut app = App::default();
    let tools = open_menu(&ctx, &mut app, "Tools");
    let spec: &[(&str, Option<&str>)] = &[
        ("Select", None),
        ("Line", Some("L")),
        ("Polyline", Some("P")),
        ("Rect", Some("R")),
        ("Circle", Some("C")),
        ("Arc", Some("A")),
        ("Text", Some("D")),
        ("Move", Some("M")),
        ("Copy", None),
        ("Rotate", None),
        ("Mirror", None),
        ("Scale", None),
        ("Trim", Some("T")),
        ("Extend", Some("X")),
        ("Delete", Some("E")),
        ("Dist", None),
    ];
    assert_menu(&tools, spec);
    let labels: Vec<&str> = spec.iter().map(|(l, _)| *l).collect();
    assert_slots(&tools, &labels, true);
}

/// AC 1 — Format and Help rows go through the same row layout.
#[test]
fn format_and_help_menu_rows() {
    let ctx = ctx();
    let mut app = App::default();
    let help = open_menu(&ctx, &mut app, "Help");
    assert_menu(
        &help,
        &[
            ("Keyboard Shortcuts…", Some("F1")),
            ("About", None),
            ("AI Settings…", None),
        ],
    );
    assert_slots(&help, &["About"], false);
    let ctx = self::ctx();
    let mut app = App::default();
    let format = open_menu(&ctx, &mut app, "Format");
    assert_menu(&format, &[("Layers…", None)]);
}

/// Two committed lines with both selected.
fn app_with_selection() -> App {
    use lasercad::document::{CreateLine, SelectionCommand};
    use lasercad::geometry::{Line, Vec2};
    let mut app = App::default();
    for y in [0.0, 10.0] {
        let line = Line::new(Vec2::new(0.0, y), Vec2::new(10.0, y));
        app.commit(Box::new(CreateLine::new(line)));
    }
    app.commit(Box::new(SelectionCommand::new(vec![0usize, 1])));
    app
}

/// Click Edit > Delete through real pointer input.
fn click_edit_delete(ctx: &egui::Context, app: &mut App) -> Painted {
    let edit = open_menu(ctx, app, "Edit");
    let row = rows(&edit, &["Undo", "Delete"])[1].clone();
    click(ctx, app, centre(&row));
    edit
}

/// AC 2 — Edit > Delete carries its icon and `Del`, erases the selection,
/// and one Ctrl+Z restores both the entities and the selection.
#[test]
fn edit_delete_erases_the_selection_in_one_undo_step() {
    let ctx = ctx();
    let mut app = app_with_selection();
    let edit = click_edit_delete(&ctx, &mut app);
    assert_menu(
        &edit,
        &[
            ("Undo", Some("Ctrl+Z")),
            ("Redo", Some("Ctrl+Y")),
            ("Delete", Some("Del")),
            ("Select All", None),
        ],
    );
    assert_slots(&edit, &["Undo", "Redo", "Delete"], true);
    assert_eq!(app.document.entity_count(), 0, "the selection is erased");
    harness::tap(
        &ctx,
        &mut app,
        egui::Key::Z,
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..egui::Modifiers::NONE
        },
    );
    assert_eq!(app.document.entity_count(), 2, "one Ctrl+Z restores them");
    assert_eq!(app.document.selection.len(), 2, "and their selection");
}

/// AC 2 — with nothing selected Edit > Delete is disabled: a click commits
/// nothing.
#[test]
fn edit_delete_is_disabled_without_a_selection() {
    let ctx = ctx();
    let mut app = app_with_selection();
    app.commit(Box::new(lasercad::document::SelectionCommand::new(Vec::<
        usize,
    >::new(
    ))));
    let before = app.history.revision();
    let _ = click_edit_delete(&ctx, &mut app);
    assert_eq!(app.document.entity_count(), 2);
    assert_eq!(app.history.revision(), before, "nothing is committed");
}
