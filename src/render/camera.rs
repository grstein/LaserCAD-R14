//! World ↔ screen transform plus zoom and pan state.
//!
//! [`Camera`] is the sole owner of the millimeter-to-pixel boundary defined in
//! `AGENTS.md` §"Units and types": world coordinates are `f64` millimeters,
//! screen coordinates are egui pixels. Every Phase-3 renderer (grid, bed,
//! entities, preview, snap markers) and every Phase-4 tool routes through this
//! type so the drawing stays aligned across zoom and pan.
//!
//! Y-axis convention: world Y grows up (mathematical / AutoCAD convention),
//! screen Y grows down (egui convention). The flip is a single minus sign in
//! [`Camera::world_to_screen`] / [`Camera::screen_to_world`].
//!
//! Pointer-event handling (mouse wheel, middle-drag, `F` key) is owned by
//! LCV-032; this demand provides only the math.
//!
//! Importing `egui` is allowed here — `render/*` is the UI side of the
//! kernel/UI boundary. Importing `eframe` or `rfd` is NOT allowed.
//!
//! Introduced by demand LCV-031.

use crate::geometry::Vec2;

/// Camera transform between world (mm, Y-up) and screen (px, Y-down) space.
///
/// Three state fields:
///
/// - `center_world`: the world-space coordinate (mm) that maps to the
///   geometric center of the viewport.
/// - `mm_per_px`: zoom factor in millimeters per screen pixel. Smaller value
///   = more zoomed in. Must stay strictly positive; the transform methods
///   debug-assert this invariant.
/// - `viewport_size_px`: current viewport size in screen pixels. Written by
///   [`crate::app::App::update`] every frame before any rendering call;
///   readers may assume it is current.
///
/// Deliberately not `Copy`: cloning is cheap but the explicit `.clone()` at
/// call sites keeps the "borrow vs. snapshot" intent clear when renderers
/// and tools share the camera.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    /// World-space coordinate (mm) that maps to the viewport center.
    pub center_world: Vec2,
    /// Zoom factor in millimeters per screen pixel. Must be `> 0.0`.
    pub mm_per_px: f64,
    /// Current viewport size in screen pixels `[width, height]`. Sentinel
    /// `[0.0, 0.0]` on a freshly defaulted [`Camera`]; overwritten by
    /// [`crate::app::App::update`] before any draw call consumes it.
    pub viewport_size_px: [f32; 2],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [0.0, 0.0],
        }
    }
}

impl Camera {
    /// Project a world-space point (mm, Y-up) to a screen-space point
    /// (px, Y-down).
    ///
    /// `viewport_size_px` is assumed to be current. The Y axis is flipped:
    /// positive world Y maps to a smaller screen Y.
    pub fn world_to_screen(&self, w: Vec2) -> egui::Pos2 {
        debug_assert!(self.mm_per_px > 0.0, "mm_per_px must be > 0");
        let half_x = f64::from(self.viewport_size_px[0]) / 2.0;
        let half_y = f64::from(self.viewport_size_px[1]) / 2.0;
        let dx = (w.x - self.center_world.x) / self.mm_per_px;
        let dy = (w.y - self.center_world.y) / self.mm_per_px;
        egui::Pos2::new((half_x + dx) as f32, (half_y - dy) as f32)
    }

    /// Unproject a screen-space point (px, Y-down) to a world-space point
    /// (mm, Y-up). Inverse of [`Camera::world_to_screen`].
    pub fn screen_to_world(&self, s: egui::Pos2) -> Vec2 {
        debug_assert!(self.mm_per_px > 0.0, "mm_per_px must be > 0");
        let half_x = f64::from(self.viewport_size_px[0]) / 2.0;
        let half_y = f64::from(self.viewport_size_px[1]) / 2.0;
        let dx_px = f64::from(s.x) - half_x;
        let dy_px = half_y - f64::from(s.y);
        Vec2::new(
            self.center_world.x + dx_px * self.mm_per_px,
            self.center_world.y + dy_px * self.mm_per_px,
        )
    }

    /// Zoom in by `factor` (caller passes a positive value `> 1.0`).
    ///
    /// Multiplies `mm_per_px` by `1.0 / factor` (smaller `mm_per_px` =
    /// more zoomed in). The viewport center stays fixed in world space.
    pub fn zoom_in(&mut self, factor: f64) {
        debug_assert!(factor > 0.0, "zoom_in factor must be > 0");
        self.mm_per_px /= factor;
    }

    /// Zoom out by `factor` (caller passes a positive value `> 1.0`).
    ///
    /// Multiplies `mm_per_px` by `factor`. The viewport center stays fixed
    /// in world space.
    pub fn zoom_out(&mut self, factor: f64) {
        debug_assert!(factor > 0.0, "zoom_out factor must be > 0");
        self.mm_per_px *= factor;
    }

