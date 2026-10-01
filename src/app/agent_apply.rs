//! LCV-122 / LCV-123 — apply one [`AgentAction`] to the live document, and
//! transcribe its outcome (ADR 0007 §D2a/§D5).
//!
//! This is the UI-thread half of the bridge. The worker thread names what it
//! wants; this file decides what really happens, against the drawing the
//! operator is really looking at, and answers with a sentence the model reads
//! back as the `tool` result.
//!
//! Three rules shape everything below.
//!
//! - **Never touch `document.entities` directly.** Every mutation is a
//!   `Box<dyn Command>` committed through `History::commit_grouped` into the
//!   turn's group (AGENTS.md §"State and mutation", ADR 0007 §D12), so agent
//!   edits get undo, redo, dirty tracking and autosave for free.
//! - **The range check lives here, and it refuses rather than fails.** The
//!   worker cannot know `entities.len()` (ADR 0007 §D1), so `index` arrives
//!   shape-checked but unbounded. An out-of-range index is information the
//!   model can act on — `AgentOutcome::Refused`, not an error that ends a turn
//!   (ADR 0007 §D2a).
//! - **Say what shifted.** Positional indices renumber on every delete, so each
//!   outcome reports the resulting entity count, and a delete names the entity
//!   it removed and the indices that moved down (ADR 0007 §D5).
//! - **Every action leaves a row.** LCV-123 AC 23: the sentence handed back to
//!   the model is appended verbatim to `agent.chat` by [`transcribe`], here,
//!   the one place that already holds both the action and its narration. The
//!   operator and the model therefore read the same words, and a transcript
//!   that disagrees with what the model was told is impossible by construction.
//!   No variant is special-cased — a query gets a row like anything else.
//!
//! Imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere, and spawns no
//! thread.

use crate::agent::tools::refusal;
use crate::agent::{AgentAction, AgentOutcome, DrawingItem};
use crate::app::agent_narrate::{batch_created, list_entities, list_selection, new_ids, pt, sweep};
use crate::app::{App, agent_capture};
use crate::document::commands::CreateEntities;
use crate::document::{
    Command, CreateArc, CreateCircle, CreateLine, Document, Entity, LayerId, check_drawing,
};
use crate::geometry::{Arc as GeoArc, Circle, Line, Vec2};

mod edit;
mod set;

/// The refusal for a capture while either opt-in is off (LCV-145 AC 2).
pub(crate) const CAPTURE_DISABLED: &str = "canvas capture is disabled in Help > AI Settings…";

/// What [`plan`] decided, before anything was applied.
enum Planned {
    /// Commit this command, then append the entity count to this sentence.
    Commit(Box<dyn Command>, String),
    /// Answer without touching the document: a query, or a refusal.
    Answer(AgentOutcome),
    /// Commit this batch of `n` entities; the sentence needs post-commit numbers.
    Batch(Box<dyn Command>, usize),
}

/// Apply one action to the app's live document and report what happened.
///
/// The single entry point the frame loop calls when an `AgentEvent::Act`
/// arrives. Mutating variants commit exactly one command — `history.revision()`
/// advances by one and `Ctrl+Z` takes it back. Query variants commit nothing
/// and leave `revision()` untouched. Either way the outcome is transcribed
/// before it is returned (AC 23).
pub fn apply(app: &mut App, action: &AgentAction) -> AgentOutcome {
    // LCV-145: a capture reads the camera, which `plan` cannot see.
    let planned = match action {
        AgentAction::CaptureCanvas(frame) => Planned::Answer(agent_capture::capture(app, frame)),
        _ => plan(action, &app.document),
    };
    let outcome = match planned {
        Planned::Answer(outcome) => outcome,
        Planned::Commit(command, sentence) => {
            let first = app.document.entity_count();
            // Into the turn's flat group (ADR 0007 §D12); `App::commit` seals.
            app.history.commit_grouped(command, &mut app.document);
            let text = with_count(&sentence, app.document.entity_count());
            AgentOutcome::Ok(text + &new_ids(&app.document, first))
        }
        Planned::Batch(command, n) => {
            let first = app.document.entity_count();
            app.history.commit_grouped(command, &mut app.document);
            let (count, revision) = (app.document.entity_count(), app.history.revision());
            let text = batch_created(n, first, count, revision);
            AgentOutcome::Ok(text + &new_ids(&app.document, first))
        }
    };
    transcribe(app, &outcome);
    outcome
}

/// Append one `agent.chat` row for `outcome`, content **verbatim** (AC 23).
///
/// Two roles, and the split is the one thing the operator most needs to see at
/// a glance: `tool` is something that happened, `refused` is something that did
/// not. LCV-125 decides how each looks; this decides only that it exists.
///
/// Called by [`apply`] for everything it decides, and by `agent_poll` for the
/// one outcome `apply` never sees — a fence refusal, which is a property of the
/// turn rather than of the action (ADR 0007 §D4).
pub(crate) fn transcribe(app: &mut App, outcome: &AgentOutcome) {
    let role = if outcome.is_refused() {
        "refused"
    } else {
        "tool"
    };
    app.agent
        .chat
        .push((role.to_owned(), outcome.text().to_owned()));
}

