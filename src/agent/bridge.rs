//! LCV-122 — the protocol between the agent thread and the UI thread.
//!
//! ADR 0007 §D1 forbids the background thread from owning a [`Document`], a
//! clone of one, or a snapshot of any kind. What it owns instead is this
//! vocabulary: it names *what it wants done* ([`AgentAction`]), hands that to
//! the UI thread inside an [`AgentEvent::Act`] carrying a one-shot reply
//! channel, and blocks until the UI thread answers with what really happened
//! ([`AgentOutcome`]) against the drawing the operator is really looking at.
//!
//! Three properties this file must keep:
//!
//! - **Kernel-pure.** No `egui`, no `eframe`, no `rfd`, no `reqwest`. Also no
//!   `Document`: the thread that can see this module is the thread that may
//!   not see the document.
//! - **Millimetres and radians.** Every coordinate, length and radius is mm;
//!   [`AgentAction::CreateArc`]'s `start` / `end` are radians. The schema the
//!   model writes against speaks `start_deg` / `end_deg`, and
//!   `crate::agent::tools::parse_tool_call` converts at that boundary so
//!   everything downstream is kernel-normal.
//! - **No fence here.** [`TurnFence`](crate::app::TurnFence) lives in
//!   `src/app/agent_turn.rs` and deliberately *not* in this module: the fence
//!   is the one value the worker thread must never evaluate, and this module
//!   is visible to the worker thread (ADR 0007 §D4).
//!
//! [`Document`]: crate::document::Document

use crate::agent::drawing::DrawingItem;
use std::sync::mpsc::Sender;

/// One thing the model wants done to the drawing.
///
/// Produced by `crate::agent::tools::parse_tool_call` from a validated tool
/// call, applied by `crate::app::agent_apply::apply`. Deliberately **not** a
/// `serde` deserialization target (ADR 0007 §Alternatives): deriving it would
/// throw away the hand-rolled per-field checks and messages that are most of
/// `tools.rs`'s value.
///
/// `index` is a **positional** handle into `Document::entities`, so a delete
/// renumbers every higher index down by one. That is disclosed in the prompt
/// and in every outcome string (ADR 0007 §D5) until stable ids land.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    /// Append a line between two endpoints, in mm.
    CreateLine {
        /// First endpoint X, mm.
        x1: f64,
        /// First endpoint Y, mm.
        y1: f64,
        /// Second endpoint X, mm.
        x2: f64,
        /// Second endpoint Y, mm.
        y2: f64,
    },
    /// Append a full circle, centre in mm and radius in mm.
    CreateCircle {
        /// Centre X, mm.
        cx: f64,
        /// Centre Y, mm.
        cy: f64,
        /// Radius, mm. Positive and finite — checked at parse time.
        r: f64,
    },
    /// Append a circular arc. Angles are **radians** (`0` = +X axis).
    CreateArc {
        /// Centre X, mm.
        cx: f64,
        /// Centre Y, mm.
        cy: f64,
        /// Radius, mm. Positive and finite — checked at parse time.
        r: f64,
        /// Start angle, radians.
        start: f64,
        /// End angle, radians.
        end: f64,
        /// `true` for a counter-clockwise sweep.
        ccw: bool,
    },
    /// Delete the entity at this positional index.
    Delete {
        /// Zero-based index into `Document::entities`.
        index: usize,
    },
    /// Translate one entity by `(dx, dy)` mm.
    Move {
        /// Zero-based index into `Document::entities`.
        index: usize,
        /// X translation, mm.
        dx: f64,
        /// Y translation, mm.
        dy: f64,
    },
    /// Read back every entity in the drawing. Commits nothing.
    QueryEntities,
    /// Read back the current selection. Commits nothing.
    QuerySelection,
    /// Render the drawing as framed in the viewport into a grayscale PNG
    /// (LCV-145, ADR 0011). Commits nothing; one step like any action.
    CaptureCanvas,
    /// Ask whether a request carrying canvas images may go to `endpoint` /
    /// `model` (ADR 0011 item 10). **Not a step**: it bypasses the fence and
    /// the step counter. Never carries the API key.
    AuthorizeUpload {
        /// The endpoint the turn was started with.
        endpoint: String,
        /// The model the turn was started with.
        model: String,
    },
    /// Append a whole validated batch as one command (LCV-144, ADR 0010).
    CreateDrawing {
        /// The entities, in order; 1..=1000, already shape-checked.
        items: Vec<DrawingItem>,
    },
    /// A tool call that failed its shape check in the worker — JSON syntax,
    /// unknown tool, or any `ToolCallError` (ADR 0007 §D15). The UI answers
    /// `Refused(reason)` without reading the document; it is still a step.
    Malformed {
        /// The tool name as the model sent it.
        tool: String,
        /// Names the tool and the field; never echoes the raw arguments.
        reason: String,
    },
}

