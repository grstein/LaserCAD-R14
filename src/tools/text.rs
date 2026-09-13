//! TextTool — AutoCAD R14 `DTEXT`. Click the insertion point, type the
//! string into the command line, type the height (or accept the 5 mm
//! default), commit as one [`CreateEntities`] carrying every Hershey stroke.
//! Uses `layout_text` (LCV-055). Never mutates the document directly.
//!
//! Raw-input state machine (ADR 0003 §D, LCV-112):
//! `Idle -> WaitingText -> WaitingHeight -> Idle`. While not `Idle`,
//! [`Tool::wants_raw_input`] is true and the command line becomes a plain
//! text field instead of being parsed — see `src/app/cmdline.rs::submit`
//! and `src/ui/command_line.rs::draw_command_line`. A tool in this mode
//! never sees per-character events; it sees exactly one submitted string
//! per phase, via [`Tool::on_raw_input`].
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-048;
//! rebuilt on raw command-line input by LCV-112.

use crate::app::App;
use crate::cmdline::parse_number;
use crate::document::{commands::CreateEntities, Document, Entity, History};
use crate::geometry::Vec2;
use crate::text::layout_text;
use crate::tools::Tool;

/// Height committed when the operator accepts the default at the height
/// prompt (empty Enter) — the R14 `DTEXT` default. Also the height used by
/// the `WaitingHeight` placement preview (AC 9): TEXT does not remember a
/// previous height (§Out of scope), so the preview always shows this value.
const DEFAULT_TEXT_HEIGHT_MM: f64 = 5.0;
/// Hershey stroke spacing; TEXT never varies it (§Out of scope).
const DEFAULT_SPACING_FACTOR: f64 = 1.0;
/// Minimum accepted height, in mm — roughly one laser kerf. Below it,
/// adjacent Hershey strokes fuse into an unreadable scorch.
const MIN_HEIGHT_MM: f64 = 0.1;
/// Maximum accepted height, in mm — `BED_MAX_MM` (LCV-114). Text taller than
/// the largest possible bed cannot be cut and would wreck zoom-extents.
const MAX_HEIGHT_MM: f64 = 2000.0;

#[derive(Debug)]
enum TextToolState {
    Idle,
    WaitingText {
        anchor: Vec2,
    },
    WaitingHeight {
        anchor: Vec2,
        text: String,
        /// Set when the previous height submission was refused, so
        /// `status_text` can show the retry prompt (AC 6) without carrying a
        /// runtime-interpolated `String` (`status_text` stays `&'static str`;
        /// TEXT never remembers a height, so a second literal is all that's
        /// needed).
        invalid: bool,
    },
}

/// Click-anchor, type-string, type-height text tool (LCV-048, rebuilt by
/// LCV-112 on raw command-line input).
///
/// `Idle` — click sets the anchor — `WaitingText`. The typed string, once
/// submitted with Enter, either cancels back to `Idle` (blank/whitespace —
/// decision 5) or advances to `WaitingHeight`. The typed height, once
/// submitted, commits **exactly one** [`CreateEntities`] carrying every
/// glyph stroke and returns to `Idle` (empty height commits at the 5 mm
/// default; an out-of-range or unparseable height re-prompts and keeps the
/// text — decisions 2–4). `Escape` / [`Tool::cancel`] returns to `Idle`
/// uncommitted from any state.
#[derive(Debug)]
pub struct TextTool {
    state: TextToolState,
}

impl Default for TextTool {
    fn default() -> Self {
        Self {
            state: TextToolState::Idle,
        }
    }
}

