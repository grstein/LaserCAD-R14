//! The `CentralPanel` viewport: painting, pointer routing, and camera actions.
//!
//! [`draw`] is the whole canvas phase of a frame — camera sync, the render
//! pipeline (grid, bed, entities, selection, preview, snap marker), pointer
//! events into the active tool, wheel zoom and middle-drag pan. It is called
//! once per frame from [`App::update_ui`](super::App::update_ui).
//!
//! No key is read here: `src/app/input.rs` is the single keyboard gate
//! (LCV-103 / ADR 0002 §A6). A `ctx.input(|i| i.key_*)` call in this file is a
//! double dispatch (defect D4) and a review blocker.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::{apply_ortho, resolve_snap, App};
use crate::document::Document;
use crate::render::Camera;
use crate::tools::{PointerButton, PointerEvent};

/// Factor applied per mouse-wheel notch. `> 1.0` zooms in; `< 1.0` zooms out.
const WHEEL_ZOOM_FACTOR: f64 = 1.1;

/// Render the canvas and route pointer input for one frame.
pub fn draw(ctx: &egui::Context, app: &mut App) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

        // Sync the camera's viewport size before any draw call consumes it.
        app.camera.viewport_size_px = [rect.width(), rect.height()];

        paint(ui, rect, app);

        // --- pointer / camera interaction (LCV-032 / LCV-041) ---
        if response.hovered() {
            if let Some(hover_pos) = response.hover_pos() {
                handle_hover(ctx, app, rect, hover_pos);
            }
        }

        // Middle-button pan.
        if response.dragged_by(egui::PointerButton::Middle) {
            handle_pan(&mut app.camera, response.drag_delta());
        }

        // Ask for a follow-up frame only while the canvas is live — see
        // `viewport_is_live` and AGENTS.md §Event flow → Repaint policy. The
        // status-bar coordinates are unaffected: `last_cursor_world` is written
        // only inside `handle_hover`, which already runs under
        // `response.hovered()`.
        if viewport_is_live(&response, app) {
            ctx.request_repaint();
        }
    });
}

/// Is anything on the canvas moving, so that this frame needs a successor?
///
/// Exactly three terms and no fourth (LCV-120): the pointer is over the canvas,
/// a drag is in progress — `dragged()` covers every button, including the
/// middle-button pan handled just above the call — or a tool preview is on
/// screen. Deliberately **not** `ToolManager::anchor`: a tool that is armed and
/// waiting for its first click has nothing moving, and waking on it would keep
/// the app running for most of a session.
///
/// Called after [`paint`], which is what assigns `app.preview_entities` from
/// the active tool, so the third term reads this frame's preview rather than
/// the previous one's.
fn viewport_is_live(response: &egui::Response, app: &App) -> bool {
    response.hovered() || response.dragged() || !app.preview_entities.is_empty()
}

/// Paint the canvas background and the whole render pipeline into `rect`.
///
/// Paint order (LCV-137 AC 1): canvas background, bed background fill, the
/// grid (when enabled), the bed border and exterior overlay, entities,
/// selection, preview, snap marker. The bed's fill is painted BEFORE the
/// grid and its border/overlay AFTER, so the grid's lines land on top of the
/// fill and are visible inside the bed rather than painted over by it — the
/// two halves of what used to be one `draw_bed` call, split for this order
/// (LCV-137 AC 2, `src/render/bed.rs`).
fn paint(ui: &egui::Ui, rect: egui::Rect, app: &mut App) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, crate::ui::CANVAS_BG);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(64)),
    );

    // The bed is the document's, rebuilt every frame (LCV-114 AC 4/AC 15):
    // no cached copy, so a `SetBedSize` shows up on the very next frame.
    let bed = crate::render::Bed::from_size_mm(app.document.bed_mm);
    crate::render::draw_bed_fill(&painter, rect, &app.camera, &bed);

    if app.grid_enabled {
        crate::render::draw_grid(&painter, rect, &app.camera);
    }

    crate::render::draw_bed(&painter, rect, &app.camera, &bed);
    crate::render::draw_entities(
        &painter,
        rect,
        &app.camera,
        &app.document.entities,
        crate::render::PaintOptions::default(),
    );
    crate::render::draw_selection_highlight(
        &painter,
        rect,
        &app.camera,
        &app.document.entities,
        &app.document.selection,
    );

    // Update preview from tool (LCV-040 AC#9).
    app.preview_entities = app.tool_manager.preview();

    crate::render::draw_preview(&painter, rect, &app.camera, &app.preview_entities);

    if let Some(snap) = &app.active_snap {
        crate::render::draw_snap_marker(&painter, rect, &app.camera, snap);
    }
}