/// What the UI thread answers for one [`AgentAction`].
///
/// Every variant travels back to the model as the `tool` result: **a refusal is
/// information, not a failure** (ADR 0007 §D2a). An out-of-range index or a
/// malformed call answers `Refused` and the model can re-read the drawing and
/// try again. A tripped fence answers `Fenced`, after which the worker sends
/// one last completion and the turn ends (§D14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentOutcome {
    /// The action was applied. Carries the sentence the model reads back.
    Ok(String),
    /// The action was not applied, and this is why.
    Refused(String),
    /// The action was not applied because the turn's fence has tripped (ADR
    /// 0007 §D14). A refusal like any other to the model and the transcript,
    /// and the one verdict on which the worker stops dispatching: it reads the
    /// UI thread's decision, it never evaluates the fence itself.
    Fenced(String),
    /// A canvas observation (LCV-145). `text` is the tool result and the
    /// transcript row; `png` goes only to the next request as an image part
    /// and is never transcribed, logged or persisted (ADR 0011 item 7).
    Observed {
        /// The frame's pixel size and mm mapping, and the revision.
        text: String,
        /// The encoded grayscale PNG.
        png: Vec<u8>,
    },
}

impl AgentOutcome {
    /// The sentence inside, whichever variant this is — what goes back to the
    /// model as the `tool` result.
    pub fn text(&self) -> &str {
        match self {
            Self::Ok(text) | Self::Refused(text) | Self::Fenced(text) => text,
            Self::Observed { text, .. } => text,
        }
    }

    /// Consume the outcome and take its sentence.
    pub fn into_text(self) -> String {
        match self {
            Self::Ok(text) | Self::Refused(text) | Self::Fenced(text) => text,
            Self::Observed { text, .. } => text,
        }
    }

    /// Was this a refusal — `Fenced` included? Drives the transcript role.
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused(_) | Self::Fenced(_))
    }

    /// Did the fence stop this turn? The worker's cue to dispatch nothing more.
    pub fn is_fenced(&self) -> bool {
        matches!(self, Self::Fenced(_))
    }
}

/// One message from the agent thread to the UI frame loop.
///
/// Exactly one channel carries all three (ADR 0007 §D3): the reply `Sender`
/// for an [`AgentEvent::Act`] is built per request by the thread and travels
/// inside the message, so `App` keeps a single `agent.rx` field and there is
/// never more than one request outstanding — the thread blocks on the answer.
///
/// Dropping the reply `Sender` without answering is the documented
/// cancellation signal: the thread's `recv()` fails and the turn ends cleanly
/// (ADR 0007 §D2).
pub enum AgentEvent {
    /// Apply `action` to the live document and answer down `reply`.
    Act {
        /// What the model asked for.
        action: AgentAction,
        /// One-shot channel for the real outcome.
        reply: Sender<AgentOutcome>,
    },
    /// Terminal: the turn finished; carries the final assistant text.
    Done(String),
    /// Terminal: the turn failed; carries a human-readable error.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    /// AC 1 — every variant of the protocol constructs, and an `Act` really
    /// does carry a live `Sender<AgentOutcome>`: the value sent through it
    /// comes back out of the paired receiver.
    #[test]
    fn every_event_variant_constructs_and_act_carries_a_reply_channel() {
        let (reply, answers) = channel::<AgentOutcome>();
        let events = [
            AgentEvent::Act {
                action: AgentAction::QueryEntities,
                reply,
            },
            AgentEvent::Done("done".into()),
            AgentEvent::Failed("failed".into()),
        ];
        assert_eq!(events.len(), 3);

        let AgentEvent::Act { action, reply } = &events[0] else {
            panic!("the first event must be Act");
        };
        assert_eq!(*action, AgentAction::QueryEntities);
        reply
            .send(AgentOutcome::Ok("applied".into()))
            .expect("the reply channel must be live");
        assert_eq!(
            answers.recv().expect("an answer"),
            AgentOutcome::Ok("applied".into())
        );
    }

