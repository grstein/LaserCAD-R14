//! TrimTool — click to remove a segment at every intersection with other entities.
//!
//! Single-click: pick the nearest Line, Circle or Arc, fold every cutter that
//! has a cut point on it ([`cut_points`], [`trim_step`]) over a copy, and
//! commit only the steps that change it: nothing, one bare [`TrimEntity`], or
//! one [`CompositeCommand`] labelled `Trim` — one undo step per click
//! (LCV-160 AC 9). Stateless — remains active after each trim.
//!
//! MUST NOT import `eframe` or `rfd`. `egui` is allowed (for [`egui::Key`]).
//! Introduced by demand LCV-050.

use crate::app::App;
use crate::document::commands::trim::{cut_points, trim_step};
use crate::document::commands::{Command, CompositeCommand};
use crate::document::{Document, Entity, History, TrimEntity};
use crate::geometry::{Arc, Vec2};
use crate::tools::{PICK_APERTURE_PT, Tool};

/// Stateless trim tool: single-click removes the clicked segment at every
/// real (segment-level) intersection with all other entities in the document.
#[derive(Debug)]
pub struct TrimTool {
    /// Live zoom in mm per screen point (LCV-162); `1.0` until forwarded.
    mm_per_pt: f64,
}

impl Default for TrimTool {
    fn default() -> Self {
        Self { mm_per_pt: 1.0 }
    }
}

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
            };
            (i, d)
        })
        .filter(|(_, d)| *d <= radius_mm)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

/// The trim steps of one click: each cutter that changes the target, folded
/// in document order over a copy. A Circle target gets a second pass, since
/// once it is an Arc a cutter with a single cut point can trim it too.
fn trim_steps(doc: &Document, target_idx: usize, pos: Vec2) -> Vec<Box<dyn Command>> {
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
    let mut steps: Vec<Box<dyn Command>> = Vec::new();
    for &cutter_idx in cutters.iter().cycle().take(cutters.len() * passes) {
        match trim_step(&current, &doc.entities[cutter_idx], pos) {
            Some(next) if next != current => {
                current = next;
                steps.push(Box::new(TrimEntity::new(target_idx, cutter_idx, pos)));
            }
            _ => {}
        }
    }
    steps
}

impl Tool for TrimTool {
    fn name(&self) -> &'static str {
        "TRIM"
    }

    fn status_text(&self) -> &'static str {
        "TRIM: Click on a segment to trim"
    }

    /// Pick the nearest entity and trim it at every cutter, as one undo step.
    /// Silent no-op (no undo entry) when the click misses or nothing changes.
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
        let mut steps = trim_steps(doc, target_idx, pos);
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

    fn cancel(&mut self) {}

    /// TRIM always waits for an entity pick (LCV-162 AC 5).
    fn wants_entity_pick(&self) -> bool {
        true
    }

    fn set_pick_scale(&mut self, mm_per_pt: f64) {
        self.mm_per_pt = mm_per_pt;
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
