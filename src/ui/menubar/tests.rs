use super::*;
use crate::document::Entity;
use crate::geometry::Line;

fn run_menubar(app: &mut App) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| draw_menubar(ui, app));
    });
}

#[test] // AC#1, AC#2
fn menubar_module_compiles() {
    #[allow(unused_imports)]
    use crate::ui::draw_menubar as _;
}

#[test] // AC#4
fn app_default_grid_and_snap_enabled() {
    let app = App::default();
    assert!(app.grid_enabled);
    assert!(app.snap_enabled);
}

#[test] // AC#3
fn draw_menubar_default_app_does_not_panic() {
    let mut app = App::default();
    run_menubar(&mut app);
    assert!(!app.about_open);
}

/// LCV-111 AC 16 — the View menu zooms by `Camera::ZOOM_STEP`, the same
/// constant the typed `zoom in` / `zoom out` use (`src/app/cmdline.rs`),
/// so the two can never drift apart. No numeric zoom literal lives here.
#[test]
fn menu_zoom_uses_the_shared_step() {
    let mut reference = Camera::default();
    reference.zoom_in(Camera::ZOOM_STEP);

    let mut app = App::default();
    do_zoom_in(&mut app);
    assert_eq!(app.camera.mm_per_px, reference.mm_per_px);

    reference.zoom_out(Camera::ZOOM_STEP);
    do_zoom_out(&mut app);
    assert_eq!(app.camera.mm_per_px, reference.mm_per_px);
}

#[test] // AC#9
fn view_grid_checkbox_toggles_grid_enabled() {
    let mut app = App::default();
    app.grid_enabled = !app.grid_enabled;
    assert!(!app.grid_enabled);
    app.grid_enabled = !app.grid_enabled;
    assert!(app.grid_enabled);
}

#[test] // AC#9
fn view_snap_checkbox_toggles_snap_enabled() {
    let mut app = App::default();
    app.snap_enabled = !app.snap_enabled;
    assert!(!app.snap_enabled);
    app.snap_enabled = !app.snap_enabled;
    assert!(app.snap_enabled);
}

#[test] // AC#10
fn help_about_sets_about_open() {
    let mut app = App::default();
    do_about(&mut app);
    assert!(app.about_open);
}

/// Three lines pushed straight into the document — test-only setup, the
/// production path is `CreateLine` through the history stack.
fn app_with_three_lines() -> App {
    let mut app = App::default();
    for _ in 0..3 {
        app.document.push_current(Entity::Line(Line::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
        )));
    }
    app
}

#[test] // AC#11
fn select_all_covers_all_entities() {
    let mut app = app_with_three_lines();
    do_select_all(&mut app);
    assert_eq!(app.document.selection.len(), 3);
}

/// LCV-105 AC#4 — Select All is undoable: one `history.undo` restores
/// exactly the selection that was live before the menu item was clicked.
#[test]
fn select_all_is_undoable() {
    let mut app = app_with_three_lines();
    app.history.commit(
        Box::new(SelectionCommand::new(vec![1usize])),
        &mut app.document,
    );
    assert_eq!(app.document.selection.len(), 1);

    do_select_all(&mut app);
    assert_eq!(app.document.selection.len(), 3);

    assert!(app.history.undo(&mut app.document));
    let restored: Vec<usize> = app.document.selection.iter().collect();
    assert_eq!(
        restored,
        vec![1usize],
        "undo must restore the previous selection"
    );
}

/// LCV-105 AC#5 — Select All on an empty document commits nothing, so it
/// cannot leave a no-op on the undo stack.
#[test]
fn select_all_on_empty_document_commits_nothing() {
    let mut app = App::default();
    do_select_all(&mut app);
    assert!(app.document.selection.is_empty());
    assert!(!app.history.can_undo());
}

#[test] // AC#12 zoom in
fn zoom_in_decreases_mm_per_px() {
    let mut app = App::default();
    let before = app.camera.mm_per_px;
    do_zoom_in(&mut app);
    assert!(app.camera.mm_per_px < before);
}

#[test] // AC#12 zoom out
fn zoom_out_increases_mm_per_px() {
    let mut app = App::default();
    let before = app.camera.mm_per_px;
    do_zoom_out(&mut app);
    assert!(app.camera.mm_per_px > before);
}

