//! egui panel for editing agent connection settings.
//!
//! One of the **two** files in `src/agent/` that may import `egui` — the other
//! is `panel.rs` (ADR 0007 §D8). Every other `agent/` submodule is kernel-pure,
//! and no file here may import `eframe` or `rfd`.
//!
//! LCV-125 took the form from two fields to four. The two it added are not new
//! behaviour: `agent_model` and `agent_step_budget` have been honoured since
//! LCV-121, they were simply only reachable by hand-editing `settings.json`.
//! LCV-143 added the fifth, the system prompt.

use egui;

use crate::agent::{prompt, AGENT_STEP_BUDGET_MAX, AGENT_STEP_BUDGET_MIN};
use crate::io::settings::Settings;

/// Minimum width of every text field, in logical pixels.
const FIELD_MIN_WIDTH: f32 = 320.0;

/// Minimum width of a grid column, so the label column of the field grid and
/// of the budget grid line up despite being two separate grids.
const LABEL_COL_WIDTH: f32 = 110.0;

/// Widest the form may get. The two sentences below are long; without a bound
/// they would each stretch the dialog to a single line hundreds of pixels wide.
const FORM_MAX_WIDTH: f32 = 470.0;

/// Hint text in the Model field: the default `Settings::agent_model`, which is
/// an OpenRouter id and therefore wrong for most other endpoints (AC 8).
const MODEL_HINT: &str = "anthropic/claude-sonnet-4.6";

/// The plaintext-key warning (AC 10), rendered as an always-visible row under
/// the API Key field.
///
/// ADR 0007 §D10 stores the key in clear text on purpose, and the masked field
/// above implies the opposite. Never a tooltip, never behind a collapsing
/// header: a warning the operator has to hover for is a warning they never read.
const PLAINTEXT_KEY_WARNING: &str = "The API key is stored in plain text in settings.json. \
                                     Anyone who can read that file can read your key.";

/// What the step budget buys, in one line (AC 9).
const STEP_BUDGET_HELP: &str = "How many tool calls one prompt may make. More steps means a \
                                bigger drawing per prompt, and more API calls.";

/// States that edits are live and persist on close (LCV-141 AC 6), next to
/// the Done button that is a second way to trigger that same close — never a
/// different semantics: the dialog stays live-edit, persist-on-close.
const LIVE_EDIT_NOTE: &str = "Changes apply immediately and are saved when this window closes.";

/// Rows the system-prompt editor asks for before its own scroll area.
const PROMPT_ROWS: usize = 6;

/// Tallest the system-prompt editor may get: a long prompt scrolls inside it
/// rather than pushing Done down the 426pt dialog body (ADR 0009).
const PROMPT_MAX_HEIGHT: f32 = 110.0;

/// What one frame of the form reported.
pub struct AgentSettingsFrame {
    /// `true` if **any** of the five fields changed this frame — the flag
    /// [`draw_agent_settings`] always reported, now carried on a named field
    /// instead of being the whole return value.
    pub changed: bool,
    /// `true` if the Done button (LCV-141 AC 6) was clicked this frame. The
    /// caller — `src/app/panels.rs::agent_settings_dialog` — closes the
    /// window through the exact same path it already runs for the × button:
    /// same `persist_settings()` call, same `was_open &&
    /// !app.agent_settings_open` guard, never a second, parallel persistence
    /// path.
    pub done_clicked: bool,
}

