//! The fixed sentences of the Agent Settings form, split out of
//! `settings_ui.rs` for the LOC cap (LCV-195). Data only: no `egui`.

/// Hint text in the Model field: the default `Settings::agent_model`, which is
/// an OpenRouter id and therefore wrong for most other endpoints (AC 8).
pub(super) const MODEL_HINT: &str = "anthropic/claude-sonnet-4.6";

/// The plaintext-key warning (AC 10), rendered as an always-visible row under
/// the API Key field.
///
/// ADR 0007 §D10 stores the key in clear text on purpose, and the masked field
/// above implies the opposite. Never a tooltip, never behind a collapsing
/// header: a warning the operator has to hover for is a warning they never read.
pub(super) const PLAINTEXT_KEY_WARNING: &str = "The API key is stored in plain text in settings.json. \
     Anyone who can read that file can read your key.";

/// What the step budget buys, in one line (AC 9).
pub(super) const STEP_BUDGET_HELP: &str = "How many tool calls one prompt may make. More steps means a \
     bigger drawing per prompt, and more API calls.";

/// States that edits are live and persist on close (LCV-141 AC 6), next to
/// the Close button that is a second way to trigger that same close — never a
/// different semantics: the dialog stays live-edit, persist-on-close.
pub(super) const LIVE_EDIT_NOTE: &str =
    "Changes apply immediately and are saved when this window closes.";

/// What the two canvas opt-ins together allow (LCV-145 AC 2, ADR 0011).
pub(super) const CANVAS_DISCLOSURE: &str = "When both are on, the agent may send a picture of the drawing \
     (not the window) to the configured provider and model.";
