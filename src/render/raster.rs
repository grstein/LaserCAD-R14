//! LCV-145 — offscreen software raster of the live drawing (ADR 0011 items 3–5).
//!
//! **Kernel-pure by declaration**: unlike the rest of `render/`, this file
//! imports no `egui`, `eframe` or `rfd` (AGENTS.md §Purity rule). The picture a
//! vision model gets is a pure function of the entities, the bed and a world
//! rectangle, so no chrome, transcript or settings pixel can ever be in it.

use crate::document::Entity;

/// Background grey level.
pub const WHITE: u8 = 255;
/// Bed outline grey level.
pub const BED_GREY: u8 = 128;
/// Entity grey level.
pub const INK: u8 = 0;

/// Rasterize `entities` and the bed outline into an 8-bit grayscale buffer.
///
/// `world` is `[x0, y0, x1, y1]` in mm, Y up. Pixel centres span it exactly:
/// the centre of column 0 is `x0`, of column `w − 1` is `x1`; row 0 is `y1`.
/// The result is row-major, `w · h` bytes.
pub fn rasterize(
    _entities: &[Entity],
    _bed_mm: [f64; 2],
    _world: [f64; 4],
    _w: u32,
    _h: u32,
) -> Vec<u8> {
    Vec::new()
}

/// Encode a row-major 8-bit grayscale buffer as a PNG.
pub fn encode_png_gray(_pixels: &[u8], _w: u32, _h: u32) -> Result<Vec<u8>, png::EncodingError> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Arc, Circle, Line, Vec2};

    const BED: [f64; 2] = [100.0, 100.0];

    fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> Entity {
        Entity::Line(Line::new(Vec2::new(x1, y1), Vec2::new(x2, y2)))
    }

    /// Round-trip through PNG so every assertion reads decoded pixels.
    fn decoded(entities: &[Entity], world: [f64; 4], w: u32, h: u32) -> Vec<u8> {
        let raw = rasterize(entities, BED, world, w, h);
        let png = encode_png_gray(&raw, w, h).expect("encode");
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().expect("png header");
        let mut buf = vec![0; reader.output_buffer_size().expect("size")];
        let info = reader.next_frame(&mut buf).expect("png frame");
        assert_eq!((info.width, info.height), (w, h));
        assert_eq!(info.color_type, png::ColorType::Grayscale);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        buf.truncate(info.buffer_size());
        buf
    }

    #[test]
    fn corner_to_corner_line_is_black_on_the_diagonal_and_white_elsewhere() {
        // Frame strictly inside the bed so the outline stays out of the picture.
        let world = [10.0, 10.0, 90.0, 90.0];
        let px = decoded(&[line(0.0, 0.0, 100.0, 100.0)], world, 81, 81);
        for row in 0..81usize {
            for col in 0..81usize {
                let want = if col + row == 80 { INK } else { WHITE };
                assert_eq!(px[row * 81 + col], want, "pixel ({col}, {row})");
            }
        }
    }

    #[test]
    fn background_bed_and_entity_have_their_pinned_grey_levels() {
        // 1 mm per pixel; the bed outline runs along the frame's edges.
        let world = [0.0, 0.0, 100.0, 100.0];
        let px = decoded(&[line(20.0, 50.0, 80.0, 50.0)], world, 101, 101);
        let at = |x: usize, y_up: usize| px[(100 - y_up) * 101 + x];
        assert_eq!(at(50, 20), WHITE, "background");
        assert_eq!(at(0, 30), BED_GREY, "left bed edge");
        assert_eq!(at(100, 70), BED_GREY, "right bed edge");
        assert_eq!(at(40, 0), BED_GREY, "bottom bed edge");
        assert_eq!(at(40, 100), BED_GREY, "top bed edge");
        assert_eq!(at(20, 50), INK, "entity start");
        assert_eq!(at(50, 50), INK, "entity middle");
        assert_eq!(at(80, 50), INK, "entity end");
        assert_eq!(at(81, 50), WHITE, "past the entity");
        assert_eq!(at(50, 51), WHITE, "entity is 1 px wide");
    }

    #[test]
    fn entities_are_drawn_over_the_bed_outline() {
        let world = [0.0, 0.0, 100.0, 100.0];
        let px = decoded(&[line(0.0, 0.0, 0.0, 100.0)], world, 101, 101);
        assert_eq!(px[50 * 101], INK);
    }

    #[test]
    fn every_circle_pixel_lies_within_a_quarter_plus_half_pixel_of_the_circle() {
        let (cx, cy, r) = (50.0, 50.0, 37.3);
        let circle = Entity::Circle(Circle::new(Vec2::new(cx, cy), r));
        let px = decoded(&[circle], [5.0, 5.0, 95.0, 95.0], 91, 91);
        let mut ink = 0;
        for row in 0..91usize {
            for col in 0..91usize {
                if px[row * 91 + col] != INK {
                    continue;
                }
                ink += 1;
                let (x, y) = (5.0 + col as f64, 95.0 - row as f64);
                let off = ((x - cx).hypot(y - cy) - r).abs();
                assert!(off <= 0.75 + 1e-9, "pixel ({col}, {row}) is {off} px off");
            }
        }
        // A closed ring: at least one pixel per column the circle spans, twice.
        assert!(ink >= (2.0 * 2.0 * r) as usize, "only {ink} ink pixels");
    }

    #[test]
    fn arc_draws_only_its_sweep() {
        let world = [0.0, 0.0, 100.0, 100.0];
        // Upper half, CCW from 0 to π: its top is inked, its bottom is not.
        let arc = Arc::new(Vec2::new(50.0, 50.0), 30.0, 0.0, std::f64::consts::PI, true);
        let px = decoded(&[Entity::Arc(arc)], world, 101, 101);
        let at = |x: usize, y_up: usize| px[(100 - y_up) * 101 + x];
        assert_eq!(at(50, 80), INK, "top of the arc");
        assert_eq!(at(50, 20), WHITE, "bottom is outside the sweep");
    }

    #[test]
    fn far_off_entities_are_clipped_without_walking_their_length() {
        let world = [0.0, 0.0, 100.0, 100.0];
        // Unclipped, these would walk ~10^12 pixels; clipped, they cost nothing.
        let far = line(-1e12, -1e12, -1e12 + 1.0, -1e12);
        let across = line(-1e12, 50.0, 1e12, 50.0);
        let huge = Entity::Circle(Circle::new(Vec2::new(-10_000.0, -10_000.0), 1e9));
        let px = decoded(&[far, across, huge], world, 101, 101);
        assert_eq!(px[50 * 101 + 50], INK, "the crossing line is drawn");
        assert_eq!(px[10 * 101 + 50], WHITE);
    }

    #[test]
    fn degenerate_sizes_do_not_panic() {
        assert!(rasterize(&[], BED, [0.0, 0.0, 1.0, 1.0], 0, 10).is_empty());
        assert_eq!(rasterize(&[], BED, [0.0, 0.0, 0.0, 0.0], 1, 1).len(), 1);
    }
}
