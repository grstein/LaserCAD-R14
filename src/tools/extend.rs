//! ExtendTool: hover near a Line endpoint → preview → click to commit.
//!
//! `on_pointer_move` picks the nearest Line endpoint within [`PICK_RADIUS_MM`]
//! and the nearest valid boundary (Line or Circle) and shows a live preview.
//! `on_pointer_down` commits [`ExtendEntity`] and resets to `Idle`. Arcs are
//! silently skipped. MUST NOT import `eframe` or `rfd`. Introduced by LCV-051.

use crate::app::App;
use crate::document::{Document, Entity, ExtendEntity, History};
use crate::geometry::{Circle, EPSILON, Line, Vec2, line_circle, line_line_infinite};
use crate::tools::Tool;

/// World-space pick radius (mm) for nearest-endpoint detection.
pub(crate) const PICK_RADIUS_MM: f64 = 5.0;

/// Compact hover state: (target_idx, extend_endpoint, boundary_idx, preview_line).
#[derive(Debug, Clone, Copy)]
struct H(usize, u8, usize, Line);

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Hover(H),
}

/// Single-click extend-to-nearest-boundary modify tool (LCV-051).
#[derive(Debug, Default)]
pub struct ExtendTool {
    state: State,
}

// Parametric t of p on infinite line through `line` (0.0 if degenerate).
#[rustfmt::skip]
fn pt(line: &Line, p: Vec2) -> f64 {
    let d = line.p2 - line.p1; let ls = d.length_squared();
    if ls <= EPSILON * EPSILON { 0.0 } else { (p - line.p1).dot(d) / ls }
}

// Valid infinite-line intersections of `tgt` with circle `b` for endpoint `ep`.
#[rustfmt::skip]
fn chits(tgt: &Line, b: &Circle, ep: u8) -> Vec<Vec2> {
    let d = tgt.p2 - tgt.p1; let len = d.length();
    if len <= EPSILON { return vec![]; }
    let dir = d / len;
    let reach = (b.center - tgt.p1).length() + b.r + len + 1.0;
    let sur = Line::new(tgt.p1 - dir * reach, tgt.p1 + dir * reach);
    line_circle(&sur, b).into_iter()
        .filter(|&p| if ep == 0 { pt(tgt, p) < -EPSILON } else { pt(tgt, p) > 1.0 + EPSILON })
        .collect()
}

