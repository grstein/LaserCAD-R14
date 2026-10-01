//! LCV-188 — the agent addresses entities by stable ids (ADR 0014): listed
//! by `query_entities` (AC 1), accepted as `id`/`ids` by every tool that
//! takes `index`/`indices` (AC 4, AC 5), returned by creations (AC 6).

use lasercad::agent::{AgentAction, AgentOutcome};
use lasercad::app::{App, apply};
use lasercad::document::Entity;
use lasercad::geometry::{Circle, Line, Vec2};

fn line(x: f64) -> Entity {
    Entity::Line(Line::new(Vec2::new(x, 0.0), Vec2::new(x, 10.0)))
}

fn ok(outcome: AgentOutcome) -> String {
    match outcome {
        AgentOutcome::Ok(text) => text,
        other => panic!("not ok: {other:?}"),
    }
}

/// AC 1 — every listed entity shows `e<N>` beside its index, and the ids
/// survive a delete while the indices shift.
#[test]
fn query_entities_lists_the_id_beside_the_index() {
    let mut app = App::default();
    app.document.push_current(line(0.0));
    app.document
        .push_current(Entity::Circle(Circle::new(Vec2::new(5.0, 5.0), 2.0)));
    app.document.push_current(line(3.0));
    app.document.remove_entity(0);
    let listing = ok(apply(&mut app, &AgentAction::QueryEntities));
    let rows: Vec<&str> = listing.lines().skip(2).collect();
    assert_eq!(
        rows,
        [
            "0 e2: circle center (5.000, 5.000) mm, r = 2.000 mm layer Cut",
            "1 e3: line (3.000, 0.000) → (3.000, 10.000) mm layer Cut",
        ]
    );
}
