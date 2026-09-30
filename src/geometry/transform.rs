//! Rigid and similarity transforms shared by the core edit commands.
//!
//! [`Transform`] is a closed enum with one variant per edit: ROTATE
//! (LCV-158) today, MIRROR (LCV-181) and SCALE (LCV-182) next. Every method
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
}

impl Transform {
    /// `true` when the transform moves nothing, within `EPSILON`: a rotation
    /// by a whole number of turns. Callers commit nothing then (LCV-158 AC8).
    pub fn is_identity(&self) -> bool {
        match *self {
            Transform::Rotate { angle, .. } => {
                let r = angle.rem_euclid(TAU);
                r <= EPSILON || TAU - r <= EPSILON
            }
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
        }
    }

    /// The image of `line`: both endpoints mapped, in order.
    pub fn line(&self, line: Line) -> Line {
        Line::new(self.point(line.p1), self.point(line.p2))
    }

    /// The image of `circle`: the center mapped; a rotation keeps the radius.
    pub fn circle(&self, circle: Circle) -> Circle {
        match *self {
            Transform::Rotate { .. } => Circle::new(self.point(circle.center), circle.r),
        }
    }

    /// The image of `arc`: the center mapped; a rotation keeps the radius and
    /// orientation and adds its angle to the start and end angles.
    pub fn arc(&self, arc: Arc) -> Arc {
        match *self {
            Transform::Rotate { angle, .. } => Arc::new(
                self.point(arc.center),
                arc.r,
                arc.start_angle + angle,
                arc.end_angle + angle,
                arc.ccw,
            ),
        }
    }
}
