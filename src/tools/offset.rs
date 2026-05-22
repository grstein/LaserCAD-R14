//! OffsetTool: two-click parallel-offset of Line and Arc entities (LCV-054).
//! First click picks entity (Line/Arc only); second click picks side.
//! Commits via CreateLine or CreateArc. `distance_mm` persists. No `eframe`/`rfd`.

use crate::app::App;
use crate::document::commands::{CreateArc, CreateLine};
use crate::document::{Document, Entity, History};
use crate::geometry::{Arc, Line, Vec2, EPSILON};
use crate::tools::select::hit::{entity_distance_to_point, PICK_THRESHOLD_MM};
use crate::tools::Tool;

#[derive(Copy, Clone, Debug)]
enum OffsetState {
    Idle,
    WaitingSide {
        _entity_index: usize,
        entity: Entity,
        cursor: Vec2,
    },
}

/// Two-click offset tool: parallel line or concentric arc at `distance_mm`.
///
/// `distance_mm` is `pub` so Phase-6 command-line wiring can set it directly.
#[derive(Debug)]
pub struct OffsetTool {
    /// Offset distance in millimeters. Persists across commits. Default: 1.0.
    pub distance_mm: f64,
    state: OffsetState,
}

impl Default for OffsetTool {
    fn default() -> Self {
        Self {
            distance_mm: 1.0,
            state: OffsetState::Idle,
        }
    }
}

fn offset_line(l: &Line, pos: Vec2, dist: f64) -> Option<Entity> {
    if l.length() <= EPSILON {
        return None;
    }
    let signed = (l.p2.x - l.p1.x) * (pos.y - l.p1.y) - (l.p2.y - l.p1.y) * (pos.x - l.p1.x);
    if signed.abs() <= EPSILON {
        return None;
    }
    let dir = l.direction().unwrap(); // safe: degenerate guard above
    let perp = if signed > 0.0 {
        Vec2::new(-dir.y, dir.x)
    } else {
        Vec2::new(dir.y, -dir.x)
    };
    Some(Entity::Line(Line {
        p1: l.p1 + perp * dist,
        p2: l.p2 + perp * dist,
    }))
}

fn offset_arc(a: &Arc, pos: Vec2, dist: f64) -> Option<Entity> {
    let cd = (pos - a.center).length();
    if (cd - a.r).abs() <= EPSILON {
        return None;
    }
    let new_r = if cd > a.r { a.r + dist } else { a.r - dist };
    if new_r <= EPSILON {
        return None;
    }
    Some(Entity::Arc(Arc { r: new_r, ..*a }))
}

fn compute_offset(entity: &Entity, pos: Vec2, dist: f64) -> Option<Entity> {
    match entity {
        Entity::Line(l) => offset_line(l, pos, dist),
        Entity::Arc(a) => offset_arc(a, pos, dist),
        Entity::Circle(_) => None,
    }
}

fn do_commit(result: Entity, doc: &mut Document, history: &mut History) {
    let cmd: Box<dyn crate::document::Command> = match result {
        Entity::Line(l) => Box::new(CreateLine::new(l)),
        Entity::Arc(a) => Box::new(CreateArc::new(a)),
        Entity::Circle(_) => unreachable!(),
    };
    history.commit(cmd, doc);
}

