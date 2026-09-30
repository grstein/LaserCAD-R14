//! LCV-116 AC 2 / AC 3 / AC 4 — the three mode indicators, in a real frame.
//!
//! `draw_statusbar` now takes `&mut App` and can flip a mode flag, which makes
//! the render path a *potential* writer of session state. These tests drive the
//! real `App::update_ui` through `tests/harness/mod.rs` and pin the two halves
//! of that contract: rendering alone changes nothing, and the one gesture the
//! indicators share with the keyboard (`F3`) lands identically.
//!
//! The pointer click on each mode pill is driven by position in
//! `tests/it/ui/visual_refresh.rs` (LCV-184). The logic behind the click is
//! covered by `src/ui/statusbar.rs::tests::toggle_click_flips_only_its_own_flag`,
//! which calls the same `apply_toggle` the click handler calls.
//!
//! ADR 0002 §A4: key taps are press+release (`harness::tap`), no `Ctrl+O` /
//! `Ctrl+S` / `Ctrl+Shift+S` is ever sent, and no test here lets the 800 ms
//! autosave debounce elapse.

use crate::harness;

use harness::{frame, tap};
use lasercad::app::App;
use lasercad::document::CreateLine;
use lasercad::geometry::snap::{SnapKind, SnapResult};
use lasercad::geometry::{Line, Vec2};

/// AC 2 / AC 3 — the bar renders with the modes in every combination that
/// matters and the frame does not toggle anything by itself. A status bar that
/// mutates the state it displays would flip a mode on every repaint.
#[test]
fn statusbar_renders_all_three_indicators_without_panic() {
    for (snap, grid, ortho) in [
        (true, true, false),
        (false, false, false),
        (true, false, true),
        (false, true, true),
        (true, true, true),
    ] {
        let ctx = egui::Context::default();
        let mut app = App {
            snap_enabled: snap,
            grid_enabled: grid,
            ortho_enabled: ortho,
            ..App::default()
        };

        // Asserted after *every* frame, and an odd number of them: a bar that
        // toggled a mode once per frame would be invisible to a check made
        // only after an even number of renders.
        for f in 0..3 {
            frame(&ctx, &mut app, vec![]);
            assert_eq!(app.snap_enabled, snap, "frame {f}: rendering flipped SNAP");
            assert_eq!(app.grid_enabled, grid, "frame {f}: rendering flipped GRID");
            assert_eq!(
                app.ortho_enabled, ortho,
                "frame {f}: rendering flipped ORTHO"
            );
        }
    }
}

/// AC 3 — rendering the bar over a document with content touches neither the
/// document nor the undo stack nor the dirty signal. `draw_statusbar` holds
/// `&mut App`; this is the fence around what it is allowed to do with it.
#[test]
fn rendering_the_bar_mutates_no_document_state() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 10.0),
    ))));
    // Two frames: the first arms `dirty_since` via `sync_dirty`, and a third
    // would still be far inside the 800 ms debounce.
    frame(&ctx, &mut app, vec![]);
    let revision = app.history.revision();
    let dirty = app.dirty_since;
    let entities = app.document.entity_count();

    frame(&ctx, &mut app, vec![]);

    assert_eq!(app.document.entity_count(), entities);
    assert_eq!(app.history.revision(), revision, "no history entry");
    assert_eq!(app.dirty_since, dirty, "no second writer of dirty_since");
    assert_eq!(entities, 1, "positive control: the line really is there");
}

/// AC 4 — `F3` flips `snap_enabled` through the real frame body, and the same
/// frame's `suppress_snap_if_disabled` clears `active_snap`, so a stale snap
/// marker cannot outlive the toggle. A status-bar click on `SNAP` runs the same
/// `apply_toggle` and therefore lands in the same place.
#[test]
fn f3_and_the_statusbar_agree() {
    let ctx = egui::Context::default();
    let mut app = App::default();
    assert!(app.snap_enabled, "a fresh app starts with snap armed");

    // A snap marker as if the cursor had just hovered a feature.
    app.active_snap = Some(SnapResult {
        point: Vec2::new(5.0, 5.0),
        kind: SnapKind::Endpoint,
        primary_idx: 0,
        secondary_idx: None,
    });

    tap(&ctx, &mut app, egui::Key::F3, egui::Modifiers::NONE);

    assert!(!app.snap_enabled, "F3 must turn snap off");
    assert!(
        app.active_snap.is_none(),
        "AC 4 — turning snap off must drop the stale marker"
    );

    frame(&ctx, &mut app, vec![]);
    assert!(!app.snap_enabled, "and it must stay off");
    assert!(app.active_snap.is_none(), "and stay clear");

    tap(&ctx, &mut app, egui::Key::F3, egui::Modifiers::NONE);
    assert!(app.snap_enabled, "F3 must turn it back on");
}

/// AC 2 — `F7` and `F8` reach their own flag and only their own, through the
/// real frame. Together with `f3_and_the_statusbar_agree` this pins that the
/// three indicators and the three function keys address three distinct flags.
#[test]
fn each_function_key_moves_exactly_one_mode() {
    for (key, snap, grid, ortho) in [
        (egui::Key::F3, false, true, false),
        (egui::Key::F7, true, false, false),
        (egui::Key::F8, true, true, true),
    ] {
        let ctx = egui::Context::default();
        let mut app = App::default();

        tap(&ctx, &mut app, key, egui::Modifiers::NONE);

        assert_eq!(app.snap_enabled, snap, "{key:?} moved SNAP wrongly");
        assert_eq!(app.grid_enabled, grid, "{key:?} moved GRID wrongly");
        assert_eq!(app.ortho_enabled, ortho, "{key:?} moved ORTHO wrongly");
    }
}

/// AC 8 — the autosave indicator's three states, reached through real frames
/// rather than by calling the formatter. `format_autosave` is `pub(crate)`, so
/// what an integration test can prove is the *state* it is fed: a fresh app has
/// never saved, an edited one is pending, and a `mark_clean` settles it.
///
/// ADR 0002 §A4 rule 2: `dirty_since` is cleared by `mark_clean`, never by
/// letting the debounce elapse, so nothing is written to the real data
/// directory.
#[test]
fn the_autosave_indicator_states_are_reachable() {
    let ctx = egui::Context::default();
    let mut app = App::default();

    frame(&ctx, &mut app, vec![]);
    assert!(app.dirty_since.is_none(), "fresh: not pending");
    assert!(app.last_autosave_at.is_none(), "fresh: never saved");

    app.commit(Box::new(CreateLine::new(Line::new(
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 10.0),
    ))));
    frame(&ctx, &mut app, vec![]);
    assert!(app.dirty_since.is_some(), "edited: a write is pending");

    app.mark_clean();
    frame(&ctx, &mut app, vec![]);
    assert!(app.dirty_since.is_none(), "settled: nothing pending");
}
