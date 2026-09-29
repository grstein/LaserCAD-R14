//! TrimTool — click to remove a segment at every intersection with other entities.
//!
//! Single-click: pick nearest entity, find all real intersecting cutters, commit
//! one [`TrimEntity`] per cutter. Stateless — remains active after each trim.
//!
//! MUST NOT import `eframe` or `rfd`. `egui` is allowed (for [`egui::Key`]).
//! Introduced by demand LCV-050.

use crate::app::App;
use crate::document::{Document, Entity, History, TrimEntity};
use crate::geometry::intersect::{circle_circle, line_circle, line_line};
use crate::geometry::{Arc, Vec2};
use crate::tools::Tool;

/// Distance tolerance for point-to-entity picking (mm).
///
/// Independent from `select/hit::PICK_THRESHOLD_MM` — the two constants do not
/// share state so they can diverge independently.
pub const PICK_THRESHOLD_MM: f64 = 5.0;

/// Stateless trim tool: single-click removes the clicked segment at every
/// real (segment-level) intersection with all other entities in the document.
#[derive(Debug, Default)]
pub struct TrimTool;

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

/// Return the index of the entity closest to `pos` within [`PICK_THRESHOLD_MM`],
/// or `None` if no entity qualifies.
fn pick_entity(pos: Vec2, entities: &[Entity]) -> Option<usize> {
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
        .filter(|(_, d)| *d <= PICK_THRESHOLD_MM)
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
}

/// `true` when `a` and `b` share at least one segment-level intersection point.
/// Any pair involving [`Entity::Arc`] returns `false` (out of scope, LCV-050).
fn entities_intersect(a: Entity, b: Entity) -> bool {
    match (a, b) {
        (Entity::Line(l1), Entity::Line(l2)) => line_line(&l1, &l2).is_some(),
        (Entity::Line(l), Entity::Circle(c)) | (Entity::Circle(c), Entity::Line(l)) => {
            !line_circle(&l, &c).is_empty()
        }
        (Entity::Circle(c1), Entity::Circle(c2)) => !circle_circle(&c1, &c2).is_empty(),
        _ => false,
    }
}

impl Tool for TrimTool {
    fn name(&self) -> &'static str {
        "TRIM"
    }

    fn status_text(&self) -> &'static str {
        "TRIM: Click on a segment to trim"
    }

    /// Pick the nearest entity and trim it at every intersecting cutter.
    /// Silent no-op when the click misses or the target has no cutters.
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        doc: &mut Document,
        history: &mut History,
    ) {
        let Some(target_idx) = pick_entity(pos, &doc.entities) else {
            return;
        };
        let target = doc.entities[target_idx];
        let cutters: Vec<usize> = (0..doc.entities.len())
            .filter(|&i| i != target_idx && entities_intersect(target, doc.entities[i]))
            .collect();
        if cutters.is_empty() {
            return;
        }
        for cutter_idx in cutters {
            history.commit(Box::new(TrimEntity::new(target_idx, cutter_idx, pos)), doc);
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

    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if matches!(key, egui::Key::Escape) {
            self.cancel();
        }
    }

    fn preview(&self) -> Vec<Entity> {
        vec![]
    }

    fn cancel(&mut self) {}
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
        TrimTool.on_pointer_down(Vec2::new(x, y), false, doc, hist);
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

    /// AC#1 — Default derive compiles.
    #[test]
    #[expect(
        clippy::default_constructed_unit_structs,
        reason = "AC#1 checks that `Default` exists on the unit struct"
    )]
    fn trim_tool_struct_constructs() {
        let _a = TrimTool::default();
        let _b: TrimTool = Default::default();
    }

    /// AC#2
    #[test]
    fn name_is_trim() {
        assert_eq!(TrimTool.name(), "TRIM");
    }

    /// AC#3
    #[test]
    fn status_text_is_constant() {
        assert_eq!(TrimTool.status_text(), "TRIM: Click on a segment to trim");
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

    /// AC#9 — two undos restore step-by-step.
    #[test]
    fn trim_two_cutters_undo_step_by_step() {
        let mut doc = doc_with(vec![
            ln(0.0, 0.0, 20.0, 0.0),
            ln(5.0, -5.0, 5.0, 5.0),
            ln(15.0, -5.0, 15.0, 5.0),
        ]);
        let mut hist = History::default();
        do_trim(&mut doc, &mut hist, 10.0, 0.0);
        hist.undo(&mut doc);
        let mid = line_at(&doc, 0);
        let at_5 = approx(mid.p1, Vec2::new(5.0, 0.0)) || approx(mid.p2, Vec2::new(5.0, 0.0));
        let at_15 = approx(mid.p1, Vec2::new(15.0, 0.0)) || approx(mid.p2, Vec2::new(15.0, 0.0));
        assert!(at_5 || at_15, "intermediate: {mid:?}");
        hist.undo(&mut doc);
        let l = line_at(&doc, 0);
        assert!(approx(l.p1, Vec2::new(0.0, 0.0)) && approx(l.p2, Vec2::new(20.0, 0.0)));
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
        TrimTool.on_pointer_up(Vec2::new(5.0, 5.0), false, &mut doc, &mut hist);
        assert!(!hist.can_undo());
    }

    /// AC#12 — preview is always empty.
    #[test]
    fn preview_always_empty() {
        assert!(TrimTool.preview().is_empty());
    }

    /// AC#13 — cancel is a no-op.
    #[test]
    fn cancel_is_noop() {
        TrimTool.cancel();
    }

    /// AC#14 — object-safe.
    #[test]
    fn object_safe() {
        let _: Box<dyn Tool> = Box::new(TrimTool);
    }
}
