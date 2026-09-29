use super::*;
use crate::geometry::EPSILON;

fn idle() -> TextTool {
    TextTool::default()
}

fn make() -> (TextTool, Document, History) {
    (idle(), Document::default(), History::default())
}

fn anchored() -> (TextTool, Document, History) {
    let (mut t, mut d, mut h) = make();
    t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
    (t, d, h)
}

// ── AC 4, 5 — the state machine ─────────────────────────────────────

#[test]
fn idle_wants_no_raw_input_then_both_waiting_states_do() {
    let mut t = idle();
    assert!(!t.wants_raw_input());

    let mut d = Document::default();
    let mut h = History::default();
    t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
    assert!(t.wants_raw_input(), "WaitingText wants raw input");

    t.on_raw_input("HELLO", &mut d, &mut h);
    assert!(t.wants_raw_input(), "WaitingHeight wants raw input");
}

#[test]
fn click_anchors_and_a_second_click_does_not_move_it() {
    let (mut t, mut d, mut h) = anchored();
    t.on_pointer_down(Vec2::new(99.0, 99.0), false, &mut d, &mut h);
    t.on_raw_input("HELLO", &mut d, &mut h);
    // The anchor is whatever the *first* click set: (0, 0), not (99, 99).
    let strokes = commit_and_get(&mut t, &mut d, &mut h, "");
    for e in &strokes {
        if let Entity::Line(l) = e {
            assert!(l.p1.x < 50.0 && l.p2.x < 50.0, "anchor must not have moved");
        }
    }
}

#[test]
fn non_empty_text_advances_to_the_height_prompt() {
    let (mut t, mut d, mut h) = anchored();
    assert!(t.on_raw_input("HELLO", &mut d, &mut h));
    assert_eq!(t.status_text(), "TEXT Specify height <5>:");
    assert_eq!(d.entity_count(), 0, "the string alone commits nothing");
}

#[test]
fn empty_text_cancels_and_commits_nothing() {
    let (mut t, mut d, mut h) = anchored();
    assert!(t.on_raw_input("", &mut d, &mut h));
    assert_eq!(t.status_text(), "TEXT Specify start point:");
    assert_eq!(d.entity_count(), 0);
    assert_eq!(h.len(), 0);
}

#[test]
fn whitespace_only_text_cancels_and_commits_nothing() {
    let (mut t, mut d, mut h) = anchored();
    assert!(t.on_raw_input("   ", &mut d, &mut h));
    assert_eq!(t.status_text(), "TEXT Specify start point:");
    assert_eq!(d.entity_count(), 0);
}

#[test]
fn escape_from_either_waiting_state_returns_to_idle_uncommitted() {
    let (mut t, d, _h) = anchored();
    t.cancel();
    assert_eq!(t.status_text(), "TEXT Specify start point:");
    assert_eq!(d.entity_count(), 0);

    let (mut t2, mut d2, mut h2) = anchored();
    t2.on_raw_input("HELLO", &mut d2, &mut h2);
    t2.cancel();
    assert_eq!(t2.status_text(), "TEXT Specify start point:");
    assert_eq!(d2.entity_count(), 0);
    assert_eq!(h2.len(), 0);
}

#[test]
fn on_raw_input_returns_false_only_when_idle() {
    let (mut t, mut d, mut h) = make();
    assert!(!t.on_raw_input("anything", &mut d, &mut h));

    let (mut t2, mut d2, mut h2) = anchored();
    assert!(t2.on_raw_input("HELLO", &mut d2, &mut h2));
    assert!(
        t2.on_raw_input("abc", &mut d2, &mut h2),
        "invalid height still returns true"
    );
}

// ── AC 5, 7, 8, decisions 2-4, 7 — the height ────────────────────────

