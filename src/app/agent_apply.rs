//! LCV-122 / LCV-123 — apply one [`AgentAction`] to the live document, and
//! transcribe its outcome (ADR 0007 §D2a/§D5).
//!
//! This is the UI-thread half of the bridge. The worker thread names what it
//! wants; this file decides what really happens, against the drawing the
//! operator is really looking at, and answers with a sentence the model reads
//! back as the `tool` result.
//!
//! Three rules shape everything below.
//!
//! - **Never touch `document.entities` directly.** Every mutation is a
//!   `Box<dyn Command>` committed through [`App::commit`] (AGENTS.md §"State
//!   and mutation"), so agent edits get undo, redo, dirty tracking and autosave
//!   for free and are indistinguishable from a human's.
//! - **The range check lives here, and it refuses rather than fails.** The
//!   worker cannot know `entities.len()` (ADR 0007 §D1), so `index` arrives
//!   shape-checked but unbounded. An out-of-range index is information the
//!   model can act on — `AgentOutcome::Refused`, not an error that ends a turn
//!   (ADR 0007 §D2a).
//! - **Say what shifted.** Positional indices renumber on every delete, so each
//!   outcome reports the resulting entity count, and a delete names the entity
//!   it removed and the indices that moved down (ADR 0007 §D5).
//! - **Every action leaves a row.** LCV-123 AC 23: the sentence handed back to
//!   the model is appended verbatim to `agent.chat` by [`transcribe`], here,
//!   the one place that already holds both the action and its narration. The
//!   operator and the model therefore read the same words, and a transcript
//!   that disagrees with what the model was told is impossible by construction.
//!   No variant is special-cased — a query gets a row like anything else.
//!
//! Imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere, and spawns no
//! thread.

use crate::agent::{AgentAction, AgentOutcome};
use crate::app::App;
use crate::document::{
    Command, CreateArc, CreateCircle, CreateLine, DeleteEntities, Document, Entity, MoveEntities,
};
use crate::geometry::{Arc as GeoArc, Circle, Line, Vec2};

/// What [`plan`] decided, before anything was applied.
enum Planned {
    /// Commit this command, then append the entity count to this sentence.
    Commit(Box<dyn Command>, String),
    /// Answer without touching the document: a query, or a refusal.
    Answer(AgentOutcome),
}

/// Apply one action to the app's live document and report what happened.
///
/// The single entry point the frame loop calls when an `AgentEvent::Act`
/// arrives. Mutating variants commit exactly one command — `history.revision()`
/// advances by one and `Ctrl+Z` takes it back. Query variants commit nothing
/// and leave `revision()` untouched. Either way the outcome is transcribed
/// before it is returned (AC 23).
pub fn apply(app: &mut App, action: &AgentAction) -> AgentOutcome {
    let outcome = match plan(action, &app.document) {
        Planned::Answer(outcome) => outcome,
        Planned::Commit(command, sentence) => {
            app.commit(command);
            AgentOutcome::Ok(with_count(&sentence, app.document.entity_count()))
        }
    };
    transcribe(app, &outcome);
    outcome
}

/// Append one `agent.chat` row for `outcome`, content **verbatim** (AC 23).
///
/// Two roles, and the split is the one thing the operator most needs to see at
/// a glance: `tool` is something that happened, `refused` is something that did
/// not. LCV-125 decides how each looks; this decides only that it exists.
///
/// Called by [`apply`] for everything it decides, and by `agent_poll` for the
/// one outcome `apply` never sees — a fence refusal, which is a property of the
/// turn rather than of the action (ADR 0007 §D4).
pub(crate) fn transcribe(app: &mut App, outcome: &AgentOutcome) {
    let role = if outcome.is_refused() {
        "refused"
    } else {
        "tool"
    };
    app.agent
        .chat
        .push((role.to_owned(), outcome.text().to_owned()));
}

