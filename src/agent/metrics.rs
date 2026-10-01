//! The counts of one agent turn (LCV-193): what it cost and how clean it was,
//! taken from the loop's own records and never from the model's text.
//!
//! One [`TurnMetrics`] lives in the UI-side turn state
//! (`app::agent_turn::TurnState::tally`); every turn exit ends with its
//! [`TurnMetrics::note`] row (ADR 0007 §D11).
//!
//! Kernel-pure: imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere.

use super::AgentOutcome;

/// The counts of one agent turn, as a plain struct tests can compare.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TurnMetrics {
    /// Step `Act`s answered: every dispatched tool call, malformed and fenced
    /// included (ADR 0007 §D13).
    pub steps: u32,
    /// Steps that moved the document's revision.
    pub applied: u32,
    /// Steps answered `Refused` or `Fenced`.
    pub refused: u32,
    /// Refused steps that repeated a call already refused this turn (LCV-192).
    pub repeated: u32,
    /// Canvas images that rode a request the model answered.
    pub captures: u32,
    /// Completions the model returned.
    pub replies: u32,
}

impl TurnMetrics {
    /// Count one step answered `outcome`; `repeated` when the call repeated
    /// one already refused this turn.
    pub fn step(&mut self, outcome: &AgentOutcome, repeated: bool) {
        let _ = (outcome, repeated);
    }

    /// The transcript line every turn ends with, zero counts included.
    pub fn note(&self) -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::TurnMetrics;
    use crate::agent::AgentOutcome;

    fn text(s: &str) -> String {
        s.to_owned()
    }

    /// AC 1, AC 4 — the line, word for word, zero counts included.
    #[test]
    fn the_note_renders_every_count_in_order() {
        assert_eq!(
            TurnMetrics::default().note(),
            "Turn: 0 steps, 0 actions applied, 0 refused (0 repeated), 0 captures sent, \
             0 model replies."
        );
        let tally = TurnMetrics {
            steps: 1,
            applied: 2,
            refused: 3,
            repeated: 4,
            captures: 5,
            replies: 6,
        };
        assert_eq!(
            tally.note(),
            "Turn: 1 steps, 2 actions applied, 3 refused (4 repeated), 5 captures sent, \
             6 model replies."
        );
    }

    /// AC 2 — `Ok` and `Observed` are steps only; `Refused` and `Fenced` are
    /// steps and refusals; a repeat is all three.
    #[test]
    fn step_classifies_each_outcome() {
        let observed = AgentOutcome::Observed {
            text: text("seen"),
            png: Vec::new(),
        };
        let cases = [
            (AgentOutcome::Ok(text("ok")), false, (1, 0, 0)),
            (observed, false, (1, 0, 0)),
            (AgentOutcome::Refused(text("no")), false, (1, 1, 0)),
            (AgentOutcome::Fenced(text("fenced")), false, (1, 1, 0)),
            (AgentOutcome::Refused(text("again")), true, (1, 1, 1)),
            (AgentOutcome::Fenced(text("again")), true, (1, 1, 1)),
        ];
        for (outcome, repeated, (steps, refused, repeats)) in cases {
            let mut tally = TurnMetrics::default();
            tally.step(&outcome, repeated);
            let want = TurnMetrics {
                steps,
                refused,
                repeated: repeats,
                ..TurnMetrics::default()
            };
            assert_eq!(tally, want, "{outcome:?} repeated={repeated}");
        }
    }

    /// AC 2 — counts accumulate; `applied`, `captures` and `replies` are not
    /// the step's business.
    #[test]
    fn steps_accumulate() {
        let mut tally = TurnMetrics::default();
        tally.step(&AgentOutcome::Ok(text("a")), false);
        tally.step(&AgentOutcome::Refused(text("b")), false);
        tally.step(&AgentOutcome::Refused(text("b")), true);
        assert_eq!(
            tally,
            TurnMetrics {
                steps: 3,
                refused: 2,
                repeated: 1,
                ..TurnMetrics::default()
            }
        );
    }
}
