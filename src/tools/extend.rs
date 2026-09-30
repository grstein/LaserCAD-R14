//! ExtendTool: hover near a Line or Arc endpoint → preview → click to commit.
//!
//! `on_pointer_move` picks the nearest Line or Arc endpoint within
//! [`PICK_APERTURE_PT`] at the live zoom and the boundary with the least [`extend_reach`] travel
//! (Line, Circle or Arc), and shows the grown entity as a live preview (a
//! Line to its boundary; an Arc along its own circle, never into a full
//! turn — LCV-160). `on_pointer_down` commits [`ExtendEntity`] and resets to
//! `Idle`. MUST NOT import `eframe` or `rfd`. Introduced by LCV-051.

use crate::app::App;
use crate::document::commands::trim::extend_reach;
use crate::document::{Document, Entity, ExtendEntity, History};
use crate::geometry::Vec2;
use crate::tools::{Mark, PICK_APERTURE_PT, Tool};
use std::borrow::Cow;

/// Compact hover state: (target_idx, extend_endpoint, boundary_idx, preview).
#[derive(Debug, Clone, Copy)]
struct H(usize, u8, usize, Entity);

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Hover(H),
}

/// Single-click extend-to-nearest-boundary modify tool (LCV-051, LCV-160).
#[derive(Debug)]
pub struct ExtendTool {
    state: State,
    /// Live zoom in mm per screen point (LCV-162); `1.0` until forwarded.
    mm_per_pt: f64,
}

impl Default for ExtendTool {
    fn default() -> Self {
        Self {
            state: State::Idle,
            mm_per_pt: 1.0,
        }
    }
}

/// The two endpoints (`0` = p1 / start, `1` = p2 / end) of an extendable
/// entity; `None` for a Circle.
fn endpoints(e: &Entity) -> Option<[Vec2; 2]> {
    match e {
        Entity::Line(l) => Some([l.p1, l.p2]),
        Entity::Arc(a) => Some([a.start_point(), a.end_point()]),
        Entity::Circle(_) => None,
    }
}

/// The hover state for `pos`: the nearest endpoint within `radius_mm` and
/// its shortest extension, or `None`.
fn hover(pos: Vec2, entities: &[Entity], radius_mm: f64) -> Option<H> {
    let mut best: Option<(usize, u8, f64)> = None;
    for (i, e) in entities.iter().enumerate() {
        let Some([p0, p1]) = endpoints(e) else {
            continue;
        };
        let (d0, d1) = ((pos - p0).length(), (pos - p1).length());
        let (md, ep) = if d0 < d1 { (d0, 0u8) } else { (d1, 1u8) };
        if md <= radius_mm && best.is_none_or(|(_, _, bd)| md < bd) {
            best = Some((i, ep, md));
        }
    }
    let (ti, ep, _) = best?;
    let (bi, grown, _) = entities
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != ti)
        .filter_map(|(j, b)| extend_reach(&entities[ti], b, ep).map(|(g, mm)| (j, g, mm)))
        .min_by(|a, b| a.2.total_cmp(&b.2))?;
    Some(H(ti, ep, bi, grown))
}

impl Tool for ExtendTool {
    fn name(&self) -> &'static str {
        "EXTEND"
    }

    fn status_text(&self) -> Cow<'_, str> {
        match self.state {
            State::Idle => "EXTEND: Click near a line or arc endpoint to extend it".into(),
            State::Hover(_) => "EXTEND: Click to extend  |  Esc to cancel".into(),
        }
    }

    fn on_pointer_move(&mut self, pos: Vec2, doc: &mut Document) {
        let radius = PICK_APERTURE_PT * self.mm_per_pt;
        self.state = hover(pos, &doc.entities, radius).map_or(State::Idle, State::Hover);
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
            State::Hover(H(_, _, _, grown)) => vec![grown],
        }
    }

    /// LCV-163 AC 3: the entity whose endpoint a click would extend, as
    /// `Hover`, then its extension as the amber `Preview` — nothing when
    /// `cursor` is `None` or no extension is in reach.
    fn feedback(&self, doc: &Document, cursor: Option<Vec2>) -> Vec<Mark> {
        let radius = PICK_APERTURE_PT * self.mm_per_pt;
        cursor
            .and_then(|c| hover(c, &doc.entities, radius))
            .map_or_else(Vec::new, |H(ti, _, _, grown)| {
                vec![Mark::Hover(ti), Mark::Preview(grown)]
            })
    }

    fn cancel(&mut self) {
        self.state = State::Idle;
    }

    /// EXTEND always waits for an entity pick (LCV-162 AC 5).
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

    /// LCV-163 AC 3 — the picked entity is `Hover`ed and its extension
    /// stays a `Preview`; nothing when the cursor is `None`.
    #[test]
    fn feedback_hovers_the_picked_entity_and_previews_the_extension() {
        let (t, d, _) = hd();
        let marks = t.feedback(&d, Some(v(5.2, 0.)));
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0], Mark::Hover(0));
        let Mark::Preview(grown) = marks[1] else {
            panic!("expected Preview, got {:?}", marks[1]);
        };
        assert!(li(grown).p2.approx_eq(v(10., 0.), EPSILON));
        assert!(t.feedback(&d, None).is_empty());
        assert!(t.feedback(&d, Some(v(2.5, 20.))).is_empty());
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

    /// LCV-162 AC 8 — the endpoint radius is 5 pt at the live zoom, and
    /// EXTEND always waits for an entity pick.
    #[test]
    fn pick_radius_follows_the_pick_scale() {
        for (scale, x, previews) in [(0.05, 5.2, true), (0.05, 5.3, false), (20.0, 85.0, true)] {
            let mut t = ExtendTool::default();
            t.set_pick_scale(scale);
            assert!(t.wants_entity_pick());
            let mut d = mk(vec![le(0., 0., 5., 0.), le(300., -1., 300., 1.)]);
            t.on_pointer_move(v(x, 0.), &mut d);
            assert_eq!(!t.preview().is_empty(), previews, "{scale} mm/pt, x {x}");
        }
    }

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

    /// LCV-160 AC 5 — an Arc endpoint previews the grown arc; the idle
    /// status names both kinds.
    #[test]
    fn hover_arc_endpoint_previews_grown_arc() {
        use crate::geometry::Arc;
        use core::f64::consts::{FRAC_PI_2, PI};
        let mut t = ExtendTool::default();
        assert!(t.status_text().contains("line or arc endpoint"));
        let quarter = Entity::Arc(Arc::new(v(0., 0.), 10., 0., FRAC_PI_2, true));
        let mut d = mk(vec![quarter, le(-5., -20., -5., 20.)]);
        t.on_pointer_move(v(0.5, 10.5), &mut d);
        let Entity::Arc(a) = t.preview()[0] else {
            panic!("expected Arc preview")
        };
        assert!((a.end_angle - 2. * PI / 3.).abs() < 1e-9 && a.start_angle == 0.);
    }
}