/// Decide, against the pre-mutation document, what this action becomes.
///
/// Everything that has to be read *before* the change — the entity a delete is
/// about to remove, the indices it is about to renumber — is captured here,
/// because afterwards it is gone.
fn plan(action: &AgentAction, doc: &Document) -> Planned {
    match *action {
        AgentAction::CreateLine { x1, y1, x2, y2 } => Planned::Commit(
            Box::new(CreateLine::new(Line::new(
                Vec2::new(x1, y1),
                Vec2::new(x2, y2),
            ))),
            format!("Line created: {} → {} mm.", pt(x1, y1), pt(x2, y2)),
        ),
        AgentAction::CreateCircle { cx, cy, r } => Planned::Commit(
            Box::new(CreateCircle::new(Circle::new(Vec2::new(cx, cy), r))),
            format!("Circle created: center {} mm, r = {r:.3} mm.", pt(cx, cy)),
        ),
        AgentAction::CreateArc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
        } => Planned::Commit(
            Box::new(CreateArc::new(GeoArc::new(
                Vec2::new(cx, cy),
                r,
                start,
                end,
                ccw,
            ))),
            format!(
                "Arc created: center {} mm, r = {r:.3} mm, {}.",
                pt(cx, cy),
                sweep(start, end, ccw)
            ),
        ),
        AgentAction::Delete { index } => match in_range(index, doc) {
            Err(refusal) => Planned::Answer(refusal),
            Ok(entity) => Planned::Commit(
                Box::new(DeleteEntities::new(vec![index])),
                format!(
                    "Deleted entity {index} ({}).{}",
                    describe(entity),
                    shift_note(index, doc.entity_count())
                ),
            ),
        },
        AgentAction::Move { index, dx, dy } => match in_range(index, doc) {
            Err(refusal) => Planned::Answer(refusal),
            Ok(entity) => Planned::Commit(
                Box::new(MoveEntities::new(vec![index], Vec2::new(dx, dy))),
                format!(
                    "Moved entity {index} ({}) by {} mm.",
                    describe(entity),
                    pt(dx, dy)
                ),
            ),
        },
        AgentAction::QueryEntities => Planned::Answer(AgentOutcome::Ok(list_entities(doc))),
        AgentAction::QuerySelection => Planned::Answer(AgentOutcome::Ok(list_selection(doc))),
    }
}

/// The check the worker thread cannot make: is `index` a real entity?
///
/// ADR 0007 §D2a — the wording of the refusal is the wording `get_index` used
/// to produce, re-homed where `entities.len()` is actually knowable.
fn in_range(index: usize, doc: &Document) -> Result<&Entity, AgentOutcome> {
    doc.entities.get(index).ok_or_else(|| {
        AgentOutcome::Refused(format!(
            "index {index} is out of range (the drawing has {} entities)",
            doc.entity_count()
        ))
    })
}

/// Append the post-mutation entity count (ADR 0007 §D5). One space, one
/// sentence, so a test can pin it.
fn with_count(sentence: &str, count: usize) -> String {
    format!("{sentence} The drawing now has {count} entities.")
}

/// Which indices a delete at `index` renumbered, given the count *before* it.
fn shift_note(index: usize, count_before: usize) -> String {
    if index + 1 < count_before {
        format!(
            " Indices {}..{} are now {}..{}.",
            index + 1,
            count_before - 1,
            index,
            count_before - 2
        )
    } else {
        " No indices shifted.".to_string()
    }
}

/// A point in the one format every agent-facing string uses: mm, 3 decimals.
fn pt(x: f64, y: f64) -> String {
    format!("({x:.3}, {y:.3})")
}

/// An arc's sweep, converted back to the degrees the operator dictated.
/// Radians are the kernel's unit; degrees are the presentation unit.
fn sweep(start: f64, end: f64, ccw: bool) -> String {
    let dir = if ccw { "ccw" } else { "cw" };
    format!("{:.1}°→{:.1}° {dir}", start.to_degrees(), end.to_degrees())
}

/// The one word that says what an entity is.
fn kind(entity: &Entity) -> &'static str {
    match entity {
        Entity::Line(_) => "line",
        Entity::Circle(_) => "circle",
        Entity::Arc(_) => "arc",
    }
}

