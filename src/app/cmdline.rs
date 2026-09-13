//! One submitted command line, resolved (LCV-111, ADR 0003 §B5).
//!
//! [`submit`] is the single place that turns the operator's text into an
//! effect. It parses with [`crate::cmdline::parse`], records the line in the
//! recall ring, and dispatches: tool aliases, toggles and zoom act on `App`
//! directly; coordinates are resolved into a
//! [`ToolInput`](crate::cmdline::ToolInput) and handed to the active tool.
//!
//! **Resolution happens here and nowhere else.** `@dx,dy` is added to the
//! active tool's [`anchor`](crate::tools::Tool::anchor) and a bare distance is
//! projected along the anchor→cursor direction *before* the tool is called, so
//! a tool never sees a string and never reads `App` (ADR 0003 §B1).
//!
//! A typed number is exact: it **bypasses snap entirely** and bypasses ortho
//! except as the direction source for direct-distance entry (ADR 0003 §B5).
//! `submit` never writes `last_cursor_world` — the cursor belongs to the
//! mouse, and faking it from a typed point would corrupt the next direction.
//!
//! MUST NOT import `eframe`, `rfd` or `crate::ui`.

use super::{apply_ortho, handle_zoom_extents, App};
use crate::cmdline::{parse, CommandInput, ToggleKind, ToolInput, ZoomKind};
use crate::geometry::{Vec2, EPSILON};
use crate::render::Camera;
use crate::tools;

/// Feedback shown when `@dx,dy` arrives with no anchor to add it to.
const NO_BASE_POINT: &str = "No base point for relative input.";

/// Feedback shown when a bare distance could not be given a direction.
const NO_DIRECTION: &str = "No direction for distance input — move the cursor or type X,Y.";

/// Execute one line of command-line text (LCV-111 AC 8, AC 9).
///
/// Called from `src/ui/command_line.rs`'s Enter branch, which does nothing
/// else: the parse, the ring push, the dispatch and the feedback all live
/// here. Order of business, fixed by ADR 0003 §B5:
///
/// 1. clear the previous result message;
/// 2. parse;
/// 3. push the line to the recall ring (rejected input included — recall
///    exists so a typo can be fixed);
/// 4. dispatch per the AC 10 table.
///
/// Never panics and never returns a value: everything it has to say, it says
/// through `app.command_feedback` or through the tool's own prompt.
pub fn submit(app: &mut App, raw: &str) {
    app.command_feedback.clear();
    let parsed = parse(raw);

    // AC 9.4 — `Empty` is not pushed: a blank Enter is R14's "accept"
    // keystroke, not a command. (`CommandHistory::push` would ignore a blank
    // entry for the ring's *contents* anyway, but it also resets the recall
    // cursor; keeping the push behind this check is the demand's order of
    // business, so a blank Enter leaves an in-progress recall walk alone.)
    if !matches!(parsed, CommandInput::Empty) {
        app.command_history.push(raw);
    }

    match parsed {
        // The new tool's prompt is the feedback; no message is set.
        CommandInput::Tool(kind) => app.tool_manager.set_tool(tools::make(kind)),
        CommandInput::Toggle(kind) => toggle(app, kind),
        CommandInput::Zoom(kind) => zoom(app, kind),
        CommandInput::Point(p) => send(app, ToolInput::Point(p)),
        CommandInput::Relative(delta) => match app.tool_manager.anchor() {
            Some(anchor) => send(app, ToolInput::Point(anchor + delta)),
            // Deterministic refusal (product decision 2): no anchor means the
            // offset designates nothing. Invent no origin.
            None => app.command_feedback = NO_BASE_POINT.to_owned(),
        },
        CommandInput::Distance(value_mm) => {
            let along = direct_distance(app, value_mm);
            send(app, ToolInput::Distance { value_mm, along });
        }
        // AC 14 — "Enter finishes the polyline" stays alive now that the
        // command line usually holds keyboard focus.
        CommandInput::Empty => super::input::route_to_tool(app, egui::Key::Enter),
        CommandInput::Unknown(text) => {
            app.command_feedback = format!("Unknown command: \"{text}\"")
        }
    }
}

