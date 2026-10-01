//! TrimTool — click to remove a segment at every intersection with other entities.
//!
//! Single-click: pick the nearest Line, Circle or Arc, fold every cutter that
//! has a cut point on it ([`cut_points`], [`trim_step`]) over a copy, and
//! commit only the steps that change it: nothing, one bare [`TrimEntity`], or
//! one [`CompositeCommand`] labelled `Trim` — one undo step per click
//! (LCV-160 AC 9). Stateless — remains active after each trim. The hover
//! feedback runs the same fold and paints what the click would remove
//! (LCV-163).
//!
//! MUST NOT import `eframe` or `rfd`. `egui` is allowed (for [`egui::Key`]).
//! Introduced by demand LCV-050.

use crate::app::App;
use crate::document::commands::trim::removed::removed_pieces;
use crate::document::commands::trim::{cut_points, trim_step};
use crate::document::commands::{Command, CompositeCommand};
use crate::document::{Document, Entity, History, TrimEntity};
use crate::geometry::{Arc, Vec2};
use crate::tools::{Mark, PICK_APERTURE_PT, Tool};

/// Stateless trim tool: single-click removes the clicked segment at every
/// real (segment-level) intersection with all other entities in the document.
#[derive(Debug)]
pub struct TrimTool {
    /// Live zoom in mm per screen point (LCV-162); `1.0` until forwarded.
    mm_per_pt: f64,
    /// Single-shot result line for [`Tool::take_message`].
    message: Option<String>,
}

impl Default for TrimTool {
    fn default() -> Self {
        Self {
            mm_per_pt: 1.0,
            message: None,
        }
    }
}

/// The result line of a TRIM or EXTEND click aimed at an ellipse, which
/// neither tool edits (ADR 0015 §6).
pub(crate) const ELLIPSE_REFUSAL: &str = "Cannot trim/extend an ellipse";

/// Distance from `p` to the nearest point on the arc's stroke.
fn arc_dist(arc: &Arc, p: Vec2) -> f64 {
    let v = p - arc.center;
    let angle = v.y.atan2(v.x);
    if arc.contains_angle(angle) {
        (v.length() - arc.r).abs()
    } else {
        (p - arc.start_point())
            .length()
            .min((p - arc.end_point()).length())
    }
}

/// Return the index of the entity closest to `pos` within `radius_mm`, or
/// `None` if no entity qualifies.
fn pick_entity(pos: Vec2, entities: &[Entity], radius_mm: f64) -> Option<usize> {
    entities
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let d = match e {
                Entity::Line(l) => l.distance_to_point(pos),
                Entity::Circle(c) => c.distance_to_point(pos).abs(),
                Entity::Arc(a) => arc_dist(a, pos),
                Entity::Ellipse(el) => el.distance_to_point(pos),
                Entity::Bezier(_) => f64::INFINITY,
            };
            (i, d)
        })
        .filter(|(_, d)| *d <= radius_mm)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

/// Fold one click over a copy of the target: the cutters that change it, in
/// the order they apply, and the kept entity. Each cutter is tried in
/// document order; a Circle target gets a second pass, since once it is an
/// Arc a cutter with a single cut point can trim it too. The click and the
/// hover feedback share this fold, so what is painted is what is removed
/// (LCV-163 AC 6).
fn trim_fold(doc: &Document, target_idx: usize, pos: Vec2) -> (Vec<usize>, Entity) {
    let original = doc.entities[target_idx];
    let cutters: Vec<usize> = (0..doc.entities.len())
        .filter(|&i| i != target_idx && !cut_points(&original, &doc.entities[i]).is_empty())
        .collect();
    let passes = if matches!(original, Entity::Circle(_)) {
        2
    } else {
        1
    };
    let mut current = original;
    let mut applied = Vec::new();
    for &cutter_idx in cutters.iter().cycle().take(cutters.len() * passes) {
        match trim_step(&current, &doc.entities[cutter_idx], pos) {
            Some(next) if next != current => {
                current = next;
                applied.push(cutter_idx);
            }
            _ => {}
        }
    }
    (applied, current)
}

