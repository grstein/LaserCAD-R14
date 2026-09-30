use super::*;

/// The roles in `agent.chat`, in order — the shape LCV-123 AC 23 fixes.
/// Asserting on roles rather than on `last()` is what makes an inserted or
/// reordered row visible instead of silently shifting the tail.
fn roles(app: &App) -> Vec<&str> {
    app.agent.chat.iter().map(|(r, _)| r.as_str()).collect()
}

/// LCV-030 AC#1 — `App::default()` produces an empty document and an
/// empty history.
#[test]
fn app_default_constructs_with_empty_document_and_history() {
    let app = App::default();
    assert_eq!(app.document.entity_count(), 0);
    assert!(!app.history.can_undo());
    assert!(!app.history.can_redo());
}

/// LCV-031 AC#13 — `App` carries a `Camera` field and it defaults to
/// [`Camera::default()`].
#[test]
fn app_default_camera_matches_camera_default() {
    let app = App::default();
    assert_eq!(app.camera, Camera::default());
    assert_eq!(app.camera.mm_per_px, 1.0);
}

/// LCV-032 AC#1 — `App` carries `last_cursor_world` defaulting to `None`.
#[test]
fn app_default_has_no_cursor_world() {
    let app = App::default();
    assert_eq!(app.last_cursor_world, None);
}

/// LCV-034 AC#7, re-homed by LCV-114 AC 3/AC 4 — the bed is the
/// document's, and a blank document starts at the 400 mm default that the
/// old `App::bed` field used to hold.
#[test]
fn app_default_bed_comes_from_the_document() {
    let app = App::default();
    assert_eq!(app.document.bed_mm, [400.0, 400.0]);
    assert_eq!(
        crate::render::Bed::from_size_mm(app.document.bed_mm),
        crate::render::Bed::default()
    );
}

/// LCV-114 AC 4 — `App` holds **no** `bed` field: a second copy of the bed
/// is a second source of truth.
///
/// The haystack is bounded to the implementation section, so this test's
/// own body cannot satisfy it, and the two positive controls below prove
/// the scan is looking at real field declarations of exactly the shape it
/// claims is missing — an absence assertion over a haystack that never
/// could have matched proves nothing.
#[test]
fn app_has_no_bed_field() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/app/mod.rs"));
    let cfg_test_at = src
        .find("\n#[cfg(test)]")
        .expect("mod.rs must have a test module to bound the scan");
    let implementation = &src[..cfg_test_at];
    assert!(
        implementation.contains("pub camera: Camera,"),
        "positive control: the struct's fields must be in the haystack"
    );
    assert!(
        implementation.contains("pub bed_dialog: Option<[f64; 2]>,"),
        "positive control: a field whose name starts with `bed` is present"
    );
    assert!(
        !implementation.contains("pub bed:"),
        "App must not own a bed; the document does (LCV-114 AC 4)"
    );
}

/// LCV-119 AC 3 / ADR 0006 — the test constructor is given no real user
/// location, so every persistence call it can reach is a no-op. This is
/// the property that stops `cargo test` writing the developer's
/// `~/.config/lasercad` and deleting their `~/.local/share/lasercad`.
#[test]
fn app_default_has_no_persistence_paths() {
    let app = App::default();
    assert_eq!(app.settings_path, None);
    assert_eq!(app.autosave_path, None);
}

/// LCV-114 AC 14 — the Bed Size… modal starts closed.
#[test]
fn app_default_has_no_bed_dialog() {
    assert_eq!(App::default().bed_dialog, None);
}

/// LCV-037 AC#7 — `App` carries `preview_entities` defaulting to empty.
#[test]
fn app_default_has_empty_preview_entities() {
    let app = App::default();
    assert!(app.preview_entities.is_empty());
}

/// LCV-058 AC#10 — `App` carries `settings` defaulting to `Settings::default()`.
#[test]
fn app_default_settings_equals_settings_default() {
    let app = App::default();
    assert_eq!(app.settings, crate::io::settings::Settings::default());
}

/// LCV-069 AC#7 / §5 — `App::default().about_open` is `false`.
#[test]
fn app_default_about_open_is_false() {
    let app = App::default();
    assert!(!app.about_open);
}

/// LCV-068 AC#3 — `App::default().command_line_input` is the empty string.
#[test]
fn app_default_command_line_input_is_empty() {
    let app = App::default();
    assert!(app.command_line_input.is_empty());
}

/// LCV-076 AC#9 — `App::default().agent_settings_open` is `false`.
#[test]
fn app_default_agent_settings_open_is_false() {
    let app = App::default();
    assert!(!app.agent_settings_open);
}

