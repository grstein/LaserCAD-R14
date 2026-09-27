//! LCV-080 — Agent panel: chat history widget and AI input row.
//!
//! Exports [`draw_agent_panel`], which renders into an existing [`egui::Ui`]
//! that lives inside a right-side panel. The channel payload it sends moved to
//! [`crate::agent::AgentEvent`] with LCV-122 — a protocol shared with a
//! kernel-pure worker thread does not belong in an egui file.
//!
//! **Purity rule**: this file may import `egui` but MUST NOT import `eframe`
//! or `rfd`. The panel function receives `ui: &mut egui::Ui` directly.
//!
//! **This file renders and reports** (ADR 0007 §D8). It spawns no thread and
//! constructs no `Document` and no `History` — until LCV-123 it did all three,
//! against a throwaway document whose geometry went nowhere, and it was the one
//! documented exception to ADR 0007 §D1. That exception is now empty:
//! `tests/lcv122_source_scans.rs` asserts no file under `src/agent/` names a
//! document, and this file's own `the_panel_renders_and_reports` asserts it
//! here by name. Submitting a prompt is one call into [`crate::app::start_turn`],
//! which owns the fence, the thread and the settings reads.
//!
//! **LCV-125 renders what LCV-123 records.** The seam is one-way: LCV-123 owns
//! the role vocabulary, every sentence and their order; this file owns only how
//! each role looks. It appends no row of its own and invents no seventh role.
//!
//! **LCV-129 adds the one control that ends a turn.** `Cancel` sits in the
//! thinking row, exists only while `agent.busy`, and calls
//! [`crate::app::cancel_turn`]. The row it leaves behind is `note` — written
//! by `agent_poll`, in the six-role vocabulary, like every other row here.

use crate::app::App;

/// Colour of an action / outcome row (AC 2).
///
/// A cool accent, deliberately neither the `#d0d0d0` the theme gives assistant
/// prose (`src/ui/theme.rs::apply_theme`) nor the red an `error` row uses, so
/// the §D5 renumbering sentence is findable in a column of chat at a glance.
/// `the_row_colours_are_distinct_under_the_real_theme` pins all three apart.
const TOOL_COLOR: egui::Color32 = egui::Color32::from_rgb(120, 190, 255);

/// Vertical space (points) the separator and the composer row below the
/// transcript always need, whether or not a turn is running (LCV-080's
/// original reservation). Generous, not exact — see [`BUSY_ROW_RESERVE`].
const COMPOSER_RESERVE: f32 = 60.0;

/// Extra vertical space the busy "Thinking… / Cancel" row and its own
/// surrounding gap need, on top of [`COMPOSER_RESERVE`], only while a turn is
/// running (LCV-141 AC 4). Generous by design: the transcript gives up a
/// little more room than that row strictly needs rather than risk the row —
/// and the composer beneath it — being pushed past the panel's clipped
/// bottom edge, which is the defect this constant exists to close.
const BUSY_ROW_RESERVE: f32 = 40.0;

/// The footer's total reservation for one frame (LCV-141 AC 4): a plain
/// function of `busy`, not an inline `if app.agent.busy` next to the block
/// below — so a source scan bounding *that* block by its own guard (AC 8's
/// `busy_block`) cannot mistake this arithmetic for it.
fn footer_reserve(busy: bool) -> f32 {
    if busy {
        COMPOSER_RESERVE + BUSY_ROW_RESERVE
    } else {
        COMPOSER_RESERVE
    }
}

// ── Public render entry ───────────────────────────────────────────────────────