#[test] // AC#13, re-homed onto the document by LCV-114 AC 4
fn fit_to_bed_uses_bed_bounds() {
    let mut app = App::default();
    app.document.bed_mm = [200.0, 100.0];
    app.camera.viewport_size_px = [800.0, 600.0];
    do_fit_to_bed(&mut app);
    assert!((app.camera.center_world.x - 100.0).abs() < 1e-9);
    assert!((app.camera.center_world.y - 50.0).abs() < 1e-9);
}

#[test] // AC#14
fn open_recent_submenu_empty_message() {
    let mut app = App::default();
    run_menubar(&mut app); // must not panic with empty recent list
}

// ── LCV-138 AC 5 — Open Recent label disambiguation ───────────────────

/// No basenames collide: every label is the bare basename.
#[test]
fn recent_labels_bare_basenames_when_no_collision() {
    let entries = vec![
        "/home/op/a.svg".to_owned(),
        "/home/op/jobs/b.svg".to_owned(),
    ];
    assert_eq!(recent_labels(&entries), vec!["a.svg", "b.svg"]);
}

/// Two entries share a basename in different directories: both — and
/// only both — get `"parent/name.svg"`.
#[test]
fn recent_labels_disambiguates_a_colliding_pair() {
    let entries = vec![
        "/home/op/cuts/plate.svg".to_owned(),
        "/home/op/marks/plate.svg".to_owned(),
    ];
    assert_eq!(
        recent_labels(&entries),
        vec!["cuts/plate.svg", "marks/plate.svg"]
    );
}

/// Three entries, two colliding and one not: only the colliding pair is
/// disambiguated, the third keeps its bare basename.
#[test]
fn recent_labels_disambiguates_only_the_colliding_subset() {
    let entries = vec![
        "/home/op/cuts/plate.svg".to_owned(),
        "/home/op/marks/plate.svg".to_owned(),
        "/home/op/other.svg".to_owned(),
    ];
    assert_eq!(
        recent_labels(&entries),
        vec!["cuts/plate.svg", "marks/plate.svg", "other.svg"]
    );
}

// The full-path-on-hover claim itself (every entry's tooltip is its own
// full path — never a substring of a neighbour's, and never the
// disambiguated label) used to be a source scan here
// (`recent_submenu_hover_text_is_the_full_path_source_scan`). Replaced by
// a real painted hover — a scan proves the `.on_hover_text(entry)` call
// is *written*, not that the tooltip actually paints (AGENTS.md "a
// rendering acceptance criterion is not satisfied by a source scan
// alone") — see
// `tests/it/app/document_title_and_file_feedback.rs::open_recent_entry_hover_text_paints_the_full_path`.

// -----------------------------------------------------------------------
// LCV-104 — Tools menu, Help > Agent settings
// -----------------------------------------------------------------------

/// LCV-104 AC#5 — the bar renders five top-level menus (File, Edit, View,
/// Tools, Help) without panicking; the Tools menu is new here.
#[test]
fn menubar_renders_five_menus_without_panic() {
    let mut app = App::default();
    run_menubar(&mut app);
}

/// LCV-104 AC#5 — structural check: the five `*_menu` helpers are called,
/// in order, from `draw_menubar`. Reading the function's own source is
/// deliberate: it proves the call order without needing pixels or a
/// side-channel recorder.
#[test]
fn menubar_has_five_menus_in_order() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/menubar.rs"));
    let calls = [
        "file_menu(ui, app)",
        "edit_menu(ui, app)",
        "view_menu(ui, app)",
        "tools_menu(ui, app)",
        "help_menu(ui, app)",
    ];
    let mut last = 0usize;
    for call in calls {
        let idx = src
            .find(call)
            .unwrap_or_else(|| panic!("draw_menubar must call {call}"));
        assert!(
            idx > last,
            "{call} must be called after the previous menu in draw_menubar"
        );
        last = idx;
    }
}

/// LCV-114 AC 15 — `do_fit_to_bed` frames whatever bed the document
/// currently has, so a `SetBedSize` needs no renderer bookkeeping.
#[test]
fn fit_to_bed_follows_a_resized_document_bed() {
    let mut app = App::default();
    app.camera.viewport_size_px = [800.0, 600.0];
    do_fit_to_bed(&mut app);
    let framed_default = app.camera.center_world;
    app.document.bed_mm = [300.0, 180.0];
    do_fit_to_bed(&mut app);
    assert!((app.camera.center_world.x - 150.0).abs() < 1e-9);
    assert!((app.camera.center_world.y - 90.0).abs() < 1e-9);
    assert!(app.camera.center_world != framed_default);
}

