//! The autosave dirty signal, debounce and flush (LCV-059 / LCV-102,
//! ADR 0002 §B).
//!
//! Two halves, both here since LCV-111 moved the signal out of `mod.rs`:
//! the *signal* — [`App::sync_dirty`](super::App::sync_dirty), the only writer
//! of `dirty_since = Some(_)`, and [`App::mark_clean`](super::App::mark_clean),
//! the only writer of `dirty_since = None` — and the *timing*: how long a
//! change waits before it is written, and the once-per-frame flush check.
//!
//! LCV-116 adds the third piece the timing half was missing:
//! [`schedule_flush_repaint`]. `flush_if_due` only runs inside a frame, and
//! egui stops repainting an idle app, so before that demand a document could
//! sit dirty and never be written at all. One
//! `Context::request_repaint_after` *while a write is pending* closes it. It
//! is deliberately conditional: a blanket per-frame repaint would also make
//! autosave fire, and would burn a core for the life of the process.
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
        // Two statements, not one: `record_autosave_outcome(app, app.write_autosave())`
        // borrows `app` mutably and immutably in the same expression.
        let wrote = app.write_autosave();
        record_autosave_outcome(app, wrote);
    }
}

/// Apply the outcome of one autosave write attempt (LCV-116 AC 7).
///
/// Split out of [`flush_if_due`] so both branches are reachable from a unit
/// test without needing a real write at all. Since LCV-119 / ADR 0006 there
/// *is* a path-injected seam underneath —
/// [`App::write_autosave`](super::App::write_autosave) writes to the autosave
/// location this process was given at boot, and is a no-op returning `false`
/// when it was given none — so a test may also drive the real write against a
/// temporary directory it owns. `flush_if_due` is the only caller and passes the real
/// `app.write_autosave()` result; a bounded source scan in this file's tests
/// pins that, so the seam cannot be rewired to a hard-coded `true`.
///
/// - `wrote == true` → stamp `last_autosave_at`. This is the only writer of
///   that field in the tree.
/// - `autosave_failed` = `!wrote` when this process has an `autosave_path`
///   (LCV-167 AC 9): a pathless process never writes, so it never "fails".
/// - Either way → [`App::mark_clean`]. Unconditional on purpose (LCV-102
///   AC 18): a failed write is dropped, not retried every frame, or the
///   debounce would spin. The next document change re-arms it.
fn record_autosave_outcome(app: &mut App, wrote: bool) {
    if wrote {
        app.last_autosave_at = Some(Instant::now());
    }
    app.autosave_failed = !wrote && app.autosave_path.is_some();
    app.mark_clean();
}

