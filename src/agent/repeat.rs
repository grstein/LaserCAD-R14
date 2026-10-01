//! The turn's memory of refused calls (LCV-192 AC 4, ADR 0007 §D15): a call
//! that repeats, byte for byte, the tool name and argument string of a call
//! already refused in this turn is not run again. It is answered with the
//! first refusal, so the model sees it is looping and changes the arguments.
//!
//! Only a `Refused` outcome is remembered: an applied call may be repeated on
//! purpose, and a `Fenced` answer is the turn ending, not the call's fault.
//! One `RefusedCalls` lives for one turn (`agent_worker.rs::drive_turn`).
//!
//! Kernel-pure: imports `egui` nowhere, `eframe` nowhere, `rfd` nowhere.

use std::collections::HashMap;

use super::AgentOutcome;

/// The refused calls of one turn, keyed by `(tool name, argument bytes)`,
/// each mapped to the text of its first refusal.
#[derive(Debug, Default)]
pub struct RefusedCalls {
    first: HashMap<(String, String), String>,
}

impl RefusedCalls {
    /// The answer for a call that repeats one already refused in this turn:
    /// `repeated call, refused before: <first refusal>; change the arguments`.
    /// `None` when `(name, args)` was never refused.
    pub fn check(&self, name: &str, args: &str) -> Option<String> {
        let first = self.first.get(&(name.to_owned(), args.to_owned()))?;
        Some(format!(
            "repeated call, refused before: {first}; change the arguments"
        ))
    }

    /// Remember `(name, args)` if `outcome` refused it; the first refusal of
    /// a call is the one kept.
    pub fn record(&mut self, name: &str, args: &str, outcome: &AgentOutcome) {
        if let AgentOutcome::Refused(text) = outcome {
            self.first
                .entry((name.to_owned(), args.to_owned()))
                .or_insert_with(|| text.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RefusedCalls;
    use crate::agent::AgentOutcome;

    const ARGS: &str = r#"{"index":7}"#;
    const FIRST: &str = "delete_entity index: 7 is out of range; expected 0..=2 \
                         (the drawing has 3 entities)";

    fn refused(text: &str) -> AgentOutcome {
        AgentOutcome::Refused(text.to_owned())
    }

    /// AC 4 — a byte-identical refused call is answered with the first
    /// refusal, quoted in the repeat text.
    #[test]
    fn a_byte_identical_refused_call_is_a_repeat() {
        let mut calls = RefusedCalls::default();
        assert_eq!(calls.check("delete_entity", ARGS), None, "nothing yet");
        calls.record("delete_entity", ARGS, &refused(FIRST));
        assert_eq!(
            calls.check("delete_entity", ARGS).as_deref(),
            Some(format!("repeated call, refused before: {FIRST}; change the arguments").as_str())
        );
    }

    /// AC 4 — the first refusal is the one quoted, whatever comes after.
    #[test]
    fn the_first_refusal_is_kept() {
        let mut calls = RefusedCalls::default();
        calls.record("delete_entity", ARGS, &refused(FIRST));
        calls.record("delete_entity", ARGS, &refused("later"));
        let text = calls.check("delete_entity", ARGS).unwrap_or_default();
        assert!(text.contains(FIRST), "{text}");
        assert!(!text.contains("later"), "{text}");
    }

    /// AC 4 — any other argument bytes or any other tool is a new call.
    #[test]
    fn other_bytes_or_another_tool_is_not_a_repeat() {
        let mut calls = RefusedCalls::default();
        calls.record("delete_entity", ARGS, &refused(FIRST));
        for (name, args) in [
            ("delete_entity", r#"{"index": 7}"#),
            ("delete_entity", r#"{"index":6}"#),
            ("delete_entity", ""),
            ("move_entity", ARGS),
        ] {
            assert_eq!(calls.check(name, args), None, "{name} {args}");
        }
    }

    /// AC 4 — only a refusal is remembered: an applied call, an observation
    /// and a fence answer never make a repeat.
    #[test]
    fn only_a_refusal_is_recorded() {
        let mut calls = RefusedCalls::default();
        let not_refusals = [
            AgentOutcome::Ok("Deleted entity 0.".to_owned()),
            AgentOutcome::Fenced("the turn is fenced".to_owned()),
            AgentOutcome::Observed {
                text: "seen".to_owned(),
                png: Vec::new(),
            },
        ];
        for outcome in &not_refusals {
            calls.record("delete_entity", ARGS, outcome);
            assert_eq!(calls.check("delete_entity", ARGS), None, "{outcome:?}");
        }
    }
}