/// Render the AI assistant chat panel into `ui`.
///
/// Call site: inside `egui::SidePanel::right("agent_panel")` in `App::update`,
/// gated by `app.agent.panel_open`. Wired in by LCV-080.
pub fn draw_agent_panel(ui: &mut egui::Ui, app: &mut App) {
    // ── Header row ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.heading("AI Assistant");
        // Push the close button to the right edge.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("×").clicked() {
                app.agent.panel_open = false;
            }
        });
    });

    ui.separator();

    // ── Chat history scroll area ──────────────────────────────────────────────
    // Reserve room for the composer row below, and — read *before* the scroll
    // area claims what is left, not after — for the busy "Thinking… / Cancel"
    // row exactly on the frames it will actually render (LCV-141 AC 4). The
    // old flat constant reserved the same amount whether or not that row
    // existed, so a long transcript could push the row itself, and the
    // composer beneath it, past the panel's clipped bottom edge.
    let scroll_height = (ui.available_height() - footer_reserve(app.agent.busy)).max(0.0);

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .max_height(scroll_height)
        .show(ui, |ui| {
            for (role, content) in &app.agent.chat {
                draw_chat_row(ui, role, content);
            }
        });

    // ── Thinking indicator, and the way out of it ─────────────────────────────
    // The button lives inside the `agent.busy` block and nowhere else: it can
    // only offer to end a turn that is running, and when none is the row does
    // not exist at all (LCV-129 AC 8). Its body is one call into the app, the
    // same shape as the Send button's — this file renders and reports, and
    // ending a turn is `agent_poll`'s alone (ADR 0007 §D8, §D11).
    if app.agent.busy {
        ui.horizontal(|ui| {
            ui.spinner();
            // Progress is counted UI-side, per `Act` received (ADR 0007 §D13).
            let turn = &app.agent.turn;
            ui.label(format!("Thinking… {} of {} steps", turn.steps, turn.limit));
            if ui.button("Cancel").clicked() {
                crate::app::cancel_turn(app);
            }
        });
    }

    ui.separator();

    // ── Input row ─────────────────────────────────────────────────────────────
    let can_send = !app.agent.busy && !app.agent.input_draft.trim().is_empty();
    let mut do_submit = false;

    ui.horizontal(|ui| {
        // Right-to-left, Send first: `desired_width(f32::INFINITY)` only caps
        // at `ui.available_width()` measured *before* the button is placed,
        // so a plain left-to-right row leaves the button no room and the row
        // grows past the panel's own width by however wide "Send" is (LCV-141
        // AC 1 / AC 5) — measured at this egui pin, an unbounded panel this
        // narrow reproduces it exactly. Placing Send first inside a
        // right-to-left layout pins it to the trailing edge and hands the
        // text field whatever `ui.available_width()` is left over.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(can_send, egui::Button::new("Send"))
                .clicked()
            {
                do_submit = true;
            }
            let text_resp = ui.add(
                egui::TextEdit::singleline(&mut app.agent.input_draft)
                    .hint_text("Ask the AI…")
                    .desired_width(ui.available_width()),
            );
            // Submit when Enter is pressed with focus in the text field.
            if text_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                do_submit = true;
            }
        });
    });

    if do_submit {
        let prompt = app.agent.input_draft.trim().to_owned();
        if !prompt.is_empty() && !app.agent.busy {
            // Cleared before the turn is armed, so the field is empty the
            // instant the operator's row appears in the chat above.
            app.agent.input_draft = String::new();
            crate::app::start_turn(app, &prompt);
        }
    }
}

// ── One transcript row ────────────────────────────────────────────────────────

