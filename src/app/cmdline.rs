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
//! Since LCV-124 one more thing happens here: [`crate::agent::classify`] says
//! whether the line is CAD at all, and a line that is not becomes one agent
//! turn (ADR 0007 §D9). The classifier is called **after** the raw-input early
//! return — moving it above would post the string an operator is typing into a
//! TEXT entity to a language model — and after the recall-ring push, so a
//! mistyped prompt is still one `ArrowUp` away.
//!
//! MUST NOT import `eframe`, `rfd` or `crate::ui`.

use super::{apply_ortho, handle_zoom_extents, App};
use crate::agent::{classify, Route};
use crate::cmdline::{parse, CommandInput, ToggleKind, ToolInput, ZoomKind};
use crate::geometry::{Vec2, EPSILON};
use crate::render::Camera;
use crate::tools;

/// Feedback shown when `@dx,dy` arrives with no anchor to add it to.
const NO_BASE_POINT: &str = "No base point for relative input.";

/// Feedback shown when a bare distance could not be given a direction.
const NO_DIRECTION: &str = "No direction for distance input — move the cursor or type X,Y.";

/// Feedback for a line addressed to an agent that has no API key (AC 6).
///
/// The leading `! ` is part of the message: it is the one command-line answer
/// that reports a missing *configuration* rather than a rejected input, and it
/// names where to fix it.
const AGENT_UNAVAILABLE: &str = "! Agent unavailable: set the API key in Help > Agent settings";

/// Feedback for a prefix with no prompt behind it (AC 4). Nothing is sent.
const AGENT_EMPTY_PROMPT: &str = "Agent prompt is empty.";

/// Feedback for a second turn while one is in flight (AC 10). One turn at a
/// time (ADR 0007 §Revisit criteria): this refuses, it does not queue.
const AGENT_BUSY: &str = "Agent is busy — wait for the current turn to finish.";

/// How much of the prompt the command line echoes back before cutting it.
const ECHO_CHARS: usize = 60;