/// LCV-070 AC#4 — snap_enabled and grid_enabled default to true.
#[test]
fn app_default_snap_enabled_is_true() {
    let app = App::default();
    assert!(app.snap_enabled);
    assert!(app.grid_enabled);
    assert!(!app.ortho_enabled);
}

/// LCV-053 AC#1 — `App::default().ortho_enabled` is `false`.
#[test]
fn app_default_ortho_is_false() {
    let app = App::default();
    assert!(!app.ortho_enabled);
}

/// LCV-062 AC#2 — App::default().current_file is None.
#[test]
fn app_default_current_file_is_none() {
    let app = App::default();
    assert!(app.current_file.is_none());
}

/// LCV-062 AC#2 — App::default().error_message is None.
#[test]
fn app_default_error_message_is_none() {
    let app = App::default();
    assert!(app.error_message.is_none());
}

// ── LCV-080 tests ─────────────────────────────────────────────────────────

/// LCV-080 AC#1 — all five agent fields have the correct default values.
#[test]
fn app_default_agent_fields() {
    let a = App::default();
    assert!(!a.agent.panel_open);
    assert!(a.agent.chat.is_empty());
    assert!(!a.agent.busy);
    assert!(a.agent.rx.is_none());
    assert!(a.agent.input_draft.is_empty());
}

/// LCV-080 AC#13, retargeted by LCV-122 AC 4 — `Done` appends an assistant
/// entry and clears busy + receiver. Same behaviour, new variant name.
#[test]
fn agent_rx_done_updates_chat_and_clears_busy() {
    use crate::agent::AgentEvent;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    tx.send(AgentEvent::done("done")).unwrap();
    poll_agent_rx(&mut app);
    assert_eq!(
        app.agent.chat.last(),
        Some(&("assistant".into(), "done".into())),
    );
    assert!(!app.agent.busy);
    assert!(app.agent.rx.is_none());
}

/// LCV-080 AC#14, retargeted by LCV-122 AC 4 — `Failed` appends an error
/// entry and clears busy + receiver.
#[test]
fn agent_rx_failed_updates_chat_and_clears_busy() {
    use crate::agent::AgentEvent;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    tx.send(AgentEvent::failed("err")).unwrap();
    poll_agent_rx(&mut app);
    assert_eq!(app.agent.chat.last(), Some(&("error".into(), "err".into())));
    assert!(!app.agent.busy);
    assert!(app.agent.rx.is_none());
}

/// ADR 0007 §D11 — a worker that ends without a verdict must still bring
/// `agent.busy` back down, or `update_ui` requests a repaint every frame
/// for the rest of the session (the LCV-120 bug, reopened silently).
///
/// No thread and no sleep: dropping the `Sender` is exactly what a
/// panicking or returning worker does, and `try_recv` reports it
/// deterministically on the very next call.
#[test]
fn a_dropped_sender_ends_the_turn_instead_of_hanging_busy() {
    use crate::agent::AgentEvent;
    let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    drop(tx);
    poll_agent_rx(&mut app);
    assert_eq!(
        app.agent.chat.last(),
        Some(&("error".into(), AGENT_LOST_MESSAGE.to_owned())),
    );
    assert!(!app.agent.busy, "a lost turn must clear agent.busy");
    assert!(app.agent.rx.is_none());
}

/// An empty but live channel is a no-op: the turn is still running, so the
/// receiver must survive to the next frame and `agent.busy` must stay up.
#[test]
fn an_empty_channel_keeps_the_turn_alive() {
    use crate::agent::AgentEvent;
    let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    poll_agent_rx(&mut app);
    assert!(app.agent.busy);
    assert!(app.agent.rx.is_some());
    drop(tx);
}

/// ADR 0007 §D2 — an `Act` is non-terminal: it mutates the live document
/// through `Command` + `History`, answers down its own reply channel, and
/// the same frame goes on to consume the `Done` behind it.
#[test]
fn an_act_is_applied_answered_and_followed_by_the_terminal_event() {
    use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
    let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
    let (reply, answers) = std::sync::mpsc::channel::<AgentOutcome>();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    // A hand-armed turn opens its group as `arm_turn` would (§D14).
    app.history.begin_group("Agent: test");
    let before = app.history.revision();
    tx.send(AgentEvent::Act {
        action: AgentAction::CreateCircle {
            layer: None,
            cx: 1.0,
            cy: 2.0,
            r: 3.0,
        },
        reply,
    })
    .unwrap();
    tx.send(AgentEvent::done("drawn")).unwrap();

    poll_agent_rx(&mut app);

    let outcome = answers.try_recv().expect("the Act must be answered");
    assert!(!outcome.is_refused(), "{outcome:?}");
    assert!(outcome.text().contains("Circle created"), "{outcome:?}");
    assert_eq!(app.document.entity_count(), 1);
    assert_eq!(app.history.revision(), before + 1);
    assert!(app.history.can_undo());
    // LCV-123 AC 23 — the row order of a one-action turn: the action's
    // `tool` row, the terminal row, then the note (AC 22).
    assert_eq!(
        roles(&app),
        ["tool", "assistant", "note"],
        "{:?}",
        app.agent.chat
    );
    assert_eq!(app.agent.chat[0].1, outcome.text(), "verbatim, AC 23");
    assert_eq!(
        app.agent.chat[1],
        ("assistant".to_owned(), "drawn".to_owned())
    );
    assert!(!app.agent.busy);
}

