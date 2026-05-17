//! Persistence: settings, autosave, recent files, file dialogs, and SVG (see
//! [`svg`] submodule).
//!
//! `io::svg` is part of the kernel and MUST NOT import UI deps; the rest of
//! `io` (dialogs, recent, etc.) may use `rfd` and `directories`.
//!
//! Submodules arrive with demands LCV-055 .. LCV-062.

pub mod svg;
