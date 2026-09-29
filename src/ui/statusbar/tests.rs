use super::*;

/// LCV-067 AC — `format_coords(None)` produces the em-dash placeholder.
#[test]
fn format_coords_none_returns_dash_placeholder() {
    assert_eq!(format_coords(None), "X: \u{2014}  Y: \u{2014}");
}

/// LCV-067 AC — positive coordinates are formatted to two decimal places,
/// right-aligned in a 6-char field. LCV-140 AC 4 — each axis carries an
/// explicit `mm` unit, directly after the number.
#[test]
fn format_coords_some_positive_values() {
    let s = format_coords(Some(Vec2::new(123.45, 67.89)));
    assert_eq!(s, "X: 123.45mm  Y:  67.89mm");
}

/// LCV-067 AC — negative x value is formatted correctly (sign included in
/// the 6-char field, extending it naturally). LCV-140 AC 4 — `mm` unit.
#[test]
fn format_coords_some_negative_x() {
    let s = format_coords(Some(Vec2::new(-5.0, 0.0)));
    assert_eq!(s, "X:  -5.00mm  Y:   0.00mm");
}

/// LCV-067 AC — zero coordinates produce all-zero output, not "—".
/// LCV-140 AC 4 — `mm` unit.
#[test]
fn format_coords_some_zero() {
    let s = format_coords(Some(Vec2::new(0.0, 0.0)));
    assert_eq!(s, "X:   0.00mm  Y:   0.00mm");
}

/// LCV-067 AC — large values are not truncated (no width cap).
#[test]
fn format_coords_large_values_not_truncated() {
    let s = format_coords(Some(Vec2::new(1234.56, 9876.54)));
    // 1234.56 is 7 chars — the field is at least that wide.
    assert!(s.contains("1234.56"), "x value present");
    assert!(s.contains("9876.54"), "y value present");
}

/// LCV-140 AC 4 / AC 6 — a six-digit signed coordinate pair renders with
/// its `mm` unit intact and undistorted, the exact fixture AC 6 names.
#[test]
fn format_coords_six_digit_signed_pair_keeps_its_mm_unit() {
    let s = format_coords(Some(Vec2::new(-1234.56, -1234.56)));
    assert_eq!(s, "X: -1234.56mm  Y: -1234.56mm");
}

// ── LCV-116 (a) — clickable mode indicators ───────────────────────────

/// LCV-116 AC 2 — the three modes, their labels and their render order.
#[test]
fn the_three_modes_have_stable_labels_in_order() {
    assert_eq!(Mode::ALL.len(), 3);
    let labels: Vec<&str> = Mode::ALL.iter().map(|m| m.label()).collect();
    assert_eq!(labels, vec!["SNAP", "GRID", "ORTHO"]);
}

/// LCV-116 AC 2 — `is_on` reads the flag the mode names, and only that one.
#[test]
fn is_on_reads_the_flag_the_mode_names() {
    let app = App {
        snap_enabled: true,
        grid_enabled: false,
        ortho_enabled: true,
        ..App::default()
    };
    assert!(Mode::Snap.is_on(&app));
    assert!(!Mode::Grid.is_on(&app));
    assert!(Mode::Ortho.is_on(&app));
}

/// LCV-116 AC 3 — clicking one indicator flips exactly its own flag: the
/// other two modes, the document, the history and `dirty_since` are all
/// untouched. One case per mode, both directions.
#[test]
fn toggle_click_flips_only_its_own_flag() {
    for mode in Mode::ALL {
        for _round in 0..2 {
            let mut app = App::default();
            app.history.commit(
                Box::new(crate::document::CreateLine::new(
                    crate::geometry::Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)),
                )),
                &mut app.document,
            );
            app.mark_clean();
            if _round == 1 {
                // Second round starts from the opposite state, so the
                // "flip" is exercised in both directions.
                apply_toggle(&mut app, mode);
                app.mark_clean();
            }

            let before: Vec<bool> = Mode::ALL.iter().map(|m| m.is_on(&app)).collect();
            let entities = app.document.entity_count();
            let revision = app.history.revision();

            apply_toggle(&mut app, mode);

            let after: Vec<bool> = Mode::ALL.iter().map(|m| m.is_on(&app)).collect();
            for (i, other) in Mode::ALL.iter().enumerate() {
                if *other == mode {
                    assert_ne!(before[i], after[i], "{mode:?} must flip");
                } else {
                    assert_eq!(before[i], after[i], "{other:?} must not move");
                }
            }
            assert_eq!(app.document.entity_count(), entities, "no document change");
            assert_eq!(app.history.revision(), revision, "no history entry");
            assert!(app.dirty_since.is_none(), "no change to dirty_since");
        }
    }
}

