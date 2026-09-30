//! Status bar widget rendered at the bottom of the application window.
//!
//! Exposes two public items:
//! - [`format_coords`] — pure string formatter for cursor coordinates (unit-testable).
//! - [`draw_statusbar`] — egui widget that reads live state from [`App`] and
//!   renders the bar.
//!
//! The badge formatters ([`format_autosave`]) and the mode
//! toggle ([`apply_toggle`]) are crate-private pure functions: the bar itself
//! cannot be scraped for text, so they are what the tests assert on.
//!
//! **No key is read here.** The two keyboard readers are
//! `src/ui/shortcuts.rs` and `src/app/input.rs` (ADR 0002 §A6); the mode
//! indicators below are a *second path* to the same three flags, never a
//! second reader of the keys that also flip them.
//!
//! Introduced by demand LCV-067; the clickable mode indicators and the
//! autosave indicator by LCV-116.

use crate::app::App;
use crate::geometry::Vec2;

/// The on/off mode pill (LCV-184).
mod pill;

/// Width, in chars, of one coordinate number: sign, four digits, point and
/// two decimals (LCV-167 AC 6).
const COORD_WIDTH: usize = 8;

/// Format cursor world-space coordinates for display in the status bar.
///
/// Returns `"X: <x>mm  Y: <y>mm"` when `pos` is `Some`: each value to
/// 2 dp with an explicit `mm` unit (LCV-140 AC 4, millimetres being canonical
/// everywhere outside `render/camera`), right-aligned to [`COORD_WIDTH`] chars
/// with figure spaces (U+2007, as wide as a digit) so no digit moves when a
/// sign appears (LCV-167 AC 6). A value that does not fit grows the string.
/// Returns `"X: —  Y: —"` (em-dash, no unit) when `pos` is `None`.
///
/// # Examples
/// ```
/// use lasercad::ui::format_coords;
/// use lasercad::geometry::Vec2;
///
/// assert_eq!(format_coords(None), "X: —  Y: —");
/// let s = format_coords(Some(Vec2::new(123.45, -67.89)));
/// assert_eq!(s, "X: \u{2007}\u{2007}123.45mm  Y: \u{2007}\u{2007}-67.89mm");
/// ```
pub fn format_coords(pos: Option<Vec2>) -> String {
    let pad = |v: f64| {
        let n = format!("{v:.2}");
        let fill = COORD_WIDTH.saturating_sub(n.chars().count());
        format!("{}{n}", "\u{2007}".repeat(fill))
    };
    match pos {
        None => "X: \u{2014}  Y: \u{2014}".to_owned(),
        Some(p) => format!("X: {}mm  Y: {}mm", pad(p.x), pad(p.y)),
    }
}

/// One of the three operator modes the status bar exposes as a clickable
/// indicator (LCV-116 AC 2).
///
/// The variant is the whole identity of a mode: it names the label, the flag
/// it reads and the flag [`apply_toggle`] flips, so the three can never drift
/// apart the way three parallel `if` arms would.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    /// `app.snap_enabled` — also toggled by `F3` and by `View > Snap`.
    Snap,
    /// `app.grid_enabled` — also toggled by `F7` and by `View > Grid`.
    Grid,
    /// `app.ortho_enabled` — also toggled by `F8` and by `View > Ortho`.
    Ortho,
}

impl Mode {
    /// The three modes, in the order the bar renders them.
    pub(crate) const ALL: [Mode; 3] = [Mode::Snap, Mode::Grid, Mode::Ortho];

    /// The indicator's label. Uppercase, matching the tool badge.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Mode::Snap => "SNAP",
            Mode::Grid => "GRID",
            Mode::Ortho => "ORTHO",
        }
    }

    /// Is this mode currently on? Drives the indicator's *selected* state, so
    /// an off mode is still visible — a badge that vanished when off would be
    /// indistinguishable from an app with no such mode (LCV-116 decision 1).
    pub(crate) fn is_on(self, app: &App) -> bool {
        match self {
            Mode::Snap => app.snap_enabled,
            Mode::Grid => app.grid_enabled,
            Mode::Ortho => app.ortho_enabled,
        }
    }

    /// The existing keyboard shortcut that also flips this mode (LCV-140 AC
    /// 3): the same key `src/ui/shortcuts.rs::dispatch_shortcuts` reads for
    /// it — restated here only as a hover-tooltip label, never a second
    /// reader of the key itself (this method reads no input).
    pub(crate) fn key_hint(self) -> &'static str {
        match self {
            Mode::Snap => "F3",
            Mode::Grid => "F7",
            Mode::Ortho => "F8",
        }
    }
}

/// Flip exactly the flag `mode` names, and nothing else (LCV-116 AC 3).
///
/// The same single-flag mutation `F3` / `F7` / `F8` perform in
/// `src/ui/shortcuts.rs`: no document mutation, no history entry, no change to
/// `dirty_since`. Snap turning **off** is picked up by the next frame's
/// `suppress_snap_if_disabled`, which clears `active_snap`, so a stale snap
/// marker cannot survive the click (AC 4).
pub(crate) fn apply_toggle(app: &mut App, mode: Mode) {
    match mode {
        Mode::Snap => app.snap_enabled = !app.snap_enabled,
        Mode::Grid => app.grid_enabled = !app.grid_enabled,
        Mode::Ortho => app.ortho_enabled = !app.ortho_enabled,
    }
}