/// An entity's mm geometry, without its kind. Split from [`describe`] because
/// the two readers want different punctuation: a listing row reads
/// `0: line (0, 0) → …`, while a delete reads `Deleted entity 0 (line, …)`.
fn geometry(entity: &Entity) -> String {
    match entity {
        Entity::Line(l) => format!("{} → {} mm", pt(l.p1.x, l.p1.y), pt(l.p2.x, l.p2.y)),
        Entity::Circle(c) => format!(
            "center {} mm, r = {:.3} mm",
            pt(c.center.x, c.center.y),
            c.r
        ),
        Entity::Arc(a) => format!(
            "center {} mm, r = {:.3} mm, {}",
            pt(a.center.x, a.center.y),
            a.r,
            sweep(a.start_angle, a.end_angle, a.ccw)
        ),
    }
}

/// One phrase naming an entity's kind and its mm geometry, so a wrong target is
/// legible in the transcript instead of invisible (ADR 0007 §D5).
fn describe(entity: &Entity) -> String {
    format!("{}, {}", kind(entity), geometry(entity))
}

/// The bed, in the same mm-to-3-decimals format as everything else.
///
/// Read from the live [`Document::bed_mm`], never from the default constant:
/// an operator who set a 300 × 200 bed and asks the agent to fill it must not
/// be told about a 400 × 400 one (AC 14).
fn bed_line(doc: &Document) -> String {
    format!("Bed {:.3} × {:.3} mm.", doc.bed_mm[0], doc.bed_mm[1])
}

/// `QueryEntities`: the whole drawing, one entity per line, indices first.
///
/// The header says the count even when it is zero — the model needs to know
/// that it looked and found nothing, which reads differently from a tool that
/// failed to answer.
fn list_entities(doc: &Document) -> String {
    if doc.entities.is_empty() {
        return format!("The drawing is empty (0 entities). {}", bed_line(doc));
    }
    let mut out = format!(
        "The drawing has {} entities. {}",
        doc.entity_count(),
        bed_line(doc)
    );
    for (i, entity) in doc.entities.iter().enumerate() {
        out.push_str(&format!("\n{i}: {} {}", kind(entity), geometry(entity)));
    }
    out
}

