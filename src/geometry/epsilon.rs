//! Kernel epsilon constant for length and area comparisons.
//!
//! See [`EPSILON`] for the rationale. UI-side tolerances (pixel snap radius,
//! hover hit-testing) are NOT this value — they live in `render/` and are
//! expressed in screen pixels.

/// Canonical kernel epsilon for length and area comparisons.
///
/// Expressed in the kernel's units: millimeters (mm) for lengths,
/// square millimeters (mm²) for areas. `1e-9 mm` sits well above
/// `f64`'s mantissa precision (~2e-16 relative) on bed-sized coordinates
/// (~1 m = 1000 mm), giving ~10 orders of magnitude of headroom for
/// accumulated rounding from typical CAD operations (translate, rotate,
/// intersect).
///
/// UI-side tolerances — pixel snap radius, hover hit-testing — are NOT
/// this epsilon. They live in `render/` and are expressed in screen pixels
/// (a viewport-dependent quantity), not in mm.
///
/// Frozen by demand LCV-010 as the Phase-1 default. A later demand that
/// finds this value too tight or too loose owns both the constant change
/// and the migration of every call site that imported it.
pub const EPSILON: f64 = 1e-9;