/// Draw the agent-settings form into `ui`.
///
/// Five labelled rows, in the order the operator meets them:
/// - **Endpoint URL** — plain single-line field editing `settings.agent_endpoint`.
/// - **Model** — plain single-line field editing `settings.agent_model`. Free
///   text, hinted with the default: the endpoint is the authority on which ids
///   exist, and it already reports a bad one through LCV-121's status mapping.
/// - **API Key** — password-masked field editing `settings.agent_api_key`,
///   followed by [`PLAINTEXT_KEY_WARNING`]. The mask stops shoulder-surfing;
///   the warning states where the bytes actually live. Both, deliberately.
/// - **Steps per turn** — a slider over
///   `AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX` (the range lives with the
///   loop that enforces it, ADR 0007 §D7) editing `settings.agent_step_budget`.
/// - **System prompt** — a multiline editor over the *effective* prompt
///   (`prompt::resolve`), with a **Restore default** button (LCV-143). The
///   text is copied into a per-frame buffer, so only a real edit writes
///   `Some(text)` — opening the dialog creates no override — and the button,
///   drawn before the editor, sets `None` so the built-in text shows on the
///   same frame.
///
/// Every text field has a minimum width of [`FIELD_MIN_WIDTH`] logical pixels.
///
/// [`AgentSettingsFrame::changed`] is `true` if **any** of the five fields
/// changed this frame, `false` otherwise — `agent_settings_dialog` persists on
/// close, so the flag is what tells the operator's edit apart from an idle
/// frame. [`AgentSettingsFrame::done_clicked`] is `true` the one frame the new
/// Done button (AC 6) is clicked.
///
/// The slider clamps as it draws (`SliderClamping::Always`), so a settings file
/// hand-edited to `5000` is written back as `4096` on the first frame the dialog
/// is open — and reports `changed` that frame, which is what gets the clamped
/// value persisted. The read-site clamp in `src/app/agent_turn.rs` stays
/// regardless: a file the dialog was never opened on is still a hand-edited file.
///
/// The body is not wrapped in its own `ScrollArea`: `agent_settings_dialog`
/// wraps the whole call in one (AC 7, ADR 0009), so a caller measuring this
/// function in isolation — every inline test below — sees exactly the rows
/// and sentences this function draws, with no scrolling machinery of its own
/// to account for.
pub fn draw_agent_settings(ui: &mut egui::Ui, settings: &mut Settings) -> AgentSettingsFrame {
    let mut changed = false;
    ui.set_max_width(FORM_MAX_WIDTH);

    grid("agent_settings_grid").show(ui, |ui| {
        ui.label("Endpoint URL");
        changed |= ui.add(field(&mut settings.agent_endpoint)).changed();
        ui.end_row();

        ui.label("Model");
        changed |= ui
            .add(field(&mut settings.agent_model).hint_text(MODEL_HINT))
            .changed();
        ui.end_row();

        ui.label("API Key");
        changed |= ui
            .add(field(&mut settings.agent_api_key).password(true))
            .changed();
        ui.end_row();
    });

    warn_label(ui, PLAINTEXT_KEY_WARNING);

    grid("agent_budget_grid").show(ui, |ui| {
        ui.label("Steps per turn");
        let before = settings.agent_step_budget;
        // Logarithmic: 1..=4096 (LCV-142) is too wide for a linear track to
        // land on small values; the number box still takes any typed value.
        let budget = ui.add(
            egui::Slider::new(
                &mut settings.agent_step_budget,
                AGENT_STEP_BUDGET_MIN..=AGENT_STEP_BUDGET_MAX,
            )
            .logarithmic(true),
        );
        // `Response::changed()` covers an interaction; the comparison covers the
        // clamp. egui writes an out-of-range stored value back into range as it
        // draws but reports nothing, so without the second term a file
        // hand-edited to `5000` would be silently corrected and never persisted.
        changed |= budget.changed() || settings.agent_step_budget != before;
        ui.end_row();
    });
    ui.add(egui::Label::new(egui::RichText::new(STEP_BUDGET_HELP).small()).wrap());

    changed |= prompt_editor(ui, settings);

    ui.add(egui::Label::new(egui::RichText::new(LIVE_EDIT_NOTE).small()).wrap());
    let done_clicked = ui.button("Done").clicked();

    AgentSettingsFrame {
        changed,
        done_clicked,
    }
}

/// The System prompt row: label, Restore default, then the editor. Returns
/// whether either changed `settings.agent_system_prompt`.
fn prompt_editor(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("System prompt");
        if ui.button("Restore default").clicked() {
            settings.agent_system_prompt = None;
            changed = true;
        }
    });
    let mut text = prompt::resolve(settings.agent_system_prompt.as_deref()).to_owned();
    let edited = egui::ScrollArea::vertical()
        .id_salt("agent_system_prompt_scroll")
        .max_height(PROMPT_MAX_HEIGHT)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .id_salt("agent_system_prompt")
                    .desired_width(f32::INFINITY)
                    .desired_rows(PROMPT_ROWS),
            )
            .changed()
        })
        .inner;
    if edited {
        settings.agent_system_prompt = Some(text);
        changed = true;
    }
    changed
}

