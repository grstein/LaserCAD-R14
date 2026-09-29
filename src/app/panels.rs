//! UI chrome around the viewport: the fixed panels and the modal dialogs.
//!
//! Three entry points, called in order from
//! [`App::update_ui`](super::App::update_ui):
//!
//! - [`draw_chrome`] — menubar, statusbar, command line, toolbar. Rendered
//!   before the `CentralPanel` so egui shrinks the canvas to what is left.
//! - [`draw_agent_side_panel`] — the AI assistant panel (LCV-080), only when
//!   `agent.panel_open`.
//! - [`draw_dialogs`] — About, Keyboard shortcuts (LCV-116), Agent Settings,
//!   the error modal, the discard-confirmation dialog (LCV-113) and the Bed
//!   size… dialog (LCV-114), rendered after the `CentralPanel` so they float
//!   above the canvas.
//!
//! No key is read here: `src/app/input.rs` is the single keyboard gate
//! (LCV-103 / ADR 0002 §A6).
//!
//! MUST NOT import `eframe` or `rfd`.

use super::{draw_bed_dialog, draw_discard_dialog, App};

/// Render the four fixed panels that frame the viewport.
pub fn draw_chrome(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("menubar").show(ctx, |ui| {
        crate::ui::draw_menubar(ui, app);
    });

    egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
        crate::ui::draw_statusbar(ui, app);
    });

    egui::TopBottomPanel::bottom("command_line").show(ctx, |ui| {
        crate::ui::draw_command_line(ui, app);
    });

    egui::SidePanel::left("toolbar")
        .resizable(false)
        .exact_width(toolbar_width(ctx))
        .show(ctx, |ui| {
            crate::ui::draw_toolbar(ui, app);
        });
}

/// Cap on the tool rail's outer width, in points (LCV-140 AC 2): its eleven
/// `TOOLS` labels never need more than this to render in full at any
/// reasonable font.
const TOOLBAR_WIDTH_CEILING: f32 = 120.0;

/// The tool rail's fixed outer width — "outer" in the same sense
/// `SidePanel::exact_width`'s own doc comment uses, i.e. including the
/// panel's frame margin.
///
/// Sized to exactly fit the widest of the `TOOLS` labels and the agent
/// toggle's own label ([`crate::ui::toolbar::AGENT_TOGGLE_LABEL`]), measured
/// at the *current* button text style — so a label never wraps onto a
/// second line inside its `SelectableLabel` (egui wraps rather than elides
/// button text by default) — plus the button padding and the panel's own
/// frame margin on both sides, two points of slack for text-layout rounding
/// at the boundary, and never wider than [`TOOLBAR_WIDTH_CEILING`] (AC 2).
///
/// Recomputed from `ctx.style()` / `ctx.fonts()` on every call, mirroring
/// `agent_panel_width_ceiling`'s "never cached" rule (a font or style change
/// between frames must be reflected immediately).
fn toolbar_width(ctx: &egui::Context) -> f32 {
    let labels = crate::ui::toolbar::TOOLS
        .iter()
        .map(|entry| entry.label)
        .chain(std::iter::once(crate::ui::toolbar::AGENT_TOGGLE_LABEL));
    toolbar_width_for(ctx, labels)
}

/// The width computation itself, parameterised over the label set so a unit
/// test can hand it a synthetic over-wide label and prove
/// [`TOOLBAR_WIDTH_CEILING`]'s clamp actually binds.
///
/// LCV-140 review, mutation testing: the shipped `TOOLS` labels never come
/// close to 120pt, so a test built only from [`toolbar_width`] cannot tell
/// the ceiling constant being raised, or the `.min(TOOLBAR_WIDTH_CEILING)`
/// clamp being deleted, from the real behaviour — both mutations left every
/// test green. `tests::toolbar_width_for_clamps_a_synthetic_over_wide_label`
/// below closes that gap.
fn toolbar_width_for<'a>(ctx: &egui::Context, labels: impl Iterator<Item = &'a str>) -> f32 {
    let style = ctx.style();
    let font_id = egui::TextStyle::Button.resolve(&style);
    let widest_text = labels
        .map(|label| {
            ctx.fonts(|f| {
                f.layout_no_wrap(label.to_owned(), font_id.clone(), egui::Color32::WHITE)
                    .size()
                    .x
            })
        })
        .fold(0.0_f32, f32::max);
    let button_padding = style.spacing.button_padding.x * 2.0;
    let frame_margin = egui::Frame::side_top_panel(&style).inner_margin;
    let outer = widest_text + button_padding + frame_margin.left + frame_margin.right + 2.0;
    outer.min(TOOLBAR_WIDTH_CEILING)
}