/// ADR 0007 §D2, the other half — an `Act` on its own ends **nothing**.
///
/// Its sibling above proves an `Act` is applied and answered, but that
/// holds just as well for an implementation that answers and then ends the
/// turn, because the `Done` behind it would tidy up anyway. So here the
/// `Act` arrives alone: the rendezvous is still open, the worker is still
/// blocked on the reply it just got, and the next tool call is still to
/// come. Killing `agent.busy` here would stop the repaints that ADR 0007
/// §D2 needs to turn the crank, and dropping the receiver would strand the
/// rest of the turn.
#[test]
fn a_lone_act_leaves_the_turn_running() {
    use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
    let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
    let (reply, answers) = std::sync::mpsc::channel::<AgentOutcome>();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    // A hand-armed turn opens its group as `arm_turn` would (§D14).
    app.history.begin_group("Agent: test");
    tx.send(AgentEvent::Act {
        action: AgentAction::CreateCircle {
            layer: None,
            cx: 0.0,
            cy: 0.0,
            r: 1.0,
        },
        reply,
    })
    .unwrap();

    poll_agent_rx(&mut app);

    assert!(answers.try_recv().is_ok(), "the Act must still be answered");
    assert_eq!(app.document.entity_count(), 1);
    assert!(
        app.agent.busy,
        "an Act is not a verdict: the turn is still running"
    );
    assert!(
        app.agent.rx.is_some(),
        "dropping the receiver here strands every later tool call"
    );
    assert_eq!(
        roles(&app),
        ["tool"],
        "an Act writes its transcript row (AC 23) and no verdict"
    );

    // The turn ends only when a terminal event arrives, on a later frame.
    tx.send(AgentEvent::done("drawn")).unwrap();
    poll_agent_rx(&mut app);
    assert_eq!(roles(&app), ["tool", "assistant", "note"]);
    assert_eq!(
        app.agent.chat[1],
        ("assistant".to_owned(), "drawn".to_owned())
    );
    assert!(!app.agent.busy);
    assert!(app.agent.rx.is_none());
}

/// The fourth turn exit (ADR 0007 §D11, extended) — the worker vanishes
/// between sending an `Act` and reading its answer.
///
/// The event channel still looks alive, because the worker's `Sender` for
/// it has not been dropped yet, so neither `Done`, nor `Failed`, nor
/// `Disconnected` will ever arrive. Only the dead reply channel says
/// anything is wrong. Leaving `agent.busy` up here is not a cosmetic bug:
/// `App::update_ui` requests a repaint on every frame while it is set, so
/// the app would spin for the rest of the session — LCV-120, reopened, with
/// a symptom that surfaces nowhere near the agent.
///
/// It writes the same row as a dropped sender: a live worker is blocked on
/// the other half of this reply channel until the answer arrives, so an
/// `Err` from `send` means the worker is gone, which means the event
/// channel is closed too. Falling through would have printed that row one
/// iteration later — the row is the fact, not the arm.
///
/// Dropping the receiving end before polling is exactly what a panicking
/// worker does, and `Sender::send` reports it on the next call. No thread,
/// no sleep.
#[test]
fn a_worker_that_stops_listening_mid_act_still_ends_the_turn() {
    use crate::agent::{AgentAction, AgentEvent, AgentOutcome};
    let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>();
    let (reply, answers) = std::sync::mpsc::channel::<AgentOutcome>();
    let mut app = App {
        agent: AgentState {
            rx: Some(rx),
            busy: true,
            ..AgentState::default()
        },
        ..App::default()
    };
    // A hand-armed turn opens its group as `arm_turn` would (§D14).
    app.history.begin_group("Agent: test");
    tx.send(AgentEvent::Act {
        action: AgentAction::CreateCircle {
            layer: None,
            cx: 0.0,
            cy: 0.0,
            r: 1.0,
        },
        reply,
    })
    .unwrap();
    drop(answers);

    poll_agent_rx(&mut app);

    assert!(
        !app.agent.busy,
        "a worker that stopped listening must not leave the app spinning"
    );
    assert!(app.agent.rx.is_none(), "nothing more can arrive");
    assert_eq!(
        roles(&app),
        ["tool", "error", "note"],
        "the applied action, the verdict, then the undo shape (AC 22, AC 23)"
    );
    assert_eq!(
        app.agent.chat.get(1),
        Some(&("error".into(), AGENT_LOST_MESSAGE.to_owned())),
        "this exit reports the same fact as a dropped sender — the worker \
             is gone — so it must show the operator the same row (ADR 0007 §D11)"
    );
    assert_eq!(
        app.document.entity_count(),
        1,
        "the action itself was applied before the answer was lost"
    );

    // The event channel is still open — this is the point of the test.
    drop(tx);
}

