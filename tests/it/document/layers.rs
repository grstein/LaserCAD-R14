//! LCV-156 — layer membership through the document commands (AC 2, AC 8).

use lasercad::document::commands::CreateEntities;
use lasercad::document::{
    CreateArc, CreateCircle, CreateLine, DeleteEntities, Document, Entity, History, Layer, LayerId,
};
use lasercad::geometry::{Arc, Circle, Line, Vec2};

const CUT: LayerId = LayerId(0);
const MARK: LayerId = LayerId(1);

fn layer(id: LayerId, name: &str, color: [u8; 3]) -> Layer {
    let name = name.to_owned();
    Layer {
        id,
        name,
        color,
        output: true,
    }
}

/// A blank document with `Cut` and `Mark`, `current` current.
fn two_layer_doc(current: LayerId) -> Document {
    let layers = vec![
        layer(CUT, "Cut", [255, 0, 0]),
        layer(MARK, "Mark", [0, 0, 255]),
    ];
    Document::from_parts([400.0; 2], layers, current, vec![], vec![]).expect("valid layers")
}

fn line(x: f64) -> Line {
    Line::new(Vec2::new(x, 0.0), Vec2::new(x, 10.0))
}

fn memberships(doc: &Document) -> Vec<LayerId> {
    (0..doc.entity_count())
        .map(|i| doc.entity_layer(i).expect("in range"))
        .collect()
}

/// AC 2 — every create command puts its entity on the current layer.
#[test]
fn create_commands_use_the_current_layer() {
    let mut doc = two_layer_doc(MARK);
    let mut history = History::new();
    history.commit(Box::new(CreateLine::new(line(0.0))), &mut doc);
    let circle = Circle::new(Vec2::new(5.0, 5.0), 2.0);
    history.commit(Box::new(CreateCircle::new(circle)), &mut doc);
    let arc = Arc::new(Vec2::new(0.0, 0.0), 3.0, 0.0, 1.0, true);
    history.commit(Box::new(CreateArc::new(arc)), &mut doc);
    let batch = vec![Entity::Line(line(1.0)), Entity::Line(line(2.0))];
    history.commit(Box::new(CreateEntities::new(batch)), &mut doc);
    assert_eq!(memberships(&doc), vec![MARK; 5]);
}

/// AC 2 — `on_layer` overrides the current layer, and redo keeps the layer
/// the command first landed on.
#[test]
fn on_layer_overrides_and_redo_keeps_the_layer() {
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    history.commit(
        Box::new(CreateLine::new(line(0.0)).on_layer(MARK)),
        &mut doc,
    );
    let batch = CreateEntities::new(vec![Entity::Line(line(1.0))]).on_layer(MARK);
    history.commit(Box::new(batch), &mut doc);
    history.commit(Box::new(CreateLine::new(line(2.0))), &mut doc);
    assert_eq!(memberships(&doc), vec![MARK, MARK, CUT]);
    assert!(history.undo(&mut doc) && history.undo(&mut doc) && history.undo(&mut doc));
    assert_eq!(doc.entity_count(), 0);
    assert!(history.redo(&mut doc) && history.redo(&mut doc) && history.redo(&mut doc));
    assert_eq!(memberships(&doc), vec![MARK, MARK, CUT]);
}

/// AC 8 — undoing a delete restores each entity on its own layer, in place.
#[test]
fn delete_undo_restores_membership() {
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    for (i, id) in [CUT, MARK, CUT, MARK].into_iter().enumerate() {
        history.commit(
            Box::new(CreateLine::new(line(i as f64)).on_layer(id)),
            &mut doc,
        );
    }
    history.commit(Box::new(DeleteEntities::new(vec![3, 1, 0])), &mut doc);
    assert_eq!(memberships(&doc), vec![CUT]);
    assert!(history.undo(&mut doc));
    assert_eq!(memberships(&doc), vec![CUT, MARK, CUT, MARK]);
    assert_eq!(doc.entities[1], Entity::Line(line(1.0)));
}

/// Tiny deterministic generator for the membership property test.
struct XorShift(u64);

impl XorShift {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// AC 2 / AC 8 — random create/delete/undo/redo sequences keep one layer per
/// entity, and every entity keeps the layer it was created on.
#[test]
fn random_command_sequences_keep_membership() {
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    for step in 0..2000 {
        match rng.below(5) {
            0 | 1 => {
                let id = if rng.below(2) == 0 { CUT } else { MARK };
                // Encode the layer in the geometry so it can be checked later.
                let x = step as f64 * 10.0 + f64::from(id.0);
                history.commit(Box::new(CreateLine::new(line(x)).on_layer(id)), &mut doc);
            }
            2 if doc.entity_count() > 0 => {
                let n = doc.entity_count() as u64;
                let mut picks: Vec<usize> = (0..3).map(|_| rng.below(n) as usize).collect();
                picks.sort_unstable();
                picks.dedup();
                history.commit(Box::new(DeleteEntities::new(picks)), &mut doc);
            }
            3 => {
                history.undo(&mut doc);
            }
            _ => {
                history.redo(&mut doc);
            }
        }
        assert_eq!(memberships(&doc).len(), doc.entities.len());
        for (i, entity) in doc.entities.iter().enumerate() {
            let Entity::Line(l) = entity else {
                panic!("only lines")
            };
            let expected = LayerId((l.p1.x as u32) % 10);
            assert_eq!(
                doc.entity_layer(i),
                Some(expected),
                "step {step}, entity {i}"
            );
        }
    }
}
