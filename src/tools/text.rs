//! TextTool — AutoCAD R14 `DTEXT`. Click the insertion point, type the
//! string into the command line, type the height (or accept the 5 mm
//! default), commit as one [`CreateEntities`] carrying every Hershey stroke.
//! Uses `layout_text` (LCV-055). Never mutates the document directly.
//!
//! Raw-input state machine (ADR 0003 §D, LCV-112):
//! `Idle -> WaitingText -> WaitingHeight -> Idle`. While not `Idle`,
//! [`Tool::wants_raw_input`] is true and the command line becomes a plain
//! text field instead of being parsed — see `src/app/cmdline.rs::submit`
//! and `src/ui/command_line.rs::draw_command_line`. A tool in this mode
//! never sees per-character events; it sees exactly one submitted string
//! per phase, via [`Tool::on_raw_input`].
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-048;
//! rebuilt on raw command-line input by LCV-112.

use crate::app::App;
use crate::cmdline::parse_number;
use crate::document::{Document, Entity, History, commands::CreateEntities};
use crate::geometry::Vec2;
use crate::text::layout_text;
use crate::tools::Tool;
use std::borrow::Cow;

/// Height committed when the operator accepts the default at the height
/// prompt (empty Enter) — the R14 `DTEXT` default. Also the height used by
/// the `WaitingHeight` placement preview (AC 9): TEXT does not remember a
/// previous height (§Out of scope), so the preview always shows this value.
const DEFAULT_TEXT_HEIGHT_MM: f64 = 5.0;
/// Hershey stroke spacing; TEXT never varies it (§Out of scope).
const DEFAULT_SPACING_FACTOR: f64 = 1.0;
/// Minimum accepted height, in mm — roughly one laser kerf. Below it,
/// adjacent Hershey strokes fuse into an unreadable scorch.
const MIN_HEIGHT_MM: f64 = 0.1;
/// Maximum accepted height, in mm — `BED_MAX_MM` (LCV-114). Text taller than
/// the largest possible bed cannot be cut and would wreck zoom-extents.
const MAX_HEIGHT_MM: f64 = 2000.0;

#[derive(Debug)]
enum TextToolState {
    Idle,
    WaitingText {
        anchor: Vec2,
    },
    WaitingHeight {
        anchor: Vec2,
        text: String,
        /// Set when the previous height submission was refused, so
        /// `status_text` can show the retry prompt (AC 6) without carrying a
        /// runtime-interpolated `String` (`status_text` stays `&'static str`;
        /// TEXT never remembers a height, so a second literal is all that's
        /// needed).
        invalid: bool,
    },
}

/// Click-anchor, type-string, type-height text tool (LCV-048, rebuilt by
/// LCV-112 on raw command-line input).
///
/// `Idle` — click sets the anchor — `WaitingText`. The typed string, once
/// submitted with Enter, either cancels back to `Idle` (blank/whitespace —
/// decision 5) or advances to `WaitingHeight`. The typed height, once
/// submitted, commits **exactly one** [`CreateEntities`] carrying every
/// glyph stroke and returns to `Idle` (empty height commits at the 5 mm
/// default; an out-of-range or unparseable height re-prompts and keeps the
/// text — decisions 2–4). `Escape` / [`Tool::cancel`] returns to `Idle`
/// uncommitted from any state.
#[derive(Debug)]
pub struct TextTool {
    state: TextToolState,
}

impl Default for TextTool {
    fn default() -> Self {
        Self {
            state: TextToolState::Idle,
        }
    }
}

