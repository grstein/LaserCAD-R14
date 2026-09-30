//! Handing one resolved input to the active tool (LCV-111 AC 10, AC 11).
//!
//! Split out of `app/cmdline.rs` by LCV-165 so that file stays under the
//! 300-LOC cap (ADR 0004): [`send`] delivers a [`ToolInput`] and reports a
//! refusal, [`direct_distance`] resolves a bare number into a point.
//!
//! MUST NOT import `eframe`, `rfd` or `crate::ui`.

use crate::app::{App, Severity, apply_ortho};
use crate::cmdline::ToolInput;
use crate::geometry::{EPSILON, Vec2};

/// Feedback shown when `@dx,dy` arrives with no anchor to add it to.
pub(super) const NO_BASE_POINT: &str = "No base point for relative input.";

/// Feedback shown when a bare distance could not be given a direction.
const NO_DIRECTION: &str = "No direction for distance input — move the cursor or type X,Y.";

/// Hand one resolved input to the active tool, then poll succession exactly
/// as the pointer path does (AC 6), so `m` ⏎ `0,0` ⏎ `@10,0` ⏎ hands back to
/// `SelectTool`. On refusal nothing is mutated and AC 15's message is set.
pub(super) fn send(app: &mut App, input: ToolInput) {
    let consumed = app
        .tool_manager
        .on_command_input(input, &mut app.document, &mut app.history);
    if consumed {
        crate::app::viewport::poll_successor(app);
        return;
    }
    let refusal = match input {
        ToolInput::Distance { along: None, .. } => NO_DIRECTION.to_owned(),
        _ => format!(
            "{} does not accept that input.",
            app.tool_manager.active_tool_name()
        ),
    };
    app.say(Severity::Warning, refusal);
}

/// Resolve direct-distance entry (AC 11): `value_mm` along the anchor→cursor
/// direction, ortho applied to that direction when F8 is on.
///
/// `None` — refuse — unless *all* of: the tool has an anchor, the pointer has
/// entered the viewport at least once, and the (post-ortho) cursor is farther
/// than `EPSILON` from the anchor. A negative `value_mm` is legal and places
/// the point on the opposite ray, which is R14 behaviour.
pub(super) fn direct_distance(app: &App, value_mm: f64) -> Option<Vec2> {
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
