//! The autosave dirty signal, debounce and flush (LCV-059 / LCV-102,
//! ADR 0002 §B).
//!
//! Two halves, both here since LCV-111 moved the signal out of `mod.rs`:
//! the *signal* — [`App::sync_dirty`](super::App::sync_dirty), the only writer
//! of `dirty_since = Some(_)`, and [`App::mark_clean`](super::App::mark_clean),
//! the only writer of `dirty_since = None` — and the *timing*: how long a
//! change waits before it is written, and the once-per-frame flush check.
//!
//! MUST NOT import `eframe` or `rfd`.

use std::time::{Duration, Instant};

use super::App;

/// Debounce delay before an unsaved change triggers an autosave write.
/// 800 ms matches LaserCAD v1 (ADR 0002 §B).
const AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(800);

/// Is an armed dirty timer old enough to flush? `dirty_since` is `None` when
/// the document is clean; `Some(t)` means it has been unsaved-dirty since
/// `t`. `now` is a parameter (not `Instant::now()` internally) so the
/// boundary can be tested without a real clock delay (ADR 0002 §A4 rule 2). The
/// boundary is inclusive: exactly `AUTOSAVE_DEBOUNCE` elapsed is due.
pub fn autosave_due(dirty_since: Option<Instant>, now: Instant) -> bool {
    match dirty_since {
        Some(since) => now.saturating_duration_since(since) >= AUTOSAVE_DEBOUNCE,
        None => false,
    }
}

/// Write the autosave file if the debounce has elapsed (LCV-059).
///
/// Called once per frame from [`App::update_ui`](super::App::update_ui),
/// immediately after the single [`App::sync_dirty`](super::App::sync_dirty)
/// call so the flush sees this frame's mutations.
pub fn flush_if_due(app: &mut App) {
    if autosave_due(app.dirty_since, Instant::now()) {
        let _ = crate::io::save_autosave(&app.document);
        // Cleared unconditionally: a failed write is dropped, not retried
        // every frame. The next document change re-arms the debounce.
        app.mark_clean();
    }
}

impl App {
    /// The **only** writer of `dirty_since = Some(_)` (ADR 0002 §B). Called
    /// exactly once per frame, from [`App::update_ui`](super::App::update_ui), immediately before the
    /// autosave-flush check.
    ///
    /// Compares `history.revision()` against `last_synced_revision`: if they
    /// differ, something committed, undid, or redid since the last sync, so
    /// `dirty_since` is armed via `get_or_insert_with(Instant::now)` — which
    /// preserves an already-set instant, so the debounce is measured from the
    /// *first* unsaved change, not the latest one — and `last_synced_revision`
    /// is advanced to the current revision. Calling it twice with no
    /// intervening mutation is a no-op the second time.
    pub(super) fn sync_dirty(&mut self) {
        let revision = self.history.revision();
        if revision != self.last_synced_revision {
            self.dirty_since.get_or_insert_with(Instant::now);
            self.last_synced_revision = revision;
        }
    }

    /// The **only** writer that resets `dirty_since` to `None`. Clears the
    /// debounce timer and resyncs `last_synced_revision` to the current
    /// `history.revision()` in one step (ADR 0002 §B).
    ///
    /// Resyncing the revision here — not just clearing `dirty_since` — is
    /// mandatory wherever `history` is replaced with a fresh one (`action_new`
    /// / `action_open` / `action_open_path`): a fresh `History` reports
    /// revision `0`, and without resyncing, the very next frame's
    /// `sync_dirty` would see `0 != last_synced_revision` and re-dirty a
    /// document that was just loaded or reset. Call this **after** any
    /// `history` replacement, never before.
    pub fn mark_clean(&mut self) {
        self.dirty_since = None;
        self.last_synced_revision = self.history.revision();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LCV-102 AC 15 — the debounce constant matches v1 (800 ms).
    #[test]
    fn autosave_debounce_is_800ms() {
        assert_eq!(AUTOSAVE_DEBOUNCE, Duration::from_millis(800));
    }

    /// LCV-102 AC 16 — `autosave_due` boundary cases, none of which pauses the
    /// clock.
    #[test]
    fn autosave_due_boundaries() {
        let now = Instant::now();
        assert!(!autosave_due(None, now), "clean document is never due");
        assert!(!autosave_due(
            now.checked_sub(Duration::from_millis(799)),
            now
        ));
        assert!(
            autosave_due(now.checked_sub(Duration::from_millis(800)), now),
            "boundary is inclusive (>=)"
        );
        assert!(autosave_due(now.checked_sub(Duration::from_secs(5)), now));
    }

    /// LCV-105 — a clean app is never flushed, so `flush_if_due` touches no
    /// file and leaves the app clean. (A dirty app is deliberately not tested
    /// here: a due flush writes to the real platform data directory —
    /// ADR 0002 §A4 rule 2.)
    #[test]
    fn flush_is_a_noop_while_clean() {
        let mut app = App::default();
        assert!(app.dirty_since.is_none());
        flush_if_due(&mut app);
        assert!(app.dirty_since.is_none());
    }
}
