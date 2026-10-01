//! Which open dialog is on top (LCV-169 AC 1): the one that Enter and
//! Escape go to.
//!
//! [`App::dialog_order`] lists the open dialogs, oldest first. Each frame,
//! [`sync_dialog_order`] drops the ones that closed and appends the ones that
//! opened, in [`Dialog`] declaration order when several open in one frame, so
//! `Error` (declared last) lands on top of anything opened with it. The
//! [`topmost`] dialog is the last entry.

use super::App;

/// One of the seven in-app dialogs, in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// Help > About LaserCAD.
    About,
    /// The keyboard-shortcuts dialog (F1).
    Shortcuts,
    /// AI Settings.
    AiSettings,
    /// Layers….
    Layers,
    /// Bed Size….
    Bed,
    /// The Save / Discard / Cancel prompt.
    Discard,
    /// The error modal.
    Error,
}

impl Dialog {
    /// Every dialog, in declaration order.
    const ALL: [Self; 7] = [
        Self::About,
        Self::Shortcuts,
        Self::AiSettings,
        Self::Layers,
        Self::Bed,
        Self::Discard,
        Self::Error,
    ];

    /// Whether this dialog is open in `app`.
    fn is_open(self, app: &App) -> bool {
        match self {
            Self::About => app.about_open,
            Self::Shortcuts => app.shortcuts_open,
            Self::AiSettings => app.agent_settings_open,
            Self::Layers => app.layers_dialog.is_some(),
            Self::Bed => app.bed_dialog.is_some(),
            Self::Discard => app.guard.pending_action.is_some(),
            Self::Error => app.error_message.is_some(),
        }
    }
}

/// Bring [`App::dialog_order`] in line with the dialogs open now: closed ones
/// leave, newly opened ones join on top, in declaration order.
pub fn sync_dialog_order(app: &mut App) {
    let mut order = std::mem::take(&mut app.dialog_order);
    order.retain(|d| d.is_open(app));
    for d in Dialog::ALL {
        if d.is_open(app) && !order.contains(&d) {
            order.push(d);
        }
    }
    app.dialog_order = order;
}

/// The dialog on top, if any is open, as of the last [`sync_dialog_order`].
pub fn topmost(app: &App) -> Option<Dialog> {
    app.dialog_order.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::PendingAction;

    #[test]
    fn nothing_open_means_no_topmost() {
        let mut app = App::default();
        sync_dialog_order(&mut app);
        assert_eq!(topmost(&app), None);
    }

    #[test]
    fn the_later_opened_dialog_is_on_top() {
        let mut app = App {
            shortcuts_open: true,
            ..App::default()
        };
        sync_dialog_order(&mut app);
        app.about_open = true;
        sync_dialog_order(&mut app);
        assert_eq!(app.dialog_order, [Dialog::Shortcuts, Dialog::About]);
        app.about_open = false;
        sync_dialog_order(&mut app);
        assert_eq!(topmost(&app), Some(Dialog::Shortcuts));
    }

    #[test]
    fn dialogs_opened_together_stack_in_declaration_order() {
        let mut app = App {
            error_message: Some("boom".into()),
            about_open: true,
            ..App::default()
        };
        app.guard.pending_action = Some(PendingAction::New);
        sync_dialog_order(&mut app);
        let want = [Dialog::About, Dialog::Discard, Dialog::Error];
        assert_eq!(app.dialog_order, want);
    }
}
