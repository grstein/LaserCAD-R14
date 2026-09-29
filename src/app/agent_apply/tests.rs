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
        AgentOutcome::Refused("index 0 is out of range (the drawing has 0 entities)".to_string())
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
                layer: None,
                x1: 0.0,
                y1: 0.0,
                x2: 20.0,
                y2: 0.0,
            },
            Entity::Line(Line::new(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0))),
        ),
        (
            AgentAction::CreateCircle {
                layer: None,
                cx: 5.0,
                cy: 6.0,
                r: 3.0,
            },
            Entity::Circle(Circle::new(Vec2::new(5.0, 6.0), 3.0)),
        ),
        (
            AgentAction::CreateArc {
                layer: None,
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
             Layers: Cut (current).\n\
             0: line (0.000, 0.000) → (10.000, 0.000) mm layer Cut\n\
             1: circle center (10.000, 10.000) mm, r = 5.000 mm layer Cut\n\
             2: arc center (0.000, 0.000) mm, r = 8.000 mm, 0.0°→90.0° ccw layer Cut"
    );
}

/// AC 14 — an empty drawing says so **and still says how many**, so the
/// model can tell "I looked and it is empty" from "the tool said nothing".
#[test]
fn query_entities_on_an_empty_drawing_still_reports_a_count_and_a_bed() {
    let mut app = App::default();
    assert_eq!(
        apply(&mut app, &AgentAction::QueryEntities).into_text(),
        "The drawing is empty (0 entities). Bed 400.000 × 400.000 mm.\n\
         Layers: Cut (current)."
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
        let first = empty.lines().next().unwrap_or_default();
        assert!(first.ends_with(expected), "{empty}");

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
            layer: None,
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

/// LCV-142 AC 10 — a malformed call is answered `Refused(reason)` verbatim,
/// commits nothing and leaves one `refused` row.
#[test]
fn a_malformed_call_is_refused_with_its_reason_and_commits_nothing() {
    let mut app = app_with(vec![line(0.0)]);
    let revision = app.history.revision();
    let reason = "tool `create_line` missing required argument `x2`";
    let action = AgentAction::Malformed {
        tool: "create_line".into(),
        reason: reason.into(),
    };
    assert_eq!(
        apply(&mut app, &action),
        AgentOutcome::Refused(reason.into())
    );
    assert_eq!(app.history.revision(), revision);
    assert_eq!(app.document.entity_count(), 1);
    assert_eq!(
        app.agent.chat,
        vec![("refused".to_owned(), reason.to_owned())]
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
        layer: None,
        cx: 0.0,
        cy: 0.0,
        r: 1.0,
    };
    let mut empty = App::default();
    assert!(
        apply(&mut empty, &create)
            .text()
            .ends_with(" The drawing now has 1 entities.")
    );

    let mut two = app_with(vec![line(0.0), line(1.0)]);
    assert!(
        apply(&mut two, &create)
            .text()
            .ends_with(" The drawing now has 3 entities.")
    );

    let mut two = app_with(vec![line(0.0), line(1.0)]);
    assert!(
        apply(&mut two, &AgentAction::Delete { index: 0 })
            .text()
            .ends_with(" The drawing now has 1 entities.")
    );

    let mut four = app_with(vec![line(0.0), line(1.0), line(2.0), line(3.0)]);
    assert!(
        apply(
            &mut four,
            &AgentAction::Move {
                index: 0,
                dx: 1.0,
                dy: 0.0
            }
        )
        .text()
        .ends_with(" The drawing now has 4 entities.")
    );
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
                layer: None,
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
                layer: None,
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
                layer: None,
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
            layer: None,
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

// ── LCV-144 AC 5, 7, 8: one batch, one command, one sentence ─────────────

fn drawing(items: Vec<DrawingItem>) -> AgentAction {
    AgentAction::CreateDrawing { layer: None, items }
}

/// LCV-144 AC 8 — both outcome sentences, character for character, on a
/// 3-entity drawing; AC 7 — one revision, one undo, whatever the count.
#[test]
fn a_batch_narrates_its_indices_count_and_revision() {
    let mut app = app_with(vec![line(0.0), line(1.0), circle()]);
    let before = app.history.revision();
    let outcome = apply(
        &mut app,
        &drawing(vec![
            DrawingItem::Line {
                x1: 0.0,
                y1: 5.0,
                x2: 1.0,
                y2: 5.0,
            },
            DrawingItem::Circle {
                cx: 2.0,
                cy: 2.0,
                r: 1.0,
            },
        ]),
    );
    assert_eq!(
        outcome,
        AgentOutcome::Ok(format!(
            "Created 2 entities (indices 3..=4). The drawing now has 5 entities. Revision {}.",
            before + 1
        ))
    );
    assert_eq!(app.history.revision(), before + 1);
    assert_eq!(app.agent.chat.last().unwrap().1, outcome.text());

    let mut app = app_with(vec![line(0.0), line(1.0), circle()]);
    let outcome = apply(
        &mut app,
        &drawing(vec![DrawingItem::Arc {
            cx: 1.0,
            cy: 1.0,
            r: 2.0,
            start: 0.0,
            end: FRAC_PI_2,
            ccw: true,
        }]),
    );
    assert_eq!(
        outcome,
        AgentOutcome::Ok(format!(
            "Created 1 entity (index 3). The drawing now has 4 entities. Revision {}.",
            before + 1
        ))
    );
    let AgentOutcome::Ok(listing) = apply(&mut app, &AgentAction::QueryEntities) else {
        panic!("a query is answered")
    };
    assert!(
        listing.contains("\n3: arc center (1.000, 1.000) mm, r = 2.000 mm, 0.0°→90.0° ccw"),
        "{listing}"
    );
    assert!(app.history.undo(&mut app.document));
    assert_eq!(
        app.document.entity_count(),
        3,
        "one undo takes the batch back"
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
/// is the one grouped commit call that must be here (ADR 0007 §D12).
#[test]
fn agent_apply_only_ever_commits() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/app/agent_apply.rs"
    ));
    let at = src
        .find("\n#[cfg(test)]")
        .expect("agent_apply.rs must have a bare #[cfg(test)] marker");
    let implementation = &src[..at];

    assert!(
        implementation.contains(concat!("history.commit_", "grouped(command")),
        "positive control: the grouped commit path must be in this file"
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