/// Width the agent panel opens at before the ceiling narrows it (LCV-080's
/// original default).
const AGENT_PANEL_DEFAULT_WIDTH: f32 = 300.0;

/// The panel may never claim more than this fraction of the whole
/// application window's width (LCV-141 AC 1-3) — not `CentralPanel`'s
/// remaining share after the toolbar and this panel already claimed theirs.
const AGENT_PANEL_WIDTH_FRACTION: f32 = 1.0 / 3.0;

/// The hard width ceiling for this frame, in logical points.
///
/// Recomputed from `ctx.screen_rect()` on every call — never cached — so it
/// holds on the very first frame, after a width dragged wide in a bigger
/// window is carried into a smaller one, during an active resize drag, and
/// after the window itself shrinks (AC 2): `SidePanel` re-clamps its
/// persisted width against `width_range` on every `show`, so a ceiling that
/// is fresh every frame is all a caller has to provide.
fn agent_panel_width_ceiling(ctx: &egui::Context) -> f32 {
    ctx.screen_rect().width() * AGENT_PANEL_WIDTH_FRACTION
}

/// Render the agent side panel (LCV-080) when it is open; a no-op otherwise.
pub fn draw_agent_side_panel(ctx: &egui::Context, app: &mut App) {
    if !app.agent.panel_open {
        return;
    }
    let ceiling = agent_panel_width_ceiling(ctx);
    let default_width = AGENT_PANEL_DEFAULT_WIDTH.min(ceiling);
    egui::SidePanel::right("agent_panel")
        .resizable(true)
        // `.max_width` must follow `.default_width`: `SidePanel::default_width`
        // widens `width_range.max` via `.at_least(default_width)` when the
        // default exceeds the existing max, and only `.max_width` narrows the
        // range unconditionally (egui-0.29.1
        // `containers/panel.rs::SidePanel::default_width`/`max_width`). Called
        // in the other order, the ceiling would silently widen back out.
        .default_width(default_width)
        .max_width(ceiling)
        .show(ctx, |ui| {
            crate::agent::draw_agent_panel(ui, app);
        });
}

/// Render the modal dialogs (LCV-069, LCV-076, LCV-062, LCV-113, LCV-114,
/// LCV-116, LCV-156).
///
/// Called after the `CentralPanel` so the windows float above the canvas.
pub fn draw_dialogs(ctx: &egui::Context, app: &mut App) {
    crate::ui::about_dialog(ctx, &mut app.about_open);
    crate::ui::shortcuts_dialog(ctx, &mut app.shortcuts_open);
    agent_settings_dialog(ctx, app);
    error_modal(ctx, app);
    draw_discard_dialog(ctx, app);
    draw_bed_dialog(ctx, app);
    crate::ui::draw_layers_dialog(ctx, app);
}

