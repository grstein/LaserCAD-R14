//! User-preference persistence: the [`Settings`] struct and its defaults.
//!
//! Reading and writing the JSON file (ADR 0006's path-injected API, the
//! `.bak` backup and the atomic write) live in `settings_store.rs` (ADR 0007
//! §D8 seam) and are re-exported here, so callers keep naming
//! `io::settings::{load_from, save_to, platform_path}`.
//!
//! ## Kernel purity
//!
//! This module MUST NOT import `egui`, `eframe`, or `rfd`.

use serde::{Deserialize, Serialize};

use crate::geometry::SnapKinds;
use crate::util::{DEFAULT_BED_HEIGHT_MM, DEFAULT_BED_WIDTH_MM, clamp_bed_mm};

pub(crate) use crate::io::settings_store::{load_from, platform_path, save_to};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by the settings *write* path.
///
/// The *read* path is infallible — it returns [`Settings::default()`] instead.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// JSON serialisation failed.
    #[error("JSON serialisation error: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem I/O failed (create dir, write tmp, rename).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Settings struct
// ---------------------------------------------------------------------------

/// Persisted user preferences.
///
/// All fields carry `#[serde(default)]` so that a settings file written by an
/// older version of the app remains loadable after new fields are added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Ordered list of recently opened file paths (most recent first).
    ///
    /// Capped at [`RECENT_FILES_CAP`] entries.  Use [`push_recent_file`] to
    /// add entries; never push directly.
    pub recent_files: Vec<String>,

    /// OpenAI-compatible API base URL.
    /// Default: `"https://openrouter.ai/api/v1"`.
    #[serde(default = "default_agent_endpoint")]
    pub agent_endpoint: String,

    /// API key sent in the `Authorization: Bearer …` header.
    /// Default: `""` (agent disabled until set).
    #[serde(default)]
    pub agent_api_key: String,

    /// Model id sent as the request body's `model` field.
    /// Default: `"anthropic/claude-sonnet-4.6"`, which matches the OpenRouter
    /// endpoint that is already the default.
    ///
    /// Needs its own serde default because the field's `Default` (an empty
    /// string) is not a model any endpoint will accept.
    #[serde(default = "default_agent_model")]
    pub agent_model: String,

    /// Tool-call dispatches one agent turn may make.
    ///
    /// Stored **verbatim**: a hand-edited file can hold anything, and the
    /// clamp lives with the loop that enforces the budget
    /// (`clamp_step_budget`), not here — `io` must not import the agent
    /// module, an edge that would invert the layering (ADR 0007 §D7).
    ///
    /// Needs its own serde default because the field's `Default` (`0`) would
    /// be a turn that cannot call a single tool. A `u32` since LCV-142 (ADR
    /// 0007 §D13); a stored `12` from an older file is honoured as is.
    #[serde(default = "default_agent_step_budget")]
    pub agent_step_budget: u32,

    /// The bed size, in millimetres `[width, height]`, that a **new** document
    /// starts at (LCV-114 AC 11).
    ///
    /// A seed, never the truth: the live bed is
    /// [`Document::bed_mm`](crate::document::Document::bed_mm). Written only
    /// by the Bed size… dialog — opening a 300 × 200 file must not re-home the
    /// operator's machine. Read it through [`Settings::clamped_default_bed_mm`]
    /// rather than directly: a hand-edited settings file can hold anything.
    ///
    /// Needs its own serde default because the field's `Default`
    /// (`[0.0, 0.0]`) would be a zero-sized bed.
    #[serde(default = "default_bed_mm")]
    pub default_bed_mm: [f64; 2],

    /// Whole-prompt override for the agent's system message (LCV-143).
    ///
    /// `None` — a missing field or an explicit `null` — means the built-in
    /// default; any string, blank included, replaces it verbatim. Stored as
    /// plain text with no knowledge of the default: resolution lives in the
    /// agent module, which `io` must not import (ADR 0007 §D7).
    #[serde(default)]
    pub agent_system_prompt: Option<String>,

    /// LCV-145 opt-in: the agent may send a picture of the drawing. Off by
    /// default; images go out only when this and
    /// [`Settings::agent_model_supports_vision`] are both on (ADR 0011).
    #[serde(default)]
    pub agent_allow_canvas_capture: bool,

    /// LCV-145 opt-in: the operator says the configured model accepts images.
    /// Never guessed from the model name. Off by default.
    #[serde(default)]
    pub agent_model_supports_vision: bool,

    /// LCV-195 opt-in: after each reply that changed the drawing, the agent
    /// is told its size and CHECK result (and shown it, when both canvas
    /// opt-ins are on too). Off by default; read live by the UI thread.
    #[serde(default)]
    pub agent_feedback_after_changes: bool,

    /// The model's context window in tokens (LCV-153); conversation memory
    /// is capped at half of it. Stored verbatim — the reader clamps with
    /// `agent::memory::clamp_context_tokens` (ADR 0007 §D7).
    #[serde(default = "default_agent_context_tokens")]
    pub agent_context_tokens: u32,

    /// Object-snap kinds enabled under View > Object snap (LCV-161). F3
    /// stays the master switch. A file predating LCV-161 loads the defaults.
    #[serde(default)]
    pub object_snaps: SnapKinds,
}

