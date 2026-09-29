//! The one integration-test binary (LCV-152).
//!
//! Every file under this directory is a module of this crate rather than its own
//! test binary, so the suite links against egui once instead of once per file.
//! The shared harness stays at `tests/harness/` and is compiled here exactly
//! once; each module that needs it says `use crate::harness;`.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "test code: helpers outside #[test] fns fail loudly and compare exact values"
)]

#[path = "../harness/mod.rs"]
mod harness;

mod agent;
mod app;
mod cmdline;
mod document;
mod geometry;
mod io_svg;
mod repo;
mod ui;