/// Execute one line of command-line text (LCV-111 AC 8, AC 9).
///
/// Called from `src/ui/command_line.rs`'s Enter branch, which does nothing
/// else: the parse, the ring push, the dispatch and the feedback all live
/// here. Order of business, fixed by ADR 0003 §B5 and, for raw mode, §D:
///
/// 1. clear the previous result message;
/// 2. if the active tool wants raw input (LCV-112) — forward the text
///    unparsed, poll succession, and return **before** the parser runs;
/// 3. parse;
/// 4. push the line to the recall ring (rejected input included — recall
///    exists so a typo can be fixed);
/// 5. classify (LCV-124, rule 4 flipped by LCV-148): a `:` / `/ai` line
///    becomes one agent turn and returns here; every other line is CAD,
///    whatever the settings say;
/// 6. dispatch per the AC 10 table.
///
/// Never panics and never returns a value: everything it has to say, it says
/// through `app.command_feedback` or through the tool's own prompt.
pub fn submit(app: &mut App, raw: &str) {
    app.command_feedback.clear();

    // Raw means raw (LCV-112 decision 1): while the active tool wants the
    // command line as a free-text field, nothing here parses the string and
    // nothing pushes it to the recall ring — `HELLO` is not a command.
    if app.tool_manager.wants_raw_input() {
        app.tool_manager
            .on_raw_input(raw, &mut app.document, &mut app.history);
        super::viewport::poll_successor(app);
        return;
    }

    let parsed = parse(raw);

    // Every submit is offered to the ring, blank ones included — including
    // `Unknown`, because recall exists so a typo can be fixed (AC 9.4).
    //
    // AC 9.4's "`Empty` is not pushed" is satisfied by `push` itself, which
    // returns before touching `entries` when the entry trims to nothing; the
    // ring's *contents* are identical either way. What the unconditional call
    // preserves is the other half of `push`'s contract (`fix(LCV-110)`,
    // 29ac39a): it resets the recall cursor first, so a blank Enter in the
    // middle of a recall walk drops the operator back at the newest entry.
    //
    // That split is v1's, verbatim: `../LaserCAD-R14/src/ui/command-line.ts`
    // resets `historyIndex = -1` in the keydown handler (line 193, before it
    // calls `execute`), while `execute` guards `if (!raw) return;` (line 84)
    // ahead of its own `pushCommandHistory`. Cursor always resets, blank never
    // appends. Guarding the call here would silently drop the reset.
    app.command_history.push(raw);

    // LCV-124 / LCV-148 — where does this line go? Since the rule-4 flip
    // `classify` decides on the `:` / `/ai` prefix alone and no longer calls
    // `parse` at all (ADR 0003 §A2a, amendment (3)); `parsed` below is this
    // function's own, single call site.
    match classify(raw, agent_available(app)) {
        // The grammar owns the line; fall through to the dispatch below.
        Route::Cad => {}
        Route::Agent(prompt) => return to_agent(app, &prompt),
        Route::Unavailable => {
            app.command_feedback = AGENT_UNAVAILABLE.to_owned();
            return;
        }
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

/// Is an agent reachable at all? **The** definition (AC 5): a configured API
/// key with something other than whitespace in it. Read once, at the call
/// site, and handed to [`classify`] as data so the classifier stays pure.
///
/// `pub(crate)` since LCV-139: `src/ui/command_line.rs` reuses this exact
/// definition to compute the context row's destination label, rather than
/// growing a second, independent availability check (LCV-139 AC 3).
/// Re-exported crate-wide as `crate::app::agent_available`.
pub(crate) fn agent_available(app: &App) -> bool {
    !app.settings.agent_api_key.trim().is_empty()
}

/// Hand one prompt to the agent — loudly (AC 8).
///
/// Only a `:` / `/ai` prefixed line ever reaches here (LCV-148 — bare free
/// text never costs a network call), so a send must still be impossible to
/// miss: the command line echoes what was sent, and the panel opens itself so
/// the prompt, the spinner and the reply are visible where they happen. The
/// operator's `user` row is [`super::start_turn`]'s to write (ADR 0007 §D3);
/// writing it here too would double every prompt in the transcript.
///
/// Two lines never reach a turn: a prefix with no prompt behind it, and a
/// second prompt while one is in flight. The empty check comes first because a
/// bare `:` is malformed whatever the turn state is, and answering "busy" to
/// it would send the operator looking for a turn that has nothing to do with
/// their typo.
fn to_agent(app: &mut App, prompt: &str) {
    if prompt.is_empty() {
        app.command_feedback = AGENT_EMPTY_PROMPT.to_owned();
        return;
    }
    if app.agent.busy {
        app.command_feedback = AGENT_BUSY.to_owned();
        return;
    }
    app.command_feedback = format!("→ agent: \"{}\"", echo(prompt));
    app.agent.panel_open = true;
    super::start_turn(app, prompt);
}

/// `prompt` cut to [`ECHO_CHARS`] characters, with an `…` when it did not fit.
///
/// Counted in `char`s, not bytes: the prompt is whatever the operator typed,
/// and slicing a UTF-8 string at byte 60 panics on the first accented word.
/// Deliberately not shared with `agent_turn::turn_label`, which cuts at a
/// different width for a different reader (the undo stack).
fn echo(prompt: &str) -> String {
    let mut out: String = prompt.chars().take(ECHO_CHARS).collect();
    if prompt.chars().nth(ECHO_CHARS).is_some() {
        out.push('…');
    }
    out
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

    /// The other half of AC 9.4: a blank Enter leaves the ring's *contents*
    /// alone but still resets the recall cursor, so the next Up starts over
    /// at the newest entry.
    ///
    /// v1 parity — `../LaserCAD-R14/src/ui/command-line.ts:193` resets
    /// `historyIndex` on every Enter, `:84` refuses to append a blank one.
    /// Ported to Rust by `fix(LCV-110)` 29ac39a, which moved the reset inside
    /// `CommandHistory::push`; this test pins the call site that reaches it.
    #[test]
    fn a_blank_enter_mid_recall_resets_the_cursor() {
        let mut app = App::default();
        submit(&mut app, "a");
        submit(&mut app, "b");
        submit(&mut app, "c");
        assert_eq!(app.command_history.older(), Some("c".to_owned()));
        assert_eq!(app.command_history.older(), Some("b".to_owned()));

        submit(&mut app, "");

        assert_eq!(
            app.command_history.older(),
            Some("c".to_owned()),
            "a blank Enter must reset the recall cursor to the newest entry"
        );
        assert_eq!(app.command_history.len(), 3, "and must not grow the ring");
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

    // ── LCV-124: routing to the agent ───────────────────────────────────────

    /// The implementation section of this file: everything before the bare
    /// `#[cfg(test)]` at column 0. Scanning the whole file would let this test
    /// module's own needles satisfy the scans below.
    fn implementation() -> &'static str {
        let src = include_str!("cmdline.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("cmdline.rs must have a bare #[cfg(test)] marker to bound the scan");
        &src[..at]
    }

    /// `submit`'s body, from its signature to the `}` at column 0 that closes
    /// it, taken out of the implementation section only.
    fn submit_body() -> &'static str {
        let implementation = implementation();
        let from = implementation
            .find(concat!("pub fn ", "submit(app"))
            .expect("cmdline.rs must declare pub fn submit before its test module");
        let body = &implementation[from..];
        let to = body
            .find("\n}\n")
            .expect("pub fn submit must be closed by a `}` at column 0");
        &body[..to]
    }

    /// How many code lines in `haystack` contain `needle`. Comment lines are
    /// skipped, so a scan is about what the compiler sees and prose stays free
    /// to name the thing it is documenting.
    fn code_hits(haystack: &str, needle: &str) -> usize {
        haystack
            .lines()
            .filter(|l| l.contains(needle) && !l.trim_start().starts_with("//"))
            .count()
    }

    /// AC 11 — the grammar is parsed in one place. Since LCV-148 `classify`
    /// calls `parse` not at all — it decides on the `:` / `/ai` prefix alone
    /// — so `submit`'s own **exactly one** `parse(` call site is now the
    /// grammar's only reader here. Adding `let again = parse(raw);` to
    /// `submit` fails this.
    ///
    /// Shown to discriminate: the two positive controls below run the same
    /// `code_hits` over the same slice, so a scan that were reading the wrong
    /// bytes — or a `submit_body` that silently sliced to nothing — fails here
    /// before the real assertion is reached.
    #[test]
    fn submit_parses_the_line_exactly_once() {
        let body = submit_body();
        assert_eq!(
            code_hits(body, concat!("class", "ify(raw")),
            1,
            "positive control: submit must call the classifier exactly once"
        );
        assert_eq!(
            code_hits(body, concat!("command_history", ".push")),
            1,
            "positive control: the slice must be submit's real body"
        );
        assert_eq!(
            code_hits(body, concat!("pars", "e(")),
            1,
            "AC 11: submit must contain exactly one parse( call site"
        );
    }

    /// AC 12 — this file stays under the 300 **implementation** LOC cap
    /// (ADR 0004): total lines minus the inline test module.
    #[test]
    fn the_implementation_section_is_under_the_loc_cap() {
        let lines = implementation().lines().count();
        assert!(
            lines <= 300,
            "ADR 0004: cmdline.rs has {lines} implementation LOC"
        );
    }

    /// AC 5 — one definition of availability, and whitespace is not a key.
    #[test]
    fn a_whitespace_only_key_is_not_a_key() {
        let mut app = App::default();
        assert!(!agent_available(&app), "a fresh app has no key configured");
        for blank in [" ", "   ", "\t", "\n"] {
            app.settings.agent_api_key = blank.to_owned();
            assert!(!agent_available(&app), "`{blank:?}` is not a key");
        }
        app.settings.agent_api_key = "sk-test".to_owned();
        assert!(agent_available(&app));
    }

    /// AC 8 — the echo is cut at 60 characters and says so with an `…`,
    /// counted in characters so an accented prompt cannot panic the slice.
    #[test]
    fn the_echo_is_cut_at_sixty_characters() {
        assert_eq!(echo("draw a square"), "draw a square");

        let exactly_sixty = "x".repeat(ECHO_CHARS);
        assert_eq!(echo(&exactly_sixty), exactly_sixty, "60 fits, uncut");

        let sixty_one = "x".repeat(ECHO_CHARS + 1);
        let cut = echo(&sixty_one);
        assert_eq!(
            cut.chars().count(),
            ECHO_CHARS + 1,
            "60 characters and the …"
        );
        assert!(cut.ends_with('…'));
        assert_eq!(&cut[..ECHO_CHARS], exactly_sixty);

        let accented = "é".repeat(ECHO_CHARS + 5);
        assert_eq!(echo(&accented).chars().count(), ECHO_CHARS + 1);
    }

    /// AC 4 — a bare `:` is refused before anything is armed, and it does
    /// **not** reach the active tool: `CommandInput::Empty` would finish a
    /// polyline the operator never meant to finish.
    #[test]
    fn an_empty_prompt_refuses_without_arming_a_turn() {
        let mut app = app_with_line_started();
        app.settings.agent_api_key = "sk-test".to_owned();
        for line in [":", ":   ", "/ai", "/ai   "] {
            submit(&mut app, line);
            assert_eq!(app.command_feedback, "Agent prompt is empty.", "{line}");
            assert!(!app.agent.busy, "{line} must arm no turn");
            assert!(app.agent.rx.is_none(), "{line}");
            assert!(app.agent.chat.is_empty(), "{line}");
            assert_eq!(
                app.tool_manager.anchor(),
                Some(Vec2::new(0.0, 0.0)),
                "{line} must not reach the active tool"
            );
        }
    }

    /// AC 10 — a second turn is refused, not queued: nothing is armed, the
    /// in-flight turn's state is untouched, and the ring still got the line.
    #[test]
    fn a_second_turn_is_refused_while_one_is_in_flight() {
        let mut app = App::default();
        app.settings.agent_api_key = "sk-test".to_owned();
        let tx = super::super::arm_turn(&mut app, "the first prompt");
        let chat_before = app.agent.chat.clone();

        submit(&mut app, ":the second prompt");

        assert_eq!(
            app.command_feedback,
            "Agent is busy — wait for the current turn to finish."
        );
        assert!(app.agent.busy, "the in-flight turn is left alone");
        assert!(app.agent.rx.is_some(), "and keeps its receiver");
        assert_eq!(app.agent.chat, chat_before, "and its transcript");
        assert_eq!(
            app.command_history.older(),
            Some(":the second prompt".to_owned()),
            "AC 9: the refused line is still recallable"
        );
        drop(tx);
    }

    /// AC 6 — without a key a prefixed line says so, verbatim, and does
    /// nothing else. The needle is built with `concat!` so this assertion
    /// cannot be satisfied by the constant it is checking.
    #[test]
    fn without_a_key_a_prefixed_line_reports_the_missing_key() {
        let mut app = app_with_line_started();
        assert!(!agent_available(&app));
        submit(&mut app, ":draw a square");
        assert_eq!(
            app.command_feedback,
            concat!(
                "! Agent unavailable: set the API key in ",
                "Help > Agent settings"
            )
        );
        assert!(!app.agent.busy);
        assert!(app.agent.rx.is_none());
        assert!(app.agent.chat.is_empty());
        assert!(!app.agent.panel_open, "and opens no panel");
        assert_eq!(app.document.entity_count(), 0);
        assert_eq!(app.tool_manager.anchor(), Some(Vec2::new(0.0, 0.0)));
    }

    /// AC 7 — without a key, unprefixed nonsense answers exactly as it did
    /// before the agent existed. The literal is not copied: it is taken from
    /// the word `tests/lcv111.rs` already pins and re-spelt for `lien`.
    #[test]
    fn without_a_key_unknown_text_is_unchanged() {
        let mut app = App::default();
        submit(&mut app, "bogus");
        let pinned = app.command_feedback.clone();
        assert!(
            pinned.contains("bogus"),
            "positive control: LCV-111's message names the word, got {pinned:?}"
        );

        submit(&mut app, "lien");
        assert_eq!(app.command_feedback, pinned.replace("bogus", "lien"));
    }

    /// AC 2 rule 2 — the prefix is absolute at the call site too: `:l` does
    /// not start LINE, it refuses for want of a key. The control is `l`.
    #[test]
    fn a_prefixed_tool_alias_does_not_reach_the_tool() {
        let mut app = App::default();
        submit(&mut app, "l");
        assert_eq!(
            app.tool_manager.active_tool_name(),
            "LINE",
            "control: the bare alias still switches tools"
        );

        let mut app = App::default();
        submit(&mut app, ":l");
        assert_eq!(app.tool_manager.active_tool_name(), "Select");
        assert!(app.command_feedback.starts_with("! Agent unavailable"));
    }

    /// AC 2 rule 3 — with a key configured, every line the grammar recognises
    /// still behaves exactly as it always has. This is the regression guard on
    /// the whole table: the typo hazard is bounded by the fact that only
    /// genuinely unrecognised text can leak.
    #[test]
    fn a_key_changes_nothing_about_a_line_the_grammar_recognises() {
        let mut app = app_with_line_started();
        app.settings.agent_api_key = "sk-test".to_owned();

        submit(&mut app, "snap");
        assert_eq!(app.command_feedback, "SNAP off");

        submit(&mut app, "@10,0");
        assert_eq!(committed_line(&app).p2, Vec2::new(10.0, 0.0));

        submit(&mut app, "c");
        assert_eq!(app.tool_manager.active_tool_name(), "CIRCLE");

        assert!(!app.agent.busy, "none of that reached the agent");
        assert!(app.agent.chat.is_empty());
        assert!(!app.agent.panel_open);
    }
}