/// LCV-116 AC 3 — a toggle is the same single-flag mutation `F3` / `F7` /
/// `F8` perform. Asserted against `dispatch_shortcuts` itself, so the two
/// paths cannot drift: same start state, same end state.
#[test]
fn a_click_and_its_function_key_agree() {
    for (mode, key) in [
        (Mode::Snap, egui::Key::F3),
        (Mode::Grid, egui::Key::F7),
        (Mode::Ortho, egui::Key::F8),
    ] {
        let mut clicked = App::default();
        let mut keyed = App::default();
        apply_toggle(&mut clicked, mode);
        assert!(crate::ui::shortcuts::dispatch_shortcuts(
            key,
            egui::Modifiers::NONE,
            false,
            &mut keyed,
        ));
        assert_eq!(clicked.snap_enabled, keyed.snap_enabled, "{mode:?}");
        assert_eq!(clicked.grid_enabled, keyed.grid_enabled, "{mode:?}");
        assert_eq!(clicked.ortho_enabled, keyed.ortho_enabled, "{mode:?}");
    }
}

// ── LCV-116 (b) — autosave indicator ──────────────────────────────────

/// LCV-116 AC 8 — all four input combinations map to the three exact
/// strings, and "pending" wins over "ever saved".
#[test]
fn format_autosave_states() {
    assert_eq!(format_autosave(true, false), "\u{25cf} autosave pending");
    assert_eq!(format_autosave(true, true), "\u{25cf} autosave pending");
    assert_eq!(format_autosave(false, true), "\u{25cb} autosaved");
    assert_eq!(format_autosave(false, false), "\u{25cb} no autosave yet");
}

/// LCV-116 AC 8 — a default app has never autosaved and has nothing
/// pending, so the bar opens on the "none yet" state.
#[test]
fn a_fresh_app_reads_no_autosave_yet() {
    let app = App::default();
    assert_eq!(
        format_autosave(app.dirty_since.is_some(), app.last_autosave_at.is_some()),
        "\u{25cb} no autosave yet"
    );
}

// ── LCV-138 — recovery label ───────────────────────────────────────────

/// AC 4 — a default `App` never shows the recovery label: the flag it
/// gates on is `false` until `App::new()`'s recovery branch sets it.
#[test]
fn recovery_label_is_absent_by_default() {
    let mut app = App::default();
    assert!(!app.title.recovered_from_autosave);
    let ctx = egui::Context::default();
    let out = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| draw_statusbar(ui, &mut app));
    });
    let painted = harness_texts(&out.shapes);
    assert!(
        !painted.iter().any(|t| t == RECOVERY_LABEL),
        "the recovery label must not paint while the flag is false: {painted:?}"
    );
}

/// AC 4 — with the flag set by hand (the only way to reach it in a test:
/// `App::new()` cannot be called, ADR 0002 §A2), the label paints, its
/// hover text is wired to the same widget, and rendering it touches none
/// of the autosave/save bookkeeping fields.
#[test]
fn recovery_label_paints_when_the_flag_is_set_and_touches_nothing_else() {
    let mut app = App {
        title: crate::app::DocumentTitleState {
            recovered_from_autosave: true,
            ..Default::default()
        },
        ..App::default()
    };
    let dirty_before = app.dirty_since;
    let autosave_before = app.last_autosave_at;

    let ctx = egui::Context::default();
    let out = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| draw_statusbar(ui, &mut app));
    });
    let painted = harness_texts(&out.shapes);
    assert!(
        painted.iter().any(|t| t == RECOVERY_LABEL),
        "the recovery label must paint while the flag is true: {painted:?}"
    );
    assert_eq!(
        app.dirty_since, dirty_before,
        "rendering the recovery label must not touch dirty_since"
    );
    assert_eq!(
        app.last_autosave_at, autosave_before,
        "rendering the recovery label must not touch last_autosave_at"
    );
}

/// AC 4 — the label's response really does carry the hover text this
/// demand requires: a source scan bounded to `draw_statusbar`, since a
/// tooltip's *content* is not itself a painted run until it is actually
/// hovered (see the behavioural test above for the *painting* half).
#[test]
fn recovery_label_carries_its_hover_text_source_scan() {
    let body = draw_statusbar_body();
    assert!(
        body.contains("ui.label(RECOVERY_LABEL).on_hover_text(RECOVERY_HOVER_TEXT)"),
        "the recovery label must carry its explanatory hover text"
    );
}