impl Tool for TextTool {
    fn name(&self) -> &'static str {
        "TEXT"
    }

    /// The AC 6 prompt table, pulled fresh every frame.
    fn status_text(&self) -> &'static str {
        match &self.state {
            TextToolState::Idle => "TEXT Specify start point:",
            TextToolState::WaitingText { .. } => "TEXT Enter text:",
            TextToolState::WaitingHeight { invalid: false, .. } => "TEXT Specify height <5>:",
            TextToolState::WaitingHeight { invalid: true, .. } => {
                "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:"
            }
        }
    }

    /// True in both waiting states: the command line is a free-text field,
    /// not the parser, until the height is accepted or the tool cancels.
    fn wants_raw_input(&self) -> bool {
        !matches!(self.state, TextToolState::Idle)
    }

    /// First click transitions `Idle -> WaitingText`. A click while already
    /// waiting is a no-op: the anchor does not move and nothing commits.
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
        if let TextToolState::Idle = self.state {
            self.state = TextToolState::WaitingText { anchor: pos };
        }
    }

    fn on_pointer_move(&mut self, _pos: Vec2, _doc: &mut Document) {}

    fn on_pointer_up(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
    }

    /// Escape is the only key `TextTool` still reads directly — every other
    /// keystroke while waiting is text for the command line, handled through
    /// [`Tool::on_raw_input`] instead. `_app` is unused; the parameter stays
    /// because [`Tool::on_key`]'s signature is shared by every tool.
    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    /// The submitted text, unparsed (ADR 0003 §D, AC 5). Returns `false`
    /// only from `Idle`, where raw input is never requested in the first
    /// place, so the app never treats this tool's own text as unhandled.
    fn on_raw_input(&mut self, raw: &str, doc: &mut Document, history: &mut History) -> bool {
        match std::mem::replace(&mut self.state, TextToolState::Idle) {
            TextToolState::Idle => false,
            TextToolState::WaitingText { anchor } => {
                // Decision 5: an empty (or whitespace-only) string cancels —
                // committing zero glyphs would put an empty undo entry on
                // the stack. `self.state` is already `Idle` from the
                // `mem::replace` above.
                if !raw.trim().is_empty() {
                    self.state = TextToolState::WaitingHeight {
                        anchor,
                        text: raw.to_owned(),
                        invalid: false,
                    };
                }
                true
            }
            TextToolState::WaitingHeight { anchor, text, .. } => {
                match resolve_height(raw) {
                    Some(height_mm) => commit(&text, anchor, height_mm, doc, history),
                    None => {
                        self.state = TextToolState::WaitingHeight {
                            anchor,
                            text,
                            invalid: true,
                        };
                    }
                }
                true
            }
        }
    }

    /// `Idle` and `WaitingText` preview nothing: the string lives in the
    /// command line's `TextEdit` and this tool does not see it until Enter
    /// (§Risks). `WaitingHeight` previews the placement at the 5 mm default
    /// while the operator decides the real height (decision 4, AC 9).
    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            TextToolState::WaitingHeight { anchor, text, .. } => layout_text(
                text,
                *anchor,
                DEFAULT_TEXT_HEIGHT_MM,
                DEFAULT_SPACING_FACTOR,
            ),
            _ => vec![],
        }
    }

    fn cancel(&mut self) {
        self.state = TextToolState::Idle;
    }
}

/// Resolve the height prompt's submitted text: `""` (or whitespace-only)
/// means the 5 mm default (decision 4); otherwise the value must parse
/// through [`parse_number`] — the product's one number grammar, so `10,5`
/// is a coordinate pair, not a height (decision 7) — and fall within
/// `[0.1, 2000.0]` inclusive (decision 2). Never clamps (decision 3): a
/// `None` here is what makes the caller stay in `WaitingHeight` and re-ask.
fn resolve_height(raw: &str) -> Option<f64> {
    if raw.trim().is_empty() {
        return Some(DEFAULT_TEXT_HEIGHT_MM);
    }
    parse_number(raw).filter(|h| (MIN_HEIGHT_MM..=MAX_HEIGHT_MM).contains(h))
}