impl Tool for OffsetTool {
    fn name(&self) -> &'static str {
        "OFFSET"
    }

    fn status_text(&self) -> &'static str {
        match self.state {
            OffsetState::Idle => "OFFSET: Click entity to offset",
            OffsetState::WaitingSide { .. } => {
                "OFFSET: Click side to offset toward  |  Esc to cancel"
            }
        }
    }

    fn on_pointer_down(&mut self, pos: Vec2, _: bool, doc: &mut Document, hist: &mut History) {
        match self.state {
            OffsetState::Idle => {
                let maybe = doc
                    .entities
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| !matches!(e, Entity::Circle(_)))
                    .map(|(i, e)| (i, entity_distance_to_point(e, pos)))
                    .filter(|(_, d)| *d <= PICK_THRESHOLD_MM)
                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i);
                if let Some(idx) = maybe {
                    self.state = OffsetState::WaitingSide {
                        _entity_index: idx,
                        entity: doc.entities[idx],
                        cursor: pos,
                    };
                }
            }
            OffsetState::WaitingSide { entity, .. } => {
                if let Some(r) = compute_offset(&entity, pos, self.distance_mm) {
                    do_commit(r, doc, hist);
                    self.state = OffsetState::Idle;
                }
            }
        }
    }

    fn on_pointer_move(&mut self, pos: Vec2, _: &mut Document) {
        if let OffsetState::WaitingSide { ref mut cursor, .. } = self.state {
            *cursor = pos;
        }
    }

    fn on_pointer_up(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}

    fn on_key(&mut self, key: egui::Key, app: &mut App) {
        match key {
            egui::Key::Escape => self.cancel(),
            egui::Key::Enter => {
                if let OffsetState::WaitingSide { entity, cursor, .. } = self.state {
                    if let Some(r) = compute_offset(&entity, cursor, self.distance_mm) {
                        do_commit(r, &mut app.document, &mut app.history);
                        self.state = OffsetState::Idle;
                    }
                }
            }
            _ => {}
        }
    }

    fn preview(&self) -> Vec<Entity> {
        if let OffsetState::WaitingSide { entity, cursor, .. } = self.state {
            compute_offset(&entity, cursor, self.distance_mm)
                .map(|e| vec![e])
                .unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn cancel(&mut self) {
        self.state = OffsetState::Idle;
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*; use crate::document::History; use crate::geometry::Circle; use core::f64::consts::FRAC_PI_2;
    fn make(es: Vec<Entity>) -> (OffsetTool, Document, History) { let mut doc = Document::default(); doc.entities.extend(es); (OffsetTool::default(), doc, History::default()) }
    fn horiz() -> Entity { Entity::Line(Line::new(Vec2::new(0.0,0.0), Vec2::new(10.0,0.0))) }
    fn arc5() -> Entity { Entity::Arc(Arc::new(Vec2::new(0.0,0.0), 5.0, 0.0, FRAC_PI_2, true)) }
    fn arc_r(r: f64) -> Entity { Entity::Arc(Arc::new(Vec2::new(0.0,0.0), r, 0.0, FRAC_PI_2, true)) }
    fn as_line(e: Entity) -> Line { match e { Entity::Line(l) => l, _ => panic!("not line") } }
    fn as_arc(e: Entity) -> Arc { match e { Entity::Arc(a) => a, _ => panic!("not arc") } }
    fn ck(t: &mut OffsetTool, p: Vec2, d: &mut Document, h: &mut History) { t.on_pointer_down(p, false, d, h); }
    fn p(x: f64, y: f64) -> Vec2 { Vec2::new(x, y) }

    #[test] fn offset_name_is_offset() { assert_eq!(OffsetTool::default().name(), "OFFSET"); }
    #[test] fn offset_default_distance_is_1mm() { assert_eq!(OffsetTool::default().distance_mm, 1.0); }
    #[test] fn idle_pick_line_transitions_to_waiting_side() { let (mut t, mut doc, mut h) = make(vec![horiz()]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); assert!(t.status_text().contains("side")); assert!(!h.can_undo()); }
    #[test] fn idle_pick_arc_transitions_to_waiting_side() { let (mut t, mut doc, mut h) = make(vec![arc5()]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); assert!(t.status_text().contains("side")); assert!(!h.can_undo()); }
    #[test] fn idle_miss_stays_idle() { let (mut t, mut doc, mut h) = make(vec![horiz()]); ck(&mut t, p(100.0,100.0), &mut doc, &mut h); assert!(t.preview().is_empty()); assert!(!h.can_undo()); assert!(t.status_text().contains("entity")); }
    #[test] fn idle_circle_ignored() { let (mut t, mut doc, mut h) = make(vec![Entity::Circle(Circle::new(p(0.0,0.0), 5.0))]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); assert!(t.status_text().contains("entity")); }
    #[test] fn side_pick_commits_create_line() { let (mut t, mut doc, mut h) = make(vec![horiz()]); t.distance_mm = 5.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(5.0,3.0), &mut doc, &mut h); assert_eq!(doc.entity_count(), 2); assert!(h.can_undo()); assert!(t.preview().is_empty()); }
    #[test] fn parallel_line_left_side() { let (mut t, mut doc, mut h) = make(vec![horiz()]); t.distance_mm = 5.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(5.0,3.0), &mut doc, &mut h); let l = as_line(doc.entities[1]); assert_eq!(l.p1, p(0.0,5.0)); assert_eq!(l.p2, p(10.0,5.0)); }
    #[test] fn parallel_line_right_side() { let (mut t, mut doc, mut h) = make(vec![horiz()]); t.distance_mm = 5.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(5.0,-3.0), &mut doc, &mut h); let l = as_line(doc.entities[1]); assert_eq!(l.p1, p(0.0,-5.0)); assert_eq!(l.p2, p(10.0,-5.0)); }
    #[test] fn side_pick_commits_create_arc() { let (mut t, mut doc, mut h) = make(vec![arc5()]); t.distance_mm = 2.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(8.0,0.0), &mut doc, &mut h); assert_eq!(doc.entity_count(), 2); assert!(h.can_undo()); }
    #[test] fn arc_outward_offset_geometry() { let (mut t, mut doc, mut h) = make(vec![arc5()]); t.distance_mm = 2.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(7.0,0.0), &mut doc, &mut h); let a = as_arc(doc.entities[1]); assert_eq!(a.r, 7.0); assert_eq!(a.center, p(0.0,0.0)); assert_eq!(a.start_angle, 0.0); assert_eq!(a.end_angle, FRAC_PI_2); assert!(a.ccw); }
    #[test] fn arc_inward_offset_geometry() { let (mut t, mut doc, mut h) = make(vec![arc5()]); t.distance_mm = 2.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(3.0,0.0), &mut doc, &mut h); assert_eq!(as_arc(doc.entities[1]).r, 3.0); }
    #[test] fn arc_inward_degenerate_no_commit() { let (mut t, mut doc, mut h) = make(vec![arc_r(2.0)]); t.distance_mm = 3.0; ck(&mut t, p(2.0,0.0), &mut doc, &mut h); ck(&mut t, p(1.0,0.0), &mut doc, &mut h); assert_eq!(doc.entity_count(), 1); assert!(!h.can_undo()); assert!(t.status_text().contains("side")); }
    #[test] fn ambiguous_line_side_no_commit() { let (mut t, mut doc, mut h) = make(vec![horiz()]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); assert_eq!(doc.entity_count(), 1); assert!(!h.can_undo()); assert!(t.status_text().contains("side")); }
    #[test] fn ambiguous_arc_side_no_commit() { let (mut t, mut doc, mut h) = make(vec![arc5()]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); assert_eq!(doc.entity_count(), 1); assert!(!h.can_undo()); }
    #[test] fn degenerate_source_line_no_panic() { let pt = p(5.0,5.0); let (mut t, mut doc, mut h) = make(vec![Entity::Line(Line::new(pt, pt))]); ck(&mut t, pt, &mut doc, &mut h); ck(&mut t, p(1.0,1.0), &mut doc, &mut h); assert!(!h.can_undo()); }
    #[test] fn preview_returns_offset_entity() { let (mut t, mut doc, mut h) = make(vec![horiz()]); t.distance_mm = 5.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); t.on_pointer_move(p(5.0,3.0), &mut doc); let pv = t.preview(); assert_eq!(pv.len(), 1); assert_eq!(as_line(pv[0]).p1, p(0.0,5.0)); }
    #[test] fn preview_empty_on_ambiguous_cursor() { let (mut t, mut doc, mut h) = make(vec![horiz()]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); t.on_pointer_move(p(5.0,0.0), &mut doc); assert!(t.preview().is_empty()); }
    #[test] fn distance_persists_after_commit() { let (mut t, mut doc, mut h) = make(vec![horiz()]); t.distance_mm = 3.0; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); ck(&mut t, p(5.0,3.0), &mut doc, &mut h); assert_eq!(t.distance_mm, 3.0); }
    #[test] fn escape_from_waiting_side_resets_to_idle() { let (mut t, mut doc, mut h) = make(vec![horiz()]); ck(&mut t, p(5.0,0.0), &mut doc, &mut h); let d = t.distance_mm; let mut app = crate::app::App::default(); t.on_key(egui::Key::Escape, &mut app); assert!(t.preview().is_empty()); assert_eq!(t.distance_mm, d); assert!(!h.can_undo()); }
    #[test] fn enter_in_waiting_side_commits() { let mut t = OffsetTool { distance_mm: 5.0, state: OffsetState::Idle }; let mut app = crate::app::App::default(); app.document.entities.push(horiz()); t.on_pointer_down(p(5.0,0.0), false, &mut app.document, &mut app.history); t.on_pointer_move(p(5.0,3.0), &mut app.document); t.on_key(egui::Key::Enter, &mut app); assert_eq!(app.document.entity_count(), 2); assert!(t.preview().is_empty()); }
    #[test] fn cancel_resets_to_idle() { let (mut t, mut doc, mut h) = make(vec![horiz()]); let d = t.distance_mm; ck(&mut t, p(5.0,0.0), &mut doc, &mut h); t.cancel(); assert!(t.preview().is_empty()); assert_eq!(t.distance_mm, d); }
    #[test] fn object_safe() { let _: Box<dyn Tool> = Box::new(OffsetTool::default()); }
}