// ── LCV-102 tests — autosave dirty tracking (ADR 0002 §B) ──────────────

use crate::document::CreateLine;
use crate::geometry::Line;

fn some_line() -> Line {
    Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0))
}

/// AC 6 — `App::default()` initialises `last_synced_revision` to `0`.
#[test]
fn app_default_last_synced_revision_is_zero() {
    let app = App::default();
    assert_eq!(app.last_synced_revision, 0);
}

/// AC 9 — the regression test for the defect this demand fixes: tools
/// bypass `App::commit` and call `history.commit` directly, so
/// `sync_dirty` must observe that mutation through `History::revision()`
/// alone. Before the fix, nothing but `App::commit` ever armed
/// `dirty_since`, so a direct `history.commit` left it `None` forever.
#[test]
fn direct_history_commit_marks_document_dirty() {
    let mut app = App::default();
    assert!(app.dirty_since.is_none());

    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.sync_dirty();

    assert!(
        app.dirty_since.is_some(),
        "a history.commit bypassing App::commit must still dirty the document"
    );
}

/// AC 7 — `sync_dirty` is idempotent without an intervening mutation, and
/// preserves the *first* dirty instant rather than refreshing it.
#[test]
fn sync_dirty_is_idempotent_without_mutation() {
    let mut app = App::default();
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);

    app.sync_dirty();
    let first = app.dirty_since.expect("first sync must arm dirty_since");

    app.sync_dirty();
    assert_eq!(
        app.dirty_since,
        Some(first),
        "a second sync with no new revision must not move the instant"
    );
}

/// AC 8 — `mark_clean` clears `dirty_since` and resyncs the revision, so
/// an immediately following `sync_dirty` stays clean.
#[test]
fn mark_clean_clears_and_resyncs() {
    let mut app = App::default();
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.sync_dirty();
    assert!(app.dirty_since.is_some());

    app.mark_clean();
    assert!(app.dirty_since.is_none());
    assert_eq!(app.last_synced_revision, app.history.revision());

    app.sync_dirty();
    assert!(
        app.dirty_since.is_none(),
        "no new revision since mark_clean, so sync_dirty must stay clean"
    );
}

/// AC 10 — undo re-dirties the document (undone away from what is saved).
#[test]
fn undo_marks_document_dirty() {
    let mut app = App::default();
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.sync_dirty();
    app.mark_clean();
    assert!(app.dirty_since.is_none());

    assert!(app.history.undo(&mut app.document));
    app.sync_dirty();
    assert!(app.dirty_since.is_some(), "undo must dirty the document");
}

/// AC 10 — redo re-dirties the document.
#[test]
fn redo_marks_document_dirty() {
    let mut app = App::default();
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.history.undo(&mut app.document);
    app.sync_dirty();
    app.mark_clean();
    assert!(app.dirty_since.is_none());

    assert!(app.history.redo(&mut app.document));
    app.sync_dirty();
    assert!(app.dirty_since.is_some(), "redo must dirty the document");
}

/// AC 11 — a no-op undo on a clean, empty history must not dirty it.
#[test]
fn no_op_undo_does_not_dirty() {
    let mut app = App::default();
    assert!(!app.history.undo(&mut app.document));
    app.sync_dirty();
    assert!(app.dirty_since.is_none());
}

/// AC 12 — replacing `history` with a fresh one and calling `mark_clean`
/// must not let the very next `sync_dirty` re-dirty the just-loaded
/// document (this is the case that motivates resyncing the revision
/// inside `mark_clean`, ADR 0002 §B).
#[test]
fn replacing_history_then_mark_clean_stays_clean() {
    let mut app = App::default();
    app.history
        .commit(Box::new(CreateLine::new(some_line())), &mut app.document);
    app.sync_dirty();
    assert!(app.dirty_since.is_some());

    app.history = History::default();
    app.mark_clean();
    assert!(app.dirty_since.is_none());

    app.sync_dirty();
    assert!(
        app.dirty_since.is_none(),
        "a fresh History at revision 0 must not re-dirty after mark_clean"
    );
}