    /// Zoom in or out around a specific screen anchor.
    ///
    /// `factor > 1.0` zooms in (`mm_per_px` shrinks); `factor < 1.0` zooms
    /// out. The world point under `screen_anchor` stays under
    /// `screen_anchor` after the call — this is the behavior wheel-zoom uses
    /// to keep the drawing pinned to the cursor.
    pub fn zoom_around(&mut self, screen_anchor: egui::Pos2, factor: f64) {
        debug_assert!(factor > 0.0, "zoom_around factor must be > 0");
        let world_before = self.screen_to_world(screen_anchor);
        self.mm_per_px /= factor;
        let world_after = self.screen_to_world(screen_anchor);
        // Shift the camera so the same world point lands under the anchor.
        self.center_world = self.center_world + (world_before - world_after);
    }

    /// Pan the camera by a screen-space delta (the operator dragged the
    /// pointer by `delta_screen_px`).
    ///
    /// Implements `center_world -= delta_screen_px * mm_per_px` so the world
    /// point under the cursor stays under the cursor as the cursor drags:
    /// dragging right by `d` px shifts `center_world.x` by `-d * mm_per_px`.
    /// The same sign applies on Y: dragging down (positive screen-Y delta)
    /// shifts `center_world.y` by `-d * mm_per_px`, which — combined with
    /// the world-Y-up flip in [`Camera::world_to_screen`] — pulls the
    /// drawing visibly down, tracking the cursor.
    pub fn pan(&mut self, delta_screen_px: egui::Vec2) {
        self.center_world.x -= f64::from(delta_screen_px.x) * self.mm_per_px;
        self.center_world.y -= f64::from(delta_screen_px.y) * self.mm_per_px;
    }

    /// Fit `bounds` to the viewport with a 10% margin on each side.
    ///
    /// `None` (empty document) resets to the default view: origin centered,
    /// `mm_per_px = 1.0`. Zero-extent bounds (a point, or a horizontal /
    /// vertical line of zero width or height) center on the point but fall
    /// back to `mm_per_px = 1.0`.
    ///
    /// Also writes `self.viewport_size_px = viewport_size_px` so callers
    /// can pass the current viewport in one call.
    pub fn zoom_extents(&mut self, bounds: Option<(Vec2, Vec2)>, viewport_size_px: [f32; 2]) {
        self.viewport_size_px = viewport_size_px;
        let Some((min, max)) = bounds else {
            self.center_world = Vec2::new(0.0, 0.0);
            self.mm_per_px = 1.0;
            return;
        };
        let cx = (min.x + max.x) / 2.0;
        let cy = (min.y + max.y) / 2.0;
        self.center_world = Vec2::new(cx, cy);

        let width_mm = max.x - min.x;
        let height_mm = max.y - min.y;
        let vw = f64::from(viewport_size_px[0]);
        let vh = f64::from(viewport_size_px[1]);
        // Zero-extent bbox (single point / zero-width line) → fallback to 1.0.
        // Zero viewport (sentinel before first frame) → fallback too.
        if width_mm <= 0.0 || height_mm <= 0.0 || vw <= 0.0 || vh <= 0.0 {
            self.mm_per_px = 1.0;
            return;
        }
        let mm_per_px_x = width_mm / vw;
        let mm_per_px_y = height_mm / vh;
        // 80% of the viewport carries the drawing → multiply by 1/0.8.
        self.mm_per_px = mm_per_px_x.max(mm_per_px_y) / 0.8;
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests covering AC#1..AC#11 of LCV-031.
    //!
    //! AC#11 round-trip tolerance note: `world_to_screen` quantizes to
    //! `egui::Pos2` (`f32`). The achievable round-trip floor is
    //! ~`f32::EPSILON * screen_coord_mag * mm_per_px`, ~1e-4 mm at typical
    //! viewport scales. Kernel `EPSILON = 1e-9` only applies to pure-f64
    //! paths; the round-trip test uses a realistic `1e-3` mm tolerance.

    use super::*;
    use crate::geometry::EPSILON;

    fn fixture() -> Camera {
        Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        }
    }

    #[test]
    fn camera_default_values() {
        // AC#1, AC#2.
        let cam = Camera::default();
        assert_eq!(cam.center_world, Vec2::new(0.0, 0.0));
        assert_eq!(cam.mm_per_px, 1.0);
        assert_eq!(cam.viewport_size_px, [0.0, 0.0]);
    }

    #[test]
    fn world_to_screen_and_back_at_origin() {
        // AC#3.
        let cam = fixture();
        assert_eq!(
            cam.world_to_screen(Vec2::new(0.0, 0.0)),
            egui::Pos2::new(400.0, 300.0)
        );
        let back = cam.screen_to_world(egui::Pos2::new(400.0, 300.0));
        assert!(back.approx_eq(Vec2::new(0.0, 0.0), EPSILON));
    }