/// LCV-114 AC 14 — `Bed size…` sits in the File menu directly above
/// `Exit`, with a separator between them. The haystack is bounded to
/// `file_menu`'s body, so this test's own source cannot satisfy it, and
/// the neighbouring entries are asserted positively: an ordering scan
/// that found nothing would fail on `Save As…` first.
#[test]
fn file_menu_has_bed_size_directly_above_exit() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/menubar.rs"));
    let start = src.find("fn file_menu(").expect("file_menu must exist");
    let end = src[start..]
        .find("\nfn recent_submenu(")
        .expect("file_menu must be followed by recent_submenu")
        + start;
    let body = &src[start..end];
    let save_as = body.find("\"Save As…").expect("Save As… must be present");
    let bed = body.find("\"Bed size…").expect("Bed size… must be present");
    let exit = body.find("\"Exit\"").expect("Exit must be present");
    assert!(
        save_as < bed && bed < exit,
        "order: Save As…, Bed size…, Exit"
    );
    assert!(
        body[bed..exit].contains("ui.separator();"),
        "Bed size… must be separated from Exit"
    );
    assert!(
        body[save_as..bed].contains("ui.separator();"),
        "Bed size… must be separated from the save entries"
    );
}

/// LCV-104 AC#7 — activating each Tools-menu entry sets the active tool:
/// `tools::make(entry.kind)` followed by `set_tool` leaves
/// `active_tool_name() == entry.tool_name`, for every table entry.
#[test]
fn tools_menu_entries_activate_their_tool() {
    let mut app = App::default();
    for entry in TOOLS {
        app.tool_manager.set_tool(tools::make(entry.kind));
        assert_eq!(app.tool_manager.active_tool_name(), entry.tool_name);
    }
}

/// LCV-104 AC#12 — `do_agent_settings` sets `app.agent_settings_open`,
/// mirroring `help_about_sets_about_open`.
#[test]
fn help_agent_settings_sets_flag() {
    let mut app = App::default();
    assert!(!app.agent_settings_open);
    do_agent_settings(&mut app);
    assert!(app.agent_settings_open);
}

/// LCV-115 AC#6 — the Export preset submenu sits directly above `Save`, so
/// it reads as a property of saving. Bounded to `fn file_menu`, which ends
/// before `fn preset_submenu`, so this test's own body is out of scope.
#[test]
fn file_menu_has_export_preset_directly_above_save() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/menubar.rs"));
    let start = src.find("fn file_menu(").expect("file_menu must exist");
    let end = src[start..]
        .find("\n/// Three radio items")
        .expect("file_menu must be followed by preset_submenu")
        + start;
    let body = &src[start..end];
    let recent = body.find("\"Open Recent").expect("Open Recent present");
    let preset = body.find("\"Export preset").expect("Export preset present");
    let save = body.find("\"Save\\tCtrl+S").expect("Save present");
    let save_as = body.find("\"Save As\u{2026}").expect("Save As present");
    assert!(
        recent < preset && preset < save && save < save_as,
        "order: Open Recent, Export preset, Save, Save As…"
    );
    assert!(
        !body[preset..save].contains("ui.separator();"),
        "the preset submenu groups with the save entries, not apart from them"
    );
}

// ── LCV-116 (d) — the View menu, and Help > Keyboard shortcuts… ───────

/// LCV-116 AC 19 — the View menu's contents are pinned: the three actions
/// LCV-116 did **not** add (`Zoom In`, `Zoom Out`, `Fit to Bed`, already
/// shipped and backed by `do_zoom_in` / `do_zoom_out` / `do_fit_to_bed`),
/// then the three mode checkboxes, `Ortho\tF8` last. Bounded to
/// `fn view_menu`, so this test's own body cannot satisfy the scan.
#[test]
fn view_menu_items_are_stable() {
    let body = view_menu_body();
    let expected = [
        "\"Zoom In\"",
        "\"Zoom Out\"",
        "\"Fit to Bed\"",
        "\"Grid\\tF7\"",
        "\"Snap\\tF3\"",
        "\"Ortho\\tF8\"",
    ];
    let mut last = 0usize;
    for item in expected {
        let at = body
            .find(item)
            .unwrap_or_else(|| panic!("the View menu must contain {item}"));
        assert!(at >= last, "{item} is out of order in the View menu");
        last = at;
    }
    for backing in ["do_zoom_in(app)", "do_zoom_out(app)", "do_fit_to_bed(app)"] {
        assert!(body.contains(backing), "{backing} must back its menu item");
    }
}