/// The `Shape::Text` runs egui actually painted, reduced to their string
/// content — the minimal local stand-in for `tests/harness/paint.rs`'s
/// collector (that harness lives under `tests/`, invisible to a unit test
/// compiled inside the library, ADR 0002 §A3).
fn harness_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn collect(shape: &egui::Shape, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(t) => out.push(t.galley.text().to_owned()),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

// ── LCV-116 static checks ─────────────────────────────────────────────

/// LCV-116 AC 2 / AC 8 — the segment order is pinned: entity count, then
/// the three indicators, then the
/// autosave indicator **last**. Bounded to `draw_statusbar`, so this
/// test's own body cannot satisfy the scan.
#[test]
fn status_bar_segment_order_is_pinned() {
    let body = draw_statusbar_body();
    let count = body
        .find("Entities: {count}")
        .expect("entity count present");
    let modes = body
        .find("for mode in Mode::ALL {")
        .expect("mode indicators present");
    let autosave = body
        .find("ui.label(autosave_str);")
        .expect("autosave indicator present");
    assert!(
        count < modes && modes < autosave,
        "order: entity count, SNAP/GRID/ORTHO, autosave last"
    );
    assert!(
        !body[..modes].contains("if "),
        "the mode indicators must not sit behind a conditional"
    );
    assert!(
        !body[modes..autosave].contains("ui.label("),
        "the autosave indicator is the last segment"
    );
}

/// LCV-116 AC 2 — the indicators are rendered unconditionally: no flag may
/// gate *whether* one appears, only whether it appears selected.
#[test]
fn the_indicators_are_always_visible() {
    let body = draw_statusbar_body();
    let loop_at = body
        .find("for mode in Mode::ALL {")
        .expect("mode loop present");
    // The needle omits the `ui` receiver: LCV-140's added `.on_hover_text`
    // call makes the chain long enough that `rustfmt` puts `ui` alone on
    // its own line, ahead of `.selectable_label(...)`.
    let label_at = body[loop_at..]
        .find(".selectable_label(mode.is_on(app), mode.label())")
        .expect("positive control: the selectable_label call is in the loop")
        + loop_at;
    assert!(
        !body[loop_at..label_at].contains("app."),
        "no flag may gate whether an indicator is rendered (decision 1)"
    );
}

/// LCV-116 AC 5 — the retired ortho badge formatter is gone from the whole
/// of `src/`. The needle is assembled from fragments so this file cannot
/// match its own scan, and so the criterion's own shell `grep` over `src/`
/// stays clean.
#[test]
fn the_retired_ortho_badge_formatter_is_gone() {
    let needle = concat!("format_", "ortho");
    let mut scanned = 0usize;
    let mut control = false;
    for path in walk_src() {
        let src = std::fs::read_to_string(&path).expect("readable source");
        assert!(
            !src.contains(needle),
            "{} still references the retired badge formatter",
            path.display()
        );
        control |= src.contains("fn format_autosave(");
        scanned += 1;
    }
    assert!(
        scanned >= 40,
        "only {scanned} files scanned — walk is broken"
    );
    assert!(
        control,
        "positive control: the walk must reach src/ui/statusbar.rs"
    );
}

/// LCV-116 AC 6 — **no key reading enters the status bar.** The whole
/// file is scanned, matching the criterion's `grep`, and every needle is
/// proved findable twice over: against a synthetic haystack and against
/// `src/app/input.rs`, one of the two real key readers. An absence
/// assertion over a haystack that could never match proves nothing.
#[test]
fn no_key_reading_enters_the_status_bar() {
    let needles = key_reader_needles();

    for needle in needles {
        assert_eq!(
            hits(&format!("prefix {needle} suffix"), &needles),
            vec![needle],
            "the scan must be able to find {needle}"
        );
    }

    let real_reader = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/input.rs"));
    let control = hits(real_reader, &needles);
    assert!(
        control.len() >= 3,
        "positive control: src/app/input.rs really does read keys, found {control:?}"
    );

    let found = hits(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/statusbar.rs")),
        &needles,
    );
    assert!(
        found.is_empty(),
        "the status bar must read no key (ADR 0002 §A6); found {found:?}"
    );
}

/// The key-reading identifiers AC 6 forbids, each assembled from fragments
/// so this file never contains the literal it is scanning for.
fn key_reader_needles() -> [&'static str; 5] {
    [
        concat!("key_", "pressed"),
        concat!("key_", "down"),
        concat!("ev", "ents"),
        concat!("consume_", "key"),
        concat!("inp", "ut("),
    ]
}

/// Which of `needles` occur in `hay`.
fn hits<'a>(hay: &str, needles: &[&'a str]) -> Vec<&'a str> {
    needles
        .iter()
        .copied()
        .filter(|needle| hay.contains(needle))
        .collect()
}

/// The source text of `fn draw_statusbar`, bounded at the test module.
fn draw_statusbar_body() -> &'static str {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui/statusbar.rs"));
    let start = src
        .find("pub fn draw_statusbar(")
        .expect("draw_statusbar must exist");
    let end = src[start..]
        .find("\n#[cfg(test)]")
        .expect("tests must follow the implementation")
        + start;
    &src[start..end]
}

/// Every `.rs` file under `src/`, recursively.
fn walk_src() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("readable directory") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") && !path.ends_with("tests.rs") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut out,
    );
    out
}
