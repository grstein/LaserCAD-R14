use super::*;
use crate::app::arm_turn;
use crate::document::CreateLine;
use crate::geometry::{Line, Vec2};

fn draw(app: &mut App, y: f64) {
    let line = Line::new(Vec2::new(0.0, y), Vec2::new(10.0, y));
    app.history
        .commit_grouped(Box::new(CreateLine::new(line)), &mut app.document);
}

fn turn() -> App {
    let mut app = App::default();
    drop(arm_turn(&mut app, "try"));
    app
}

fn refused(outcome: &AgentOutcome) -> &str {
    assert!(outcome.is_refused(), "{outcome:?}");
    outcome.text()
}

/// AC 7 — 1 to 32 characters of `A-Z a-z 0-9 _ -`, nothing else.
#[test]
fn valid_names_are_1_to_32_safe_characters() {
    for ok in ["a", "Z", "0", "_", "-", "try_2-b", &"x".repeat(32)] {
        assert!(valid_name(ok), "{ok}");
    }
    for bad in ["", &"x".repeat(33), "a b", "a.b", "é", "a\n", "a/b"] {
        assert!(!valid_name(bad), "{bad:?}");
    }
}

/// AC 5 — setting an existing name moves it; a target keeps itself and
/// forgets what was set after it; `start` is 0 and forgets everything.
#[test]
fn the_list_moves_names_and_truncates_after_the_target() {
    let mut c = Checkpoints::default();
    assert_eq!((c.known(), c.target(START)), ("start".to_owned(), Some(0)));
    c.set("a", 1);
    c.set("b", 2);
    c.set("c", 3);
    c.set("a", 4);
    assert_eq!(c.known(), "start, b, c, a");
    assert_eq!(
        (c.target("a"), c.target("b"), c.target("x")),
        (Some(4), Some(2), None)
    );
    c.truncate_after("c");
    assert_eq!(c.known(), "start, b, c");
    c.truncate_after(START);
    assert_eq!(c.known(), "start");
}

/// AC 1 — a checkpoint answers with its mark and moves neither the drawing
/// nor the revision.
#[test]
fn checkpoint_records_the_group_length() {
    let mut app = turn();
    draw(&mut app, 0.0);
    let (rev, count) = (app.history.revision(), app.document.entity_count());
    let got = checkpoint(&mut app, "a");
    assert_eq!(
        got,
        AgentOutcome::Ok("Checkpoint a set at 1 changes.".to_owned())
    );
    assert_eq!(
        (app.history.revision(), app.document.entity_count()),
        (rev, count)
    );
    assert_eq!(app.agent.turn.checkpoints.target("a"), Some(1));
}

/// AC 2, AC 5 — a rollback undoes past the mark, keeps the target and
/// forgets the later checkpoints; a second rollback undoes nothing.
#[test]
fn rollback_rewinds_to_the_mark_and_forgets_later_checkpoints() {
    let mut app = turn();
    draw(&mut app, 0.0);
    checkpoint(&mut app, "a");
    draw(&mut app, 1.0);
    checkpoint(&mut app, "b");
    draw(&mut app, 2.0);
    let got = rollback(&mut app, "a");
    let text = "Rolled back to a: 2 changes undone, 1 entities.";
    assert_eq!(got, AgentOutcome::Ok(text.to_owned()));
    assert_eq!(app.agent.turn.checkpoints.known(), "start, a");
    let rev = app.history.revision();
    let again = rollback(&mut app, "a");
    let text = "Rolled back to a: 0 changes undone, 1 entities.";
    assert_eq!(again, AgentOutcome::Ok(text.to_owned()));
    assert_eq!(
        app.history.revision(),
        rev,
        "k = 0 does not move the revision"
    );
}

/// AC 4, AC 7 — `start` cannot be set; a bad name is refused unechoed and an
/// unknown one named, each listing the known checkpoints; nothing changes.
#[test]
fn bad_unknown_and_start_names_are_refused_with_the_known_list() {
    let mut app = turn();
    draw(&mut app, 0.0);
    checkpoint(&mut app, "a");
    let rev = app.history.revision();
    let form = expected_form("name");
    let got = checkpoint(&mut app, "bad name!");
    let want = format!("checkpoint name: invalid; expected {form}. Known checkpoints: start, a.");
    assert_eq!(refused(&got), want);
    let got = rollback(&mut app, &"x".repeat(33));
    assert!(refused(&got).starts_with("rollback name: invalid;"));
    assert!(!refused(&got).contains("xxx"), "the payload is not echoed");
    let got = rollback(&mut app, "b");
    assert_eq!(
        refused(&got),
        "rollback name: no checkpoint named \"b\"; expected the name of a known \
         checkpoint. Known checkpoints: start, a."
    );
    let got = checkpoint(&mut app, "start");
    assert_eq!(
        refused(&got),
        "checkpoint name: \"start\" is built in and cannot be set; expected any other \
         checkpoint name. Known checkpoints: start, a."
    );
    assert_eq!(app.agent.turn.checkpoints.known(), "start, a");
    assert_eq!((app.history.revision(), app.history.group_len()), (rev, 1));
}

/// Outside a turn both tools are refused before anything is read.
#[test]
fn no_open_turn_refuses_both_tools() {
    let mut app = App::default();
    for got in [checkpoint(&mut app, "a"), rollback(&mut app, START)] {
        assert_eq!(refused(&got), NO_TURN);
    }
}

/// AC 9 — a new turn knows only `start`.
#[test]
fn a_new_turn_forgets_the_old_checkpoints() {
    let mut app = turn();
    checkpoint(&mut app, "a");
    assert_eq!(app.agent.turn.checkpoints.known(), "start, a");
    app.history.end_group();
    drop(arm_turn(&mut app, "again"));
    assert_eq!(app.agent.turn.checkpoints.known(), "start");
}