    /// AC 1 — the seven action variants exist with the documented field names
    /// and types. Compile proof plus a distinctness check, so a duplicated
    /// variant cannot hide.
    #[test]
    fn the_seven_action_variants_are_distinct() {
        let actions = [
            AgentAction::CreateLine {
                x1: 0.0,
                y1: 0.0,
                x2: 20.0,
                y2: 0.0,
            },
            AgentAction::CreateCircle {
                cx: 1.0,
                cy: 2.0,
                r: 3.0,
            },
            AgentAction::CreateArc {
                cx: 0.0,
                cy: 0.0,
                r: 5.0,
                start: 0.0,
                end: core::f64::consts::FRAC_PI_2,
                ccw: true,
            },
            AgentAction::Delete { index: 7 },
            AgentAction::Move {
                index: 0,
                dx: 3.0,
                dy: 4.0,
            },
            AgentAction::QueryEntities,
            AgentAction::QuerySelection,
        ];
        assert_eq!(actions.len(), 7);
        for (i, a) in actions.iter().enumerate() {
            for (j, b) in actions.iter().enumerate() {
                assert_eq!(
                    i == j,
                    std::mem::discriminant(a) == std::mem::discriminant(b),
                    "variants {i} and {j} must be distinct"
                );
            }
        }
    }

    /// AC 1 — `AgentOutcome` carries a string either way and says which it is.
    #[test]
    fn outcome_exposes_its_text_and_its_kind() {
        let ok = AgentOutcome::Ok("applied".into());
        let refused = AgentOutcome::Refused("index 7 is out of range".into());
        assert_eq!(ok.text(), "applied");
        assert!(!ok.is_refused());
        assert!(refused.is_refused());
        assert_eq!(refused.clone().into_text(), "index 7 is out of range");
        assert_eq!(ok.into_text(), "applied");
    }

    /// LCV-142 AC 9 — `Fenced` is a refusal to the transcript and the only
    /// outcome the worker stops on.
    #[test]
    fn fenced_is_a_refusal_and_the_only_stop() {
        let fenced = AgentOutcome::Fenced("changed".into());
        assert!(fenced.is_refused());
        assert!(fenced.is_fenced());
        assert!(!AgentOutcome::Refused("r".into()).is_fenced());
        assert!(!AgentOutcome::Ok("o".into()).is_fenced());
        assert_eq!(fenced.text(), "changed");
        assert_eq!(fenced.into_text(), "changed");
    }

    /// AC 2 — this file is kernel-pure: no UI crate and no HTTP crate reaches
    /// it, and neither does the document.
    ///
    /// Bounded at the bare `#[cfg(test)]` at column 0 and every needle built
    /// with `concat!`, so the scan cannot match the literals written right
    /// here (the canonical shape is `guard_is_runtime_not_cfg` in
    /// `src/io/dialogs.rs`). The positive control is tight: it looks for the
    /// one `use` line the implementation really has, so an empty or
    /// mis-sliced haystack fails instead of passing vacuously.
    #[test]
    fn bridge_is_kernel_pure() {
        let src = include_str!("bridge.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("bridge.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("use std::sync::mpsc::", "Sender;")),
            "positive control: the implementation section must be real source"
        );

        for forbidden in [
            concat!("e", "gui"),
            concat!("e", "frame"),
            concat!("r", "fd"),
            concat!("req", "west"),
            concat!("crate::", "document"),
        ] {
            let hit = implementation
                .lines()
                .find(|line| line.contains(forbidden) && !line.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "AC 2: bridge.rs must not name `{forbidden}` in code, found `{hit:?}`"
            );
        }
    }

    /// LCV-145 AC 8 — an observation reads as its text only, is not a
    /// refusal and does not stop the turn; the PNG never leaks into the text.
    #[test]
    fn observed_exposes_only_its_text() {
        let observed = AgentOutcome::Observed {
            text: "Canvas 2×1 px".into(),
            png: vec![0x89, b'P', b'N', b'G'],
        };
        assert_eq!(observed.text(), "Canvas 2×1 px");
        assert!(!observed.is_refused());
        assert!(!observed.is_fenced());
        assert_eq!(observed.into_text(), "Canvas 2×1 px");
        let auth = AgentAction::AuthorizeUpload {
            endpoint: "https://e".into(),
            model: "m".into(),
        };
        assert_ne!(auth, AgentAction::CaptureCanvas);
    }

    /// AC 7 — the action enum is hand-built, never deserialized. A
    /// `Deserialize` derive here would let a future demand delete `tools.rs`'s
    /// per-field checks without noticing.
    #[test]
    fn agent_action_carries_no_deserialize_derive() {
        let src = include_str!("bridge.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("bridge.rs must have a bare #[cfg(test)] marker");
        let implementation = &src[..at];

        assert!(
            implementation.contains(concat!("pub enum ", "AgentAction")),
            "positive control: the enum must be declared in this file"
        );
        assert!(
            !implementation.contains(concat!("Deser", "ialize")),
            "AC 7: AgentAction must not be a serde deserialization target"
        );
    }
}
