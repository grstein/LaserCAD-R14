//! [`CommandHistory`] — the 50-entry command-line recall ring.
//!
//! ADR 0003 §A4: the ring is a transcript, not a set. Consecutive duplicates
//! are kept, and `push`/`older`/`newer` return owned `String`s so a caller
//! never fights a borrow of two `App` fields. This module has no consumer
//! until LCV-111 wires an instance onto `App`; its correctness is proven
//! entirely by the unit tests below.

use std::collections::VecDeque;

/// The command-line recall ring: a 50-entry, oldest-evicted transcript of
/// submitted command-line text, with an independent Up/Down recall cursor.
///
/// Duplicates are **not** deduplicated (ADR 0003 §A4, product decision 3):
/// `l`, `l`, `l` is three entries, and Up walks back through all three.
#[derive(Debug, Default, Clone)]
pub struct CommandHistory {
    /// Oldest entry at the front, newest at the back.
    entries: VecDeque<String>,
    /// `None` = no recall in progress. `Some(i)` indexes `entries`, where
    /// `entries.len() - 1` is the newest.
    cursor: Option<usize>,
}

impl CommandHistory {
    /// Maximum number of entries retained. The 51st `push` evicts the
    /// oldest.
    pub const CAPACITY: usize = 50;

    /// Record a submitted line.
    ///
    /// The entry is trimmed first. If the trimmed entry is empty it is
    /// **ignored entirely** — `len()` is unchanged and the recall cursor is
    /// left exactly as it was, so an empty push mid-recall does not disturb
    /// an in-progress Up/Down walk.
    ///
    /// Otherwise the trimmed entry is appended as the newest, evicting the
    /// oldest first if the ring already held [`CAPACITY`](Self::CAPACITY)
    /// entries, and the recall cursor resets to "no recall in progress".
    /// Duplicates are stored, not merged.
    pub fn push(&mut self, entry: &str) {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.entries.len() == Self::CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(trimmed.to_owned());
        self.cursor = None;
    }

    /// Recall an older entry (the Up key).
    ///
    /// The first call after a `push` (or on a fresh ring) returns the
    /// newest entry. Each subsequent call steps one entry further back.
    /// Once the oldest entry is reached, further calls keep returning it —
    /// the cursor does not wrap and does not return `None`. Returns `None`
    /// only when the ring is empty.
    pub fn older(&mut self) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = match self.cursor {
            None => self.entries.len() - 1,
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
        };
        self.cursor = Some(idx);
        self.entries.get(idx).cloned()
    }

    /// Recall a newer entry (the Down key).
    ///
    /// Returns `None` when no recall is in progress (nothing to walk
    /// forward from) and `None` when the walk steps past the newest entry —
    /// in the latter case the cursor resets to "no recall in progress" and
    /// the caller is expected to clear its own input field. Returns `None`
    /// on an empty ring.
    pub fn newer(&mut self) -> Option<String> {
        let cursor = self.cursor?;
        if cursor + 1 >= self.entries.len() {
            self.cursor = None;
            return None;
        }
        let idx = cursor + 1;
        self.cursor = Some(idx);
        self.entries.get(idx).cloned()
    }

    /// Current number of retained entries (never exceeds
    /// [`CAPACITY`](Self::CAPACITY)).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if no entry has ever been pushed (or all pushes were blank).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_is_fifty() {
        assert_eq!(CommandHistory::CAPACITY, 50);
    }

    #[test]
    fn push_ignores_blank_entries() {
        let mut h = CommandHistory::default();
        h.push("");
        h.push("   ");
        h.push("\t");
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());
    }

    #[test]
    fn push_trims() {
        let mut h = CommandHistory::default();
        h.push("  50,25  ");
        assert_eq!(h.older(), Some("50,25".to_owned()));
    }

    /// Named after decision 3 so deleting it is visible in a diff: the ring
    /// does not deduplicate consecutive identical entries.
    #[test]
    fn push_keeps_duplicates() {
        let mut h = CommandHistory::default();
        h.push("l");
        h.push("l");
        h.push("l");
        assert_eq!(h.len(), 3);
        assert_eq!(h.older(), Some("l".to_owned()));
        assert_eq!(h.older(), Some("l".to_owned()));
        assert_eq!(h.older(), Some("l".to_owned()));
    }

    #[test]
    fn ring_evicts_the_oldest_past_fifty() {
        let mut h = CommandHistory::default();
        for i in 1..=60 {
            h.push(&format!("entry {i}"));
        }
        assert_eq!(h.len(), 50);
        assert_eq!(h.older(), Some("entry 60".to_owned()));
        // Already consumed one `older()` above; 49 more reaches the oldest
        // retained entry, "entry 11" (60 pushes, 10 evicted).
        let mut last = None;
        for _ in 0..49 {
            last = h.older();
        }
        assert_eq!(last, Some("entry 11".to_owned()));
    }

    #[test]
    fn older_stops_at_the_oldest() {
        let mut h = CommandHistory::default();
        h.push("a");
        h.push("b");
        h.push("c");

        assert_eq!(h.older(), Some("c".to_owned()));
        assert_eq!(h.older(), Some("b".to_owned()));
        assert_eq!(h.older(), Some("a".to_owned()));
        // Past the oldest: stays at "a", does not wrap, does not return None.
        assert_eq!(h.older(), Some("a".to_owned()));
        assert_eq!(h.older(), Some("a".to_owned()));
    }

    #[test]
    fn newer_without_recall_returns_none() {
        let mut h = CommandHistory::default();
        h.push("a");
        assert_eq!(h.newer(), None);
    }

    #[test]
    fn newer_past_the_newest_returns_none() {
        let mut h = CommandHistory::default();
        h.push("a");
        h.push("b");
        h.push("c");

        assert_eq!(h.older(), Some("c".to_owned()));
        // Walked past the newest: None, and the cursor resets.
        assert_eq!(h.newer(), None);
        // The reset is real: the next older() starts again from the newest.
        assert_eq!(h.older(), Some("c".to_owned()));
    }

    #[test]
    fn older_twice_then_newer_returns_the_more_recent_entry() {
        let mut h = CommandHistory::default();
        h.push("a");
        h.push("b");
        h.push("c");

        assert_eq!(h.older(), Some("c".to_owned()));
        assert_eq!(h.older(), Some("b".to_owned()));
        assert_eq!(h.newer(), Some("c".to_owned()));
    }

    #[test]
    fn push_resets_the_cursor() {
        let mut h = CommandHistory::default();
        h.push("a");
        h.push("b");
        h.push("c");
        assert_eq!(h.older(), Some("c".to_owned()));
        assert_eq!(h.older(), Some("b".to_owned()));

        h.push("d");
        assert_eq!(
            h.older(),
            Some("d".to_owned()),
            "a push mid-recall must reset the cursor to the new newest entry"
        );
    }

    #[test]
    fn empty_ring_returns_none_both_ways() {
        let mut h = CommandHistory::default();
        assert_eq!(h.older(), None);
        assert_eq!(h.newer(), None);
    }
}
