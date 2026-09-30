//! LCV-181 AC8 — a mirrored arc exports as one `A` path command: the same
//! `large-arc-flag` as its source, the inverted `sweep-flag`, and the same
//! path as the equivalent arc drawn directly.

use core::f64::consts::{FRAC_PI_2, PI};
use lasercad::document::{Document, Entity};
use lasercad::geometry::{Arc, Transform, Vec2};
use lasercad::io::svg::export_svg;

/// The only `<path d="…"/>` of a one-arc document, split into its tokens:
/// `M sx sy A r r 0 large sweep ex ey`.
fn path_tokens(arc: Arc) -> Vec<String> {
    let mut doc = Document::default();
    doc.push_current(Entity::Arc(arc));
    let svg = export_svg(&doc);
    let start = svg.find("<path d=\"").expect("one arc path") + "<path d=\"".len();
    let end = start + svg[start..].find('"').expect("closing quote");
    let tokens: Vec<String> = svg[start..end]
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    assert_eq!(tokens.len(), 11, "{tokens:?}");
    assert_eq!((tokens[0].as_str(), tokens[3].as_str()), ("M", "A"));
    tokens
}

/// Across the vertical line x = 100.
fn across_x_100() -> Transform {
    Transform::Mirror {
        a: Vec2::new(100.0, 0.0),
        b: Vec2::new(100.0, 1.0),
    }
}

/// A small CCW arc mirrors into the CW arc drawn directly: same path, the
/// `sweep` flag inverted, `large` kept.
#[test]
fn mirrored_small_ccw_arc_inverts_sweep_and_keeps_large() {
    let src = Arc::new(Vec2::new(50.0, 50.0), 10.0, 0.0, FRAC_PI_2, true);
    let mirrored = path_tokens(across_x_100().arc(src));
    let direct = path_tokens(Arc::new(Vec2::new(150.0, 50.0), 10.0, PI, FRAC_PI_2, false));
    let source = path_tokens(src);
    assert_eq!(mirrored, direct);
    assert_eq!(mirrored[7], source[7], "large kept");
    assert_eq!((source[8].as_str(), mirrored[8].as_str()), ("0", "1"));
}

/// A large CCW arc (sweep 3π/2) keeps `large = 1` and inverts `sweep`.
#[test]
fn mirrored_large_ccw_arc_inverts_sweep_and_keeps_large() {
    let src = Arc::new(Vec2::new(50.0, 50.0), 10.0, 0.0, 3.0 * FRAC_PI_2, true);
    let mirrored = path_tokens(across_x_100().arc(src));
    let direct = path_tokens(Arc::new(
        Vec2::new(150.0, 50.0),
        10.0,
        PI,
        -FRAC_PI_2,
        false,
    ));
    let source = path_tokens(src);
    assert_eq!(mirrored, direct);
    assert_eq!((source[7].as_str(), mirrored[7].as_str()), ("1", "1"));
    assert_eq!((source[8].as_str(), mirrored[8].as_str()), ("0", "1"));
}