fn default_agent_endpoint() -> String {
    "https://openrouter.ai/api/v1".to_string()
}

fn default_agent_model() -> String {
    "anthropic/claude-sonnet-4.6".to_string()
}

/// The literal `256` rather than `AGENT_STEP_BUDGET_DEFAULT`: importing the
/// agent module from `io` would invert the module layering (ADR 0007 §D7). A
/// test in this file pins the two numbers to each other instead.
fn default_agent_step_budget() -> u32 {
    256
}

/// The literal for `agent::memory::CONTEXT_TOKENS_DEFAULT`, pinned by a test
/// for the same layering reason as the step budget.
fn default_agent_context_tokens() -> u32 {
    128_000
}

fn default_bed_mm() -> [f64; 2] {
    [DEFAULT_BED_WIDTH_MM, DEFAULT_BED_HEIGHT_MM]
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_files: Vec::new(),
            agent_endpoint: default_agent_endpoint(),
            agent_api_key: String::new(),
            agent_model: default_agent_model(),
            agent_step_budget: default_agent_step_budget(),
            default_bed_mm: default_bed_mm(),
            agent_system_prompt: None,
            agent_allow_canvas_capture: false,
            agent_model_supports_vision: false,
            agent_feedback_after_changes: false,
            agent_context_tokens: default_agent_context_tokens(),
            object_snaps: SnapKinds::default(),
        }
    }
}

/// Maximum number of entries kept in [`Settings::recent_files`].
const RECENT_FILES_CAP: usize = 10;

impl Settings {
    /// Insert `path` into the recent-files list.
    ///
    /// 1. Removes any existing occurrence of `path` (deduplication).
    /// 2. Inserts `path` at position 0 (most-recent first).
    /// 3. Truncates the list to [`RECENT_FILES_CAP`] entries.
    pub fn push_recent_file(&mut self, path: String) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(RECENT_FILES_CAP);
    }

    /// [`Settings::default_bed_mm`] with each axis held inside
    /// `BED_MIN_MM ..= BED_MAX_MM` (LCV-114 AC 11).
    ///
    /// The stored value comes from a JSON file the operator can edit, so every
    /// reader — `File > New` and the cold-boot seed — goes through this rather
    /// than trusting the field.
    pub fn clamped_default_bed_mm(&self) -> [f64; 2] {
        [
            clamp_bed_mm(self.default_bed_mm[0]),
            clamp_bed_mm(self.default_bed_mm[1]),
        ]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