/// The Agent Settings window (LCV-076). Persists the settings when the window
/// closes, whether by the × button, the Done button (LCV-141 AC 6), or
/// programmatically.
fn agent_settings_dialog(ctx: &egui::Context, app: &mut App) {
    let was_open = app.agent_settings_open;
    // Set from inside the content closure below when Done is clicked. Kept
    // separate from `agent_settings_open` itself: `Window::open` already
    // borrows that field for the whole `.show()` call, so a second mutable
    // borrow of the same field from the content closure would not compile —
    // this is the one new piece of state the Done button needs, read only
    // after every borrow above has ended (LCV-141 AC 6).
    let mut done_clicked = false;
    {
        // The window borrows `agent_settings_open` and `settings` mutably for
        // its whole lifetime; `App::persist_settings` needs `&App`, so the
        // borrows are scoped and the write happens after they end (LCV-119).
        let open = &mut app.agent_settings_open;
        let settings = &mut app.settings;
        egui::Window::new("Agent Settings")
            .open(open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // LCV-141 AC 7: bounded scrolling, so a form that grows in a
                // later demand scrolls instead of pushing Done off the bottom
                // of the window (ADR 0009).
                egui::ScrollArea::vertical().show(ui, |ui| {
                    done_clicked = crate::agent::draw_agent_settings(ui, settings).done_clicked;
                });
            });
    }
    // Done runs the exact same close as the × button: it only ever sets the
    // same flag the window's own `Window::open` would have set, and the one
    // guard below fires either way — never a second, parallel persistence
    // path (AC 6).
    if done_clicked {
        app.agent_settings_open = false;
    }
    // Save on dialog close (× button, Done, or programmatic close).
    if was_open && !app.agent_settings_open {
        app.persist_settings();
    }
}

