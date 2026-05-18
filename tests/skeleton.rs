//! Integration test verifying that all public modules are correctly wired and importable.
//! This test validates AC#6 and AC#7 of LCV-001.

#[test]
fn module_tree_is_wired() {
    // Pure compile-time check: every top-level module re-exported by lib.rs
    // must be addressable. The test body uses the module items to force
    // the compiler to resolve all imports and module paths.
    //
    // `render` is not referenced here yet — LCV-030 retired its
    // `pub const MODULE` placeholder and the module carries no public
    // surface until LCV-031 (Camera) lands. The `lib.rs` `pub mod render;`
    // declaration plus the binary build itself prove the module compiles
    // and is reachable; once a public item exists (Camera, Viewport, …)
    // this test grows a witness for it.
    use lasercad::{agent, app, document, geometry, io, text, tools, ui, util};

    // Each module is witnessed by an addressable public item. Modules that
    // still carry the LCV-001 `pub const MODULE` placeholder use it;
    // `app`, after LCV-030, uses a real item (`App`) instead. The
    // placeholders retire as each module's owning demand lands.
    let _ = (
        &agent::MODULE,
        std::any::TypeId::of::<app::App>(),
        &document::SCHEMA_VERSION,
        &geometry::EPSILON,
        &io::MODULE,
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