/// Resolve the cursor position and route pointer events while the viewport is
/// hovered: snap, ortho lock, press / move / release, and wheel zoom.
fn handle_hover(ctx: &egui::Context, app: &mut App, rect: egui::Rect, hover_pos: egui::Pos2) {
    // `hover_pos` is global (the whole window); `rect.min` is the viewport's
    // own origin, which is nonzero whenever a panel claims space before the
    // `CentralPanel` — always, since the menubar and toolbar always do, and
    // further when the agent panel is open. Every camera call below wants
    // the viewport-local point, so the subtraction happens exactly once,
    // here, and every caller below (snap, world_pos, wheel zoom) reuses the
    // one result (LCV-137 AC 3).
    let local_pos = hover_pos - rect.min.to_vec2();

    // `resolve_snap` takes the global `hover_pos` plus `rect` and does this
    // same subtraction internally.
    if app.snap_enabled {
        app.active_snap = resolve_snap(hover_pos, rect, &app.camera, &app.document.entities);
    }
    let world_pos = app
        .active_snap
        .map(|s| s.point)
        .unwrap_or_else(|| app.camera.screen_to_world(local_pos));
    // Ortho lock (LCV-053): clamp to nearest cardinal axis from the active
    // tool's anchor, when ortho mode is active.
    let world_pos = if app.ortho_enabled {
        match app.tool_manager.anchor() {
            Some(anchor) => apply_ortho(anchor, world_pos),
            None => world_pos,
        }
    } else {
        world_pos
    };
    app.last_cursor_world = Some(world_pos);

    // Pointer events (LCV-041).
    if ctx.input(|i| i.pointer.primary_pressed()) {
        let shift = ctx.input(|i| i.modifiers.shift);
        send_pointer(
            app,
            PointerEvent::Press {
                world_pos,
                button: PointerButton::Primary,
                shift,
            },
        );
        // LCV-049: poll for a successor tool after press events.
        poll_successor(app);
    }
    send_pointer(app, PointerEvent::Move { world_pos });
    if ctx.input(|i| i.pointer.primary_released()) {
        let shift = ctx.input(|i| i.modifiers.shift);
        send_pointer(
            app,
            PointerEvent::Release {
                world_pos,
                button: PointerButton::Primary,
                shift,
            },
        );
        // LCV-049: poll for a successor tool after release events.
        poll_successor(app);
    }

    // Wheel zoom around cursor. `zoom_around` (reached through
    // `handle_wheel_zoom`) treats its anchor as viewport-local, so this
    // passes `local_pos` — computed once, above — never the global
    // `hover_pos` (LCV-137 AC 3/AC 4).
    let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
    if scroll_y != 0.0 {
        let factor = if scroll_y > 0.0 {
            WHEEL_ZOOM_FACTOR
        } else {
            1.0 / WHEEL_ZOOM_FACTOR
        };
        handle_wheel_zoom(&mut app.camera, local_pos, factor);
    }
}

/// Hand one pointer event to the active tool.
fn send_pointer(app: &mut App, event: PointerEvent) {
    app.tool_manager
        .on_pointer_event(&event, &mut app.document, &mut app.history);
}

/// Honour a tool succession request (LCV-049): a tool that finished may name
/// the tool that replaces it. Polled after press and release, never after a
/// move — and, since LCV-111, after a consumed command-line input, so the
/// typed path hands over exactly like the pointer path (`pub(super)` for
/// `super::cmdline`; there is deliberately only one copy of this body).
pub(super) fn poll_successor(app: &mut App) {
    if let Some(t) = app.tool_manager.take_successor() {
        app.tool_manager.set_tool(t);
    }
}

/// Apply a wheel-zoom step to `camera` anchored at `screen_anchor`.
pub fn handle_wheel_zoom(camera: &mut Camera, screen_anchor: egui::Pos2, factor: f64) {
    camera.zoom_around(screen_anchor, factor);
}

/// Pan the camera by a screen-space delta (middle-drag).
pub fn handle_pan(camera: &mut Camera, delta_screen_px: egui::Vec2) {
    camera.pan(delta_screen_px);
}