impl Tool for TextTool {
    fn name(&self) -> &'static str {
        "TEXT"
    }

    /// The AC 6 prompt table, pulled fresh every frame.
    fn status_text(&self) -> Cow<'_, str> {
        match &self.state {
            TextToolState::Idle => "TEXT  Specify start point:".into(),
            TextToolState::WaitingText { .. } => "TEXT  Specify text:".into(),
            TextToolState::WaitingHeight { invalid: false, .. } => {
                format!("TEXT  Specify height <{DEFAULT_TEXT_HEIGHT_MM}>:").into()
            }
            // The refusal keeps its sentence before `Specify` (LCV-165).
            TextToolState::WaitingHeight { invalid: true, .. } => format!(
                "TEXT  Height must be between {MIN_HEIGHT_MM} and {MAX_HEIGHT_MM} mm. \
                 Specify height <{DEFAULT_TEXT_HEIGHT_MM}>:"
            )
            .into(),
        }
    }

    /// True in both waiting states: the command line is a free-text field,
    /// not the parser, until the height is accepted or the tool cancels.
    fn wants_raw_input(&self) -> bool {
        !matches!(self.state, TextToolState::Idle)
    }

    /// First click transitions `Idle -> WaitingText`. A click while already
    /// waiting is a no-op: the anchor does not move and nothing commits.
    fn on_pointer_down(
        &mut self,
        pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
        if let TextToolState::Idle = self.state {
            self.state = TextToolState::WaitingText { anchor: pos };
        }
    }

    fn on_pointer_move(&mut self, _pos: Vec2, _doc: &mut Document) {}

    fn on_pointer_up(
        &mut self,
        _pos: Vec2,
        _shift: bool,
        _doc: &mut Document,
        _history: &mut History,
    ) {
    }

    /// Escape is the only key `TextTool` still reads directly — every other
    /// keystroke while waiting is text for the command line, handled through
    /// [`Tool::on_raw_input`] instead. `_app` is unused; the parameter stays
    /// because [`Tool::on_key`]'s signature is shared by every tool.
    fn on_key(&mut self, key: egui::Key, _app: &mut App) {
        if key == egui::Key::Escape {
            self.cancel();
        }
    }

    /// The submitted text, unparsed (ADR 0003 §D, AC 5). Returns `false`
    /// only from `Idle`, where raw input is never requested in the first
    /// place, so the app never treats this tool's own text as unhandled.
    fn on_raw_input(&mut self, raw: &str, doc: &mut Document, history: &mut History) -> bool {
        match std::mem::replace(&mut self.state, TextToolState::Idle) {
            TextToolState::Idle => false,
            TextToolState::WaitingText { anchor } => {
                // Decision 5: an empty (or whitespace-only) string cancels —
                // committing zero glyphs would put an empty undo entry on
                // the stack. `self.state` is already `Idle` from the
                // `mem::replace` above.
                if !raw.trim().is_empty() {
                    self.state = TextToolState::WaitingHeight {
                        anchor,
                        text: raw.to_owned(),
                        invalid: false,
                    };
                }
                true
            }
            TextToolState::WaitingHeight { anchor, text, .. } => {
                match resolve_height(raw) {
                    Some(height_mm) => commit(&text, anchor, height_mm, doc, history),
                    None => {
                        self.state = TextToolState::WaitingHeight {
                            anchor,
                            text,
                            invalid: true,
                        };
                    }
                }
                true
            }
        }
    }

    /// `Idle` and `WaitingText` preview nothing: the string lives in the
    /// command line's `TextEdit` and this tool does not see it until Enter
    /// (§Risks). `WaitingHeight` previews the placement at the 5 mm default
    /// while the operator decides the real height (decision 4, AC 9).
    fn preview(&self) -> Vec<Entity> {
        match &self.state {
            TextToolState::WaitingHeight { anchor, text, .. } => layout_text(
                text,
                *anchor,
                DEFAULT_TEXT_HEIGHT_MM,
                DEFAULT_SPACING_FACTOR,
            ),
            _ => vec![],
        }
    }

    fn cancel(&mut self) {
        self.state = TextToolState::Idle;
    }
}

/// Resolve the height prompt's submitted text: `""` (or whitespace-only)
/// means the 5 mm default (decision 4); otherwise the value must parse
/// through [`parse_number`] — the product's one number grammar, so `10,5`
/// is a coordinate pair, not a height (decision 7) — and fall within
/// `[0.1, 2000.0]` inclusive (decision 2). Never clamps (decision 3): a
/// `None` here is what makes the caller stay in `WaitingHeight` and re-ask.
fn resolve_height(raw: &str) -> Option<f64> {
    if raw.trim().is_empty() {
        return Some(DEFAULT_TEXT_HEIGHT_MM);
    }
    parse_number(raw).filter(|h| (MIN_HEIGHT_MM..=MAX_HEIGHT_MM).contains(h))
}

/// Lay out `text` at `height_mm` and commit it as one [`CreateEntities`]
/// (decision 6 — exactly one undo entry for the whole string, via
/// `history.commit`, never `App::commit`). A glyphless result (e.g. a string
/// with no Hershey strokes at all) commits nothing (AC 7).
fn commit(text: &str, anchor: Vec2, height_mm: f64, doc: &mut Document, history: &mut History) {
    let strokes = layout_text(text, anchor, height_mm, DEFAULT_SPACING_FACTOR);
    if !strokes.is_empty() {
        history.commit(Box::new(CreateEntities::new(strokes)), doc);
    }
}

#[cfg(test)]
mod tests;
