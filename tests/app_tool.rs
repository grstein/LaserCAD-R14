//! Integration tests for `App` tool and command integration (LCV-040).

use lasercad::app::App;
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};

/// LCV-040 AC#6 — `App::default().tool_manager` has `SelectTool` active.
#[test]
fn app_default_tool_manager_has_select() {
    let app = App::default();
    assert_eq!(app.tool_manager.active_tool_name(), "Select");
}

/// LCV-040 AC#7, AC#8 — `App::commit` adds entity and pushes onto history.
#[test]
fn app_commit_adds_entity_and_pushes_history() {
    let mut app = App::default();
    assert_eq!(app.document.entity_count(), 0);
    assert!(!app.history.can_undo());

    let line = Line::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0));
    app.commit(Box::new(CreateLine::new(line)));

    assert_eq!(app.document.entity_count(), 1);
    assert!(app.history.can_undo());
}
