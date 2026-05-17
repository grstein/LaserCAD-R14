//! Integration test verifying that all public modules are correctly wired and importable.
//! This test validates AC#6 and AC#7 of LCV-001.

#[test]
fn module_tree_is_wired() {
    // Pure compile-time check: every top-level module re-exported by lib.rs
    // must be addressable. The test body uses the module items to force
    // the compiler to resolve all imports and module paths.
    use lasercad::{agent, app, document, geometry, io, render, text, tools, ui, util};

    // Force the compiler to resolve each module by accessing a type or constant.
    // Each module's mod.rs defines a `pub const MODULE: &str` for this purpose.
    let _ = (
        &agent::MODULE,
        &app::MODULE,
        &document::MODULE,
        &geometry::MODULE,
        &io::MODULE,
        &render::MODULE,
        &text::MODULE,
        &tools::MODULE,
        &ui::MODULE,
        &util::MODULE,
    );
}