/// Drive `t` from `WaitingHeight` (after typing `HELLO`) with a single
/// height submission and return the entities the document now holds.
fn commit_and_get(
    t: &mut TextTool,
    d: &mut Document,
    h: &mut History,
    height_raw: &str,
) -> Vec<Entity> {
    t.on_raw_input(height_raw, d, h);
    d.entities.clone()
}

#[test]
fn empty_height_commits_at_five_millimetres() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("H", &mut d, &mut h);
    t.on_raw_input("", &mut d, &mut h);
    assert_eq!(h.len(), 1);
    let max_y = d
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(l.p1.y.max(l.p2.y)),
            _ => None,
        })
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((max_y - 5.0).abs() < EPSILON);
    assert_eq!(t.status_text(), "TEXT Specify start point:");
}

#[test]
fn valid_height_commits_at_that_height() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("H", &mut d, &mut h);
    t.on_raw_input("12.5", &mut d, &mut h);
    assert_eq!(h.len(), 1);
    let max_y = d
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(l.p1.y.max(l.p2.y)),
            _ => None,
        })
        .fold(f64::NEG_INFINITY, f64::max);
    assert!((max_y - 12.5).abs() < EPSILON);
}

#[test]
fn height_below_the_floor_is_refused() {
    for bad in ["0", "0.05", "-3"] {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input(bad, &mut d, &mut h);
        assert_eq!(d.entity_count(), 0, "{bad} must not commit");
        assert_eq!(h.len(), 0);
        assert!(matches!(
            t.state,
            TextToolState::WaitingHeight { invalid: true, .. }
        ));
    }
}

#[test]
fn height_above_the_ceiling_is_refused() {
    for bad in ["2000.1", "1e9"] {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input(bad, &mut d, &mut h);
        assert_eq!(d.entity_count(), 0, "{bad} must not commit");
        assert_eq!(h.len(), 0);
    }
}

#[test]
fn height_boundaries_are_inclusive() {
    for good in ["0.1", "2000"] {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input(good, &mut d, &mut h);
        assert_eq!(h.len(), 1, "{good} must commit");
        assert!(d.entity_count() > 0, "{good} must produce strokes");
    }
}

#[test]
fn unparseable_height_is_refused() {
    for bad in ["abc", "10mm", "nan", "inf"] {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input(bad, &mut d, &mut h);
        assert_eq!(d.entity_count(), 0, "{bad} must not commit");
        assert_eq!(h.len(), 0);
    }
}

/// Decision 7 / AC 5 — the comma is the coordinate separator everywhere
/// in the product; it is never a decimal point in a height.
#[test]
fn comma_is_not_a_decimal_point_in_a_height() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("H", &mut d, &mut h);
    t.on_raw_input("10,5", &mut d, &mut h);
    assert_eq!(d.entity_count(), 0);
    assert_eq!(h.len(), 0);
    assert!(matches!(
        t.state,
        TextToolState::WaitingHeight { invalid: true, .. }
    ));
}

#[test]
fn a_refused_height_keeps_the_text_and_sets_the_retry_prompt() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("HELLO", &mut d, &mut h);
    t.on_raw_input("abc", &mut d, &mut h);
    assert_eq!(
        t.status_text(),
        "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:"
    );
    match &t.state {
        TextToolState::WaitingHeight { text, .. } => assert_eq!(text, "HELLO"),
        other => panic!("expected WaitingHeight, got {other:?}"),
    }
    assert_eq!(d.entity_count(), 0);
}

#[test]
fn a_refused_height_then_a_valid_one_commits_normally() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("HELLO", &mut d, &mut h);
    t.on_raw_input("abc", &mut d, &mut h);
    assert_eq!(d.entity_count(), 0);
    t.on_raw_input("10", &mut d, &mut h);
    assert_eq!(h.len(), 1);
    assert!(d.entity_count() > 0);
    assert_eq!(t.status_text(), "TEXT Specify start point:");
}

