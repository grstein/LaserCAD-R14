//! Integration test verifying that all public modules are correctly wired and importable.
//! This test validates AC#6 and AC#7 of LCV-001.

#[test]
fn module_tree_is_wired() {
    // Pure compile-time check: every top-level module re-exported by lib.rs
    // must be addressable. The test body uses the module items to force
    // the compiler to resolve all imports and module paths.
    //
    // Every module is witnessed by a *real* public item. The LCV-001
    // `pub const MODULE` placeholders retired with LCV-105 — the demands that
    // own `agent`, `io`, `tools` and `util` have all landed, so each of them
    // now has genuine public surface to point at.
    use lasercad::{agent, app, cmdline, document, geometry, io, render, text, tools, ui, util};

    let _ = (
        std::any::TypeId::of::<agent::AgentEvent>(),
        std::any::TypeId::of::<app::App>(),
        // `cmdline` added by LCV-110 — the command-line parser kernel.
        std::any::TypeId::of::<cmdline::CommandHistory>(),
        &document::SCHEMA_VERSION,
        &geometry::EPSILON,
        std::any::TypeId::of::<io::ImportedSvg>(),
        std::any::TypeId::of::<render::Camera>(),
        &text::CAP_HEIGHT_HERSHEY,
        std::any::TypeId::of::<tools::ToolManager>(),
        ui::CANVAS_BG,
        &util::DEFAULT_BED_WIDTH_MM,
    );
}

/// LCV-021 AC#8 — `Document` and `Selection` are re-exported through
/// `document::mod` so external callers reach them as
/// `lasercad::document::{Document, Selection}` without a deep path.
#[test]
fn document_and_selection_reexported() {
    use lasercad::document::{Document, Selection};

    let doc = Document::default();
    assert_eq!(doc.entity_count(), 0);
    assert!(doc.bounds().is_none());

    // `Selection` is reachable and default-constructible — placeholder body
    // is owned by LCV-027.
    let _ = Selection::default();
}

/// LCV-038 AC#11 — `App` carries `active_snap: Option<SnapResult>` defaulting
/// to `None`.
#[test]
fn app_default_has_no_active_snap() {
    let app = lasercad::app::App::default();
    assert!(app.active_snap.is_none());
}