/// Return the autosave indicator string (LCV-116 AC 8).
///
/// Three permanent states, no timer and no animation — a relative
/// "saved N seconds ago" would need a repaint every second for the life of the
/// process and would go stale the moment repaints stopped:
///
/// - a write is pending → `"● autosave pending"`;
/// - otherwise, at least one write succeeded this session → `"○ autosaved"`;
/// - otherwise → `"○ no autosave yet"`.
///
/// `write_pending` wins over `ever_saved`: what the operator needs to know is
/// whether the *current* state of the drawing is on disk.
///
/// This does **not** report autosave *failures*. The error behind
/// `App::write_autosave`'s `false` is
/// swallowed today (LCV-102); surfacing it is its own demand. What the
/// indicator distinguishes is "an autosave has happened this session" from
/// "none has", which is honest with the information available.
pub(crate) fn format_autosave(write_pending: bool, ever_saved: bool) -> &'static str {
    if write_pending {
        "\u{25cf} autosave pending"
    } else if ever_saved {
        "\u{25cb} autosaved"
    } else {
        "\u{25cb} no autosave yet"
    }
}

/// The recovery label's text (LCV-138 AC 4) — distinct from
/// [`format_autosave`]'s badge, which reports whether a *write* happened this
/// session; this one reports whether the session's document *started* from a
/// crash-safety copy at all, a fact `format_autosave` cannot see.
pub(crate) const RECOVERY_LABEL: &str = "Recovered (not saved)";

/// The recovery label's hover text, spelling out the autosave-vs-save
/// distinction so "recovered" never leaves the operator guessing.
pub(crate) const RECOVERY_HOVER_TEXT: &str = "This drawing was restored from a crash-safety \
     copy. The file on disk, if any, is untouched until you save.";

/// Render the status bar into `ui`.
///
/// Displays — left to right — cursor coordinates, the active tool name
/// (uppercased), the document entity count, the current-layer dropdown
/// (LCV-156, `src/ui/layer_combo.rs`), the three clickable mode
/// indicators `SNAP` / `GRID` / `ORTHO` (LCV-116, always visible; LCV-184 on/off
/// pills, filled iff their flag is on), the autosave
/// indicator, and — only while [`App::title`]'s
/// `recovered_from_autosave` is set (LCV-138) — a recovery label with an
/// explanatory tooltip, all separated in the existing style.
///
/// Takes `&mut App` since LCV-116: the mode indicators write back the flag
/// they display. `src/app/panels.rs::draw_chrome` is the only caller and
/// already holds `&mut App`.
///
/// **Call site**: add a `TopBottomPanel::bottom("statusbar")` *before* the
/// `CentralPanel` in `App::update`.
pub fn draw_statusbar(ui: &mut egui::Ui, app: &mut App) {
    let coord_str = format_coords(app.last_cursor_world);
    let tool_str = app.tool_manager.active_tool_name().to_uppercase();
    let count = app.document.entity_count();
    let autosave_str = format_autosave(app.dirty_since.is_some(), app.last_autosave_at.is_some());

    // At most one indicator can be clicked per frame; the flip is applied
    // after the loop so the borrow of `app` inside it stays shared.
    let mut toggled: Option<Mode> = None;
    let mut picked_layer = None;

    ui.horizontal(|ui| {
        // LCV-184 AC 5: fixed-width digits, so the readout does not jitter.
        ui.label(egui::RichText::new(&coord_str).monospace());
        ui.separator();
        ui.label(&tool_str);
        ui.separator();
        ui.label(format!("Entities: {count}"));
        ui.separator();
        picked_layer = super::layer_combo::layer_combo(ui, app);
        for mode in Mode::ALL {
            ui.separator();
            let hint = format!("{} ({})", mode.label(), mode.key_hint());
            if pill::mode_pill(ui, mode.is_on(app), mode.label())
                .on_hover_text(hint)
                .clicked()
            {
                toggled = Some(mode);
            }
        }
        ui.separator();
        ui.label(autosave_str);
        // LCV-138 AC 4 — distinct from the autosave badge above: this
        // session's document came back from the crash-safety copy at boot.
        // Reading the flag and painting a label/tooltip touches nothing
        // else — no `mark_saved`/`mark_clean`, no `dirty_since`, no
        // `last_autosave_at`.
        if app.title.recovered_from_autosave {
            ui.separator();
            ui.label(RECOVERY_LABEL).on_hover_text(RECOVERY_HOVER_TEXT);
        }
    });

    if let Some(mode) = toggled {
        apply_toggle(app, mode);
    }
    if let Some(id) = picked_layer {
        app.commit(Box::new(crate::document::SetCurrentLayer::new(id)));
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
