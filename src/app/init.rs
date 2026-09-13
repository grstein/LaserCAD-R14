//! The two [`App`] constructors (LCV-115 file split).
//!
//! Moved verbatim out of `src/app/mod.rs`, which was at 296 of the 300-LOC
//! implementation cap and could not absorb LCV-115's `export_preset` field
//! otherwise (ADR 0004 leaves this file's seam to the demand that crosses).
//! Pure relocation: no behaviour change.
//!
//! The pair belongs together and is the half of `mod.rs` that could move:
//! ADR 0002 §A2 defines them *as* a pair — [`App::default`] is the **test**
//! constructor and touches no filesystem, [`App::new`] is **boot-only** and
//! reads the platform config and data directories — and they are also what
//! grows when `struct App` grows, since every new field adds an initialiser
//! here. The struct itself stays in `mod.rs` with the module's public
//! surface, where it cannot move without moving [`App`].
//!
//! The `app_default_*` value assertions stay in `mod.rs`'s test module, next
//! to the field declarations whose defaults they mirror. The one test that
//! reads `App::new`'s own source moved here with the function.
//!
//! MUST NOT import `eframe` or `rfd`.

use super::App;
use crate::cmdline::CommandHistory;
use crate::document::{Document, History};
use crate::io::settings::Settings;
use crate::io::Preset;
use crate::render::Camera;
use crate::tools::ToolManager;

/// The test constructor (ADR 0002 §A2). Touches no filesystem: `settings`
/// is `Settings::default()`, `document` is never replaced with an autosaved
/// or persisted one. Safe to call from any `#[cfg(test)]` context. Boot code
/// must use [`App::new`] instead, which additionally reads the platform
/// config directory and the platform data directory.
impl Default for App {
    fn default() -> Self {
        Self {
            document: Document::default(),
            history: History::default(),
            camera: Camera::default(),
            last_cursor_world: None,
            preview_entities: Vec::new(),
            active_snap: None,
            tool_manager: ToolManager::default(),
            settings: Settings::default(),
            dirty_since: None,
            last_synced_revision: 0,
            about_open: false,
            agent_settings_open: false,
            bed_dialog: None,
            export_preset: Preset::Cut,
            command_line_input: String::new(),
            command_history: CommandHistory::default(),
            command_feedback: String::new(),
            focus_command_line: false,
            command_line_focused: false,
            snap_enabled: true,
            grid_enabled: true,
            ortho_enabled: false,
            agent_panel_open: false,
            agent_chat: Vec::new(),
            agent_input_draft: String::new(),
            agent_busy: false,
            agent_rx: None,
            current_file: None,
            error_message: None,
            saved_revision: None,
            pending_action: None,
        }
    }
}

impl App {
    /// Construct the application: boot-only. Reads the platform **config**
    /// directory (via [`Settings::load`]) and the platform **data**
    /// directory (via [`crate::io::load_autosave`]) — the two real
    /// filesystem locations `App::default()` never touches. MUST NOT be
    /// called from tests (ADR 0002 §A2); tests use [`App::default`].
    ///
    /// Calls [`Self::default()`] for all fields, then:
    /// - loads persisted settings — recent files, agent endpoint, agent API
    ///   key — overwriting the default `settings`;
    /// - overwrites `document` with the autosaved one, if present and
    ///   schema-compatible (LCV-059 AC#1) — the envelope's own `bed_mm` wins;
    /// - otherwise seeds the blank document's bed from
    ///   `settings.default_bed_mm` (LCV-114 AC 11).
    ///
    /// Performs no write of its own: no `settings.save()`, no autosave
    /// write, no file created on the startup path.
    pub fn new() -> Self {
        let mut app = Self {
            settings: Settings::load(),
            ..Self::default()
        };
        if let Some(recovered) = crate::io::load_autosave() {
            app.document = recovered;
        } else {
            app.document.bed_mm = app.settings.clamped_default_bed_mm();
        }
        app
    }
}

#[cfg(test)]
mod tests {
    /// LCV-114 AC 11, boot half — a cold start with no autosave seeds the
    /// blank document from the settings default; a recovered autosave keeps
    /// its own bed.
    ///
    /// A bounded source scan, because `App::new` is the one constructor tests
    /// may not call (ADR 0002 §A2): it reads the platform config and data
    /// directories. The seed logic itself is covered behaviourally by
    /// `io::file_actions::tests::new_document_seeds_bed_from_settings` and the
    /// recovery half by `io::autosave::tests::envelope_round_trips_bed_mm`.
    ///
    /// The haystack stops at the test module, so this test's own body cannot
    /// satisfy it.
    #[test]
    fn boot_seeds_the_bed_only_when_no_autosave_is_recovered() {
        let src = include_str!("init.rs");
        let cfg_test_at = src
            .find("\n#[cfg(test)]")
            .expect("init.rs must have a test module to bound the scan");
        let implementation = &src[..cfg_test_at];
        let start = implementation
            .find("pub fn new() -> Self {")
            .expect("App::new must exist");
        let body = &implementation[start..];
        let recovered = body
            .find("app.document = recovered;")
            .expect("the autosave branch must install the recovered document");
        let seed = body
            .find("app.document.bed_mm = app.settings.clamped_default_bed_mm();")
            .expect("the cold-start branch must seed the bed from settings");
        assert!(
            recovered < seed && body[recovered..seed].contains("} else {"),
            "the seed must be the else-branch: a recovered envelope keeps its own bed"
        );
    }
}