/// Lay out `text` at `height_mm` and commit it as one [`CreateEntities`]
/// (decision 6 — exactly one undo entry for the whole string, via
/// `history.commit`, never `App::commit`). A glyphless result (e.g. a string
/// with no Hershey strokes at all) commits nothing (AC 7).
fn commit(text: &str, anchor: Vec2, height_mm: f64, doc: &mut Document, history: &mut History) {
    let strokes = layout_text(text, anchor, height_mm, DEFAULT_SPACING_FACTOR);
    if !strokes.is_empty() {
        history.commit(Box::new(CreateEntities::new(strokes)), doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::EPSILON;

    fn idle() -> TextTool {
        TextTool::default()
    }

    fn make() -> (TextTool, Document, History) {
        (idle(), Document::default(), History::default())
    }

    fn anchored() -> (TextTool, Document, History) {
        let (mut t, mut d, mut h) = make();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        (t, d, h)
    }

    // ── AC 4, 5 — the state machine ─────────────────────────────────────

    #[test]
    fn idle_wants_no_raw_input_then_both_waiting_states_do() {
        let mut t = idle();
        assert!(!t.wants_raw_input());

        let mut d = Document::default();
        let mut h = History::default();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        assert!(t.wants_raw_input(), "WaitingText wants raw input");

        t.on_raw_input("HELLO", &mut d, &mut h);
        assert!(t.wants_raw_input(), "WaitingHeight wants raw input");
    }

    #[test]
    fn click_anchors_and_a_second_click_does_not_move_it() {
        let (mut t, mut d, mut h) = anchored();
        t.on_pointer_down(Vec2::new(99.0, 99.0), false, &mut d, &mut h);
        t.on_raw_input("HELLO", &mut d, &mut h);
        // The anchor is whatever the *first* click set: (0, 0), not (99, 99).
        let strokes = commit_and_get(&mut t, &mut d, &mut h, "");
        for e in &strokes {
            if let Entity::Line(l) = e {
                assert!(l.p1.x < 50.0 && l.p2.x < 50.0, "anchor must not have moved");
            }
        }
    }

    #[test]
    fn non_empty_text_advances_to_the_height_prompt() {
        let (mut t, mut d, mut h) = anchored();
        assert!(t.on_raw_input("HELLO", &mut d, &mut h));
        assert_eq!(t.status_text(), "TEXT Specify height <5>:");
        assert_eq!(d.entity_count(), 0, "the string alone commits nothing");
    }

    #[test]
    fn empty_text_cancels_and_commits_nothing() {
        let (mut t, mut d, mut h) = anchored();
        assert!(t.on_raw_input("", &mut d, &mut h));
        assert_eq!(t.status_text(), "TEXT Specify start point:");
        assert_eq!(d.entity_count(), 0);
        assert_eq!(h.len(), 0);
    }

    #[test]
    fn whitespace_only_text_cancels_and_commits_nothing() {
        let (mut t, mut d, mut h) = anchored();
        assert!(t.on_raw_input("   ", &mut d, &mut h));
        assert_eq!(t.status_text(), "TEXT Specify start point:");
        assert_eq!(d.entity_count(), 0);
    }

    #[test]
    fn escape_from_either_waiting_state_returns_to_idle_uncommitted() {
        let (mut t, d, _h) = anchored();
        t.cancel();
        assert_eq!(t.status_text(), "TEXT Specify start point:");
        assert_eq!(d.entity_count(), 0);

        let (mut t2, mut d2, mut h2) = anchored();
        t2.on_raw_input("HELLO", &mut d2, &mut h2);
        t2.cancel();
        assert_eq!(t2.status_text(), "TEXT Specify start point:");
        assert_eq!(d2.entity_count(), 0);
        assert_eq!(h2.len(), 0);
    }

    #[test]
    fn on_raw_input_returns_false_only_when_idle() {
        let (mut t, mut d, mut h) = make();
        assert!(!t.on_raw_input("anything", &mut d, &mut h));

        let (mut t2, mut d2, mut h2) = anchored();
        assert!(t2.on_raw_input("HELLO", &mut d2, &mut h2));
        assert!(
            t2.on_raw_input("abc", &mut d2, &mut h2),
            "invalid height still returns true"
        );
    }

    // ── AC 5, 7, 8, decisions 2-4, 7 — the height ────────────────────────

    /// Drive `t` from `WaitingHeight` (after typing `HELLO`) with a single
    /// height submission and return the entities the document now holds.
    fn commit_and_get(
        t: &mut TextTool,
        d: &mut Document,
        h: &mut History,
        height_raw: &str,
    ) -> Vec<Entity> {
        t.on_raw_input(height_raw, d, h);
        d.entities.clone()
    }

    #[test]
    fn empty_height_commits_at_five_millimetres() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input("", &mut d, &mut h);
        assert_eq!(h.len(), 1);
        let max_y = d
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Line(l) => Some(l.p1.y.max(l.p2.y)),
                _ => None,
            })
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max_y - 5.0).abs() < EPSILON);
        assert_eq!(t.status_text(), "TEXT Specify start point:");
    }

    #[test]
    fn valid_height_commits_at_that_height() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input("12.5", &mut d, &mut h);
        assert_eq!(h.len(), 1);
        let max_y = d
            .entities
            .iter()
            .filter_map(|e| match e {
                Entity::Line(l) => Some(l.p1.y.max(l.p2.y)),
                _ => None,
            })
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max_y - 12.5).abs() < EPSILON);
    }

    #[test]
    fn height_below_the_floor_is_refused() {
        for bad in ["0", "0.05", "-3"] {
            let (mut t, mut d, mut h) = anchored();
            t.on_raw_input("H", &mut d, &mut h);
            t.on_raw_input(bad, &mut d, &mut h);
            assert_eq!(d.entity_count(), 0, "{bad} must not commit");
            assert_eq!(h.len(), 0);
            assert!(matches!(
                t.state,
                TextToolState::WaitingHeight { invalid: true, .. }
            ));
        }
    }

    #[test]
    fn height_above_the_ceiling_is_refused() {
        for bad in ["2000.1", "1e9"] {
            let (mut t, mut d, mut h) = anchored();
            t.on_raw_input("H", &mut d, &mut h);
            t.on_raw_input(bad, &mut d, &mut h);
            assert_eq!(d.entity_count(), 0, "{bad} must not commit");
            assert_eq!(h.len(), 0);
        }
    }

    #[test]
    fn height_boundaries_are_inclusive() {
        for good in ["0.1", "2000"] {
            let (mut t, mut d, mut h) = anchored();
            t.on_raw_input("H", &mut d, &mut h);
            t.on_raw_input(good, &mut d, &mut h);
            assert_eq!(h.len(), 1, "{good} must commit");
            assert!(d.entity_count() > 0, "{good} must produce strokes");
        }
    }

    #[test]
    fn unparseable_height_is_refused() {
        for bad in ["abc", "10mm", "nan", "inf"] {
            let (mut t, mut d, mut h) = anchored();
            t.on_raw_input("H", &mut d, &mut h);
            t.on_raw_input(bad, &mut d, &mut h);
            assert_eq!(d.entity_count(), 0, "{bad} must not commit");
            assert_eq!(h.len(), 0);
        }
    }

    /// Decision 7 / AC 5 — the comma is the coordinate separator everywhere
    /// in the product; it is never a decimal point in a height.
    #[test]
    fn comma_is_not_a_decimal_point_in_a_height() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input("10,5", &mut d, &mut h);
        assert_eq!(d.entity_count(), 0);
        assert_eq!(h.len(), 0);
        assert!(matches!(
            t.state,
            TextToolState::WaitingHeight { invalid: true, .. }
        ));
    }

    #[test]
    fn a_refused_height_keeps_the_text_and_sets_the_retry_prompt() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("HELLO", &mut d, &mut h);
        t.on_raw_input("abc", &mut d, &mut h);
        assert_eq!(
            t.status_text(),
            "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:"
        );
        match &t.state {
            TextToolState::WaitingHeight { text, .. } => assert_eq!(text, "HELLO"),
            other => panic!("expected WaitingHeight, got {other:?}"),
        }
        assert_eq!(d.entity_count(), 0);
    }

    #[test]
    fn a_refused_height_then_a_valid_one_commits_normally() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("HELLO", &mut d, &mut h);
        t.on_raw_input("abc", &mut d, &mut h);
        assert_eq!(d.entity_count(), 0);
        t.on_raw_input("10", &mut d, &mut h);
        assert_eq!(h.len(), 1);
        assert!(d.entity_count() > 0);
        assert_eq!(t.status_text(), "TEXT Specify start point:");
    }

    /// AC 8 — `'H'` spans exactly `hy ∈ [-9, 0]` (baseline to cap), so at a
    /// 10 mm height its top sits at exactly `y = 10.0`. Deliberately *not*
    /// `'O'`: `'O'` spans `hy ∈ [-9, +9]` and descends below the baseline, so
    /// its y-extent is twice the cap height — a wrong scale would silently
    /// pass with that oracle instead of failing.
    #[test]
    fn ten_millimetre_h_spans_exactly_ten_millimetres() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        t.on_raw_input("10", &mut d, &mut h);
        assert_eq!(h.len(), 1);

        let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
        for e in &d.entities {
            if let Entity::Line(l) = e {
                min_y = min_y.min(l.p1.y).min(l.p2.y);
                max_y = max_y.max(l.p1.y).max(l.p2.y);
            }
        }
        assert!((min_y - 0.0).abs() < 1e-9, "baseline at y=0, got {min_y}");
        assert!((max_y - 10.0).abs() < 1e-9, "cap top at y=10, got {max_y}");
    }

    #[test]
    fn commit_is_a_single_command() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("HELLO", &mut d, &mut h);
        t.on_raw_input("10", &mut d, &mut h);
        assert_eq!(h.len(), 1);
        assert!(d.entity_count() > 0);
    }

    #[test]
    fn layout_with_no_glyphs_commits_nothing() {
        // A run of unsupported codepoints produces no Hershey strokes at all.
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("\u{00e9}\u{00e8}", &mut d, &mut h);
        t.on_raw_input("10", &mut d, &mut h);
        assert_eq!(d.entity_count(), 0, "a glyphless string commits nothing");
        assert_eq!(h.len(), 0);
        assert_eq!(
            t.status_text(),
            "TEXT Specify start point:",
            "state still returns to Idle"
        );
    }

    // ── AC 6 — prompts ───────────────────────────────────────────────────

    #[test]
    fn prompt_matches_each_state() {
        assert_eq!(idle().status_text(), "TEXT Specify start point:");

        let (mut t, mut d, mut h) = anchored();
        assert_eq!(t.status_text(), "TEXT Enter text:");

        t.on_raw_input("HELLO", &mut d, &mut h);
        assert_eq!(t.status_text(), "TEXT Specify height <5>:");

        t.on_raw_input("abc", &mut d, &mut h);
        assert_eq!(
            t.status_text(),
            "TEXT Height must be between 0.1 and 2000 mm. Specify height <5>:"
        );
    }

    // ── AC 9 — preview ───────────────────────────────────────────────────

    #[test]
    fn preview_is_empty_until_the_height_prompt() {
        let mut t = idle();
        assert!(t.preview().is_empty());

        let mut d = Document::default();
        let mut h = History::default();
        t.on_pointer_down(Vec2::new(0.0, 0.0), false, &mut d, &mut h);
        assert!(
            t.preview().is_empty(),
            "WaitingText previews nothing (AC 9)"
        );
    }

    #[test]
    fn preview_at_the_height_prompt_uses_the_default_height() {
        let (mut t, mut d, mut h) = anchored();
        t.on_raw_input("H", &mut d, &mut h);
        let pv = t.preview();
        assert!(!pv.is_empty());
        let max_y = pv
            .iter()
            .filter_map(|e| match e {
                Entity::Line(l) => Some(l.p1.y.max(l.p2.y)),
                _ => None,
            })
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (max_y - DEFAULT_TEXT_HEIGHT_MM).abs() < EPSILON,
            "the WaitingHeight preview always uses the 5 mm default"
        );
    }

    // ── object safety, name ──────────────────────────────────────────────

    #[test]
    fn text_tool_default_is_idle_and_object_safe() {
        let t = idle();
        assert_eq!(t.name(), "TEXT");
        assert!(t.preview().is_empty());
        let _: Box<dyn Tool> = Box::new(idle());
    }

    #[test]
    fn pointer_move_and_up_are_noops() {
        let (mut t, mut d, mut h) = anchored();
        t.on_pointer_move(Vec2::new(100.0, 200.0), &mut d);
        t.on_pointer_up(Vec2::new(10.0, 10.0), false, &mut d, &mut h);
        assert_eq!(
            t.status_text(),
            "TEXT Enter text:",
            "neither call advances the phase"
        );
    }
}
