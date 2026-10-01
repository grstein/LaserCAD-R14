//! LCV-188 — stable entity ids in the document (ADR 0014).

use lasercad::document::commands::CreateEntities;
use lasercad::document::{
    Command, CopyEntities, CreateArc, CreateCircle, CreateLine, DeleteEntities, Document, Entity,
    EntityId, ExtendEntity, History, Layer, LayerId, MoveEntities, TransformEntities, TrimEntity,
};
use lasercad::geometry::{Arc, Circle, Line, Transform, Vec2};

fn line(x: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x, 0.0), Vec2::new(x, 10.0)))
}

/// Every entity's id, printed as the agent sees it.
fn ids(doc: &Document) -> Vec<String> {
    (0..doc.entity_count())
        .map(|i| doc.entity_id(i).expect("in range").to_string())
        .collect()
}

/// AC 2 — pushes take `e1, e2, …`; a delete changes no other entity's id.
#[test]
fn pushes_get_fresh_ids_and_a_delete_keeps_the_others() {
    let mut doc = Document::default();
    for x in 0..4 {
        doc.push_current(line(f64::from(x)));
    }
    assert_eq!(ids(&doc), ["e1", "e2", "e3", "e4"]);
    doc.remove_entity(1);
    assert_eq!(ids(&doc), ["e1", "e3", "e4"]);
    assert_eq!(doc.entity_id(3), None);
}

/// AC 2 — `entity_id` and `index_of` are inverse; an unknown id has no index.
#[test]
fn index_of_and_entity_id_round_trip() {
    let mut doc = Document::default();
    for x in 0..3 {
        doc.push_current(line(f64::from(x)));
    }
    doc.remove_entity(0);
    for i in 0..doc.entity_count() {
        let id = doc.entity_id(i).expect("in range");
        assert_eq!(doc.index_of(id), Some(i));
    }
    assert_eq!(doc.index_of(EntityId(1)), None);
    assert_eq!(doc.index_of(EntityId(4)), None);
    assert_eq!(EntityId(12).to_string(), "e12");
}

/// AC 2 — `from_parts` numbers its entities `e1..=en`, and the next push
/// continues at `e(n+1)`.
#[test]
fn from_parts_numbers_e1_to_en() {
    let layers = vec![Layer::default_cut()];
    let cut = LayerId(0);
    let entities = vec![line(0.0), line(1.0), line(2.0)];
    let mut doc =
        Document::from_parts([400.0; 2], layers, cut, entities, vec![cut; 3]).expect("valid parts");
    assert_eq!(ids(&doc), ["e1", "e2", "e3"]);
    doc.push_current(line(3.0));
    assert_eq!(ids(&doc), ["e1", "e2", "e3", "e4"]);
}

/// AC 2 — a push after deletes and truncates never reuses an id.
#[test]
fn a_push_after_deletes_and_truncates_never_reuses_an_id() {
    let mut doc = Document::default();
    for x in 0..5 {
        doc.push_current(line(f64::from(x)));
    }
    doc.remove_entity(4);
    doc.truncate_entities(2);
    doc.push_current(line(9.0));
    assert_eq!(ids(&doc), ["e1", "e2", "e6"]);
}

fn seg(ax: f64, ay: f64, bx: f64, by: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(ax, ay), Vec2::new(bx, by)))
}

/// Four entities `e1..=e4`: a horizontal line, a vertical line crossing it at
/// x = 5, a short line, and a vertical boundary at x = 20.
fn seeded() -> Document {
    let mut doc = Document::default();
    doc.push_current(seg(0.0, 0.0, 10.0, 0.0));
    doc.push_current(seg(5.0, -5.0, 5.0, 5.0));
    doc.push_current(seg(0.0, 8.0, 4.0, 8.0));
    doc.push_current(seg(20.0, -10.0, 20.0, 10.0));
    doc
}

/// Commit `cmd` on [`seeded`], undo, redo: undo gives back the ids before the
/// commit and redo the ids after it.
fn assert_round_trip(label: &str, cmd: Box<dyn Command>) {
    let mut doc = seeded();
    let mut history = History::new();
    let before = ids(&doc);
    history.commit(cmd, &mut doc);
    let after = ids(&doc);
    assert!(history.undo(&mut doc), "{label}: undo");
    assert_eq!(ids(&doc), before, "{label}: undo restores the ids");
    assert!(history.redo(&mut doc), "{label}: redo");
    assert_eq!(ids(&doc), after, "{label}: redo restores the ids");
}

