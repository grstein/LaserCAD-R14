//! Rigid and similarity transforms shared by the core edit commands.
//!
//! [`Transform`] is a closed enum with one variant per edit: ROTATE
//! (LCV-158) and MIRROR (LCV-181) today, SCALE (LCV-182) next. Every method
//! returns a new value; nothing here mutates in place. Angles are radians,
//! CCW-positive from +X, like the rest of the kernel.
//!
//! A rotation by an arbitrary angle is not exactly invertible in `f64`, so
//! callers that need undo keep a snapshot of the original values
//! (`TransformEntities`).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use core::f64::consts::TAU;

use crate::geometry::{Arc, Circle, EPSILON, Line, Vec2};

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
}

impl Transform {
    /// `true` when the transform moves nothing, within `EPSILON`: a rotation
    /// by a whole number of turns, or a mirror whose two points coincide.
    /// Callers commit nothing then (LCV-158 AC8).
    pub fn is_identity(&self) -> bool {
        match *self {
            Transform::Rotate { angle, .. } => {
                let r = angle.rem_euclid(TAU);
                r <= EPSILON || TAU - r <= EPSILON
            }
            Transform::Mirror { .. } => self.mirror_axis().is_none(),
        }
    }

    /// The mirror line as `(a, unit direction)`, or `None` for a rotation or
    /// for coincident mirror points.
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
        }
    }

    /// The image of `line`: both endpoints mapped, in order.
    pub fn line(&self, line: Line) -> Line {
        Line::new(self.point(line.p1), self.point(line.p2))
    }

    /// The image of `circle`: the center mapped; rotation and mirror keep the
    /// radius.
    pub fn circle(&self, circle: Circle) -> Circle {
        match *self {
            Transform::Rotate { .. } | Transform::Mirror { .. } => {
                Circle::new(self.point(circle.center), circle.r)
            }
        }
    }

    /// The image of `arc`: the center mapped, the radius kept. A rotation keeps
    /// the orientation and adds its angle to the start and end angles. A mirror
    /// across a line at angle θ maps each angle to `2θ − angle` and reverses the
    /// orientation, so the start and end points are the images of the source's,
    /// in the same roles, and the result is still one arc (LCV-181 AC8).
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
        }
    }
}
