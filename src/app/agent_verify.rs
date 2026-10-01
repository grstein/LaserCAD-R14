//! LCV-197 — has the in-flight turn verified what it drew?
//!
//! The UI half of the verify reminder (ADR 0007 Amended (15)): the loop asks
//! `AgentAction::VerifyDue` once, on a text-only reply, and this state says
//! yes when an applied action of this turn is not followed by an answered
//! verification call. Pure bookkeeping, no document access.

use crate::agent::{AgentAction, AgentOutcome};
use crate::app::App;

/// The transcript `note` row a granted reminder leaves (LCV-197 AC 4).
pub(crate) const VERIFY_NOTE: &str = "Asked the agent to verify its work.";

/// `AgentAction::VerifyDue`: `Ok("")` with a [`VERIFY_NOTE`] row when the
/// reminder is due, else `Refused("")`. Not a step, not fenced, not counted.
pub(crate) fn answer(app: &mut App) -> AgentOutcome {
    if !app.agent.turn.verify.due() {
        return AgentOutcome::Refused(String::new());
    }
    app.agent
        .chat
        .push(("note".to_owned(), VERIFY_NOTE.to_owned()));
    AgentOutcome::Ok(String::new())
}

/// The turn's verification bookkeeping, reached as `app.agent.turn.verify`.
#[derive(Debug, Default)]
pub struct VerifyState {
    /// An applied action is not followed by an answered verification call.
    unverified: bool,
    /// The reminder was already granted in this turn.
    reminded: bool,
}

impl VerifyState {
    /// Record one answered step: `moved` is whether it moved the revision.
    /// A move sets `unverified`; a verification call whose answer is neither
    /// refused nor fenced clears it.
    pub(crate) fn after(&mut self, action: &AgentAction, outcome: &AgentOutcome, moved: bool) {
        if moved {
            self.unverified = true;
        } else if is_verification(action) && !outcome.is_refused() {
            self.unverified = false;
        }
    }

    /// Is the reminder due? Yes at most once per turn, and only while the
    /// turn's last applied action is unverified; a yes marks it granted.
    pub(crate) fn due(&mut self) -> bool {
        let due = self.unverified && !self.reminded;
        self.reminded |= due;
        due
    }
}

/// `measure`, `check_drawing`, `capture_canvas` and `query_entities`: the
/// calls that verify a drawing (LCV-197).
pub(crate) fn is_verification(action: &AgentAction) -> bool {
    matches!(
        action,
        AgentAction::Measure(_)
            | AgentAction::CheckDrawing
            | AgentAction::CaptureCanvas(_)
            | AgentAction::QueryEntities
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{CaptureFrame, MeasureQuery, MeasureRequest, MeasureTargets};

    fn create() -> AgentAction {
        AgentAction::CreateLine {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 0.0,
            layer: None,
        }
    }

    fn measure() -> AgentAction {
        AgentAction::Measure(MeasureRequest {
            query: MeasureQuery::Length,
            points: Vec::new(),
            targets: MeasureTargets::Indices(vec![0]),
        })
    }

    fn ok() -> AgentOutcome {
        AgentOutcome::Ok("done".to_owned())
    }

    fn applied() -> VerifyState {
        let mut state = VerifyState::default();
        state.after(&create(), &ok(), true);
        state
    }

    /// AC 3 — an applied action makes the reminder due once; a second ask
    /// is a no (AC 5).
    #[test]
    fn an_apply_is_due_once() {
        let mut state = applied();
        assert!(state.due());
        assert!(!state.due());
    }

    /// AC 3 — each answered verification call after the apply clears it.
    #[test]
    fn an_answered_verification_clears_it() {
        let observed = AgentOutcome::Observed {
            text: "seen".to_owned(),
            png: Vec::new(),
        };
        let cases = [
            (measure(), ok()),
            (AgentAction::CheckDrawing, ok()),
            (AgentAction::CaptureCanvas(CaptureFrame::Drawing), observed),
            (AgentAction::QueryEntities, ok()),
        ];
        for (action, outcome) in cases {
            let mut state = applied();
            state.after(&action, &outcome, false);
            assert!(!state.due(), "{action:?} verifies");
        }
    }

    /// AC 3 — a refused or fenced verification verified nothing.
    #[test]
    fn a_refused_verification_leaves_it_due() {
        for outcome in [
            AgentOutcome::Refused("no".to_owned()),
            AgentOutcome::Fenced("fence".to_owned()),
        ] {
            let mut state = applied();
            state.after(&measure(), &outcome, false);
            assert!(state.due(), "{outcome:?}");
        }
    }

    /// AC 3 — an apply after a verification needs verifying again.
    #[test]
    fn an_apply_after_a_verification_is_due() {
        let mut state = applied();
        state.after(&AgentAction::CheckDrawing, &ok(), false);
        state.after(&create(), &ok(), true);
        assert!(state.due());
    }

    /// AC 3 — a step that neither moves nor verifies changes nothing.
    #[test]
    fn a_selection_query_does_not_verify() {
        let mut state = applied();
        state.after(&AgentAction::QuerySelection, &ok(), false);
        assert!(state.due());
    }

    /// AC 7 — a query-only turn is never due.
    #[test]
    fn a_query_only_turn_is_never_due() {
        let mut state = VerifyState::default();
        state.after(&AgentAction::QueryEntities, &ok(), false);
        state.after(&AgentAction::QuerySelection, &ok(), false);
        state.after(&create(), &AgentOutcome::Refused("no".to_owned()), false);
        assert!(!state.due());
        assert!(!state.due());
    }
}