impl Tool for ExtendTool {
    fn name(&self) -> &'static str {
        "EXTEND"
    }

    fn status_text(&self) -> &'static str {
        match self.state {
            State::Idle => "EXTEND: Click near a line endpoint to extend it",
            State::Hover(_) => "EXTEND: Click to extend  |  Esc to cancel",
        }
    }

    #[rustfmt::skip]
    fn on_pointer_move(&mut self, pos: Vec2, doc: &mut Document) {
        let mut best: Option<(usize, u8, f64)> = None;
        for (i, e) in doc.entities.iter().enumerate() {
            let Entity::Line(l) = e else { continue };
            let d0 = (pos - l.p1).length(); let d1 = (pos - l.p2).length();
            let (md, ep) = if d0 < d1 { (d0, 0u8) } else { (d1, 1u8) };
            if md > PICK_RADIUS_MM { continue; }
            if best.map(|(_, _, bd)| md < bd).unwrap_or(true) { best = Some((i, ep, md)); }
        }
        let Some((ti, ep, _)) = best else { self.state = State::Idle; return; };
        let Entity::Line(tgt) = doc.entities[ti] else { self.state = State::Idle; return; };
        let ep_pos = if ep == 0 { tgt.p1 } else { tgt.p2 };
        let mut bnd: Option<(f64, usize, Vec2)> = None;
        for (j, e) in doc.entities.iter().enumerate() {
            if j == ti { continue; }
            let hits: Vec<Vec2> = match e {
                Entity::Line(b) => match line_line_infinite(&tgt, b) {
                    Some(x) => { let ok = if ep == 0 { pt(&tgt, x) < -EPSILON } else { pt(&tgt, x) > 1.0 + EPSILON }; if ok { vec![x] } else { vec![] } }
                    None => vec![],
                },
                Entity::Circle(b) => chits(&tgt, b, ep),
                _ => vec![],
            };
            for x in hits {
                let dist = (x - ep_pos).length();
                if bnd.map(|(bd, _, _)| dist < bd).unwrap_or(true) { bnd = Some((dist, j, x)); }
            }
        }
        let Some((_, bi, x)) = bnd else { self.state = State::Idle; return; };
        let pl = if ep == 0 { Line::new(x, tgt.p2) } else { Line::new(tgt.p1, x) };
        self.state = State::Hover(H(ti, ep, bi, pl));
    }

    fn on_pointer_down(&mut self, _: Vec2, _: bool, doc: &mut Document, history: &mut History) {
        if let State::Hover(H(ti, ep, bi, _)) = self.state {
            history.commit(Box::new(ExtendEntity::new(ti, bi, ep)), doc);
            self.state = State::Idle;
        }
    }

    fn on_pointer_up(&mut self, _: Vec2, _: bool, _: &mut Document, _: &mut History) {}

    fn on_key(&mut self, key: egui::Key, _: &mut App) {
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    fn preview(&self) -> Vec<Entity> {
        match self.state {
            State::Idle => vec![],
            State::Hover(H(_, _, _, pl)) => vec![Entity::Line(pl)],
        }
    }

    fn cancel(&mut self) {
        self.state = State::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::History;
    use crate::geometry::{Circle, EPSILON, Line, Vec2};

    #[rustfmt::skip] fn v(x: f64, y: f64) -> Vec2 { Vec2::new(x, y) }
    #[rustfmt::skip] fn le(ax: f64, ay: f64, bx: f64, by: f64) -> Entity { Entity::Line(Line::new(v(ax,ay),v(bx,by))) }
    #[rustfmt::skip] fn ce(cx: f64, cy: f64, r: f64) -> Entity { Entity::Circle(Circle::new(v(cx,cy),r)) }
    #[rustfmt::skip] fn mk(es: Vec<Entity>) -> Document {
     let mut doc = Document::default();
     es.into_iter().for_each(|e| doc.push_current(e));
     doc
 }
    #[rustfmt::skip] fn li(e: Entity) -> Line { if let Entity::Line(l) = e { l } else { panic!("expected Line") } }
    #[rustfmt::skip]
    fn hd() -> (ExtendTool, Document, History) {
        let d = mk(vec![le(0.,0.,5.,0.), le(10.,-1.,10.,1.)]);
        (ExtendTool::default(), d, History::default())
    }

    #[test]
    #[rustfmt::skip]
    fn name_is_extend() { assert_eq!(ExtendTool::default().name(), "EXTEND"); }

    #[test]
    #[rustfmt::skip]
    fn is_object_safe() { let _: Box<dyn Tool> = Box::new(ExtendTool::default()); }

    #[test]
    #[rustfmt::skip]
    fn cancel_from_idle_stays_idle() { let mut t = ExtendTool::default(); t.cancel(); assert!(t.preview().is_empty()); }

    #[test]
    fn escape_key_resets_to_idle() {
        let (mut t, mut d, _) = hd();
        t.on_pointer_move(v(5.2, 0.), &mut d);
        assert!(!t.preview().is_empty());
        t.on_key(egui::Key::Escape, &mut crate::app::App::default());
        assert!(t.preview().is_empty());
        assert!(t.status_text().contains("Click near"));
    }

    #[test]
    fn status_text_transitions() {
        let (mut t, mut d, _) = hd();
        assert!(t.status_text().contains("Click near"));
        t.on_pointer_move(v(5.2, 0.), &mut d);
        assert!(t.status_text().contains("Click to extend"));
    }

    #[test]
    #[rustfmt::skip]
    fn no_line_in_doc_stays_idle() { let mut t = ExtendTool::default(); let mut d = Document::default(); t.on_pointer_move(v(5.,0.),&mut d); assert!(t.preview().is_empty()); }

    #[test]
    #[rustfmt::skip]
    fn cursor_far_from_all_endpoints_stays_idle() { let mut t = ExtendTool::default(); let mut d = mk(vec![le(0.,0.,10.,0.)]); t.on_pointer_move(v(50.,50.),&mut d); assert!(t.preview().is_empty()); }

    #[test]
    #[rustfmt::skip]
    fn single_line_no_boundary_stays_idle() { let mut t = ExtendTool::default(); let mut d = mk(vec![le(0.,0.,5.,0.)]); t.on_pointer_move(v(5.1,0.),&mut d); assert!(t.preview().is_empty()); }

    #[test]
    fn hover_line_boundary_preview() {
        let (mut t, mut d, _) = hd();
        t.on_pointer_move(v(5.2, 0.), &mut d);
        let pv = t.preview();
        assert_eq!(pv.len(), 1);
        let l = li(pv[0]);
        assert!(l.p1.approx_eq(v(0., 0.), EPSILON) && l.p2.approx_eq(v(10., 0.), EPSILON));
    }

    #[test]
    fn hover_circle_boundary_preview() {
        let mut t = ExtendTool::default();
        let mut d = mk(vec![le(-5., 0., -3., 0.), ce(0., 0., 2.)]);
        t.on_pointer_move(v(-2.8, 0.), &mut d);
        assert!(li(t.preview()[0]).p2.approx_eq(v(-2., 0.), EPSILON));
    }

    #[test]
    fn nearest_boundary_selected() {
        let mut t = ExtendTool::default();
        let mut d = mk(vec![
            le(0., 0., 5., 0.),
            le(10., -1., 10., 1.),
            le(20., -1., 20., 1.),
        ]);
        t.on_pointer_move(v(5.2, 0.), &mut d);
        assert!(li(t.preview()[0]).p2.approx_eq(v(10., 0.), EPSILON));
    }

    #[test]
    fn hover_extend_endpoint_zero() {
        let mut t = ExtendTool::default();
        let mut d = mk(vec![le(5., 0., 10., 0.), le(0., -1., 0., 1.)]);
        t.on_pointer_move(v(5.2, 0.), &mut d);
        let l = li(t.preview()[0]);
        assert!(l.p1.approx_eq(v(0., 0.), EPSILON) && l.p2.approx_eq(v(10., 0.), EPSILON));
    }

    #[test]
    fn tiebreak_endpoint_is_one() {
        let mut t = ExtendTool::default();
        let mut d = mk(vec![le(0., 0., 10., 0.), le(20., -1., 20., 1.)]);
        t.on_pointer_move(v(5., 0.), &mut d); // midpoint: d0 == d1
        let l = li(t.preview()[0]);
        assert!(l.p1.approx_eq(v(0., 0.), EPSILON)); // ep=1 → p1 unchanged
        assert!(!l.p2.approx_eq(v(10., 0.), EPSILON)); // p2 extended
    }

    #[test]
    fn pointer_down_commits_and_resets_to_idle() {
        let (mut t, mut d, mut h) = hd();
        t.on_pointer_move(v(5.2, 0.), &mut d);
        t.on_pointer_down(v(5.2, 0.), false, &mut d, &mut h);
        assert!(li(d.entities[0]).p2.approx_eq(v(10., 0.), EPSILON));
        assert!(h.can_undo() && t.preview().is_empty());
    }

    #[test]
    fn extend_via_tool_is_undoable() {
        let (mut t, mut d, mut h) = hd();
        t.on_pointer_move(v(5.2, 0.), &mut d);
        t.on_pointer_down(v(5.2, 0.), false, &mut d, &mut h);
        h.undo(&mut d);
        assert!(li(d.entities[0]).p2.approx_eq(v(5., 0.), EPSILON));
    }

    #[test]
    fn pointer_down_idle_is_noop() {
        let mut t = ExtendTool::default();
        let mut d = mk(vec![le(0., 0., 5., 0.)]);
        let mut h = History::default();
        t.on_pointer_down(v(5.1, 0.), false, &mut d, &mut h);
        assert!(!h.can_undo() && d.entity_count() == 1);
    }
}