/// One two-column form grid. Two of them, so the long sentences between and
/// below them get the full form width to wrap into rather than a grid cell.
fn grid(id: &str) -> egui::Grid {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([8.0, 6.0])
        .min_col_width(LABEL_COL_WIDTH)
}

/// One single-line text field of at least [`FIELD_MIN_WIDTH`] points.
fn field(text: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(text).min_size(egui::vec2(FIELD_MIN_WIDTH, 0.0))
}

/// One always-visible warning line, wrapped inside the form width.
fn warn_label(ui: &mut egui::Ui, text: &str) {
    let warn = ui.visuals().warn_fg_color;
    ui.add(egui::Label::new(egui::RichText::new(text).color(warn)).wrap());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The implementation section — everything before the bare `#[cfg(test)]`
    /// at column 0 — with comment lines dropped, so a scan can match neither
    /// its own literal nor the prose describing the code it is checking.
    fn implementation_code() -> String {
        let src = include_str!("settings_ui.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("settings_ui.rs must have a bare #[cfg(test)] marker");
        let code: Vec<&str> = src[..at]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect();
        assert!(
            code.len() > 40,
            "positive control: the haystack must be the whole implementation, got {} lines",
            code.len()
        );
        code.join("\n")
    }

    fn occurrences(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    /// A headless form frame. Returns what `draw_agent_settings` reported.
    fn run_form(ctx: &egui::Context, settings: &mut Settings, events: Vec<egui::Event>) -> bool {
        let mut changed = false;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            events,
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = draw_agent_settings(ui, settings).changed;
            });
        });
        changed
    }

    /// A complete key tap: press and release, both `repeat: false`.
    fn key_tap(key: egui::Key) -> Vec<egui::Event> {
        vec![
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }

    /// Walk focus `hops` fields into the form with Tab, then type `text`.
    ///
    /// Focus is granted at the end of the frame that asks for it, so the Tab
    /// frames and the typing frame are separate frames — the same trap the
    /// command-line harness documents.
    fn type_into_field(settings: &mut Settings, hops: usize, text: &str) -> bool {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        run_form(&ctx, settings, Vec::new());
        for _ in 0..hops {
            run_form(&ctx, settings, key_tap(egui::Key::Tab));
        }
        run_form(&ctx, settings, vec![egui::Event::Text(text.to_owned())])
    }

    /// Which of the four fields moved away from its default.
    fn moved(settings: &Settings) -> Vec<&'static str> {
        let d = Settings::default();
        let mut out = Vec::new();
        if settings.agent_endpoint != d.agent_endpoint {
            out.push("endpoint");
        }
        if settings.agent_model != d.agent_model {
            out.push("model");
        }
        if settings.agent_api_key != d.agent_api_key {
            out.push("api_key");
        }
        if settings.agent_step_budget != d.agent_step_budget {
            out.push("step_budget");
        }
        out
    }

    /// §T3 — AC 6 — no synthetic input → returns false.
    #[test]
    fn no_input_returns_false() {
        let ctx = egui::Context::default();
        let mut settings = Settings::default();
        let mut changed = false;
        let _output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = draw_agent_settings(ui, &mut settings).changed;
            });
        });
        assert!(!changed);
    }

    /// AC 8 — **source scan**: a Model row sits between Endpoint URL and API
    /// Key, hinted with the default id, at the same minimum width as its
    /// siblings, and — unlike the key — it is not masked.
    #[test]
    fn ac8_the_model_field_is_plain_hinted_and_sized_source_scan() {
        let implementation = implementation_code();
        let label = concat!("\"Mo", "del\"");
        let endpoint_at = implementation
            .find(concat!("\"Endpoint ", "URL\""))
            .unwrap();
        let model_at = implementation
            .find(label)
            .unwrap_or_else(|| panic!("the form must carry a {label} row"));
        let key_at = implementation.find(concat!("\"API ", "Key\"")).unwrap();
        assert!(
            endpoint_at < model_at && model_at < key_at,
            "Model sits between Endpoint URL and API Key"
        );
        assert!(
            implementation.contains(concat!("hint_", "text(MODEL_HINT)")),
            "the Model field must carry the default id as hint text"
        );
        assert_eq!(
            MODEL_HINT, "anthropic/claude-sonnet-4.6",
            "the hint must be the default `Settings::agent_model`"
        );
        assert_eq!(
            occurrences(&implementation, concat!(".pass", "word(true)")),
            1,
            "exactly one field is masked, and it is the key — not the model"
        );
        assert_eq!(
            occurrences(&implementation, concat!("field(&mut set", "tings.")),
            3,
            "all three text fields go through `field`, which sets the minimum width"
        );
        assert_eq!(FIELD_MIN_WIDTH, 320.0);
    }

    /// AC 9 — **source scan**: the slider's range is the two constants by
    /// name, never the literals. A range spelt `1..=32` drifts silently the
    /// day the loop's budget changes.
    #[test]
    fn ac9_the_slider_range_is_the_named_constants_source_scan() {
        let implementation = implementation_code();
        for needle in [
            concat!("Slider::", "new("),
            concat!("AGENT_STEP_BUDGET_", "MIN"),
            concat!("AGENT_STEP_BUDGET_", "MAX"),
            concat!("\"Steps per ", "turn\""),
            // Anchored at the call site, not on the name: `STEP_BUDGET_HELP`
            // alone is satisfied by the `const` declaration, so deleting the
            // label that renders it would leave this scan green.
            concat!("RichText::new(STEP_BUDGET_", "HELP)"),
        ] {
            assert!(
                implementation.contains(needle),
                "the budget row must contain `{needle}`"
            );
        }
        for literal in [concat!("1..", "=32"), concat!("1..", "=4096")] {
            let witness = "Slider::new(&mut b, 1..=32) Slider::new(&mut b, 1..=4096)";
            assert!(
                witness.contains(literal),
                "control: `{literal}` must be a needle that can match something"
            );
            assert!(
                !implementation.contains(literal),
                "the range must be the constants, not the literals"
            );
        }
        assert_eq!(
            STEP_BUDGET_HELP,
            concat!(
                "How many tool calls one prompt may make. More steps means a ",
                "bigger drawing per prompt, and more API calls."
            ),
            "the explanation is the sentence the demand fixed"
        );
    }

    /// AC 10 — **source scan**: the warning is rendered as a visible row, and
    /// nothing in this file hides text behind a hover or a collapsing header.
    /// A scan, because "is on screen without hovering" is not observable at
    /// egui 0.29 — the reviewer's eye and the manual smoke cover appearance.
    #[test]
    fn ac10_the_plaintext_warning_is_always_visible_source_scan() {
        let implementation = implementation_code();
        assert_eq!(
            PLAINTEXT_KEY_WARNING,
            concat!(
                "The API key is stored in plain text in settings.json. ",
                "Anyone who can read that file can read your key."
            ),
            "the warning is the sentence the demand fixed, word for word"
        );
        assert!(
            implementation.contains(concat!("warn_label(ui, PLAINTEXT_KEY_", "WARNING)")),
            "the warning must be drawn as its own visible row"
        );
        assert!(
            implementation.contains(concat!("warn_fg", "_color")),
            "and drawn in the warning colour"
        );
        let witness = "ui.label(\"key\").on_hover_text(PLAINTEXT_KEY_WARNING); ui.collapsing(…)";
        for hidden in [concat!("on_hover", "_text"), concat!("collap", "sing")] {
            assert!(
                witness.contains(hidden),
                "control: `{hidden}` must be a needle that can match something"
            );
            assert!(
                !implementation.contains(hidden),
                "the warning must not be reachable only through `{hidden}`"
            );
        }
    }

    /// AC 10 — a **rendering** assertion beside the scan: the warning folds
    /// inside the form width instead of stretching the dialog into a strip.
    #[test]
    fn ac10_the_form_stays_within_its_width_with_both_sentences() {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut settings = Settings::default();
        let mut size = egui::Vec2::ZERO;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // A scope, because a panel's own `min_rect` is the whole panel.
                size = ui
                    .scope(|ui| {
                        draw_agent_settings(ui, &mut settings);
                    })
                    .response
                    .rect
                    .size();
            });
        });
        assert!(
            size.x <= FORM_MAX_WIDTH + 1.0,
            "the form must stay inside {FORM_MAX_WIDTH} points, took {}",
            size.x
        );
        assert!(
            size.y > 100.0,
            "four rows and two sentences take more than one line, took {}",
            size.y
        );
    }

    /// AC 11 — every text field reports **its own** change. Order-agnostic on
    /// purpose: what matters is that three Tab stops each take a keystroke,
    /// each move exactly one field, and each report `true`.
    #[test]
    fn ac11_each_text_field_reports_its_own_change() {
        let mut seen: Vec<&str> = Vec::new();
        for hops in 1..=3 {
            let mut settings = Settings::default();
            let changed = type_into_field(&mut settings, hops, "Z");
            let which = moved(&settings);
            assert_eq!(
                which.len(),
                1,
                "one keystroke must move exactly one field, moved {which:?}"
            );
            assert!(changed, "a change in `{}` must be reported", which[0]);
            seen.push(which[0]);
        }
        seen.sort_unstable();
        assert_eq!(seen, ["api_key", "endpoint", "model"]);
    }

    /// AC 11 / AC 12 — the fourth field, and the egui trap behind it.
    ///
    /// `egui::Context::run` may run the frame body **more than once**: the
    /// first frame of this form runs two passes (the grid sizing itself), and
    /// the flag the caller sees is the one the *last* pass produced. So a
    /// budget that is already out of range when the dialog opens is clamped in
    /// pass 1 and reported by nobody in pass 2 — which is harmless only
    /// because `src/app/panels.rs::agent_settings_dialog` persists on close
    /// unconditionally. A budget that leaves the range while the dialog is
    /// open is a single pass, and is reported. Both are asserted here so the
    /// difference cannot be mistaken for a bug later.
    #[test]
    fn ac12_an_out_of_range_budget_is_clamped_by_the_slider() {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut settings = Settings {
            agent_step_budget: 5000,
            ..Settings::default()
        };

        run_form(&ctx, &mut settings, Vec::new());

        assert_eq!(
            settings.agent_step_budget, AGENT_STEP_BUDGET_MAX,
            "opening the dialog on a hand-edited 5000 must show 4096"
        );
        assert_eq!(
            crate::agent::clamp_step_budget(settings.agent_step_budget),
            AGENT_STEP_BUDGET_MAX,
            "and the read site still agrees (LCV-123 AC 18)"
        );
        assert!(
            !run_form(&ctx, &mut settings, Vec::new()),
            "an idle frame on an in-range value reports nothing"
        );
    }

    /// AC 11 — the fourth field reports a change, on the single-pass frame
    /// where there is one to report.
    #[test]
    fn ac11_the_budget_field_reports_its_own_change() {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut settings = Settings::default();
        assert!(
            !run_form(&ctx, &mut settings, Vec::new()),
            "the dialog opens on an untouched form"
        );

        settings.agent_step_budget = 5000;
        let changed = run_form(&ctx, &mut settings, Vec::new());

        assert!(changed, "the clamp is a change, and must be persisted");
        assert_eq!(settings.agent_step_budget, 4096, "LCV-142 AC 2");
        assert_eq!(moved(&settings), ["step_budget"], "and only that field");
    }

    /// LCV-142 AC 2 — a stored in-range 4096 survives drawing the form and is
    /// not reported as a change, on the first frame and on a later one. This
    /// is what breaks if the control's maximum stays at the old 32.
    #[test]
    fn lcv142_a_stored_4096_survives_the_form_unchanged() {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut settings = Settings {
            agent_step_budget: 4096,
            ..Settings::default()
        };
        assert!(!run_form(&ctx, &mut settings, Vec::new()), "first frame");
        assert_eq!(settings.agent_step_budget, 4096);
        assert!(!run_form(&ctx, &mut settings, Vec::new()), "a later frame");
        assert_eq!(settings.agent_step_budget, 4096);
        assert_eq!(AGENT_STEP_BUDGET_MAX, 4096);
    }

    /// AC 12 — the other end of the range, from the same direction.
    #[test]
    fn ac12_a_zero_budget_is_clamped_to_the_minimum() {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut settings = Settings {
            agent_step_budget: 0,
            ..Settings::default()
        };

        run_form(&ctx, &mut settings, Vec::new());
        assert_eq!(settings.agent_step_budget, AGENT_STEP_BUDGET_MIN);
    }
}
