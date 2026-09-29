//! Document file schema version contract.
//!
//! [`SCHEMA_VERSION`] (re-exported from [`crate::document::entity`]) names the
//! shape of the on-disk document envelope. The envelope is not committed by
//! this demand — serialization arrives with LCV-058 — but the integer is
//! pinned here so every future format-incompatible change has a single,
//! visible knob to bump.
//!
//! ## Envelope sketch (informational, not binding code)
//!
//! ```json
//! { "schema_version": 2, "layers": [ ... ], "entities": [ ... ], ... }
//! ```
//!
//! ## Bump policy
//!
//! - **Bump** when a change would prevent an older build from cleanly loading
//!   a newer file: removing a field, renaming a field, changing a field's
//!   type, changing the meaning of an existing value, or introducing a new
//!   required field without a migration path.
//! - **Do NOT bump** when a change is backward-compatible: adding a new
//!   optional field that older builds can ignore, broadening a numeric range
//!   that is already nullable, or adding a wholly new variant that older
//!   builds can refuse with a clear error (note: adding `Entity` variants is
//!   one such case; see [`crate::document::entity`] notes).
//!
//! Introduced by demand LCV-020.

pub use super::entity::SCHEMA_VERSION;