/// Decide, against the pre-mutation document, what this action becomes.
///
/// Everything that has to be read *before* the change — the entity a delete is
/// about to remove, the indices it is about to renumber — is captured here,
/// because afterwards it is gone.
fn plan(action: &AgentAction, doc: &Document) -> Planned {
    // LCV-156: a named layer must exist; no name means the current layer.
    let layer = match target_layer(action.tool_name(), action.layer(), doc) {
        Ok(id) => id,
        Err(refusal) => return Planned::Answer(refusal),
    };
    match *action {
        AgentAction::CreateLine { x1, y1, x2, y2, .. } => Planned::Commit(
            Box::new(
                CreateLine::new(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2))).on_layer(layer),
            ),
            format!("Line created: {} → {} mm.", pt(x1, y1), pt(x2, y2)),
        ),
        AgentAction::CreateCircle { cx, cy, r, .. } => Planned::Commit(
            Box::new(CreateCircle::new(Circle::new(Vec2::new(cx, cy), r)).on_layer(layer)),
            format!("Circle created: center {} mm, r = {r:.3} mm.", pt(cx, cy)),
        ),
        AgentAction::CreateArc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
            ..
        } => Planned::Commit(
            Box::new(
                CreateArc::new(GeoArc::new(Vec2::new(cx, cy), r, start, end, ccw)).on_layer(layer),
            ),
            format!(
                "Arc created: center {} mm, r = {r:.3} mm, {}.",
                pt(cx, cy),
                sweep(start, end, ccw)
            ),
        ),
        AgentAction::Delete { index } => edit::delete(index, doc),
        AgentAction::Move { index, dx, dy } => edit::move_(index, dx, dy, doc),
        AgentAction::Copy { index, dx, dy } => edit::copy(index, dx, dy, doc),
        AgentAction::Rotate { index, x, y, angle } => edit::rotate(index, x, y, angle, doc),
        AgentAction::Mirror {
            index,
            x1,
            y1,
            x2,
            y2,
            erase_source,
        } => {
            let line = (Vec2::new(x1, y1), Vec2::new(x2, y2));
            edit::mirror(index, line, erase_source, doc)
        }
        AgentAction::Scale {
            index,
            x,
            y,
            factor,
        } => edit::scale(index, Vec2::new(x, y), factor, doc),
        AgentAction::Set {
            ref indices,
            ref op,
        } => set::plan(action.tool_name(), indices, op, doc),
        AgentAction::ById { ref ids, ref op } => set::by_ids(action.tool_name(), ids, op, doc),
        AgentAction::QueryEntities => Planned::Answer(AgentOutcome::Ok(list_entities(doc))),
        AgentAction::QuerySelection => Planned::Answer(AgentOutcome::Ok(list_selection(doc))),
        AgentAction::CheckDrawing => {
            Planned::Answer(AgentOutcome::Ok(check_drawing(doc).lines().join("\n")))
        }
        // LCV-194: answered by `measure::answer` from T10 on.
        AgentAction::Measure(_) => Planned::Answer(AgentOutcome::Refused(refusal(
            "measure",
            "query",
            "not measured yet",
            "a later build",
        ))),
        // One command for the whole batch (ADR 0010 §1, §5).
        AgentAction::CreateDrawing { ref items, .. } => Planned::Batch(
            Box::new(CreateEntities::new(items.iter().map(entity_of).collect()).on_layer(layer)),
            items.len(),
        ),
        // Never planned against the document: `apply` answers a capture from
        // the live app, and `agent_poll` answers an upload check, a note
        // and a reply before the fence. Reaching here is a routing slip, so
        // the answer is the safe one — nothing is rendered and nothing is
        // authorised (LCV-145, LCV-187, LCV-193).
        AgentAction::CaptureCanvas(_)
        | AgentAction::AuthorizeUpload { .. }
        | AgentAction::Note(_)
        | AgentAction::Replied { .. } => {
            Planned::Answer(AgentOutcome::Refused(CAPTURE_DISABLED.to_owned()))
        }
        // §D15: answered from the reason alone; the document is not read.
        AgentAction::Malformed { ref reason, .. } => {
            Planned::Answer(AgentOutcome::Refused(reason.clone()))
        }
    }
}

/// The layer a creation lands on: the named one, resolved by key, else the
/// current one. An unknown name is refused naming the layers (ADR 0012 §6),
/// in the LCV-192 shape; the name is at most 64 characters (`layer_arg`).
fn target_layer(tool: &str, name: Option<&str>, doc: &Document) -> Result<LayerId, AgentOutcome> {
    let Some(name) = name else {
        return Ok(doc.current_layer());
    };
    doc.layer_by_name(name).map(|l| l.id).ok_or_else(|| {
        let names: Vec<String> = doc
            .layers()
            .iter()
            .map(|l| format!("\"{}\"", l.name))
            .collect();
        let reason = format!("unknown layer \"{name}\"");
        let expected = format!("one of {}", names.join(", "));
        AgentOutcome::Refused(refusal(tool, "layer", &reason, &expected))
    })
}

/// One batch item as the entity the matching scalar arm would build.
fn entity_of(item: &DrawingItem) -> Entity {
    match *item {
        DrawingItem::Line { x1, y1, x2, y2 } => {
            Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
        }
        DrawingItem::Circle { cx, cy, r } => Entity::Circle(Circle::new(Vec2::new(cx, cy), r)),
        DrawingItem::Arc {
            cx,
            cy,
            r,
            start,
            end,
            ccw,
        } => Entity::Arc(GeoArc::new(Vec2::new(cx, cy), r, start, end, ccw)),
    }
}

/// Append the post-mutation entity count (ADR 0007 §D5). One space, one
/// sentence, so a test can pin it.
fn with_count(sentence: &str, count: usize) -> String {
    format!("{sentence} The drawing now has {count} entities.")
}

#[cfg(test)]
mod tests;