/// `QuerySelection`: which indices are selected, in ascending order.
///
/// `Selection` iterates a `HashSet`, whose order is not guaranteed, so the
/// indices are sorted before they are rendered — otherwise the same selection
/// would narrate differently from run to run.
fn list_selection(doc: &Document) -> String {
    let mut indices: Vec<usize> = doc.selection.iter().collect();
    indices.sort_unstable();
    if indices.is_empty() {
        return format!(
            "Nothing is selected (the drawing has {} entities).",
            doc.entity_count()
        );
    }
    let list: Vec<String> = indices.iter().map(|i| i.to_string()).collect();
    format!(
        "{} of {} entities are selected: {}.",
        indices.len(),
        doc.entity_count(),
        list.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::SelectionCommand;
    use core::f64::consts::FRAC_PI_2;

    /// ADR 0002 §A2: never `App::new()` in a test — it reads the developer's
    /// real settings and autosave paths. The fixture commits through the same
    /// contract the production code does, so `revision()` starts where a real
    /// session's would.
    fn app_with(entities: Vec<Entity>) -> App {
        let mut app = App::default();
        for entity in entities {
            let command: Box<dyn Command> = match entity {
                Entity::Line(l) => Box::new(CreateLine::new(l)),
                Entity::Circle(c) => Box::new(CreateCircle::new(c)),
                Entity::Arc(a) => Box::new(CreateArc::new(a)),
            };
            app.commit(command);
        }
        app
    }

    fn line(y: f64) -> Entity {
        Entity::Line(Line::new(Vec2::new(0.0, y), Vec2::new(10.0, y)))
    }

    fn circle() -> Entity {
        Entity::Circle(Circle::new(Vec2::new(10.0, 10.0), 5.0))
    }

    // ── AC 6: the range check lives here and refuses ─────────────────────────

    /// AC 6, app side — an index past the end of the drawing is refused, in
    /// the wording `get_index` used to produce, and **nothing** happens: no
    /// commit, no revision bump, no entity lost.
    #[test]
    fn an_out_of_range_index_is_refused_and_changes_nothing() {
        let mut app = app_with(vec![line(0.0), line(1.0), circle()]);
        let before = app.history.revision();

        for action in [
            AgentAction::Delete { index: 7 },
            AgentAction::Move {
                index: 7,
                dx: 1.0,
                dy: 1.0,
            },
        ] {
            let outcome = apply(&mut app, &action);
            assert_eq!(
                outcome,
                AgentOutcome::Refused(
                    "index 7 is out of range (the drawing has 3 entities)".to_string()
                ),
                "{action:?}"
            );
            assert!(outcome.text().contains("out of range"));
            assert!(outcome.text().contains('3'));
            assert_eq!(app.history.revision(), before, "a refusal commits nothing");
            assert_eq!(app.document.entity_count(), 3);
        }
    }

    /// AC 6 — the boundary is `len`, not `len + 1`: the last real index is
    /// applied and the one past it is refused.
    #[test]
    fn the_range_boundary_is_exactly_the_entity_count() {
        let mut app = app_with(vec![line(0.0), line(1.0)]);
        assert!(apply(&mut app, &AgentAction::Delete { index: 2 }).is_refused());
        assert!(!apply(&mut app, &AgentAction::Delete { index: 1 }).is_refused());
    }

    /// AC 6 — an empty drawing refuses index 0 and says so in its own terms.
    #[test]
    fn an_empty_drawing_refuses_index_zero() {
        let mut app = App::default();
        let outcome = apply(&mut app, &AgentAction::Delete { index: 0 });
        assert_eq!(
            outcome,
            AgentOutcome::Refused(
                "index 0 is out of range (the drawing has 0 entities)".to_string()
            )
        );
    }

    // ── AC 8: one action, one command, one revision ──────────────────────────

    /// AC 8 — each create lands the right entity, bumps the revision by
    /// exactly one, and is undoable back to the previous state.
    #[test]
    fn every_create_commits_exactly_one_undoable_command() {
        let cases = [
            (
                AgentAction::CreateLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 20.0,
                    y2: 0.0,
                },
                Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0))),
            ),
            (
                AgentAction::CreateCircle {
                    cx: 5.0,
                    cy: 6.0,
                    r: 3.0,
                },
                Entity::Circle(Circle::new(Vec2::new(5.0, 6.0), 3.0)),
            ),
            (
                AgentAction::CreateArc {
                    cx: 0.0,
                    cy: 0.0,
                    r: 4.0,
                    start: 0.0,
                    end: FRAC_PI_2,
                    ccw: true,
                },
                Entity::Arc(GeoArc::new(Vec2::new(0.0, 0.0), 4.0, 0.0, FRAC_PI_2, true)),
            ),
        ];
        for (action, expected) in cases {
            let mut app = App::default();
            let before = app.history.revision();
            let outcome = apply(&mut app, &action);
            assert!(!outcome.is_refused(), "{action:?} → {outcome:?}");
            assert_eq!(app.document.entities, vec![expected], "{action:?}");
            assert_eq!(app.history.revision(), before + 1, "{action:?}");
            assert!(app.history.can_undo(), "{action:?}");
            assert!(app.history.undo(&mut app.document));
            assert!(app.document.entities.is_empty(), "{action:?}");
        }
    }

    /// AC 8 — a delete removes the entity it named and one `undo` puts it back
    /// where it was, `PartialEq`-equal and in the same slot.
    #[test]
    fn delete_commits_one_undoable_command() {
        let mut app = app_with(vec![line(0.0), circle(), line(2.0)]);
        let before = app.history.revision();
        let outcome = apply(&mut app, &AgentAction::Delete { index: 1 });
        assert!(!outcome.is_refused(), "{outcome:?}");
        assert_eq!(app.document.entities, vec![line(0.0), line(2.0)]);
        assert_eq!(app.history.revision(), before + 1);
        assert!(app.history.can_undo());
        assert!(app.history.undo(&mut app.document));
        assert_eq!(app.document.entities, vec![line(0.0), circle(), line(2.0)]);
    }

    /// AC 8 — a move translates exactly the entity it named, and undo reverses
    /// it precisely.
    #[test]
    fn move_commits_one_undoable_command() {
        let mut app = app_with(vec![line(0.0), line(5.0)]);
        let before = app.history.revision();
        let outcome = apply(
            &mut app,
            &AgentAction::Move {
                index: 0,
                dx: 3.0,
                dy: 4.0,
            },
        );
        assert!(!outcome.is_refused(), "{outcome:?}");
        let Entity::Line(moved) = app.document.entities[0] else {
            panic!("entity 0 must still be a line")
        };
        assert_eq!(moved.p1, Vec2::new(3.0, 4.0));
        assert_eq!(moved.p2, Vec2::new(13.0, 4.0));
        assert_eq!(
            app.document.entities[1],
            line(5.0),
            "entity 1 must not move"
        );
        assert_eq!(app.history.revision(), before + 1);
        assert!(app.history.undo(&mut app.document));
        assert_eq!(app.document.entities, vec![line(0.0), line(5.0)]);
    }

    /// AC 8 — the two query variants read and format. Nothing commits, so
    /// `revision()` and the entity count are untouched, and there is nothing
    /// on the undo stack to take back.
    #[test]
    fn queries_commit_nothing() {
        let mut app = app_with(vec![line(0.0), circle()]);
        let before = app.history.revision();
        let stack_depth = app.history.len();
        for action in [AgentAction::QueryEntities, AgentAction::QuerySelection] {
            let outcome = apply(&mut app, &action);
            assert!(!outcome.is_refused(), "{action:?} → {outcome:?}");
            assert!(!outcome.text().is_empty());
            assert_eq!(app.history.revision(), before, "{action:?} must not commit");
            assert_eq!(app.document.entity_count(), 2, "{action:?}");
            assert_eq!(app.history.len(), stack_depth, "{action:?}");
        }
    }

    /// LCV-123 AC 14 — `QueryEntities` really reads the live drawing: the
    /// header counts it, the bed line measures it, and one row per entity
    /// names its index, its kind and its mm geometry.
    ///
    /// The listing rows are asserted **whole**, not by `contains`, because the
    /// row is the thing the model reads back before it decides which index to
    /// delete: a stray or missing field there is a wrong cut.
    #[test]
    fn query_entities_lists_the_live_drawing() {
        let mut app = app_with(vec![
            line(0.0),
            circle(),
            Entity::Arc(GeoArc::new(Vec2::new(0.0, 0.0), 8.0, 0.0, FRAC_PI_2, true)),
        ]);
        assert_eq!(
            apply(&mut app, &AgentAction::QueryEntities).into_text(),
            "The drawing has 3 entities. Bed 400.000 × 400.000 mm.\n\
             0: line (0.000, 0.000) → (10.000, 0.000) mm\n\
             1: circle center (10.000, 10.000) mm, r = 5.000 mm\n\
             2: arc center (0.000, 0.000) mm, r = 8.000 mm, 0.0°→90.0° ccw"
        );
    }

    /// AC 14 — an empty drawing says so **and still says how many**, so the
    /// model can tell "I looked and it is empty" from "the tool said nothing".
    #[test]
    fn query_entities_on_an_empty_drawing_still_reports_a_count_and_a_bed() {
        let mut app = App::default();
        assert_eq!(
            apply(&mut app, &AgentAction::QueryEntities).into_text(),
            "The drawing is empty (0 entities). Bed 400.000 × 400.000 mm."
        );
    }

    /// AC 14 — the bed line follows [`Document::bed_mm`], never a constant.
    ///
    /// Run at two different, non-square bed sizes: a hardcoded `400.000 ×
    /// 400.000` passes the test above and fails both rows here, and a line that
    /// printed the same number twice fails on the asymmetric bed.
    #[test]
    fn the_bed_line_follows_the_documents_own_bed() {
        for (bed, expected) in [
            ([300.0, 200.0], "Bed 300.000 × 200.000 mm."),
            ([1200.5, 600.25], "Bed 1200.500 × 600.250 mm."),
        ] {
            let mut app = App::default();
            app.document.bed_mm = bed;
            let empty = apply(&mut app, &AgentAction::QueryEntities).into_text();
            assert!(empty.ends_with(expected), "{empty}");

            let mut app = app_with(vec![line(0.0)]);
            app.document.bed_mm = bed;
            let listed = apply(&mut app, &AgentAction::QueryEntities).into_text();
            assert!(
                listed.starts_with(&format!("The drawing has 1 entities. {expected}")),
                "{listed}"
            );
        }
    }

    /// AC 8 — `QuerySelection` reports the live selection in **ascending index
    /// order**.
    ///
    /// `Selection` iterates a `HashSet`, whose order is documented as not
    /// guaranteed, so an implementation that skipped the sort would narrate the
    /// same selection differently from run to run. A three-index case does not
    /// prove anything — three elements come out ascending by luck one time in
    /// six, and that is exactly how this survived the first mutation pass. Two
    /// independent twelve-index selections do: each `HashSet` gets its own
    /// `RandomState`, so the unsorted implementation would have to draw
    /// ascending order twice out of `12!` arrangements.
    #[test]
    fn query_selection_reports_sorted_indices() {
        let entities: Vec<Entity> = (0..14).map(|i| line(i as f64)).collect();
        let mut app = app_with(entities);
        assert_eq!(
            apply(&mut app, &AgentAction::QuerySelection).into_text(),
            "Nothing is selected (the drawing has 14 entities).",
            "AC 15 — an empty selection still reports the drawing's size"
        );

        for chosen in [
            vec![11, 3, 7, 0, 13, 5, 9, 1, 12, 4, 8, 2],
            vec![2, 9, 0, 6, 11, 4, 13, 8, 1, 10, 3, 7],
        ] {
            app.commit(Box::new(SelectionCommand::new(chosen.clone())));
            let text = apply(&mut app, &AgentAction::QuerySelection).into_text();
            let mut expected = chosen.clone();
            expected.sort_unstable();
            let rendered: Vec<String> = expected.iter().map(|i| i.to_string()).collect();
            assert_eq!(
                text,
                format!("12 of 14 entities are selected: {}.", rendered.join(", ")),
                "the indices must be ascending, whatever order the HashSet yields"
            );
        }
    }

    /// AC 15 — the 2-of-3 wording, with the indices ascending and comma-space
    /// separated, on the exact shape the demand names.
    #[test]
    fn query_selection_reports_two_of_three() {
        let mut app = app_with(vec![line(0.0), line(1.0), circle()]);
        app.commit(Box::new(SelectionCommand::new(vec![2, 0])));
        assert_eq!(
            apply(&mut app, &AgentAction::QuerySelection).into_text(),
            "2 of 3 entities are selected: 0, 2."
        );
    }

    /// AC 15 — an empty drawing has nothing selected and says `0`.
    #[test]
    fn query_selection_on_an_empty_drawing() {
        let mut app = App::default();
        assert_eq!(
            apply(&mut app, &AgentAction::QuerySelection).into_text(),
            "Nothing is selected (the drawing has 0 entities)."
        );
    }

    // ── AC 23: every action leaves a row ─────────────────────────────────────

    /// AC 23 — one row per action, role `tool`, content the outcome
    /// **verbatim**: the operator and the model read the same sentence.
    #[test]
    fn an_applied_action_appends_its_outcome_verbatim() {
        let mut app = App::default();
        let outcome = apply(
            &mut app,
            &AgentAction::CreateLine {
                x1: 0.0,
                y1: 0.0,
                x2: 20.0,
                y2: 0.0,
            },
        );
        assert_eq!(
            app.agent.chat,
            vec![("tool".to_owned(), outcome.text().to_owned())]
        );
        assert!(app.agent.chat[0].1.starts_with("Line created:"));
    }

    /// AC 23 — a refusal is role `refused`, not `tool`. Mutation (k) writes
    /// `tool` here; this is the assertion that catches it.
    #[test]
    fn a_refused_action_appends_a_refused_row() {
        let mut app = App::default();
        let outcome = apply(&mut app, &AgentAction::Delete { index: 7 });
        assert!(outcome.is_refused());
        assert_eq!(
            app.agent.chat,
            vec![("refused".to_owned(), outcome.text().to_owned())]
        );
    }

    /// AC 23 — a query is not special-cased: it appends a `tool` row like
    /// anything else, and it still commits nothing. Mutation (l) skips this
    /// row; the role sequence here is what goes red.
    #[test]
    fn a_query_appends_a_row_like_any_other_action() {
        let mut app = app_with(vec![line(0.0)]);
        let before = app.history.revision();
        let entities = apply(&mut app, &AgentAction::QueryEntities);
        let selection = apply(&mut app, &AgentAction::QuerySelection);
        assert_eq!(
            app.agent.chat,
            vec![
                ("tool".to_owned(), entities.text().to_owned()),
                ("tool".to_owned(), selection.text().to_owned()),
            ]
        );
        assert_eq!(
            app.history.revision(),
            before,
            "AC 16 — a query commits nothing"
        );
    }

    // ── AC 9: every mutating outcome carries the resulting count ─────────────

    /// AC 9 — the count is rendered from the document *after* the mutation, at
    /// two different starting sizes so a hardcoded string cannot pass. Deleting
    /// the suffix (mutation (e)) turns every row of this red.
    #[test]
    fn every_mutating_outcome_reports_the_resulting_count() {
        let create = AgentAction::CreateCircle {
            cx: 0.0,
            cy: 0.0,
            r: 1.0,
        };
        let mut empty = App::default();
        assert!(apply(&mut empty, &create)
            .text()
            .ends_with(" The drawing now has 1 entities."));

        let mut two = app_with(vec![line(0.0), line(1.0)]);
        assert!(apply(&mut two, &create)
            .text()
            .ends_with(" The drawing now has 3 entities."));

        let mut two = app_with(vec![line(0.0), line(1.0)]);
        assert!(apply(&mut two, &AgentAction::Delete { index: 0 })
            .text()
            .ends_with(" The drawing now has 1 entities."));

        let mut four = app_with(vec![line(0.0), line(1.0), line(2.0), line(3.0)]);
        assert!(apply(
            &mut four,
            &AgentAction::Move {
                index: 0,
                dx: 1.0,
                dy: 0.0
            }
        )
        .text()
        .ends_with(" The drawing now has 4 entities."));
    }

    /// AC 9 — the create sentences keep the wording and the precision they had
    /// before LCV-122 split dispatch apart: mm to 3 decimals, degrees to 1.
    #[test]
    fn the_create_sentences_are_unchanged_apart_from_the_suffix() {
        let mut app = App::default();
        assert_eq!(
            apply(
                &mut app,
                &AgentAction::CreateLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 20.0,
                    y2: 0.0
                }
            )
            .into_text(),
            "Line created: (0.000, 0.000) → (20.000, 0.000) mm. \
             The drawing now has 1 entities."
        );
        let mut app = App::default();
        assert_eq!(
            apply(
                &mut app,
                &AgentAction::CreateCircle {
                    cx: 5.0,
                    cy: 5.0,
                    r: 3.0
                }
            )
            .into_text(),
            "Circle created: center (5.000, 5.000) mm, r = 3.000 mm. \
             The drawing now has 1 entities."
        );
        let mut app = App::default();
        assert_eq!(
            apply(
                &mut app,
                &AgentAction::CreateArc {
                    cx: 0.0,
                    cy: 0.0,
                    r: 1.0,
                    start: 0.0,
                    end: FRAC_PI_2,
                    ccw: true
                }
            )
            .into_text(),
            "Arc created: center (0.000, 0.000) mm, r = 1.000 mm, 0.0°→90.0° ccw. \
             The drawing now has 1 entities."
        );
    }

    /// AC 9 — a clockwise arc says `cw`, and says it without saying `ccw`.
    #[test]
    fn a_clockwise_arc_says_cw() {
        let mut app = App::default();
        let text = apply(
            &mut app,
            &AgentAction::CreateArc {
                cx: 0.0,
                cy: 0.0,
                r: 1.0,
                start: 0.0,
                end: FRAC_PI_2,
                ccw: false,
            },
        )
        .into_text();
        assert!(text.contains("0.0°→90.0° cw."), "{text}");
        assert!(!text.contains("ccw"), "{text}");
    }

    // ── AC 10: what was deleted, and what moved ──────────────────────────────

    /// AC 10 — deleting index 1 of 4 names the entity it removed, in mm, and
    /// states the renumbering it caused.
    #[test]
    fn a_delete_names_the_entity_and_the_indices_that_shifted() {
        let mut app = app_with(vec![line(0.0), circle(), line(2.0), line(3.0)]);
        assert_eq!(
            apply(&mut app, &AgentAction::Delete { index: 1 }).into_text(),
            "Deleted entity 1 (circle, center (10.000, 10.000) mm, r = 5.000 mm). \
             Indices 2..3 are now 1..2. The drawing now has 3 entities."
        );
    }

    /// AC 10 — deleting the last entity shifts nothing, and says so rather
    /// than printing a degenerate range.
    #[test]
    fn deleting_the_last_entity_shifts_no_indices() {
        let mut app = app_with(vec![line(0.0), line(1.0), circle()]);
        let text = apply(&mut app, &AgentAction::Delete { index: 2 }).into_text();
        assert!(text.contains(" No indices shifted."), "{text}");
        assert!(!text.contains("are now"), "{text}");

        let mut only = app_with(vec![circle()]);
        let text = apply(&mut only, &AgentAction::Delete { index: 0 }).into_text();
        assert!(text.contains(" No indices shifted."), "{text}");
        assert!(text.ends_with(" The drawing now has 0 entities."), "{text}");
    }

    /// AC 10 — the shift note is computed from the count *before* the delete;
    /// deleting index 0 of 5 renumbers 1..4 down to 0..3.
    #[test]
    fn the_shift_note_is_computed_before_the_mutation() {
        let mut app = app_with(vec![line(0.0), line(1.0), line(2.0), line(3.0), line(4.0)]);
        let text = apply(&mut app, &AgentAction::Delete { index: 0 }).into_text();
        assert!(text.contains(" Indices 1..4 are now 0..3."), "{text}");
    }

    /// AC 10 — a move echoes the entity it moved, described from **before** the
    /// translation, plus the delta.
    #[test]
    fn a_move_names_the_entity_it_moved() {
        let mut app = app_with(vec![circle(), line(0.0)]);
        assert_eq!(
            apply(
                &mut app,
                &AgentAction::Move {
                    index: 0,
                    dx: 3.0,
                    dy: 4.0
                }
            )
            .into_text(),
            "Moved entity 0 (circle, center (10.000, 10.000) mm, r = 5.000 mm) \
             by (3.000, 4.000) mm. The drawing now has 2 entities."
        );
    }

    /// AC 10 — `describe` covers all three entity kinds, and an arc reports
    /// degrees even though the kernel stored radians.
    #[test]
    fn describe_covers_every_entity_kind() {
        assert_eq!(
            describe(&line(0.0)),
            "line, (0.000, 0.000) → (10.000, 0.000) mm"
        );
        assert_eq!(
            describe(&circle()),
            "circle, center (10.000, 10.000) mm, r = 5.000 mm"
        );
        assert_eq!(
            describe(&Entity::Arc(GeoArc::new(
                Vec2::new(1.0, 2.0),
                3.0,
                0.0,
                FRAC_PI_2,
                false
            ))),
            "arc, center (1.000, 2.000) mm, r = 3.000 mm, 0.0°→90.0° cw"
        );
    }

    // ── AC 8: the commit path, not a direct mutation ─────────────────────────

    /// AC 8 / AGENTS.md §"State and mutation" — this file must never reach into
    /// `document.entities` to change it. Mutation (f) writes exactly that, and
    /// the revision assertions above are what fail; this scan is the second
    /// line of defence and names the offence directly.
    ///
    /// Bounded at the bare `#[cfg(test)]` at column 0 with `concat!` needles,
    /// so it cannot match the literals in this very test. The positive control
    /// is the one commit call that must be here.
    #[test]
    fn agent_apply_only_ever_commits() {
        let src = include_str!("agent_apply.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("agent_apply.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("app.co", "mmit(command)")),
            "positive control: the App commit path must be in this file"
        );
        for forbidden in [
            concat!("entities.pu", "sh"),
            concat!("entities.re", "move"),
            concat!("entities.in", "sert"),
            concat!("entities.cl", "ear"),
            concat!("entities[", ""),
            concat!("std::thread", "::spawn"),
            concat!("e", "frame"),
            concat!("r", "fd"),
        ] {
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "agent_apply.rs must not contain `{forbidden}`: {hit:?}"
            );
        }
    }
}
