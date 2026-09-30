//! The dock message's severity (LCV-165 AC 1).
//!
//! `App::command_feedback` stays a plain `String` so its many readers are
//! untouched; its sibling `App::command_feedback_severity` says how the dock
//! paints it. Both are written together, through [`App::say`] and nowhere
//! else, so a message can never keep a stale colour.
//!
//! MUST NOT import `eframe`, `rfd` or `crate::ui`.

use super::App;

/// How serious one dock message is; the dock picks its colour from this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Severity {
    /// The operation or configuration failed (`status.error`).
    Error,
    /// The input was refused (`status.warning`).
    #[default]
    Warning,
    /// A result or acknowledgement (`text.primary`).
    Info,
}

impl App {
    /// Show `text` in the command dock at `severity` (LCV-165 AC 1).
    pub fn say(&mut self, severity: Severity, text: impl Into<String>) {
        self.command_feedback = text.into();
        self.command_feedback_severity = severity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `say` writes the text and its severity together.
    #[test]
    fn say_writes_text_and_severity_together() {
        let mut app = App::default();
        assert_eq!(app.command_feedback_severity, Severity::Warning);
        app.say(Severity::Info, "DIST = 1");
        assert_eq!(app.command_feedback, "DIST = 1");
        assert_eq!(app.command_feedback_severity, Severity::Info);
        app.say(Severity::Error, String::from("failed"));
        assert_eq!(app.command_feedback_severity, Severity::Error);
    }
}
