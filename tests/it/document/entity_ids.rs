//! LCV-188 — stable entity ids in the document (ADR 0014).

use lasercad::document::{Document, Entity, EntityId, Layer, LayerId};
use lasercad::geometry::{Line, Vec2};

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

/// AC 2 — `from_parts` numbers its entities `e1..=en`.
#[test]
fn from_parts_numbers_e1_to_en() {
    let layers = vec![Layer::default_cut()];
    let cut = LayerId(0);
    let entities = vec![line(0.0), line(1.0), line(2.0)];
    let doc = Document::from_parts([400.0; 2], layers, cut, entities, vec![cut; 3])
        .expect("valid parts");
    assert_eq!(ids(&doc), ["e1", "e2", "e3"]);
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
