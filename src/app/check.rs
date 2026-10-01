//! `check` (LCV-190): run the drawing check, summarise it in the dock and
//! keep its lines for the Check window.
//!
//! Read-only: no `Command`, no history entry, no selection change (AC 8).
//!
//! MUST NOT import `eframe`, `rfd` or `crate::ui`.

use super::{App, Severity};
use crate::document::check_drawing;

impl App {
    /// Check the drawing (LCV-190 AC 1, AC 7).
    ///
    /// With findings, the dock shows the summary lines joined by `; ` (one
    /// `CHECK: ` prefix) as a warning and [`App::check_report`] takes every report line, which opens
    /// (or refreshes) the Check window. A clean drawing shows
    /// `CHECK: no problems found.` and closes the window.
    pub fn run_check(&mut self) {
        let report = check_drawing(&self.document);
        let lines = report.lines();
        if report.findings.is_empty() {
            self.check_report = None;
            self.say(Severity::Info, lines.join("; "));
            return;
        }
        let summary = lines.len() - report.findings.len();
        let dock: Vec<&str> = lines[..summary]
            .iter()
            .enumerate()
            .map(|(i, l)| match i {
                0 => l.as_str(),
                _ => l.strip_prefix("CHECK: ").unwrap_or(l),
            })
            .collect();
        self.say(Severity::Warning, dock.join("; "));
        self.check_report = Some(lines);
    }
}