/// Hand one resolved input to the active tool, then poll succession exactly
/// as the pointer path does (AC 6), so `m` ⏎ `0,0` ⏎ `@10,0` ⏎ hands back to
/// `SelectTool`. On refusal nothing is mutated and AC 15's message is set.
fn send(app: &mut App, input: ToolInput) {
    let consumed = app
        .tool_manager
        .on_command_input(input, &mut app.document, &mut app.history);
    if consumed {
        super::viewport::poll_successor(app);
        return;
    }
    app.command_feedback = match input {
        ToolInput::Distance { along: None, .. } => NO_DIRECTION.to_owned(),
        _ => format!(
            "{} does not accept that input.",
            app.tool_manager.active_tool_name()
        ),
    };
}

/// Resolve direct-distance entry (AC 11): `value_mm` along the anchor→cursor
/// direction, ortho applied to that direction when F8 is on.
///
/// `None` — refuse — unless *all* of: the tool has an anchor, the pointer has
/// entered the viewport at least once, and the (post-ortho) cursor is farther
/// than `EPSILON` from the anchor. A negative `value_mm` is legal and places
/// the point on the opposite ray, which is R14 behaviour.
fn direct_distance(app: &App, value_mm: f64) -> Option<Vec2> {
    let anchor = app.tool_manager.anchor()?;
    let cursor = app.last_cursor_world?;
    let cursor = if app.ortho_enabled {
        apply_ortho(anchor, cursor)
    } else {
        cursor
    };
    let delta = cursor - anchor;
    let length = delta.length();
    if length <= EPSILON {
        return None;
    }
    Some(anchor + delta * (value_mm / length))
}

/// Flip one drawing aid — identical semantics to F3 / F7 / F8 — and report
/// the new state as `SNAP on` / `GRID off` / … (AC 10).
fn toggle(app: &mut App, kind: ToggleKind) {
    let (flag, name) = match kind {
        ToggleKind::Snap => (&mut app.snap_enabled, "SNAP"),
        ToggleKind::Grid => (&mut app.grid_enabled, "GRID"),
        ToggleKind::Ortho => (&mut app.ortho_enabled, "ORTHO"),
    };
    *flag = !*flag;
    let state = if *flag { "on" } else { "off" };
    app.command_feedback = format!("{name} {state}");
}