/// Fit the document bounds to the viewport.
///
/// No-ops on a zero-area viewport (LCV-103). `Camera::default()` carries the
/// `[0.0, 0.0]` sentinel until the first `CentralPanel` syncs a real size, and
/// the keyboard gate runs before that panel — zooming to a zero-area rect
/// would reset the camera for no reason.
pub fn handle_zoom_extents(camera: &mut Camera, document: &Document, viewport_size: [f32; 2]) {
    if viewport_size[0] <= 0.0 || viewport_size[1] <= 0.0 {
        return;
    }
    camera.zoom_extents(document.bounds(), viewport_size);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    /// The source above the bare `#[cfg(test)]` at column 0 — the only slice a
    /// source scan may read (ADR 0004's boundary, reused here). Bounding every
    /// haystack this way is what stops a scan from matching the needle literal
    /// in its own assertion and passing vacuously. Needles are built with
    /// `concat!` on top of that, so the same text never appears verbatim in
    /// both halves of a check.
    fn implementation_of(src: &str) -> &str {
        let at = src
            .find("\n#[cfg(test)]")
            .expect("a scanned file must have a bare #[cfg(test)] to bound the scan");
        &src[..at]
    }

    /// Same boundary, for files that may carry no inline test module at all.
    fn implementation_or_all(src: &str) -> &str {
        match src.find("\n#[cfg(test)]") {
            Some(at) => &src[..at],
            None => src,
        }
    }

    /// Every `.rs` file under `dir`, recursively.
    fn rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("the source tree must be readable") {
            let path = entry.expect("a readable directory entry").path();
            if path.is_dir() {
                rs_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// LCV-120 AC 1 — the predicate is private, named, and has exactly three
    /// terms in the agreed order, with no fourth on the tool anchor.
    ///
    /// The `anchor` absence assertion is the one that could pass vacuously, so
    /// it carries its own discrimination proof: the same needle *is* present in
    /// the file at large (`handle_hover` reads `tool_manager.anchor()`), which
    /// shows the scan would find it if it had been added to the predicate.
    #[test]
    fn the_live_predicate_has_exactly_three_terms() {
        let implementation = implementation_of(include_str!("viewport.rs"));
        let signature = concat!(
            "fn viewport_is_",
            "live(response: &egui::Response, app: &App) -> bool {"
        );
        let body = implementation
            .split_once(signature)
            .expect("AC 1: viewport_is_live must exist with that exact signature")
            .1
            .split_once("\n}")
            .expect("the predicate must close")
            .0;
        assert!(
            !body.trim().is_empty(),
            "positive control: the scanned predicate body must not be empty"
        );
        assert!(
            !implementation.contains(concat!("pub fn viewport_is_", "live")),
            "AC 1: the predicate is private"
        );

        let hovered = body
            .find(concat!("response.", "hovered()"))
            .expect("AC 1: term 1 is response.hovered()");
        let dragged = body
            .find(concat!("response.", "dragged()"))
            .expect("AC 1: term 2 is response.dragged(), which covers every button");
        let preview = body
            .find(concat!("!app.preview_", "entities.is_empty()"))
            .expect("AC 1: term 3 is the live tool preview");
        assert!(
            hovered < dragged && dragged < preview,
            "AC 1: the three terms must appear in that order"
        );
        assert_eq!(
            body.matches("||").count(),
            2,
            "AC 1: exactly three terms, no fourth"
        );

        let anchor = concat!("anch", "or()");
        assert!(
            !body.contains(anchor),
            "AC 1: no fourth term on the tool anchor — an armed tool has nothing moving"
        );
        assert!(
            implementation.contains(anchor),
            "discrimination control: the anchor needle is findable in this file, \
             so its absence from the predicate is a real absence"
        );
    }

    /// LCV-120 AC 2 — the repaint request is inside the guard, and the guard
    /// sits after `paint` (so `preview_entities` is this frame's) and after the
    /// middle-drag pan block.
    #[test]
    fn the_canvas_repaint_is_guarded_by_the_predicate() {
        let implementation = implementation_of(include_str!("viewport.rs"));
        let draw = implementation
            .split_once("pub fn draw(")
            .expect("draw must exist")
            .1
            .split_once("\n}\n")
            .expect("draw must close")
            .0;

        let guard = draw
            .find(concat!("if viewport_is_", "live(&response, app) {"))
            .expect("AC 2: the repaint must be guarded by the named predicate");
        let call = draw
            .find(concat!("ctx.request_", "repaint();"))
            .expect("positive control: draw must still ask for frames at all");
        assert!(guard < call, "AC 2: the guard must precede the request");
        assert_eq!(
            implementation
                .matches(concat!("ctx.request_", "repaint"))
                .count(),
            1,
            "AC 2: exactly one repaint call site in this file"
        );

        let paint = draw
            .find("paint(ui, rect, app);")
            .expect("positive control: draw must paint");
        let pan = draw
            .find("handle_pan(&mut app.camera, response.drag_delta());")
            .expect("positive control: draw must handle the middle-drag pan");
        assert!(
            paint < guard && pan < guard,
            "AC 2: the guard reads this frame's preview and this frame's drag"
        );
    }

    /// LCV-120 AC 3 — the comment no longer justifies the repaint with a
    /// feature that does not exist. `Camera::pan` / `zoom_around` /
    /// `zoom_extents` are all instantaneous; there is no animation in this tree.
    #[test]
    fn the_repaint_comment_claims_no_animation() {
        let implementation = implementation_of(include_str!("viewport.rs"));
        assert!(
            implementation.contains(concat!("follow-up ", "frame")),
            "positive control: the replacement comment must be in the scanned slice"
        );
        for claim in [
            concat!("smooth ", "camera"),
            concat!("anim", "at"),
            concat!("Always ", "repaint"),
        ] {
            assert!(
                !implementation.contains(claim),
                "AC 3: the comment must not claim {claim:?} — no such feature exists"
            );
        }
    }

    /// LCV-120 AC 8 — every `ctx.request_repaint*` call in `src/` is inside an
    /// `if`, and there are exactly three of them: the agent turn in flight, the
    /// pending autosave write, and the live canvas.
    ///
    /// The walk is over the real tree rather than three `include_str!`s so a
    /// fourth site added in a fourth file is caught too.
    #[test]
    fn every_repaint_request_in_src_is_conditional() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        rs_files(&root, &mut files);
        files.sort();
        assert!(
            files.len() > 30,
            "positive control: the walk must see the whole tree, saw {}",
            files.len()
        );

        // AC 8's needle, deliberately un-prefixed: `ui.ctx().request_repaint()`
        // is the natural spelling inside a panel closure — which is exactly
        // where future frame work lands — and a `ctx.`-prefixed needle walks
        // straight past it. Comment lines are skipped so the broadened needle
        // does not match doc prose such as `src/app/autosave.rs`'s header.
        let needle = concat!("request_", "repaint");
        let mut sites = Vec::new();
        for path in &files {
            let src = std::fs::read_to_string(path).expect("a readable source file");
            let lines: Vec<&str> = implementation_or_all(&src).lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if !line.contains(needle) || line.trim().starts_with("//") {
                    continue;
                }
                // The nearest preceding statement, comments and blanks skipped.
                // The accepted shape is deliberately narrow: the guard must be
                // the `if` that opens the block the call sits in. A correctly
                // guarded call that is not the first statement of its block
                // fails this — on purpose, because the bias here is to fail and
                // be looked at rather than to wave a fourth site through.
                let guard = lines[..i]
                    .iter()
                    .rev()
                    .map(|l| l.trim())
                    .find(|l| !l.is_empty() && !l.starts_with("//"))
                    .unwrap_or("");
                // Forward slashes on every platform: rendering the whole path
                // with `to_string_lossy` or `Path::display` emits `\` on
                // Windows and turns the comparison below into a CI-only
                // failure (AGENTS.md §Implementation Rules; LCV-116 fixed the
                // same trap at `900f0c7`).
                let relative = path
                    .strip_prefix(&root)
                    .expect("every walked file is under src/")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                sites.push((relative, guard.to_string()));
            }
        }
        // Sorted on the rendered path, so the order cannot depend on where the
        // platform separator sorts relative to the file names either.
        sites.sort();

        let files: Vec<&str> = sites.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(
            files,
            ["app/autosave.rs", "app/mod.rs", "app/viewport.rs"],
            "AC 8: exactly three repaint call sites live in src/"
        );
        for (file, guard) in &sites {
            assert!(
                guard.starts_with("if ") && guard.ends_with('{'),
                "AC 8: the repaint in {file} must sit inside an `if`, found {guard:?}"
            );
        }
    }

    /// LCV-114 AC 4/AC 15 — the canvas builds its bed from the document
    /// every frame and keeps no copy, so a `SetBedSize` is visible on the
    /// next frame with no cache to invalidate.
    ///
    /// A bounded source scan: `draw_viewport` needs a live `egui::Ui` and a
    /// painter, and the property at stake is *where the size comes from*.
    /// Each claim carries a positive control over the same slice, so an
    /// absence assertion cannot pass vacuously.
    #[test]
    fn the_canvas_bed_comes_from_the_document() {
        let src = include_str!("viewport.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("viewport.rs must have a test module to bound the scan");
        let implementation = &src[..cfg_test_at];
        assert!(
            implementation.contains("crate::render::draw_bed(&painter, rect, &app.camera, &bed)"),
            "positive control: the canvas must draw the bed"
        );
        assert!(
            implementation.contains("crate::render::Bed::from_size_mm(app.document.bed_mm)"),
            "the drawn bed must be built from the document (AC 4)"
        );
        assert!(
            !implementation.contains("app.bed"),
            "there is no App-owned bed to read (AC 4)"
        );
    }

    /// LCV-032 AC#8 — wheel zoom helper with positive factor zooms in.
    #[test]
    fn wheel_zoom_dispatch_positive_scroll_zooms_in() {
        let mut cam = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.1);
        assert!(cam.mm_per_px < 1.0);
    }

    /// LCV-032 AC#8 — wheel zoom helper with factor < 1 zooms out.
    #[test]
    fn wheel_zoom_dispatch_negative_scroll_zooms_out() {
        let mut cam = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.0 / 1.1);
        assert!(cam.mm_per_px > 1.0);
    }

    /// LCV-032 AC#8 — wheel zoom helper with factor 1.0 is a no-op.
    #[test]
    fn wheel_zoom_dispatch_unity_scroll_is_noop() {
        let mut cam = Camera {
            center_world: Vec2::new(0.0, 0.0),
            mm_per_px: 1.0,
            viewport_size_px: [800.0, 600.0],
        };
        handle_wheel_zoom(&mut cam, egui::Pos2::new(400.0, 300.0), 1.0);
        assert!((cam.mm_per_px - 1.0).abs() < 1e-12);
    }

    /// LCV-032 AC#8 — zoom extents dispatch resets on empty document.
    #[test]
    fn zoom_extents_dispatch_on_empty_document_resets() {
        let mut cam = Camera {
            center_world: Vec2::new(7.0, -3.0),
            mm_per_px: 4.0,
            viewport_size_px: [10.0, 10.0],
        };
        let doc = Document::default();
        handle_zoom_extents(&mut cam, &doc, [800.0, 600.0]);
        assert_eq!(cam.center_world, Vec2::new(0.0, 0.0));
        assert_eq!(cam.mm_per_px, 1.0);
        assert_eq!(cam.viewport_size_px, [800.0, 600.0]);
    }

    /// LCV-103 AC#14 — `handle_zoom_extents` no-ops on a zero-area viewport.
    ///
    /// The keyboard gate runs before the `CentralPanel`, so on frame 0 it
    /// reads the `[0.0, 0.0]` sentinel of a fresh `Camera`. Pressing `F`
    /// then must leave the camera exactly as it was.
    #[test]
    fn zoom_extents_noop_on_zero_area_viewport() {
        let mut cam = Camera {
            center_world: Vec2::new(7.0, -3.0),
            mm_per_px: 4.0,
            viewport_size_px: [0.0, 0.0],
        };
        let doc = Document::default();
        handle_zoom_extents(&mut cam, &doc, [0.0, 0.0]);
        assert_eq!(cam.center_world, Vec2::new(7.0, -3.0));
        assert_eq!(cam.mm_per_px, 4.0);
        assert_eq!(cam.viewport_size_px, [0.0, 0.0]);

        // A viewport with one zero dimension is equally degenerate.
        handle_zoom_extents(&mut cam, &doc, [800.0, 0.0]);
        assert_eq!(cam.mm_per_px, 4.0);
    }

    /// A painted shape, reduced to the geometry these tests may assert on —
    /// never color (AGENTS.md): a rect's screen rectangle plus its stroke
    /// width (`0.0` for a filled rect, `> 0.0` for a stroked one — `paint`'s
    /// two calls, `rect_filled` / `rect_stroke`, each set exactly one of
    /// `fill` / `stroke` and default the other to
    /// `Stroke`/`Color32::TRANSPARENT`), or a line segment (identity only —
    /// `draw_grid` is the only caller in `paint` that emits one on this
    /// fixture).
    #[derive(Debug, Clone, Copy)]
    enum Kind {
        Rect(egui::Rect, f32),
        Line,
    }

    /// Flatten `shape` into `out`, recursing into `Shape::Vec` — the only
    /// shape variant at egui 0.29.1 that contains other shapes (mirrors
    /// `tests/harness/paint.rs::collect_text`'s identical recursion for
    /// text runs).
    fn flatten_shape(shape: &egui::Shape, out: &mut Vec<Kind>) {
        match shape {
            egui::Shape::Rect(r) => out.push(Kind::Rect(r.rect, r.stroke.width)),
            egui::Shape::LineSegment { .. } => out.push(Kind::Line),
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| flatten_shape(s, out)),
            _ => {}
        }
    }

    /// Two rects are "the same" up to sub-point float noise.
    fn close_rect(a: egui::Rect, b: egui::Rect) -> bool {
        (a.min.x - b.min.x).abs() < 0.01
            && (a.min.y - b.min.y).abs() < 0.01
            && (a.max.x - b.max.x).abs() < 0.01
            && (a.max.y - b.max.y).abs() < 0.01
    }

    /// LCV-137 AC 1/AC 2 — paint order inside the private `paint()`: canvas
    /// background, bed fill, the grid (when enabled), the bed border plus
    /// exterior overlay, then — on this fixture's empty document, empty
    /// selection and absent snap — nothing else. Toggling `grid_enabled`
    /// off removes the grid lines and nothing else.
    ///
    /// `paint` is called directly (it is private, and this is its own
    /// module) with a `rect` whose origin is nonzero — modelling a window
    /// where the menubar and toolbar have already claimed their share, per
    /// this demand's own rule that a same-origin rect cannot see a
    /// global/local mixup. Every shape is identified by computed screen
    /// geometry, never by color: the bed's fill/border share one
    /// independently-computed rect (`bed_rect` below, from `Bed::corners`
    /// and `Camera::world_to_screen` — the same formula `paint` itself
    /// uses), distinguished from each other only by `stroke.width`, and the
    /// bed/rect/overlay sizes are chosen so none of them collides with any
    /// other by coincidence. Scoped to shapes whose `clip_rect` matches
    /// `rect` exactly (`ui.painter_at(rect)`'s own clip), which is what
    /// keeps chrome shapes — there are none in this bare-`ctx.run` fixture,
    /// but the same scoping is what makes this safe to run inside the real
    /// `App::update_ui` too — out of the count.
    ///
    /// A control that fails when `paint`'s call order is reverted (bed fill
    /// after the grid — bug #1), or when the grid/bed split is undone (a
    /// mutation tried and confirmed to fail this test during review).
    #[test]
    fn ac1_ac2_grid_paints_between_bed_fill_and_bed_border_and_toggles_off() {
        let rect =
            egui::Rect::from_min_size(egui::Pos2::new(50.0, 30.0), egui::Vec2::new(400.0, 300.0));

        let mut rects_by_toggle: Vec<Vec<(egui::Rect, f32)>> = Vec::new();
        for grid_enabled in [true, false] {
            let mut app = App {
                grid_enabled,
                ..App::default()
            };
            app.document.bed_mm = [100.0, 50.0];
            app.camera = Camera {
                center_world: Vec2::new(50.0, 25.0),
                mm_per_px: 1.0,
                viewport_size_px: [rect.width(), rect.height()],
            };
            assert!(
                app.document.entities.is_empty()
                    && app.document.selection.is_empty()
                    && app.active_snap.is_none(),
                "positive control: nothing but bed/grid may paint in this fixture"
            );

            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(1.0);
            let out = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| paint(ui, rect, &mut app));
            });

            let mut kinds: Vec<Kind> = Vec::new();
            for clipped in &out.shapes {
                if close_rect(clipped.clip_rect, rect) {
                    flatten_shape(&clipped.shape, &mut kinds);
                }
            }

            let bed = crate::render::Bed::from_size_mm(app.document.bed_mm);
            let corners = bed.corners();
            let offset = rect.min.to_vec2();
            let bed_rect = egui::Rect::from_two_pos(
                app.camera.world_to_screen(corners[0]) + offset,
                app.camera.world_to_screen(corners[2]) + offset,
            );

            let bed_fill_idx = kinds
                .iter()
                .position(
                    |k| matches!(k, Kind::Rect(r, w) if close_rect(*r, bed_rect) && *w == 0.0),
                )
                .expect("AC 1: the bed's fill must be painted at the bed's own screen rect");
            let bed_border_idx = kinds
                .iter()
                .position(|k| matches!(k, Kind::Rect(r, w) if close_rect(*r, bed_rect) && *w > 0.0))
                .expect("AC 1: the bed's border must be painted at the bed's own screen rect");
            let line_indices: Vec<usize> = kinds
                .iter()
                .enumerate()
                .filter(|(_, k)| matches!(k, Kind::Line))
                .map(|(i, _)| i)
                .collect();

            if grid_enabled {
                let first_line = *line_indices
                    .first()
                    .expect("AC 1: the grid must paint lines when grid_enabled is true");
                let last_line = *line_indices.last().expect("checked non-empty above");
                assert!(
                    bed_fill_idx < first_line,
                    "AC 1: the bed fill must paint before every grid line"
                );
                assert!(
                    last_line < bed_border_idx,
                    "AC 1: the bed border must paint after every grid line"
                );
            } else {
                assert!(
                    line_indices.is_empty(),
                    "AC 2: no grid lines must paint when grid_enabled is false"
                );
                assert!(
                    bed_fill_idx < bed_border_idx,
                    "AC 2: the bed fill/border order is unaffected by the grid toggle"
                );
            }

            let rects_only: Vec<(egui::Rect, f32)> = kinds
                .iter()
                .filter_map(|k| match k {
                    Kind::Rect(r, w) => Some((*r, *w)),
                    Kind::Line => None,
                })
                .collect();
            rects_by_toggle.push(rects_only);
        }

        // AC 2's other half: every non-grid shape — canvas background, bed
        // fill, bed border, the exterior overlay strips — is identical,
        // in the same emission order, whether or not the grid is enabled.
        assert_eq!(
            rects_by_toggle[0].len(),
            rects_by_toggle[1].len(),
            "AC 2: the same set of non-grid shapes must paint whether or not \
             the grid is enabled"
        );
        for (with_grid, without_grid) in rects_by_toggle[0].iter().zip(&rects_by_toggle[1]) {
            assert!(
                close_rect(with_grid.0, without_grid.0)
                    && (with_grid.1 - without_grid.1).abs() < 1e-6,
                "AC 2: {with_grid:?} != {without_grid:?} across the grid toggle"
            );
        }
    }

    /// LCV-137 AC 3 — `handle_hover`'s wheel-zoom call site subtracts the
    /// viewport's origin (`rect.min`) from the global `hover_pos` exactly
    /// once, before the result ever reaches `handle_wheel_zoom` /
    /// `Camera::zoom_around` — the same pattern `resolve_snap` and the
    /// `world_pos` line right above it already use. Mirrors the style of
    /// `the_live_predicate_has_exactly_three_terms`.
    #[test]
    fn ac3_wheel_zoom_call_site_subtracts_the_viewport_origin_source_scan() {
        let implementation = implementation_of(include_str!("viewport.rs"));
        let signature = "fn handle_hover(ctx: &egui::Context, app: &mut App, rect: egui::Rect, hover_pos: egui::Pos2) {";
        let body = implementation
            .split_once(signature)
            .expect("AC 3: handle_hover must exist with that exact signature")
            .1
            .split_once("\n}\n")
            .expect("handle_hover must close")
            .0;

        let subtract_at = body
            .find("hover_pos - rect.min.to_vec2()")
            .expect("AC 3: handle_hover must subtract rect.min from hover_pos");
        let call_at = body
            .find("handle_wheel_zoom(&mut app.camera, local_pos, factor)")
            .expect("AC 3: handle_wheel_zoom must be called with the subtracted anchor");
        assert!(
            subtract_at < call_at,
            "AC 3: the subtraction must happen before the call into handle_wheel_zoom"
        );
        assert_eq!(
            body.matches("hover_pos - rect.min.to_vec2()").count(),
            1,
            "AC 3: exactly one subtraction site — computed once, reused for \
             snap, world_pos and the wheel-zoom anchor alike"
        );
        assert!(
            !body.contains("handle_wheel_zoom(&mut app.camera, hover_pos, factor)"),
            "AC 3: the wheel-zoom anchor must never be the raw global hover_pos"
        );
    }
}
