//! The import report (LCV-171): what [`super::import_svg`] met but did not
//! turn into geometry, as `(label, count)` entries in order of first
//! occurrence, one entry per label (AC 8).
//!
//! Kernel-pure: MUST NOT import `egui`, `eframe`, or `rfd`.

/// Report builder: [`Report::note`] bumps a label's count or appends it.
#[derive(Debug, Default)]
pub(super) struct Report {
    entries: Vec<(String, usize)>,
}

impl Report {
    /// Count one more `label`.
    pub(super) fn note(&mut self, label: &str) {
        match self.entries.iter_mut().find(|(known, _)| known == label) {
            Some((_, count)) => *count += 1,
            None => self.entries.push((label.to_owned(), 1)),
        }
    }

    /// The entries in order of first occurrence.
    pub(super) fn finish(self) -> Vec<(String, usize)> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC 8 — first-occurrence order, one entry per label.
    #[test]
    fn note_keeps_first_occurrence_order_and_merges_repeats() {
        let mut report = Report::default();
        for label in ["a", "b", "a"] {
            report.note(label);
        }
        let want = vec![("a".to_owned(), 2), ("b".to_owned(), 1)];
        assert_eq!(report.finish(), want);
    }
}
