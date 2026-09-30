//! What a trim removes (LCV-163 AC 4): the piece(s) of the original target
//! outside the kept piece, so the TRIM preview paints exactly what the click
//! deletes (AC 6).
//!
//! `kept` is the result of folding one or more [`super::trim_step`]s over
//! `original`: a sub-segment of a Line, a sub-arc of a Circle or an Arc with
//! the same centre, radius and direction. Pieces shorter than [`EPSILON`] mm
//! are dropped.
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::parametric_t;
use crate::document::Entity;
use crate::geometry::{Arc, EPSILON, Line};

/// The piece(s) of `original` that are not `kept`, in the original's order.
///
/// Line: the 0–2 end segments outside the kept one. Circle: the arc
/// complementary to the kept arc. Arc: the 0–2 sub-arcs outside the kept
/// span, same centre, radius and direction. An unchanged target, or a `kept`
/// of a shape `original` cannot trim to, removes nothing.
pub(crate) fn removed_pieces(original: &Entity, kept: &Entity) -> Vec<Entity> {
    let pieces = match (*original, *kept) {
        (Entity::Line(o), Entity::Line(k)) => line_pieces(o, k),
        (Entity::Circle(_), Entity::Arc(k)) => {
            vec![Entity::Arc(Arc::new(
                k.center,
                k.r,
                k.end_angle,
                k.start_angle,
                k.ccw,
            ))]
        }
        (Entity::Arc(o), Entity::Arc(k)) => vec![
            Entity::Arc(Arc::new(o.center, o.r, o.start_angle, k.start_angle, o.ccw)),
            Entity::Arc(Arc::new(o.center, o.r, k.end_angle, o.end_angle, o.ccw)),
        ],
        _ => Vec::new(),
    };
    pieces.into_iter().filter(|p| !is_degenerate(p)).collect()
}

/// The end segments of `original` outside `kept`, ordered by parametric `t`.
fn line_pieces(original: Line, kept: Line) -> Vec<Entity> {
    let (a, b) = (kept.p1, kept.p2);
    let (lo, hi) = if parametric_t(&original, a) <= parametric_t(&original, b) {
        (a, b)
    } else {
        (b, a)
    };
    vec![
        Entity::Line(Line::new(original.p1, lo)),
        Entity::Line(Line::new(hi, original.p2)),
    ]
}

/// True when `piece` is shorter than [`EPSILON`] mm.
fn is_degenerate(piece: &Entity) -> bool {
    match piece {
        Entity::Line(l) => l.length() < EPSILON,
        Entity::Arc(a) => a.arc_length() < EPSILON,
        Entity::Circle(c) => c.r < EPSILON,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle, Vec2};
    use core::f64::consts::{FRAC_PI_2, PI};

    fn line(x1: f64, x2: f64) -> Entity {
        Entity::Line(Line::new(Vec2::new(x1, 0.0), Vec2::new(x2, 0.0)))
    }

    fn arc(s: f64, e: f64, ccw: bool) -> Arc {
        Arc::new(Vec2::new(0.0, 0.0), 10.0, s, e, ccw)
    }

    fn as_arc(e: &Entity) -> Arc {
        match e {
            Entity::Arc(a) => *a,
            other => panic!("expected Arc, got {other:?}"),
        }
    }

    #[test]
    fn line_with_nothing_removed_yields_no_piece() {
        assert!(removed_pieces(&line(0.0, 10.0), &line(0.0, 10.0)).is_empty());
    }

    #[test]
    fn line_with_one_removed_end_yields_that_segment() {
        assert_eq!(
            removed_pieces(&line(0.0, 10.0), &line(0.0, 4.0)),
            vec![line(4.0, 10.0)]
        );
        assert_eq!(
            removed_pieces(&line(0.0, 10.0), &line(6.0, 10.0)),
            vec![line(0.0, 6.0)]
        );
    }

    #[test]
    fn line_with_two_removed_ends_yields_both_segments() {
        assert_eq!(
            removed_pieces(&line(0.0, 10.0), &line(3.0, 7.0)),
            vec![line(0.0, 3.0), line(7.0, 10.0)]
        );
    }

    #[test]
    fn circle_yields_the_complementary_arc() {
        let circle: Entity = Entity::Circle(Circle::new(Vec2::new(0.0, 0.0), 10.0));
        let kept = arc(0.0, FRAC_PI_2, true);
        let pieces = removed_pieces(&circle, &Entity::Arc(kept));
        assert_eq!(pieces, vec![Entity::Arc(arc(FRAC_PI_2, 0.0, true))]);
        let a = as_arc(&pieces[0]);
        assert!((a.sweep_angle() + kept.sweep_angle() - 2.0 * PI).abs() < 1e-12);
    }

    #[test]
    fn ccw_arc_with_zero_one_two_pieces() {
        let o = arc(0.0, PI, true);
        assert!(removed_pieces(&Entity::Arc(o), &Entity::Arc(o)).is_empty());
        let one = removed_pieces(&Entity::Arc(o), &Entity::Arc(arc(0.0, 1.0, true)));
        assert_eq!(one, vec![Entity::Arc(arc(1.0, PI, true))]);
        let two = removed_pieces(&Entity::Arc(o), &Entity::Arc(arc(1.0, 2.0, true)));
        assert_eq!(
            two,
            vec![
                Entity::Arc(arc(0.0, 1.0, true)),
                Entity::Arc(arc(2.0, PI, true))
            ]
        );
    }

    #[test]
    fn cw_arc_with_zero_one_two_pieces() {
        let o = arc(PI, 0.0, false);
        assert!(removed_pieces(&Entity::Arc(o), &Entity::Arc(o)).is_empty());
        let one = removed_pieces(&Entity::Arc(o), &Entity::Arc(arc(2.0, 0.0, false)));
        assert_eq!(one, vec![Entity::Arc(arc(PI, 2.0, false))]);
        let two = removed_pieces(&Entity::Arc(o), &Entity::Arc(arc(2.0, 1.0, false)));
        assert_eq!(
            two,
            vec![
                Entity::Arc(arc(PI, 2.0, false)),
                Entity::Arc(arc(1.0, 0.0, false))
            ]
        );
        for p in &two {
            assert!(!as_arc(p).ccw, "pieces keep the CW direction");
        }
    }

    /// An arc across ±π: 3π/4 → −3π/4 CCW, kept around π.
    #[test]
    fn arc_across_plus_minus_pi_keeps_short_pieces() {
        let o = arc(0.75 * PI, -0.75 * PI, true);
        let kept = arc(0.9 * PI, -0.9 * PI, true);
        let pieces = removed_pieces(&Entity::Arc(o), &Entity::Arc(kept));
        assert_eq!(pieces.len(), 2);
        for p in &pieces {
            let s = as_arc(p).sweep_angle();
            assert!((s - 0.15 * PI).abs() < 1e-9, "piece sweep {s}");
        }
    }

    #[test]
    fn degenerate_pieces_are_dropped() {
        let o = line(0.0, 10.0);
        let kept = line(1e-12, 10.0 - 1e-12);
        assert!(removed_pieces(&o, &kept).is_empty());
        let o = arc(0.0, PI, true);
        let kept = arc(1e-13, PI - 1e-13, true);
        assert!(removed_pieces(&Entity::Arc(o), &Entity::Arc(kept)).is_empty());
    }
}