/// AC 3 — undo and redo restore ids for every creating and editing command;
/// in-place edits keep them.
#[test]
fn undo_and_redo_restore_ids_for_every_command() {
    let v = Vec2::new;
    let arc = Arc::new(v(0.0, 0.0), 3.0, 0.0, 1.0, true);
    let rotate = Transform::Rotate {
        base: v(0.0, 0.0),
        angle: 0.5,
    };
    let mirror = Transform::Mirror {
        a: v(0.0, -1.0),
        b: v(1.0, -1.0),
    };
    let cases: Vec<(&str, Box<dyn Command>)> = vec![
        (
            "line",
            Box::new(CreateLine::new(Line::new(v(1.0, 1.0), v(2.0, 2.0)))),
        ),
        (
            "circle",
            Box::new(CreateCircle::new(Circle::new(v(3.0, 3.0), 1.0))),
        ),
        ("arc", Box::new(CreateArc::new(arc))),
        (
            "batch",
            Box::new(CreateEntities::new(vec![line(30.0), line(31.0)])),
        ),
        (
            "copy",
            Box::new(CopyEntities::new(vec![0, 2], v(0.0, 40.0))),
        ),
        (
            "rotate",
            Box::new(TransformEntities::new(vec![0, 2], rotate)),
        ),
        (
            "mirror keep",
            Box::new(TransformEntities::new(vec![1, 3], mirror).with_keep_source(true)),
        ),
        ("delete", Box::new(DeleteEntities::new(vec![0, 2]))),
        ("move", Box::new(MoveEntities::new(vec![1, 3], v(1.0, 0.0)))),
        ("trim", Box::new(TrimEntity::new(0, 1, v(2.0, 0.0)))),
        ("extend", Box::new(ExtendEntity::new(2, 3, 1))),
    ];
    for (label, cmd) in cases {
        assert_round_trip(label, cmd);
    }
    let mut doc = seeded();
    let mut history = History::new();
    history.commit(Box::new(TrimEntity::new(0, 1, v(2.0, 0.0))), &mut doc);
    history.commit(Box::new(ExtendEntity::new(2, 3, 1)), &mut doc);
    assert_eq!(
        ids(&doc),
        ["e1", "e2", "e3", "e4"],
        "in-place edits keep ids"
    );
}

/// AC 3 — a mixed history gives the same id list after undo-all and redo-all.
#[test]
fn a_mixed_history_replays_the_same_ids() {
    let v = Vec2::new;
    let mut doc = seeded();
    let mut history = History::new();
    let start = ids(&doc);
    history.commit(
        Box::new(CreateLine::new(Line::new(v(1.0, 1.0), v(2.0, 2.0)))),
        &mut doc,
    );
    history.commit(Box::new(DeleteEntities::new(vec![1, 3])), &mut doc);
    history.commit(
        Box::new(CopyEntities::new(vec![0, 2], v(0.0, 40.0))),
        &mut doc,
    );
    history.commit(
        Box::new(CreateEntities::new(vec![line(30.0), line(31.0)])),
        &mut doc,
    );
    history.commit(Box::new(DeleteEntities::new(vec![0, 4])), &mut doc);
    let rotate = Transform::Rotate {
        base: v(0.0, 0.0),
        angle: 0.5,
    };
    let keep = TransformEntities::new(vec![1], rotate).with_keep_source(true);
    history.commit(Box::new(keep), &mut doc);
    let end = ids(&doc);
    while history.undo(&mut doc) {}
    assert_eq!(ids(&doc), start);
    while history.redo(&mut doc) {}
    assert_eq!(ids(&doc), end);
    doc.push_current(line(50.0));
    assert_eq!(
        doc.entity_id(doc.entity_count() - 1).map(|id| id.0),
        Some(11)
    );
}

/// ADR 0014 — putting back an id that is live would duplicate it: debug
/// builds catch it.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "restored id e1 is live")]
fn restoring_a_live_id_is_caught_in_debug() {
    let mut doc = Document::default();
    doc.push_current(line(0.0));
    let cut = doc.current_layer();
    doc.insert_entity(0, line(1.0), cut, EntityId(1));
}

/// ADR 0014 — putting back an id never handed out would collide with a
/// later push: debug builds catch it.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "restored id e2 never handed out")]
fn restoring_an_unissued_id_is_caught_in_debug() {
    let mut doc = Document::default();
    doc.push_current(line(0.0));
    let cut = doc.current_layer();
    doc.insert_entity(0, line(1.0), cut, EntityId(2));
}