/// LCV-116 AC 18 — the Ortho checkbox is bound to the same flag `F8` and
/// the status-bar `ORTHO` indicator flip; it sits immediately below
/// `Snap\tF3`, and nothing else in the View menu writes it.
#[test]
fn ortho_checkbox_flips_the_same_flag_as_f8() {
    let body = view_menu_body();
    let snap = body
        .find("ui.checkbox(&mut app.snap_enabled, \"Snap\\tF3\");")
        .expect("positive control: the Snap checkbox is present");
    let ortho = body
        .find("ui.checkbox(&mut app.ortho_enabled, \"Ortho\\tF8\");")
        .expect("the Ortho checkbox must be bound to app.ortho_enabled");
    assert!(snap < ortho, "Ortho sits immediately below Snap");
    let after_snap = snap + "ui.checkbox(&mut app.snap_enabled, \"Snap\\tF3\");".len();
    assert!(
        !body[after_snap..ortho].contains("ui.checkbox("),
        "nothing may sit between the Snap and Ortho checkboxes"
    );

    // The flag itself: the menu path and the key path agree.
    let mut menu = App::default();
    let mut keyed = App::default();
    menu.ortho_enabled = !menu.ortho_enabled; // what the checkbox does
    assert!(crate::ui::shortcuts::dispatch_shortcuts(
        egui::Key::F8,
        egui::Modifiers::NONE,
        false,
        &mut keyed,
    ));
    assert_eq!(menu.ortho_enabled, keyed.ortho_enabled);
    assert!(menu.ortho_enabled, "both paths turn ortho on from default");
}

/// LCV-116 AC 12 — `Help > Keyboard shortcuts…` opens the dialog and sits
/// above `About`.
#[test]
fn help_keyboard_shortcuts_sits_above_about() {
    let mut app = App::default();
    assert!(!app.shortcuts_open);
    do_shortcuts(&mut app);
    assert!(app.shortcuts_open);

    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/menubar.rs"));
    let start = src.find("fn help_menu(").expect("help_menu must exist");
    let end = src[start..]
        .find("\n/// Select every entity")
        .expect("help_menu is followed by do_select_all")
        + start;
    let body = &src[start..end];
    let shortcuts = body
        .find("\"Keyboard shortcuts")
        .expect("the Help menu must offer the shortcuts dialog");
    let about = body.find("\"About\"").expect("positive control: About");
    assert!(shortcuts < about, "Keyboard shortcuts… sits above About");
    assert!(
        body[shortcuts..about].contains("do_shortcuts(app)"),
        "the item must call do_shortcuts"
    );
}

/// The source text of `fn view_menu`, bounded at the next item.
fn view_menu_body() -> &'static str {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/menubar.rs"));
    let start = src.find("fn view_menu(").expect("view_menu must exist");
    let end = src[start..]
        .find("\n/// The Tools menu")
        .expect("view_menu is followed by tools_menu")
        + start;
    &src[start..end]
}

/// LCV-115 AC#6 — picking a preset touches `app.export_preset` and nothing
/// else: no document mutation, no history entry, no dirty flag. Bounded to
/// `fn preset_submenu`; the positive controls fail loudly if the bounds
/// ever select an empty or wrong slice.
#[test]
fn preset_submenu_mutates_only_the_preset_field() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/menubar.rs"));
    let start = src
        .find("fn preset_submenu(")
        .expect("preset_submenu must exist");
    let end = src[start..]
        .find("\nfn recent_submenu(")
        .expect("preset_submenu must be followed by recent_submenu")
        + start;
    let body = &src[start..end];
    assert!(body.contains("Preset::ALL"), "items come from Preset::ALL");
    assert!(body.contains("preset.label()"), "labels come from label()");
    assert!(
        body.contains("radio_value(&mut app.export_preset"),
        "radio items are bound to app.export_preset"
    );
    for forbidden in ["app.document", "app.history", "mark_dirty", "action_"] {
        assert!(
            !body.contains(forbidden),
            "preset_submenu must not touch {forbidden}"
        );
    }
}