/// AC 8 — `'H'` spans exactly `hy ∈ [-9, 0]` (baseline to cap), so at a
/// 10 mm height its top sits at exactly `y = 10.0`. Deliberately *not*
/// `'O'`: `'O'` spans `hy ∈ [-9, +9]` and descends below the baseline, so
/// its y-extent is twice the cap height — a wrong scale would silently
/// pass with that oracle instead of failing.
#[test]
fn ten_millimetre_h_spans_exactly_ten_millimetres() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("H", &mut d, &mut h);
    t.on_raw_input("10", &mut d, &mut h);
    assert_eq!(h.len(), 1);

    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for e in &d.entities {
        if let Entity::Line(l) = e {
            min_y = min_y.min(l.p1.y).min(l.p2.y);
            max_y = max_y.max(l.p1.y).max(l.p2.y);
        }
    }
    assert!((min_y - 0.0).abs() < 1e-9, "baseline at y=0, got {min_y}");
    assert!((max_y - 10.0).abs() < 1e-9, "cap top at y=10, got {max_y}");
}

#[test]
fn commit_is_a_single_command() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("HELLO", &mut d, &mut h);
    t.on_raw_input("10", &mut d, &mut h);
    assert_eq!(h.len(), 1);
    assert!(d.entity_count() > 0);
}

#[test]
fn layout_with_no_glyphs_commits_nothing() {
    // A run of unsupported codepoints produces no Hershey strokes at all.
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("\u{00e9}\u{00e8}", &mut d, &mut h);
    t.on_raw_input("10", &mut d, &mut h);
    assert_eq!(d.entity_count(), 0, "a glyphless string commits nothing");
    assert_eq!(h.len(), 0);
    assert_eq!(
        t.status_text(),
        "TEXT Specify start point:",
        "state still returns to Idle"
    );
}

// ── AC 6 — prompts ───────────────────────────────────────────────────

#[test]
fn prompt_matches_each_state() {
    assert_eq!(idle().status_text(), "TEXT Specify start point:");

    let (mut t, mut d, mut h) = anchored();
    assert_eq!(t.status_text(), "TEXT Enter text:");

    t.on_raw_input("HELLO", &mut d, &mut h);
    assert_eq!(t.status_text(), "TEXT Specify height <5>:");

    t.on_raw_input("abc", &mut d, &mut h);
    assert_eq!(
        t.status_text(),
        "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:"
    );
}

// ── AC 9 — preview ───────────────────────────────────────────────────

#[test]
fn preview_is_empty_until_the_height_prompt() {
    let mut t = idle();
    assert!(t.preview().is_empty());

    let mut d = Document::default();
    let mut h = History::default();
    t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
    assert!(
        t.preview().is_empty(),
        "WaitingText previews nothing (AC 9)"
    );
}

#[test]
fn preview_at_the_height_prompt_uses_the_default_height() {
    let (mut t, mut d, mut h) = anchored();
    t.on_raw_input("H", &mut d, &mut h);
    let pv = t.preview();
    assert!(!pv.is_empty());
    let max_y = pv
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(l.p1.y.max(l.p2.y)),
            _ => None,
        })
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (max_y - DEFAULT_TEXT_HEIGHT_MM).abs() < EPSILON,
        "the WaitingHeight preview always uses the 5 mm default"
    );
}

// ── object safety, name ──────────────────────────────────────────────

#[test]
fn text_tool_default_is_idle_and_object_safe() {
    let t = idle();
    assert_eq!(t.name(), "TEXT");
    assert!(t.preview().is_empty());
    let _: Box<dyn Tool> = Box::new(idle());
}

#[test]
fn pointer_move_and_up_are_noops() {
    let (mut t, mut d, mut h) = anchored();
    t.on_pointer_move(Vec2::new(100.0, 200.0), &mut d);
    t.on_pointer_up(Vec2::new(10.0, 10.0), false, &mut d, &mut h);
    assert_eq!(
        t.status_text(),
        "TEXT Enter text:",
        "neither call advances the phase"
    );
}