/// Ask egui for one more frame while an autosave write is still pending
/// (LCV-116 AC 9).
///
/// Called from [`App::update_ui`](super::App::update_ui) immediately *after*
/// [`flush_if_due`], so a write that just landed leaves `dirty_since` at
/// `None` and schedules nothing. A clean app therefore requests no repaint
/// here at all and egui is free to go idle — which is the whole point, and is
/// what the LCV-116 integration test asserts alongside the dirty case.
///
/// The delay is the debounce's remainder, so the follow-up frame arrives just
/// as the write becomes due rather than immediately.
pub fn schedule_flush_repaint(ctx: &egui::Context, app: &App) {
    if let Some(since) = app.dirty_since {
        ctx.request_repaint_after(AUTOSAVE_DEBOUNCE.saturating_sub(since.elapsed()));
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
    /// file and leaves the app clean.
    #[test]
    fn flush_is_a_noop_while_clean() {
        let mut app = App::default();
        assert!(app.dirty_since.is_none());
        flush_if_due(&mut app);
        assert!(app.dirty_since.is_none());
    }

    /// LCV-119 — the case ADR 0002 §A4 rule 2 used to forbid outright: a
    /// **due** flush on an `App::default()`. It writes nothing, because
    /// `autosave_path` is `None`, and still clears the debounce so the check
    /// cannot spin once per frame. Before LCV-119 this test could not exist —
    /// it would have written the developer's real data directory.
    #[test]
    fn a_due_flush_with_no_injected_path_writes_nothing_and_still_settles() {
        let mut app = App {
            dirty_since: Instant::now().checked_sub(Duration::from_secs(5)),
            ..App::default()
        };
        assert!(autosave_due(app.dirty_since, Instant::now()));

        flush_if_due(&mut app);

        assert!(
            app.last_autosave_at.is_none(),
            "a pathless App must not claim a save it did not make"
        );
        assert!(app.dirty_since.is_none(), "but the debounce is cleared");
    }

    /// LCV-116 AC 7 — a successful write stamps `last_autosave_at` and clears
    /// the debounce.
    #[test]
    fn last_autosave_at_is_set_only_on_success() {
        let mut app = App {
            dirty_since: Some(Instant::now()),
            ..App::default()
        };
        assert!(app.last_autosave_at.is_none());

        record_autosave_outcome(&mut app, true);

        assert!(
            app.last_autosave_at.is_some(),
            "an Ok write must stamp the timestamp"
        );
        assert!(app.dirty_since.is_none(), "and clear the debounce");
    }

    /// LCV-116 AC 7 / LCV-102 AC 18 — a **failed** write stamps nothing but
    /// still clears the debounce, so the flush cannot spin once per frame.
    #[test]
    fn a_failed_write_clears_the_debounce_without_claiming_a_save() {
        let mut app = App {
            dirty_since: Some(Instant::now()),
            ..App::default()
        };

        record_autosave_outcome(&mut app, false);

        assert!(
            app.last_autosave_at.is_none(),
            "a failed write must not claim a save the operator does not have"
        );
        assert!(
            app.dirty_since.is_none(),
            "but the debounce is still cleared (LCV-102 AC 18)"
        );
    }

    /// LCV-167 AC 9 — a failed write with an autosave path marks the badge
    /// failed; the next successful write clears it.
    #[test]
    fn a_failed_write_with_a_path_is_flagged_until_a_success() {
        let mut app = App {
            autosave_path: Some(std::path::PathBuf::from("autosave.json")),
            ..App::default()
        };
        assert!(!app.autosave_failed);
        record_autosave_outcome(&mut app, false);
        assert!(app.autosave_failed, "a real write failed");
        record_autosave_outcome(&mut app, true);
        assert!(!app.autosave_failed, "a success clears it");
    }

    /// LCV-167 AC 9 — a process that persists nothing is never "failed".
    #[test]
    fn a_pathless_flush_is_never_flagged_failed() {
        let mut app = App::default();
        record_autosave_outcome(&mut app, false);
        assert!(!app.autosave_failed);
    }

    /// LCV-116 AC 7 — a second successful write moves the timestamp forward,
    /// so the indicator reflects the latest write and not the first one.
    #[test]
    fn a_later_success_moves_the_timestamp_forward() {
        let mut app = App::default();
        record_autosave_outcome(&mut app, true);
        let first = app.last_autosave_at.expect("stamped");
        std::thread::sleep(Duration::from_millis(2));
        record_autosave_outcome(&mut app, true);
        assert!(app.last_autosave_at.expect("stamped") > first);
    }

    /// LCV-116 AC 7 — `App::default()` has never autosaved.
    #[test]
    fn default_app_has_no_autosave_timestamp() {
        assert!(App::default().last_autosave_at.is_none());
    }

    /// LCV-116 AC 7 — the seam `record_autosave_outcome` exists for
    /// testability must still be fed by the **real** write result. Bounded to
    /// `fn flush_if_due`, so this test's own body cannot satisfy the scan.
    #[test]
    fn flush_feeds_the_real_write_result_into_the_outcome() {
        let body = flush_if_due_body();
        assert!(
            body.contains(concat!("let wrote = app.", "write_autosave();")),
            "flush_if_due must pass the real write result, not a literal"
        );
        assert!(
            body.contains(concat!("record_autosave_outcome(app, ", "wrote);")),
            "positive control: the real result reaches the outcome helper"
        );
        for literal in [
            concat!("record_autosave_outcome(app, ", "true)"),
            concat!("record_autosave_outcome(app, ", "false)"),
            concat!("let _ = app.", "write_autosave()"),
        ] {
            assert!(!body.contains(literal), "flush_if_due must not {literal}");
        }
    }

    /// LCV-116 AC 7 — exactly one writer of `last_autosave_at` in the whole
    /// implementation half of this file, and it is the `Ok` branch.
    #[test]
    fn the_timestamp_has_a_single_writer() {
        let implementation = implementation_source();
        let writes: Vec<_> = implementation
            .match_indices("last_autosave_at = ")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(writes.len(), 1, "exactly one writer of last_autosave_at");
        let guard = implementation
            .find("if wrote {")
            .expect("positive control: the Ok branch guard");
        assert!(guard < writes[0], "the write sits behind the Ok branch");
        assert!(
            implementation.contains("app.mark_clean();"),
            "positive control: mark_clean is still called"
        );
    }

    /// LCV-116 AC 9 — the repaint request is conditional on a pending write.
    /// A blanket `ctx.request_repaint()` would make the autosave fire too and
    /// would burn a core forever; this scan is what rejects it.
    #[test]
    fn the_repaint_request_is_conditional() {
        let implementation = implementation_source();
        let start = implementation
            .find("pub fn schedule_flush_repaint(")
            .expect("schedule_flush_repaint must exist");
        let body = &implementation[start..];
        assert!(
            body.contains("if let Some(since) = app.dirty_since {"),
            "the repaint must be guarded on a pending write"
        );
        assert!(
            body.contains("ctx.request_repaint_after("),
            "positive control: the scheduled call is present"
        );
        assert!(
            !implementation.contains("ctx.request_repaint()"),
            "no unconditional per-frame repaint may live in this module"
        );
    }

    /// The implementation half of this file — everything before the bare
    /// `#[cfg(test)]` anchor. Bounding every scan to it is what stops a test
    /// body from satisfying its own assertion (a defect this project has hit
    /// four times).
    fn implementation_source() -> &'static str {
        let src = include_str!("autosave.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("autosave.rs must have a bare #[cfg(test)] anchor");
        &src[..cfg_test_at]
    }

    /// The source text of `fn flush_if_due`, up to the next item.
    fn flush_if_due_body() -> &'static str {
        let implementation = implementation_source();
        let start = implementation
            .find("pub fn flush_if_due(")
            .expect("flush_if_due must exist");
        let end = implementation[start..]
            .find("\n/// Apply the outcome")
            .expect("flush_if_due is followed by record_autosave_outcome")
            + start;
        &implementation[start..end]
    }
}