impl Tool for TrimTool {
    fn name(&self) -> &'static str {
        "TRIM"
    }

    fn status_text(&self) -> &'static str {
        "TRIM: Click on a segment to trim"
    }

    /// Pick the nearest entity and trim it at every cutter, as one undo step.
    /// Silent no-op (no undo entry) when the click misses or nothing changes;
    /// an ellipse is refused with [`ELLIPSE_REFUSAL`].
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        let radius = PICK_APERTURE_PT * self.mm_per_pt;
        let Some(target_idx) = pick_entity(pos, &doc.entities, radius) else {
            return;
        };
        if matches!(doc.entities[target_idx], Entity::Ellipse(_)) {
            self.message = Some(ELLIPSE_REFUSAL.to_owned());
            return;
        }
        let (cutters, _) = trim_fold(doc, target_idx, pos);
        let mut steps: Vec<Box<dyn Command>> = cutters
            .into_iter()
            .map(|c| Box::new(TrimEntity::new(target_idx, c, pos)) as Box<dyn Command>)
            .collect();
        let command = match steps.len() {
            0 => return,
            1 => steps.remove(0),
            _ => Box::new(CompositeCommand::new(steps, "Trim")),
        };
        history.commit(command, doc);
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

    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if matches!(key, egui::Key::Escape) {
            self.cancel();
        }
    }

    fn preview(&self) -> Vec<Entity> {
        vec![]
    }

    /// LCV-163 AC 3/AC 4: the entity a click would trim, as `Hover`, and
    /// what it would lose, as `Danger` — nothing when `cursor` is `None`.
    fn feedback(&self, doc: &Document, cursor: Option<Vec2>) -> Vec<Mark> {
        let radius = PICK_APERTURE_PT * self.mm_per_pt;
        let Some((pos, target)) =
            cursor.and_then(|c| pick_entity(c, &doc.entities, radius).map(|t| (c, t)))
        else {
            return Vec::new();
        };
        let (_, kept) = trim_fold(doc, target, pos);
        let removed = removed_pieces(&doc.entities[target], &kept);
        std::iter::once(Mark::Hover(target))
            .chain(removed.into_iter().map(Mark::Danger))
            .collect()
    }

    fn cancel(&mut self) {}

    /// TRIM always waits for an entity pick (LCV-162 AC 5).
    fn wants_entity_pick(&self) -> bool {
        true
    }

    fn set_pick_scale(&mut self, mm_per_pt: f64) {
        self.mm_per_pt = mm_per_pt;
    }

    fn take_message(&mut self) -> Option<String> {
        self.message.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, Entity, History};
    use crate::geometry::{Circle, EPSILON, Line, Vec2};

    fn ln(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
        Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
    }
    fn doc_with(entities: Vec<Entity>) -> Document {
        let mut doc = Document::default();
        entities.into_iter().for_each(|e| doc.push_current(e));
        doc
    }
    fn do_trim(doc: &mut Document, hist: &mut History, x: f64, y: f64) {
        TrimTool::default().on_pointer_down(Vec2::new(x, y), false, doc, hist);
    }
    fn line_at(doc: &Document, idx: usize) -> Line {
        match doc.entities[idx] {
            Entity::Line(l) => l,
            ref e => panic!("expected Line at [{idx}], got {e:?}"),
        }
    }
    fn approx(a: Vec2, b: Vec2) -> bool {
        a.approx_eq(b, EPSILON)
    }

    /// AC#1 — `Default` exists; LCV-162 — at the default 1 mm/pt.
    #[test]
    fn trim_tool_struct_constructs() {
        let _b: TrimTool = Default::default();
        assert_eq!(TrimTool::default().mm_per_pt, 1.0);
    }

    /// LCV-162 AC 8 — the pick radius is 5 pt at the live zoom, and TRIM
    /// always waits for an entity pick.
    #[test]
    fn pick_radius_follows_the_pick_scale() {
        for (scale, y, trims) in [(0.05, 0.2, true), (0.05, 0.3, false), (20.0, 80.0, true)] {
            let mut doc = doc_with(vec![ln(0.0, 0.0, 10.0, 0.0), ln(5.0, -500.0, 5.0, 500.0)]);
            let mut hist = History::default();
            let mut tool = TrimTool::default();
            tool.set_pick_scale(scale);
            assert!(tool.wants_entity_pick());
            tool.on_pointer_down(Vec2::new(2.0, y), false, &mut doc, &mut hist);
            assert_eq!(hist.can_undo(), trims, "{scale} mm/pt, {y} mm");
        }
    }

    /// AC#2
    #[test]
    fn name_is_trim() {
        assert_eq!(TrimTool::default().name(), "TRIM");
    }

    /// AC#3
    #[test]
    fn status_text_is_constant() {
        assert_eq!(
            TrimTool::default().status_text(),
            "TRIM: Click on a segment to trim"
        );
    }

    /// AC#4 — miss (> 5 mm) is a no-op.
    #[test]
    fn pointer_down_miss_is_noop() {
        let mut doc = doc_with(vec![ln(0.0, 0.0, 10.0, 0.0)]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 0.0, 20.0);
        assert!(!hist.can_undo());
    }

    /// AC#5 — parallel lines never intersect → no-op.
    #[test]
    fn pointer_down_no_intersect_is_noop() {
        let mut doc = doc_with(vec![ln(0.0, 0.0, 10.0, 0.0), ln(0.0, 5.0, 10.0, 5.0)]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 5.0, 0.1);
        assert!(!hist.can_undo());
    }

    /// AC#6 — keep left of x=5 intersection.
    #[test]
    fn trim_line_line_keep_left() {
        let mut doc = doc_with(vec![ln(0.0, 0.0, 10.0, 0.0), ln(5.0, -5.0, 5.0, 5.0)]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 2.0, 0.0);
        assert!(hist.can_undo());
        let l = line_at(&doc, 0);
        assert!(approx(l.p1, Vec2::new(0.0, 0.0)) && approx(l.p2, Vec2::new(5.0, 0.0)));
        assert_eq!(doc.entities[1], ln(5.0, -5.0, 5.0, 5.0));
    }

    /// AC#7 — keep right of x=5 intersection.
    #[test]
    fn trim_line_line_keep_right() {
        let mut doc = doc_with(vec![ln(0.0, 0.0, 10.0, 0.0), ln(5.0, -5.0, 5.0, 5.0)]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 8.0, 0.0);
        let l = line_at(&doc, 0);
        assert!(approx(l.p1, Vec2::new(5.0, 0.0)) && approx(l.p2, Vec2::new(10.0, 0.0)));
    }

    /// AC#8 — click between two cutters removes the middle segment.
    #[test]
    fn trim_two_cutters_middle_removed() {
        let mut doc = doc_with(vec![
            ln(0.0, 0.0, 20.0, 0.0),
            ln(5.0, -5.0, 5.0, 5.0),
            ln(15.0, -5.0, 15.0, 5.0),
        ]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 10.0, 0.0);
        assert!(hist.can_undo());
        let l = line_at(&doc, 0);
        assert!(approx(l.p1, Vec2::new(5.0, 0.0)) && approx(l.p2, Vec2::new(15.0, 0.0)));
        assert_eq!(doc.entities[1], ln(5.0, -5.0, 5.0, 5.0));
        assert_eq!(doc.entities[2], ln(15.0, -5.0, 15.0, 5.0));
    }

    /// LCV-160 AC 9 (supersedes LCV-050 AC#9) — one click over two cutters
    /// is one undo step: a single undo restores the original line.
    #[test]
    fn trim_two_cutters_is_one_undo_step() {
        let mut doc = doc_with(vec![
            ln(0.0, 0.0, 20.0, 0.0),
            ln(5.0, -5.0, 5.0, 5.0),
            ln(15.0, -5.0, 15.0, 5.0),
        ]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 10.0, 0.0);
        assert_eq!(hist.len(), 1);
        hist.undo(&mut doc);
        assert_eq!(doc.entities[0], ln(0.0, 0.0, 20.0, 0.0));
        assert!(!hist.can_undo());
    }

    /// AC#10 — circle trimmed by crossing line yields an Arc.
    #[test]
    fn trim_circle_line_yields_arc() {
        let mut doc = doc_with(vec![
            Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 5.0)),
            ln(-6.0, 0.0, 6.0, 0.0),
        ]);
        let mut hist = History::default();
        // distance from (0,4) to circle stroke = |4-5| = 1 mm < threshold
        do_trim(&mut doc, &mut hist, 0.0, 4.0);
        assert!(hist.can_undo());
        assert!(matches!(doc.entities[0], Entity::Arc(_)));
        assert_eq!(doc.entities[1], ln(-6.0, 0.0, 6.0, 0.0));
    }

    /// AC#11 — pointer_up is always a no-op.
    #[test]
    fn pointer_up_is_noop() {
        let mut doc = Document::default();
        let mut hist = History::default();
        TrimTool::default().on_pointer_up(Vec2::new(5.0, 5.0), false, &mut doc, &mut hist);
        assert!(!hist.can_undo());
    }

    /// AC#12 — preview is always empty.
    #[test]
    fn preview_always_empty() {
        assert!(TrimTool::default().preview().is_empty());
    }

    /// AC#13 — cancel is a no-op.
    #[test]
    fn cancel_is_noop() {
        TrimTool::default().cancel();
    }

    /// AC#14 — object-safe.
    #[test]
    fn object_safe() {
        let _: Box<dyn Tool> = Box::new(TrimTool::default());
    }

    /// LCV-163 AC 3/AC 4 — the hover marks the target and paints, as
    /// `Danger`, exactly the pieces the click would remove.
    #[test]
    fn feedback_hovers_the_target_and_marks_the_removed_pieces() {
        let doc = doc_with(vec![
            ln(0.0, 0.0, 20.0, 0.0),
            ln(5.0, -5.0, 5.0, 5.0),
            ln(15.0, -5.0, 15.0, 5.0),
        ]);
        let marks = TrimTool::default().feedback(&doc, Some(Vec2::new(10.0, 0.5)));
        assert_eq!(
            marks,
            vec![
                Mark::Hover(0),
                Mark::Danger(ln(0.0, 0.0, 5.0, 0.0)),
                Mark::Danger(ln(15.0, 0.0, 20.0, 0.0)),
            ]
        );
    }

    /// LCV-163 AC 3/AC 9 — nothing without a cursor or off every entity; a
    /// target nothing cuts is hovered with no danger.
    #[test]
    fn feedback_is_empty_off_the_canvas_or_off_every_entity() {
        let doc = doc_with(vec![ln(0.0, 0.0, 10.0, 0.0), ln(0.0, 5.0, 10.0, 5.0)]);
        let tool = TrimTool::default();
        assert!(tool.feedback(&doc, None).is_empty());
        assert!(tool.feedback(&doc, Some(Vec2::new(0.0, 20.0))).is_empty());
        assert_eq!(
            tool.feedback(&doc, Some(Vec2::new(5.0, 0.1))),
            vec![Mark::Hover(0)]
        );
    }

    /// LCV-160 — a Circle target's second pass: a one-point cutter listed
    /// before the two-point one still trims the arc the latter leaves.
    #[test]
    fn trim_circle_second_pass_uses_one_point_cutter() {
        let mut doc = doc_with(vec![
            Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 10.0)),
            ln(0.0, 0.0, 0.0, 20.0),
            ln(-20.0, 0.0, 20.0, 0.0),
        ]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 7.0, 7.1);
        let Entity::Arc(a) = doc.entities[0] else {
            panic!("expected Arc, got {:?}", doc.entities[0]);
        };
        assert!(approx(a.start_point(), Vec2::new(10.0, 0.0)), "{a:?}");
        assert!(approx(a.end_point(), Vec2::new(0.0, 10.0)), "{a:?}");
        assert_eq!(hist.len(), 1);
    }
}