/// Render one `agent.chat` row the way its role deserves (LCV-125 AC 1).
///
/// The vocabulary is **closed at six**, and every value in it is written
/// elsewhere — this function only decides how each one looks:
///
/// | role | written by | what it says |
/// |---|---|---|
/// | `user` | LCV-123 AC 3 (`app::arm_turn`), and LCV-124 when the prompt arrives from the command line | the prompt, verbatim |
/// | `tool` | LCV-123 AC 23 (`app::agent_apply::transcribe`) | one action that happened |
/// | `refused` | LCV-123 AC 23 (same site, plus `app::agent_poll` for a fence refusal) | one action that did not |
/// | `assistant` | LCV-123 AC 7 (`app::agent_poll::end_turn`) | the model's closing prose |
/// | `error` | LCV-123 AC 7 (same site) | the turn failed, and why |
/// | `note` | LCV-123 AC 11 (`app::agent_poll::finish_turn`) | the turn's undo shape |
///
/// Why `tool` and `refused` are loud: ADR 0007 §D5 makes the outcome sentence
/// the *disclosure mechanism* for positional indices shifting under the model
/// (`Deleted entity 1 (…). Indices 2..3 are now 1..2.`). A disclosure nobody
/// can pick out of a grey column is not a disclosure.
///
/// The `_` arm is the safety net: a role this build does not know still gets
/// rendered, plainly, rather than silently dropped.
fn draw_chat_row(ui: &mut egui::Ui, role: &str, content: &str) {
    match role {
        "user" => {
            // Right-aligned. `.wrap()` is load-bearing here and only here: a
            // right-to-left layout defaults to `TextWrapMode::Extend`, so a
            // long prompt would run off the panel instead of folding.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add(egui::Label::new(content).wrap());
            });
        }
        "assistant" => {
            ui.add(egui::Label::new(content).wrap());
        }
        "error" => {
            ui.add(egui::Label::new(egui::RichText::new(content).color(egui::Color32::RED)).wrap());
        }
        "tool" => {
            // One `ui.add` per row in a vertical layout, so two consecutive
            // actions are two lines and never run on (AC 2).
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("▸ {content}"))
                        .text_style(egui::TextStyle::Monospace)
                        .color(TOOL_COLOR),
                )
                .wrap(),
            );
        }
        "refused" => {
            // The refusal text is LCV-123's and is rendered verbatim; only the
            // marker and the colour are added here.
            let warn = ui.visuals().warn_fg_color;
            ui.add(
                egui::Label::new(egui::RichText::new(format!("⚠ {content}")).color(warn)).wrap(),
            );
        }
        "note" => {
            // The separator bounds the end of a turn, so the undo shape is
            // findable without scrolling back through it (AC 4).
            ui.separator();
            ui.add(
                egui::Label::new(egui::RichText::new(content).text_style(egui::TextStyle::Small))
                    .wrap(),
            );
        }
        _ => {
            ui.label(content);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::agent::AgentEvent;

    /// LCV-141 AC 4 — the busy row's reservation is strictly additional, and
    /// only while a turn is running.
    #[test]
    fn footer_reserve_adds_the_busy_row_only_while_busy() {
        assert_eq!(super::footer_reserve(false), super::COMPOSER_RESERVE);
        assert_eq!(
            super::footer_reserve(true),
            super::COMPOSER_RESERVE + super::BUSY_ROW_RESERVE
        );
    }

    /// The implementation section — everything before the bare `#[cfg(test)]`
    /// at column 0 — **with comment lines removed**.
    ///
    /// Both halves matter. The bound stops a scan matching the test literal
    /// written next to it; dropping comments stops it matching the prose that
    /// describes the code, which is what would let a deleted match arm keep
    /// passing on the strength of the doc table above it.
    fn implementation_code() -> String {
        let src = include_str!("panel.rs");
        let at = src
            .find("\n#[cfg(test)]")
            .expect("panel.rs must have a bare #[cfg(test)] marker");
        let code: Vec<&str> = src[..at]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect();
        assert!(
            code.len() > 60,
            "positive control: the haystack must be the whole implementation, got {} lines",
            code.len()
        );
        code.join("\n")
    }

    /// The body of one `match` arm: from `marker` to the start of the next arm.
    ///
    /// This is what makes the per-role scans below discriminate. A needle
    /// looked up in the whole file would still be found after its arm was
    /// deleted — `ui.separator()` appears three times in this file, only one of
    /// them in the `note` arm — while a missing `marker` fails here, by name.
    fn arm_body(implementation: &str, marker: &str) -> String {
        let start = implementation
            .find(marker)
            .unwrap_or_else(|| panic!("panel.rs must render a `{marker}` arm"));
        let rest = &implementation[start + marker.len()..];
        let end = rest.find(" =>").unwrap_or(rest.len());
        rest[..end].to_owned()
    }

    /// The six role markers, in render order, each built with `concat!`.
    fn role_markers() -> [(&'static str, String); 6] {
        [
            ("user", format!("{}{}", "\"us", "er\" =>")),
            ("assistant", format!("{}{}", "\"assist", "ant\" =>")),
            ("error", format!("{}{}", "\"err", "or\" =>")),
            ("tool", format!("{}{}", "\"to", "ol\" =>")),
            ("refused", format!("{}{}", "\"refu", "sed\" =>")),
            ("note", format!("{}{}", "\"no", "te\" =>")),
        ]
    }

    /// LCV-080 AC#3, retargeted by LCV-122 — the terminal events the spawned
    /// thread sends are distinct, and they are now the bridge's, not the
    /// panel's. `panel.rs` declaring its own payload type again is what this
    /// test's sibling scan in `tests/lcv122_source_scans.rs` catches.
    #[test]
    fn the_two_terminal_events_are_distinct() {
        let done = AgentEvent::Done("x".into());
        let failed = AgentEvent::Failed("e".into());
        assert_ne!(
            std::mem::discriminant(&done),
            std::mem::discriminant(&failed),
            "Done and Failed must be distinct variants",
        );
    }

    /// LCV-123 AC 2 / LCV-125 AC 7 / ADR 0007 §D8 — the panel renders and
    /// reports. It builds no document state and starts no thread; the whole of
    /// submitting is one call into `crate::app::start_turn`.
    ///
    /// A bounded source scan, because the property is *what this file is
    /// allowed to contain* and no runtime test can observe absence. The
    /// haystack stops at the bare `#[cfg(test)]` at column 0 and every needle
    /// is built with `concat!`, so this test cannot match its own source — and
    /// the loop below proves it: each needle is first found in a witness string
    /// that spells out what the pre-LCV-123 `submit` did, so a needle that had
    /// been silently misspelt fails here instead of passing vacuously.
    #[test]
    fn the_panel_renders_and_reports() {
        let implementation = implementation_code();

        assert!(
            implementation.contains(concat!("crate::app::start", "_turn(app, &prompt)")),
            "positive control: submitting must be one call into start_turn"
        );

        // What `submit` used to be, verbatim enough for every needle to hit.
        let witness = "let mut doc = Document::default(); \
                       let mut history = History::default(); \
                       let (tx, rx) = std::sync::mpsc::channel::<AgentEvent>(); \
                       std::thread::spawn(move || {}); fn submit(app: &mut App)";
        for forbidden in [
            concat!("Doc", "ument"),
            concat!("His", "tory"),
            concat!("thread", "::spawn"),
            concat!("mpsc", "::channel"),
            concat!("fn sub", "mit"),
        ] {
            assert!(
                witness.contains(forbidden),
                "control: `{forbidden}` must be a needle that can match something"
            );
            let hit = implementation
                .lines()
                .find(|l| l.contains(forbidden) && !l.trim_start().starts_with("//"));
            assert!(
                hit.is_none(),
                "panel.rs must not name `{forbidden}`: {hit:?}"
            );
        }
    }

    /// AC 1 — **source scan**, not a rendering assertion: the six roles LCV-123
    /// writes each have their own arm, and the `_` fallback survives next to
    /// them. What the six *look like* is the reviewer's eye and the manual
    /// smoke; what this pins is that none of them falls through.
    #[test]
    fn ac1_the_six_roles_each_have_an_arm_source_scan() {
        let implementation = implementation_code();
        for (name, marker) in role_markers() {
            assert!(
                implementation.contains(&marker),
                "role `{name}` must have its own arm ({marker})"
            );
        }
        assert!(
            implementation.contains(concat!("_ =", "> {")),
            "the plain-label fallback arm must stay as the safety net"
        );
    }

    /// AC 6 — **source scan**: the panel never reads the API key. The key's
    /// absence from the transcript is proven at runtime in
    /// `tests/lcv125_agent_panel_and_settings.rs`; this pins that the panel
    /// does not even have the field in hand.
    #[test]
    fn ac6_the_panel_never_reads_the_api_key_source_scan() {
        let implementation = implementation_code();
        let needle = concat!("agent_api", "_key");
        let witness = "let key = app.settings.agent_api_key.clone();";
        assert!(
            witness.contains(needle),
            "control: `{needle}` must be a needle that can match something"
        );
        assert!(
            !implementation.contains(needle),
            "panel.rs must never read `{needle}`"
        );
    }

    /// AC 2 — **source scan** over the `tool` arm alone: monospace, the `▸`
    /// marker, and a colour that is not the error red.
    #[test]
    fn ac2_the_tool_arm_is_monospace_marked_and_coloured_source_scan() {
        let implementation = implementation_code();
        let arm = arm_body(&implementation, &role_markers()[3].1);
        for needle in [
            concat!("TextStyle::Mono", "space"),
            "▸ {content}",
            concat!("TOOL_", "COLOR"),
        ] {
            assert!(
                arm.contains(needle),
                "the tool arm must contain `{needle}`: {arm}"
            );
        }
        assert!(
            !arm.contains(concat!("Color32::", "RED")),
            "an action is not an error: {arm}"
        );
    }

    /// AC 3 — **source scan** over the `refused` arm alone: the `⚠` marker and
    /// egui's warning colour, which is neither the prose colour nor the red.
    #[test]
    fn ac3_the_refused_arm_is_warning_coloured_and_marked_source_scan() {
        let implementation = implementation_code();
        let arm = arm_body(&implementation, &role_markers()[4].1);
        for needle in [concat!("warn_fg", "_color"), "⚠ {content}"] {
            assert!(
                arm.contains(needle),
                "the refused arm must contain `{needle}`: {arm}"
            );
        }
        assert!(
            !arm.contains(concat!("Color32::", "RED")),
            "a refusal is not an error (AC 3): {arm}"
        );
    }

    /// AC 4 — **source scan** over the `note` arm alone: the leading separator
    /// that bounds the turn, and the small text style.
    #[test]
    fn ac4_the_note_arm_is_small_after_a_separator_source_scan() {
        let implementation = implementation_code();
        let arm = arm_body(&implementation, &role_markers()[5].1);
        for needle in [
            concat!("ui.separ", "ator()"),
            concat!("TextStyle::Sm", "all"),
        ] {
            assert!(
                arm.contains(needle),
                "the note arm must contain `{needle}`: {arm}"
            );
        }
    }

    /// AC 5 — **source scan**: the scroll area still sticks to the bottom, and
    /// every one of the six role arms wraps its text.
    #[test]
    fn ac5_rows_wrap_and_the_scroll_sticks_to_bottom_source_scan() {
        let implementation = implementation_code();
        assert!(
            implementation.contains(concat!(".stick_to_bot", "tom(true)")),
            "the transcript must keep following the newest row"
        );
        for (name, marker) in role_markers() {
            let arm = arm_body(&implementation, &marker);
            assert!(
                arm.contains(concat!(".wr", "ap()")),
                "the `{name}` arm must wrap rather than widen the panel: {arm}"
            );
        }
    }

    /// AC 2 / AC 3 — a **runtime** assertion, not a scan: under the theme the
    /// app actually applies, the four row colours are four different colours.
    /// If `apply_theme` ever moves prose onto the accent, this fails here.
    #[test]
    fn the_row_colours_are_distinct_under_the_real_theme() {
        let ctx = egui::Context::default();
        crate::ui::apply_theme(&ctx);
        let visuals = ctx.style().visuals.clone();
        let prose = visuals.text_color();
        let warn = visuals.warn_fg_color;
        let error = egui::Color32::RED;
        for (a, b) in [
            (super::TOOL_COLOR, prose),
            (super::TOOL_COLOR, warn),
            (super::TOOL_COLOR, error),
            (warn, prose),
            (warn, error),
            (prose, error),
        ] {
            assert_ne!(a, b, "these two rows would look the same");
        }
    }

    /// The body of the `if app.agent.busy { .. }` block, brace-matched.
    ///
    /// Slicing to the closing brace is what makes the two LCV-129 scans
    /// discriminate: a `Cancel` button moved one line down, out of the block
    /// and into the unconditional part of the panel, is still in the file and
    /// still spelled the same — but it is no longer in *this* string.
    fn busy_block(implementation: &str) -> String {
        let head = concat!("if app.agent", ".busy {");
        let start = implementation
            .find(head)
            .unwrap_or_else(|| panic!("panel.rs must guard its thinking row on `{head}`"))
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
        panic!("the busy block is never closed — panel.rs does not parse");
    }

    /// LCV-129 AC 8 — **source scan**: the Cancel button is inside the busy
    /// block, and its body is one call into the app.
    ///
    /// What it is painted like, and that it is painted at all, is asserted at
    /// runtime in `tests/lcv129_agent_timeout_and_cancel.rs`. What only a scan
    /// can say is the second half: that the click handler does *nothing else* —
    /// no `agent.busy` assignment here, no `agent.rx` cleared here, because
    /// ending a turn is `agent_poll`'s alone (ADR 0007 §D11).
    #[test]
    fn ac8_the_cancel_button_lives_inside_the_busy_block_source_scan() {
        let implementation = implementation_code();
        let block = busy_block(&implementation);

        for needle in [
            concat!("\"Can", "cel\""),
            concat!("crate::app::cancel", "_turn(app)"),
        ] {
            assert!(
                block.contains(needle),
                "AC 8: `{needle}` must sit inside the busy block: {block}"
            );
        }

        let witness = "app.agent.busy = false; app.agent.rx = None;";
        for forbidden in [
            concat!("agent.busy =", " false"),
            concat!("agent.rx =", " "),
        ] {
            assert!(
                witness.contains(forbidden),
                "control: `{forbidden}` must be a needle that can match something"
            );
            assert!(
                !implementation.contains(forbidden),
                "AC 8 / §D11: panel.rs must not end a turn itself (`{forbidden}`)"
            );
        }
    }

    /// One bounded `egui::Ui` of `width` points, handed to `draw_chat_row`;
    /// answers with the space the row actually took.
    fn row_size(role: &str, content: &str, width: f32) -> egui::Vec2 {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(1.0);
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
                // A scope, because a panel's own `min_rect` is the whole panel:
                // the child's is the space the row actually asked for.
                size = ui
                    .scope(|ui| {
                        ui.set_max_width(width);
                        super::draw_chat_row(ui, role, content);
                    })
                    .response
                    .rect
                    .size();
            });
        });
        size
    }

    /// AC 5 — a **rendering** assertion: a 300-character row folds inside the
    /// panel width instead of running off it. The `user` arm is the one that
    /// can really fail — its right-to-left layout defaults to `Extend` — so
    /// dropping `.wrap()` there makes this fail rather than merely the scan.
    #[test]
    fn ac5_a_three_hundred_character_row_wraps_within_the_width() {
        let long = "outcome ".repeat(38); // 304 characters
        assert!(long.len() >= 300);
        for role in ["user", "assistant", "tool", "refused", "note"] {
            let size = row_size(role, &long, 240.0);
            assert!(
                size.x <= 241.0,
                "a `{role}` row widened the panel to {}",
                size.x
            );
            assert!(
                size.y > 40.0,
                "a `{role}` row of 300 characters must fold onto several lines, took {}",
                size.y
            );
        }
    }
}
