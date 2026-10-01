//! LCV-166 AC 5–7 — the View menu's order, Zoom Extents and Zoom All.
//!
//! Rows are found and clicked through real pointer input with the
//! `menu_rows` helpers; cameras are compared with a twin `App` driven by the
//! same frames.

use super::menu_rows::{assert_menu, assert_slots, centre, click, ctx, open_menu, rows, settle};
use crate::harness;

use lasercad::app::App;
use lasercad::document::CreateLine;
use lasercad::geometry::{Line, Vec2};
use lasercad::render::Camera;

/// The View menu, top to bottom (AC 5).
const VIEW_ORDER: [&str; 9] = [
    "Zoom In",
    "Zoom Out",
    "Zoom Extents",
    "Zoom All",
    "Fit to Bed",
    "Grid",
    "Snap",
    "Object Snap",
    "Ortho",
];

/// One line reaching far outside the default bed on both sides.
fn app_with_stray_line() -> App {
    let mut app = App::default();
    let line = Line::new(Vec2::new(-150.0, -60.0), Vec2::new(900.0, 700.0));
    app.commit(Box::new(CreateLine::new(line)));
    app
}

/// Open View and click `row`; return the camera it leaves.
fn click_view_row(app: &mut App, row: &str) -> Camera {
    let ctx = ctx();
    let view = open_menu(&ctx, app, "View");
    let at = centre(rows(&view, &["Zoom In", row])[1]);
    click(&ctx, app, at);
    let _ = settle(&ctx, app);
    app.camera.clone()
}

/// Whether world point `w` projects inside `camera`'s viewport.
fn in_view(camera: &Camera, w: Vec2) -> bool {
    let p = camera.world_to_screen(w);
    let [vw, vh] = camera.viewport_size_px;
    (0.0..=vw).contains(&p.x) && (0.0..=vh).contains(&p.y)
}

/// AC 5 / AC 2 — View rows in R14 order; Zoom Extents carries its icon and
/// `F`, Zoom All an empty slot.
#[test]
fn view_menu_order_with_zoom_extents_and_zoom_all() {
    let ctx = ctx();
    let mut app = App::default();
    let view = open_menu(&ctx, &mut app, "View");
    let found = rows(&view, &VIEW_ORDER);
    for pair in found.windows(2) {
        assert!(
            pair[0].rect.center().y < pair[1].rect.center().y,
            "`{}` must sit above `{}`",
            pair[0].text,
            pair[1].text
        );
    }
    assert_menu(&view, &[("Zoom In", None), ("Zoom Extents", Some("F"))]);
    assert_slots(&view, &["Zoom Extents"], true);
    assert_slots(&view, &["Zoom All"], false);
}

/// AC 6 — View > Zoom Extents leaves the camera `F` leaves.
#[test]
fn zoom_extents_row_frames_like_f() {
    let mut menu = app_with_stray_line();
    let framed = click_view_row(&mut menu, "Zoom Extents");

    let ctx = ctx();
    let mut keyed = app_with_stray_line();
    let _ = settle(&ctx, &mut keyed);
    harness::tap(&ctx, &mut keyed, egui::Key::F, egui::Modifiers::NONE);
    assert_eq!(framed, keyed.camera, "the menu row and F frame alike");
    assert_ne!(framed.mm_per_px, 1.0, "positive control: the camera moved");
}

/// AC 7 — View > Zoom All shows the bed and the stray line together.
#[test]
fn zoom_all_frames_the_bed_and_the_drawing() {
    let mut app = app_with_stray_line();
    let bed = app.document.bed_mm;
    let camera = click_view_row(&mut app, "Zoom All");
    for w in [
        Vec2::new(-150.0, -60.0),
        Vec2::new(900.0, 700.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(bed[0], bed[1]),
    ] {
        assert!(in_view(&camera, w), "{w:?} must be in view: {camera:?}");
    }
    let mut bed_only = camera.clone();
    bed_only.frame_bed(bed);
    assert!(
        !in_view(&bed_only, Vec2::new(900.0, 700.0)),
        "positive control: Fit to Bed alone leaves the stray end out"
    );
}

/// AC 7 — with no entities Zoom All frames the bed, exactly as Fit to Bed.
#[test]
fn zoom_all_on_an_empty_document_frames_the_bed() {
    let mut app = App::default();
    let camera = click_view_row(&mut app, "Zoom All");
    let mut expected = camera.clone();
    expected.frame_bed(app.document.bed_mm);
    assert_eq!(camera, expected);
}
