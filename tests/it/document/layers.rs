//! LCV-156 — layer membership through the document commands (AC 2, AC 8).

use lasercad::document::commands::CreateEntities;
use lasercad::document::{AddLayer, DeleteLayer, EditLayer, SetCurrentLayer, SetEntityLayers};
use lasercad::document::{
    CreateArc, CreateCircle, CreateLine, DeleteEntities, Document, Entity, History, Layer,
    LayerError, LayerId,
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

// ── T7: layer commands, one undo step each (AC 4, AC 6, AC 7, AC 8) ──────

fn names(doc: &Document) -> Vec<&str> {
    doc.layers().iter().map(|l| l.name.as_str()).collect()
}

/// AC 4 / AC 8 — adding a layer appends it; undo removes it; redo brings it
/// back with the same id.
#[test]
fn add_layer_is_one_undo_step_and_redo_keeps_the_id() {
    let mut doc = Document::default();
    let mut history = History::new();
    history.commit(
        Box::new(AddLayer::new("Engrave", [0, 170, 0], false)),
        &mut doc,
    );
    assert_eq!(names(&doc), ["Cut", "Engrave"]);
    let added = doc.layers()[1].clone();
    assert_eq!((added.color, added.output), ([0, 170, 0], false));
    assert_ne!(added.id, doc.layers()[0].id);
    assert!(history.undo(&mut doc));
    assert_eq!(names(&doc), ["Cut"]);
    assert!(history.redo(&mut doc));
    assert_eq!(doc.layers()[1], added);
}

/// AC 4 / AC 8 — rename, recolor and Output toggle are one step.
#[test]
fn edit_layer_is_one_undo_step() {
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    let before = doc.layers()[1].clone();
    let edited = Layer {
        name: "Fine mark".into(),
        color: [1, 2, 3],
        output: false,
        ..before.clone()
    };
    history.commit(Box::new(EditLayer::new(edited.clone())), &mut doc);
    assert_eq!(doc.layers()[1], edited);
    assert!(history.undo(&mut doc));
    assert_eq!(doc.layers()[1], before);
}

/// AC 4 / AC 8 — deleting a layer, current or not, is undone in place; a
/// deleted current layer hands "current" to the first remaining layer.
#[test]
fn delete_layer_restores_position_and_current() {
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    history.commit(
        Box::new(AddLayer::new("Engrave", [0, 170, 0], true)),
        &mut doc,
    );
    history.commit(Box::new(DeleteLayer::new(MARK)), &mut doc);
    assert_eq!(names(&doc), ["Cut", "Engrave"]);
    assert!(history.undo(&mut doc));
    assert_eq!(names(&doc), ["Cut", "Mark", "Engrave"]);
    history.commit(Box::new(DeleteLayer::new(CUT)), &mut doc);
    assert_eq!(names(&doc), ["Mark", "Engrave"]);
    assert_eq!(doc.current_layer(), MARK);
    assert!(history.undo(&mut doc));
    assert_eq!(
        (names(&doc), doc.current_layer()),
        (vec!["Cut", "Mark", "Engrave"], CUT)
    );
}

/// AC 7 — the checks the app runs before committing refuse a non-empty
/// layer and the last layer.
#[test]
fn delete_checks_refuse_non_empty_and_last_layer() {
    let mut doc = two_layer_doc(CUT);
    doc.push_entity(Entity::Line(line(0.0)), MARK);
    assert!(
        doc.check_delete_layer(MARK)
            .unwrap_err()
            .to_string()
            .contains("Mark")
    );
    let lone = Document::default();
    let only = lone.layers()[0].id;
    assert!(lone.check_delete_layer(only).is_err());
}

/// AC 6 — the checks refuse a duplicate name (sanitised, any case) or color.
#[test]
fn add_and_rename_checks_refuse_duplicates() {
    let doc = two_layer_doc(CUT);
    assert!(
        doc.check_new_layer("MARK!", [9, 9, 9])
            .unwrap_err()
            .to_string()
            .contains("Mark")
    );
    assert!(
        doc.check_new_layer("Engrave", [0, 0, 255])
            .unwrap_err()
            .to_string()
            .contains("#0000ff")
    );
    assert!(doc.check_edit_layer(MARK, "cut", [0, 0, 255]).is_err());
    assert!(doc.check_edit_layer(MARK, "Mark", [255, 0, 0]).is_err());
    assert!(doc.check_edit_layer(MARK, "mark", [0, 0, 255]).is_ok());
}

/// AC 4 / AC 8 — changing the current layer is one step.
#[test]
fn set_current_layer_is_one_undo_step() {
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    history.commit(Box::new(SetCurrentLayer::new(MARK)), &mut doc);
    assert_eq!(doc.current_layer(), MARK);
    let before = history.revision();
    assert!(history.undo(&mut doc));
    assert_ne!(history.revision(), before);
    assert_eq!(doc.current_layer(), CUT);
}

/// AC 4 / AC 8 — moving the selection to a layer is one step.
#[test]
fn set_entity_layers_is_one_undo_step() {
    let mut doc = two_layer_doc(CUT);
    let mut history = History::new();
    for (i, id) in [CUT, MARK, CUT].into_iter().enumerate() {
        doc.push_entity(Entity::Line(line(i as f64)), id);
    }
    history.commit(
        Box::new(SetEntityLayers::new(vec![0, 1, 2], MARK)),
        &mut doc,
    );
    assert_eq!(memberships(&doc), vec![MARK; 3]);
    assert!(history.undo(&mut doc));
    assert_eq!(memberships(&doc), vec![CUT, MARK, CUT]);
    assert!(history.redo(&mut doc));
    assert_eq!(memberships(&doc), vec![MARK; 3]);
}

/// AC 8 — each layer change is a named undo step.
#[test]
fn layer_commands_carry_their_undo_labels() {
    use lasercad::document::Command;
    let labels = [
        AddLayer::new("X", [1, 2, 3], true).label().to_owned(),
        EditLayer::new(layer(MARK, "M", [0, 0, 1]))
            .label()
            .to_owned(),
        DeleteLayer::new(MARK).label().to_owned(),
        SetCurrentLayer::new(MARK).label().to_owned(),
        SetEntityLayers::new(vec![0], MARK).label().to_owned(),
    ];
    assert_eq!(
        labels,
        [
            "Add Layer",
            "Edit Layer",
            "Delete Layer",
            "Set Current Layer",
            "Move to Layer"
        ]
    );
}

/// LCV-170 AC 9 — a name with a control character is refused by the check
/// that gates `AddLayer` and `EditLayer`, and by `Document::from_parts`.
#[test]
fn control_character_names_are_refused() {
    let doc = two_layer_doc(CUT);
    for name in ["A\tB", "Cut\u{7}"] {
        let want = Err(LayerError::ControlChar(name.to_owned()));
        assert_eq!(doc.check_new_layer(name, [9, 9, 9]), want);
        assert_eq!(doc.check_edit_layer(MARK, name, [0, 0, 255]), want);
        let layers = vec![layer(CUT, name, [255, 0, 0])];
        let opened = Document::from_parts([400.0; 2], layers, CUT, vec![], vec![]);
        assert_eq!(opened.err(), want.err());
    }
}
