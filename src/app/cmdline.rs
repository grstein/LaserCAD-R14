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

use super::{App, apply_ortho, handle_zoom_extents};
use crate::agent::{Route, classify};
use crate::cmdline::{CommandInput, ToggleKind, ToolInput, ZoomKind, parse};
use crate::geometry::{EPSILON, Vec2};
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
const AGENT_UNAVAILABLE: &str = "! AI unavailable: set the API key in Help > AI Settings…";

/// Feedback for a prefix with no prompt behind it (AC 4). Nothing is sent.
const AGENT_EMPTY_PROMPT: &str = "AI prompt is empty.";

/// Feedback for a second turn while one is in flight (AC 10). One turn at a
/// time (ADR 0007 §Revisit criteria): this refuses, it does not queue.
const AGENT_BUSY: &str = "AI is busy — wait for the current turn to finish.";

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
        CommandInput::Layers => app.open_layers_dialog(),
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
    app.command_feedback = format!("→ AI: \"{}\"", echo(prompt));
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
mod tests;