/// The error modal (LCV-062) — rendered last so it floats above everything.
fn error_modal(ctx: &egui::Context, app: &mut App) {
    if let Some(msg) = app.error_message.clone()
        && crate::ui::error_dialog(ctx, "Error", &msg) {
            app.error_message = None;
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LCV-140 AC 2, mutation-testing follow-up — an over-wide synthetic
    /// label forces the ceiling clamp to actually bind: this fails if
    /// `TOOLBAR_WIDTH_CEILING` is raised (the returned width would then
    /// exceed today's 120.0), and fails if `.min(TOOLBAR_WIDTH_CEILING)` is
    /// deleted (the returned width would be the synthetic label's own huge
    /// natural size). The real `toolbar_width(ctx)` — built only from the
    /// shipped `TOOLS` labels, which never reach the ceiling — cannot prove
    /// either.
    #[test]
    fn toolbar_width_for_clamps_a_synthetic_over_wide_label() {
        let ctx = egui::Context::default();
        // Fonts are not available until the first `Context::run` (egui-0.29.1
        // `context.rs::Context::fonts`); one empty pass is enough to prime them.
        let _ = ctx.run(egui::RawInput::default(), |_| {});

        let huge_label = "M".repeat(400);
        let width = toolbar_width_for(&ctx, std::iter::once(huge_label.as_str()));

        // The expected value is AC 2's own number, 120.0 — hard-coded on
        // purpose, never `TOOLBAR_WIDTH_CEILING` itself: comparing against
        // that symbol would make this assertion true for *any* value the
        // constant holds (raising it to 300 would just move both sides of
        // the comparison together), which is exactly the mutation this test
        // exists to catch.
        assert_eq!(
            width, 120.0,
            "an over-wide label must clamp to exactly AC 2's 120pt ceiling, got {width}"
        );
        assert_eq!(
            width, TOOLBAR_WIDTH_CEILING,
            "positive control: 120.0 must actually be today's TOOLBAR_WIDTH_CEILING"
        );
    }

    /// LCV-105 — the agent side panel is skipped entirely while the panel is
    /// closed, which is what keeps the canvas full-width by default.
    #[test]
    fn agent_side_panel_is_skipped_while_closed() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        assert!(!app.agent.panel_open);
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw_agent_side_panel(ctx, &mut app);
        });
        assert!(!app.agent.panel_open);
    }

    /// The implementation section — everything before the bare `#[cfg(test)]`
    /// at column 0 — with comment lines dropped, so neither scan below can
    /// match its own literal or the prose next to it.
    fn implementation_code() -> String {
        let src = include_str!("panels.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("panels.rs must have a bare #[cfg(test)] marker");
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

    /// LCV-125 AC 13 — **source scan**: the settings dialog reaches
    /// `draw_agent_settings` through `src/agent/mod.rs`'s re-export, not
    /// through a deep path into the module's private file layout (AGENTS.md
    /// §Module tree). The compile is half the test — the deep path stops
    /// resolving only once `pub use settings_ui::draw_agent_settings;` exists —
    /// and this is the half that notices it coming back.
    ///
    /// Scoped to this one call by decision: the tree carries twelve more
    /// cross-module deep paths, several of them legitimate, and a scan shipped
    /// with a twelve-entry grandfather list is the weakest shape this
    /// repository has. That is `architect`'s call, in its own demand.
    #[test]
    fn ac13_the_settings_dialog_uses_the_module_re_export_source_scan() {
        let implementation = implementation_code();
        let deep = concat!("crate::agent::settings", "_ui::");
        let witness = "crate::agent::settings_ui::draw_agent_settings(ui, settings);";
        assert!(
            witness.contains(deep),
            "control: `{deep}` must be a needle that can match something"
        );
        assert!(
            !implementation.contains(deep),
            "the dialog must not reach into `agent`'s file layout"
        );
        assert!(
            implementation.contains(concat!(
                "crate::agent::draw_agent",
                "_settings(ui, settings)"
            )),
            "positive control: the dialog must still draw the form"
        );
    }

    /// The body of `if done_clicked { .. }` inside `agent_settings_dialog`,
    /// brace-matched — the same slicing idiom `src/agent/panel.rs::busy_block`
    /// uses for LCV-129's Cancel button, so a line moved out of the guarded
    /// block is no longer in *this* string even though it is still in the file.
    fn done_clicked_block(implementation: &str) -> String {
        let head = concat!("if done_", "clicked {");
        let start = implementation
            .find(head)
            .unwrap_or_else(|| panic!("agent_settings_dialog must guard a `{head}` block"))
            + head.len();
        let mut depth = 1usize;
        for (offset, ch) in implementation[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return implementation[start..start + offset].to_owned();
                    }
                }
                _ => {}
            }
        }
        panic!("the done_clicked block is never closed — panels.rs does not parse");
    }

    /// LCV-141 AC 6 — **source scan**: Done's whole effect is setting the same
    /// flag `Window::open` sets for ×, not a second call into
    /// `App::persist_settings`. The one persist call below — reached through
    /// `was_open && !app.agent_settings_open`, which is true after *either*
    /// path — is what actually saves, and this pins that Done does not also
    /// reach it directly.
    #[test]
    fn ac6_done_only_sets_the_shared_close_flag_source_scan() {
        let implementation = implementation_code();
        let block = done_clicked_block(&implementation);

        assert!(
            block.contains(concat!("agent_settings", "_open = false")),
            "AC 6: the done_clicked block must set the same flag × sets: {block}"
        );

        let witness =
            "if done_clicked { app.agent_settings_open = false; app.persist_settings(); }";
        let forbidden = concat!("persist_", "settings");
        assert!(
            witness.contains(forbidden),
            "control: `{forbidden}` must be a needle that can match something"
        );
        assert!(
            !block.contains(forbidden),
            "AC 6: Done must not call `{forbidden}` itself — that is the shared \
             guard's job, reached the same way × reaches it: {block}"
        );
    }

    /// LCV-141 AC 7 — **source scan**: the dialog's body is wrapped in a
    /// bounded `ScrollArea`, matching `src/ui/shortcuts_dialog.rs`'s own
    /// pattern (ADR 0009). `tests/it/agent/panel_width_and_settings.rs`
    /// proves the *mechanism* — a `Window` + `ScrollArea` absorbs an
    /// overflowing body instead of the window growing — against a
    /// hand-built harness, because nothing outside this private function can
    /// inject a growth probe into the real dialog; this scan is what ties
    /// that proof back to `agent_settings_dialog` itself.
    #[test]
    fn ac7_the_dialog_wraps_its_body_in_a_scroll_area_source_scan() {
        let implementation = implementation_code();
        assert!(
            implementation.contains(concat!("ScrollArea::", "vertical()")),
            "AC 7: agent_settings_dialog must wrap its content in a ScrollArea"
        );
    }

    /// LCV-105 — the dialog phase on a default `App` renders nothing modal and
    /// leaves every flag untouched.
    #[test]
    fn dialogs_on_default_app_do_not_panic() {
        let ctx = egui::Context::default();
        let mut app = App::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw_dialogs(ctx, &mut app);
        });
        assert!(!app.about_open);
        assert!(!app.agent_settings_open);
        assert!(app.error_message.is_none());
    }
}
