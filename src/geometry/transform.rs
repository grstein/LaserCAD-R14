//! Rigid and similarity transforms shared by the core edit commands.
//!
//! [`Transform`] is a closed enum with one variant per edit: ROTATE
//! (LCV-158), MIRROR (LCV-181) and SCALE (LCV-182). Every method
//! returns a new value; nothing here mutates in place. Angles are radians,
//! CCW-positive from +X, like the rest of the kernel.
//!
//! A rotation by an arbitrary angle is not exactly invertible in `f64`, so
//! callers that need undo keep a snapshot of the original values
//! (`TransformEntities`).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use core::f64::consts::TAU;

use crate::geometry::{Arc, Bezier, Circle, EPSILON, Ellipse, EllipseSpan, Line, Vec2};

/// A geometric transform applied to points and to every kernel primitive.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Transform {
    /// Rotate about `base` by `angle` radians, counter-clockwise positive.
    Rotate {
        /// The fixed point of the rotation, in mm.
        base: Vec2,
        /// The rotation angle in radians, CCW-positive.
        angle: f64,
    },
    /// Reflect across the line through `a` and `b`. Coincident points (within
    /// `EPSILON`) define no line; the transform then moves nothing.
    Mirror {
        /// First point of the mirror line, in mm.
        a: Vec2,
        /// Second point of the mirror line, in mm.
        b: Vec2,
    },
    /// Scale uniformly about `base` by `factor`. Callers only build it with
    /// `factor > 0` (LCV-182 AC5).
    Scale {
        /// The fixed point of the scale, in mm.
        base: Vec2,
        /// The uniform scale factor, positive.
        factor: f64,
    },
}

impl Transform {
    /// `true` when the transform moves nothing, within `EPSILON`: a rotation
    /// by a whole number of turns, a mirror whose two points coincide, or a
    /// scale by 1. Callers commit nothing then (LCV-158 AC8, LCV-182 AC6).
    pub fn is_identity(&self) -> bool {
        match *self {
            Transform::Rotate { angle, .. } => {
                let r = angle.rem_euclid(TAU);
                r <= EPSILON || TAU - r <= EPSILON
            }
            Transform::Mirror { .. } => self.mirror_axis().is_none(),
            Transform::Scale { factor, .. } => (factor - 1.0).abs() <= EPSILON,
        }
    }

    /// The mirror line as `(a, unit direction)`, or `None` for any other
    /// transform or for coincident mirror points.
    fn mirror_axis(&self) -> Option<(Vec2, Vec2)> {
        match *self {
            Transform::Mirror { a, b } if a.distance(b) > EPSILON => {
                (b - a).normalize().map(|u| (a, u))
            }
            _ => None,
        }
    }

    /// The image of point `p`.
    pub fn point(&self, p: Vec2) -> Vec2 {
        match *self {
            Transform::Rotate { base, angle } => {
                let (sin, cos) = angle.sin_cos();
                let d = p - base;
                base + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
            }
            Transform::Mirror { .. } => match self.mirror_axis() {
                Some((a, u)) => {
                    let d = p - a;
                    a + u * (2.0 * d.dot(u)) - d
                }
                None => p,
            },
            Transform::Scale { base, factor } => base + (p - base) * factor,
        }
    }

    /// The image of `line`: both endpoints mapped, in order.
    pub fn line(&self, line: Line) -> Line {
        Line::new(self.point(line.p1), self.point(line.p2))
    }

    /// The image of `circle`: the center mapped; rotation and mirror keep the
    /// radius, a scale multiplies it by its factor.
    pub fn circle(&self, circle: Circle) -> Circle {
        match *self {
            Transform::Rotate { .. } | Transform::Mirror { .. } => {
                Circle::new(self.point(circle.center), circle.r)
            }
            Transform::Scale { factor, .. } => {
                Circle::new(self.point(circle.center), circle.r * factor)
            }
        }
    }

    /// The image of `arc`: the center mapped, the radius kept. A rotation keeps
    /// the orientation and adds its angle to the start and end angles. A mirror
    /// across a line at angle θ maps each angle to `2θ − angle` and reverses the
    /// orientation, so the start and end points are the images of the source's,
    /// in the same roles, and the result is still one arc (LCV-181 AC8). A
    /// scale multiplies the radius and keeps the angles and orientation.
    pub fn arc(&self, arc: Arc) -> Arc {
        match *self {
            Transform::Rotate { angle, .. } => Arc::new(
                self.point(arc.center),
                arc.r,
                arc.start_angle + angle,
                arc.end_angle + angle,
                arc.ccw,
            ),
            Transform::Mirror { .. } => match self.mirror_axis() {
                Some((_, u)) => {
                    let two_theta = 2.0 * u.y.atan2(u.x);
                    Arc::new(
                        self.point(arc.center),
                        arc.r,
                        two_theta - arc.start_angle,
                        two_theta - arc.end_angle,
                        !arc.ccw,
                    )
                }
                None => arc,
            },
            Transform::Scale { factor, .. } => Arc::new(
                self.point(arc.center),
                arc.r * factor,
                arc.start_angle,
                arc.end_angle,
                arc.ccw,
            ),
        }
    }

    /// The image of `ellipse` (ADR 0015 §4): the center mapped; a rotation
    /// adds its angle to the rotation; a mirror across a line at angle θ sets
    /// the rotation to `2θ − rotation`, negates the span angles and flips the
    /// direction, so the ends keep their roles; a scale multiplies both radii.
    pub fn ellipse(&self, e: Ellipse) -> Ellipse {
        let center = self.point(e.center);
        match *self {
            Transform::Rotate { angle, .. } => Ellipse {
                center,
                rotation: e.rotation + angle,
                ..e
            },
            Transform::Mirror { .. } => match self.mirror_axis() {
                Some((_, u)) => Ellipse {
                    center,
                    rotation: 2.0 * u.y.atan2(u.x) - e.rotation,
                    span: e.span.map(|s| EllipseSpan::new(-s.start, -s.end, !s.ccw)),
                    ..e
                },
                None => e,
            },
            Transform::Scale { factor, .. } => Ellipse {
                center,
                rx: e.rx * factor,
                ry: e.ry * factor,
                ..e
            },
        }
    }

    /// Map every control point through [`Transform::point`]; the degree is
    /// kept. Exact by affine invariance (ADR 0016 §3).
    pub fn bezier(&self, b: Bezier) -> Bezier {
        b.map(|p| self.point(p))
    }
}
