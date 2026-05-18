//! Integration test verifying that all public modules are correctly wired and importable.
//! This test validates AC#6 and AC#7 of LCV-001.

#[test]
fn module_tree_is_wired() {
    // Pure compile-time check: every top-level module re-exported by lib.rs
    // must be addressable. The test body uses the module items to force
    // the compiler to resolve all imports and module paths.
    //
    // `render`'s witness, after LCV-031, is the [`render::Camera`] type —
    // the first real public surface in the module. The previous LCV-030
    // gap is now closed.
    use lasercad::{agent, app, document, geometry, io, render, text, tools, ui, util};

    // Each module is witnessed by an addressable public item. Modules that
    // still carry the LCV-001 `pub const MODULE` placeholder use it;
    // `app`, after LCV-030, uses a real item (`App`); `render`, after
    // LCV-031, uses `Camera`. Placeholders retire as each module's owning
    // demand lands.
    let _ = (
        &agent::MODULE,
        std::any::TypeId::of::<app::App>(),
        &document::SCHEMA_VERSION,
        &geometry::EPSILON,
        &io::MODULE,
        std::any::TypeId::of::<render::Camera>(),
        &text::MODULE,
        &tools::MODULE,
        &ui::MODULE,
        &util::MODULE,
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