    #[test]
    fn y_axis_is_flipped() {
        // AC#4 — positive world Y → smaller screen Y.
        let cam = fixture();
        assert_eq!(
            cam.world_to_screen(Vec2::new(0.0, 10.0)),
            egui::Pos2::new(400.0, 290.0)
        );
    }

    #[test]
    fn zoom_changes_scale() {
        // AC#5.
        let mut zoomed_out = fixture();
        zoomed_out.mm_per_px = 2.0;
        assert_eq!(
            zoomed_out.world_to_screen(Vec2::new(10.0, 0.0)),
            egui::Pos2::new(405.0, 300.0)
        );
        let mut zoomed_in = fixture();
        zoomed_in.mm_per_px = 0.5;
        assert_eq!(
            zoomed_in.world_to_screen(Vec2::new(10.0, 0.0)),
            egui::Pos2::new(420.0, 300.0)
        );
    }

    #[test]
    fn pan_shifts_center_world() {
        // AC#6.
        let mut cam = fixture();
        cam.pan(egui::Vec2::new(100.0, 0.0));
        assert!(cam.center_world.approx_eq(Vec2::new(-100.0, 0.0), EPSILON));
    }

    #[test]
    fn zoom_around_preserves_world_under_anchor() {
        // AC#7 — applied in both zoom directions.
        let mut cam = fixture();
        let anchor = egui::Pos2::new(123.0, 87.0);
        let before = cam.screen_to_world(anchor);
        cam.zoom_around(anchor, 2.0);
        assert!(before.approx_eq(cam.screen_to_world(anchor), 1e-9));
        cam.zoom_around(anchor, 0.5);
        assert!(before.approx_eq(cam.screen_to_world(anchor), 1e-9));
    }

    #[test]
    fn zoom_extents_fits_known_bbox() {
        // AC#8.
        let mut cam = Camera::default();
        cam.zoom_extents(
            Some((Vec2::new(0.0, 0.0), Vec2::new(100.0, 50.0))),
            [800.0, 600.0],
        );
        assert!(cam.center_world.approx_eq(Vec2::new(50.0, 25.0), EPSILON));
        let expected = (100.0_f64 / 800.0).max(50.0_f64 / 600.0) / 0.8;
        assert!((cam.mm_per_px - expected).abs() < EPSILON);
        assert!((cam.mm_per_px - 0.15625).abs() < 1e-12);
        assert_eq!(cam.viewport_size_px, [800.0, 600.0]);
    }

    #[test]
    fn zoom_extents_on_empty_bounds_uses_default_view() {
        // AC#9.
        let mut cam = Camera {
            center_world: Vec2::new(7.0, -3.0),
            mm_per_px: 4.0,
            viewport_size_px: [10.0, 10.0],
        };
        cam.zoom_extents(None, [800.0, 600.0]);
        assert_eq!(cam.center_world, Vec2::new(0.0, 0.0));
        assert_eq!(cam.mm_per_px, 1.0);
        assert_eq!(cam.viewport_size_px, [800.0, 600.0]);
    }

    #[test]
    fn zoom_extents_on_degenerate_bounds_falls_back() {
        // AC#10 — zero-extent bbox centers on the point, `mm_per_px = 1.0`.
        let mut cam = Camera::default();
        cam.zoom_extents(
            Some((Vec2::new(5.0, 5.0), Vec2::new(5.0, 5.0))),
            [800.0, 600.0],
        );
        assert_eq!(cam.center_world, Vec2::new(5.0, 5.0));
        assert_eq!(cam.mm_per_px, 1.0);
    }

    #[test]
    fn roundtrip_after_pan_zoom_pan() {
        // AC#11 — see module-level note on the 1e-3 tolerance.
        let mut cam = fixture();
        cam.pan(egui::Vec2::new(37.0, -12.0));
        cam.zoom_around(egui::Pos2::new(200.0, 150.0), 1.7);
        cam.pan(egui::Vec2::new(-5.0, 22.0));
        for x in [-50.0_f64, 0.0, 10.5, 100.0] {
            for y in [-25.0_f64, 0.0, 7.25, 80.0] {
                let w = Vec2::new(x, y);
                let back = cam.screen_to_world(cam.world_to_screen(w));
                assert!(
                    back.approx_eq(w, 1e-3),
                    "round-trip failed for {w:?}: got {back:?}"
                );
            }
        }
    }

    #[test]
    fn zoom_in_and_zoom_out_are_inverses() {
        // zoom_in / zoom_out: inverses; leave center_world untouched.
        let mut cam = fixture();
        let original = cam.center_world;
        cam.zoom_in(2.0);
        assert!((cam.mm_per_px - 0.5).abs() < EPSILON);
        cam.zoom_out(2.0);
        assert!((cam.mm_per_px - 1.0).abs() < EPSILON);
        assert_eq!(cam.center_world, original);
    }
}