/// Typed zoom. Shares [`Camera::ZOOM_STEP`] with the View menu (AC 16) so the
/// two can never drift apart.
fn zoom(app: &mut App, kind: ZoomKind) {
    match kind {
        ZoomKind::In => app.camera.zoom_in(Camera::ZOOM_STEP),
        ZoomKind::Out => app.camera.zoom_out(Camera::ZOOM_STEP),
        ZoomKind::Extents => {
            let viewport_size = app.camera.viewport_size_px;
            handle_zoom_extents(&mut app.camera, &app.document, viewport_size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmdline::CommandHistory;
    use crate::document::{commands::CreateLine, Entity};
    use crate::geometry::Line;
    use crate::tools::{CircleTool, LineTool, SelectTool};

    /// An app with `LineTool` active and its first point already fixed at the
    /// origin, i.e. `anchor() == Some((0, 0))`.
    fn app_with_line_started() -> App {
        let mut app = App::default();
        app.tool_manager.set_tool(Box::new(LineTool::default()));
        submit(&mut app, "0,0");
        assert_eq!(app.tool_manager.anchor(), Some(Vec2::new(0.0, 0.0)));
        app
    }

    fn committed_line(app: &App) -> Line {
        match app.document.entities.last() {
            Some(Entity::Line(l)) => *l,
            other => panic!("expected a committed line, got {other:?}"),
        }
    }

    /// AC 10 — an absolute point is committed verbatim, to the last bit.
    #[test]
    fn absolute_point_commits_at_exact_coordinates() {
        let mut app = app_with_line_started();
        submit(&mut app, "12.2,-3.75");
        assert_eq!(app.document.entity_count(), 1);
        let line = committed_line(&app);
        assert_eq!(line.p1, Vec2::new(0.0, 0.0));
        assert_eq!(line.p2, Vec2::new(12.2, -3.75));
        assert!(app.command_feedback.is_empty());
    }

    /// AC 10 — `@dx,dy` is added to the active tool's anchor.
    #[test]
    fn relative_adds_to_the_anchor() {
        let mut app = app_with_line_started();
        submit(&mut app, "@100,0");
        assert_eq!(committed_line(&app).p2, Vec2::new(100.0, 0.0));
        // The anchor advanced with the chain, so the next `@` is relative to
        // the new endpoint.
        submit(&mut app, "@0,50");
        assert_eq!(committed_line(&app).p2, Vec2::new(100.0, 50.0));
        assert_eq!(app.history.len(), 2);
    }

    /// AC 10 — with no anchor there is no base point: refuse, say why, and
    /// mutate nothing (product decision 2).
    #[test]
    fn relative_without_an_anchor_is_refused() {
        let mut app = App::default();
        app.tool_manager.set_tool(Box::new(LineTool::default()));
        submit(&mut app, "@100,0");
        assert_eq!(app.document.entity_count(), 0);
        assert_eq!(app.history.len(), 0);
        assert_eq!(app.command_feedback, "No base point for relative input.");
        assert_eq!(app.tool_manager.anchor(), None, "phase untouched");
    }

    /// AC 11 — a bare distance travels along the anchor→cursor direction.
    #[test]
    fn distance_projects_along_the_cursor_direction() {
        let mut app = app_with_line_started();
        app.last_cursor_world = Some(Vec2::new(200.0, 0.0));
        submit(&mut app, "50");
        assert_eq!(committed_line(&app).p2, Vec2::new(50.0, 0.0));

        // A negative magnitude places the point on the opposite ray (R14).
        let mut app = app_with_line_started();
        app.last_cursor_world = Some(Vec2::new(0.0, 10.0));
        submit(&mut app, "-25");
        assert_eq!(committed_line(&app).p2, Vec2::new(0.0, -25.0));
    }

    /// AC 11 — with F8 on, the direction is the ortho-clamped one, so a
    /// slightly-off cursor still yields an exactly axis-aligned point.
    #[test]
    fn distance_uses_the_ortho_direction_when_f8_is_on() {
        let mut app = app_with_line_started();
        app.last_cursor_world = Some(Vec2::new(200.0, 30.0));
        app.ortho_enabled = true;
        submit(&mut app, "50");
        assert_eq!(committed_line(&app).p2, Vec2::new(50.0, 0.0));

        // The same cursor without ortho follows the raw direction instead.
        let mut app = app_with_line_started();
        app.last_cursor_world = Some(Vec2::new(200.0, 30.0));
        submit(&mut app, "50");
        assert_ne!(committed_line(&app).p2, Vec2::new(50.0, 0.0));
        assert!((committed_line(&app).p2.length() - 50.0).abs() <= 1e-9);
    }

    /// AC 11, AC 15 — no cursor means no direction: refuse deterministically.
    #[test]
    fn distance_without_a_cursor_is_refused() {
        let mut app = app_with_line_started();
        assert_eq!(app.last_cursor_world, None);
        submit(&mut app, "50");
        assert_eq!(app.document.entity_count(), 0);
        assert_eq!(app.history.len(), 0);
        assert_eq!(
            app.command_feedback,
            "No direction for distance input — move the cursor or type X,Y."
        );
        assert_eq!(app.tool_manager.anchor(), Some(Vec2::new(0.0, 0.0)));
    }

    /// AC 11 — a cursor sitting on the anchor is a zero-length direction, not
    /// a direction of +X.
    #[test]
    fn distance_with_cursor_equal_to_anchor_is_refused() {
        let mut app = app_with_line_started();
        app.last_cursor_world = Some(Vec2::new(0.0, 0.0));
        submit(&mut app, "50");
        assert_eq!(app.document.entity_count(), 0);
        assert_eq!(
            app.command_feedback,
            "No direction for distance input — move the cursor or type X,Y."
        );
    }

    /// AC 13 — the cursor belongs to the mouse. A typed point must not move
    /// it, or the next direct-distance direction would be wrong.
    #[test]
    fn typed_point_does_not_move_last_cursor_world() {
        let mut app = app_with_line_started();
        app.last_cursor_world = Some(Vec2::new(5.0, 5.0));
        submit(&mut app, "100,0");
        assert_eq!(app.document.entity_count(), 1);
        assert_eq!(app.last_cursor_world, Some(Vec2::new(5.0, 5.0)));
    }

    /// AC 12 — a typed number is exact: snap never rounds it to a nearby
    /// endpoint, even with an entity sitting right next to it.
    #[test]
    fn typed_point_ignores_snap() {
        let mut app = App::default();
        assert!(app.snap_enabled, "snap is on by default");
        app.commit(Box::new(CreateLine::new(Line::new(
            Vec2::new(100.0, 0.0),
            Vec2::new(120.0, 0.0),
        ))));
        app.tool_manager.set_tool(Box::new(LineTool::default()));
        submit(&mut app, "0,0");
        submit(&mut app, "99.9,0.05");
        assert_eq!(committed_line(&app).p2, Vec2::new(99.9, 0.05));
        assert_eq!(app.active_snap, None, "submit must not run the snap engine");
    }

    /// AC 10 — typed toggles are F3 / F7 / F8 by another name, and each
    /// reports its new state.
    #[test]
    fn toggles_match_f3_f7_f8() {
        let mut app = App::default();
        assert!(app.snap_enabled && app.grid_enabled && !app.ortho_enabled);

        submit(&mut app, "snap");
        assert!(!app.snap_enabled);
        assert_eq!(app.command_feedback, "SNAP off");
        submit(&mut app, "SNAP");
        assert!(app.snap_enabled);
        assert_eq!(app.command_feedback, "SNAP on");

        submit(&mut app, "grid");
        assert!(!app.grid_enabled);
        assert_eq!(app.command_feedback, "GRID off");

        submit(&mut app, "ortho");
        assert!(app.ortho_enabled);
        assert_eq!(app.command_feedback, "ORTHO on");
    }

    /// AC 16 — typed zoom and the View menu share `Camera::ZOOM_STEP`, so
    /// they can never drift apart. (`src/ui/menubar.rs`'s own test asserts the
    /// menu side of the same constant; `src/app/` must not import `crate::ui`.)
    #[test]
    fn zoom_in_uses_the_same_step_as_the_view_menu() {
        assert_eq!(Camera::ZOOM_STEP, 1.25);

        let mut reference = Camera::default();
        reference.zoom_in(Camera::ZOOM_STEP);

        let mut app = App::default();
        submit(&mut app, "zoom in");
        assert_eq!(app.camera.mm_per_px, reference.mm_per_px);
        assert!(app.command_feedback.is_empty());

        reference.zoom_out(Camera::ZOOM_STEP);
        submit(&mut app, "zoom out");
        assert_eq!(app.camera.mm_per_px, reference.mm_per_px);
    }

    /// AC 10 — unrecognised text is echoed back verbatim, in its original
    /// case, and changes nothing else.
    #[test]
    fn unknown_sets_feedback_and_mutates_nothing() {
        let mut app = app_with_line_started();
        submit(&mut app, "  Bogus  ");
        assert_eq!(app.command_feedback, "Unknown command: \"Bogus\"");
        assert_eq!(app.document.entity_count(), 0);
        assert_eq!(app.history.len(), 0);
        assert_eq!(app.tool_manager.active_tool_name(), "LINE");
        assert_eq!(app.tool_manager.anchor(), Some(Vec2::new(0.0, 0.0)));
    }

    /// AC 15 — a tool that refuses the input says so by name and leaves
    /// everything alone.
    #[test]
    fn a_refusing_tool_is_named_in_the_feedback() {
        let mut app = App::default();
        app.tool_manager.set_tool(Box::new(SelectTool::default()));
        submit(&mut app, "50,25");
        assert_eq!(app.command_feedback, "Select does not accept that input.");
        assert_eq!(app.document.entity_count(), 0);
        assert_eq!(app.history.len(), 0);
    }

    /// AC 9.4 — rejected input is still recallable: recall exists so a typo
    /// can be fixed.
    #[test]
    fn rejected_input_is_still_pushed_to_the_ring() {
        let mut app = App::default();
        submit(&mut app, "  bogus  ");
        submit(&mut app, "@1,1");
        assert_eq!(app.command_history.len(), 2);
        assert_eq!(app.command_history.older(), Some("@1,1".to_owned()));
        assert_eq!(app.command_history.older(), Some("bogus".to_owned()));
    }

    /// AC 9.4 — a blank Enter is an "accept" keystroke, not a command, and
    /// never enters the ring.
    #[test]
    fn empty_is_not_pushed_to_the_ring() {
        let mut app = App::default();
        submit(&mut app, "l");
        submit(&mut app, "");
        submit(&mut app, "   ");
        assert_eq!(app.command_history.len(), 1);
        assert_eq!(app.command_history.older(), Some("l".to_owned()));
    }

    /// AC 14 — a blank line routes Enter to the active tool, which is what
    /// keeps "Enter finishes the polyline" alive.
    #[test]
    fn empty_routes_enter_to_the_active_tool() {
        let mut app = app_with_line_started();
        submit(&mut app, "");
        assert_eq!(
            app.tool_manager.anchor(),
            None,
            "Enter must reach LineTool::on_key and reset it to idle"
        );
    }

    /// AC 10 — a tool alias swaps the active tool, and the new prompt is the
    /// only feedback needed.
    #[test]
    fn tool_alias_activates_the_tool_without_feedback() {
        let mut app = App::default();
        submit(&mut app, "c");
        assert_eq!(app.tool_manager.active_tool_name(), "CIRCLE");
        assert_eq!(
            app.tool_manager.active_status_text(),
            "CIRCLE Specify center point:"
        );
        assert!(app.command_feedback.is_empty());
    }

    /// AC 4, AC 34 — the mouse-free circle: no cursor anywhere, and the bare
    /// number is the radius.
    #[test]
    fn circle_radius_needs_no_cursor() {
        let mut app = App::default();
        app.tool_manager.set_tool(Box::new(CircleTool::default()));
        submit(&mut app, "50,50");
        submit(&mut app, "25");
        assert_eq!(app.last_cursor_world, None);
        match app.document.entities.first() {
            Some(Entity::Circle(c)) => {
                assert_eq!(c.center, Vec2::new(50.0, 50.0));
                assert_eq!(c.r, 25.0);
            }
            other => panic!("expected a circle, got {other:?}"),
        }
    }

    /// AC 9.1 — every submit starts by clearing the previous message, so a
    /// successful command never leaves a stale error on screen.
    #[test]
    fn every_submit_clears_the_previous_feedback() {
        let mut app = app_with_line_started();
        submit(&mut app, "nope");
        assert!(!app.command_feedback.is_empty());
        submit(&mut app, "100,0");
        assert!(app.command_feedback.is_empty());
    }

    /// AC 20 — the ring starts empty on a fresh app.
    #[test]
    fn a_fresh_app_has_an_empty_ring() {
        let app = App::default();
        assert_eq!(app.command_history.len(), CommandHistory::default().len());
        assert_eq!(app.command_history.len(), 0);
        assert!(app.command_feedback.is_empty());
        assert!(!app.focus_command_line);
        assert!(!app.command_line_focused);
    }
}
