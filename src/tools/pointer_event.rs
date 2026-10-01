//! PointerEvent: unified pointer event type for tool dispatch (LCV-041).
//!
//! The three variants — `Move`, `Press`, `Release` — carry the effective
//! world-space cursor position in millimetres. The caller (see
//! [`super::ToolManager::on_pointer_event`]) resolves the active snap and
//! passes the snapped point as `world_pos` when a snap is active.
//!
//! `PointerButton` lets the manager filter for primary-button events before
//! delegating to the active tool; secondary and middle events are no-ops for
//! now (future demands can add context-menu / orbit handling).
//!
//! MUST NOT import `eframe` or `rfd`. Introduced by demand LCV-041.

use crate::geometry::Vec2;

/// Which mouse button was involved in a [`PointerEvent::Press`] or
/// [`PointerEvent::Release`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// Left / primary button — the main draw button.
    Primary,
    /// Right / secondary button — never reaches a tool: the viewport turns
    /// a right press into Enter on an empty line (LCV-165 AC 6).
    Secondary,
    /// Middle button / wheel click — reserved for pan (handled outside tools).
    Middle,
}

/// A pointer event delivered to the active tool via
/// [`super::ToolManager::on_pointer_event`].
///
/// World positions are in millimetres (Y-up). The caller resolves the active
/// snap and substitutes the snapped point when a snap is live.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerEvent {
    /// Cursor moved to `world_pos`.
    ///
    /// Fired every frame while the cursor hovers the viewport, regardless of
    /// button state.
    Move {
        /// World-space cursor position (mm, Y-up).
        world_pos: Vec2,
    },
    /// A button was pressed at `world_pos`.
    Press {
        /// World-space cursor position (mm, Y-up).
        world_pos: Vec2,
        /// The button that was pressed.
        button: PointerButton,
        /// `true` when the Shift key is held at the moment of the press.
        shift: bool,
    },
    /// A button was released at `world_pos`.
    Release {
        /// World-space cursor position (mm, Y-up).
        world_pos: Vec2,
        /// The button that was released.
        button: PointerButton,
        /// `true` when the Shift key is held at the moment of the release.
        shift: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PointerButton has exactly three variants and derives PartialEq / Eq.
    #[test]
    fn pointer_button_variants_are_distinct() {
        assert_ne!(PointerButton::Primary, PointerButton::Secondary);
        assert_ne!(PointerButton::Primary, PointerButton::Middle);
        assert_ne!(PointerButton::Secondary, PointerButton::Middle);
    }

    /// PointerEvent::Move carries world_pos through the variant.
    #[test]
    fn pointer_event_move_carries_world_pos() {
        let ev = PointerEvent::Move {
            world_pos: Vec2::new(3.0, 4.0),
        };
        if let PointerEvent::Move { world_pos } = ev {
            assert_eq!(world_pos, Vec2::new(3.0, 4.0));
        } else {
            panic!("wrong variant");
        }
    }

    /// PointerEvent::Press and Release carry world_pos, button, and shift.
    #[test]
    fn pointer_event_press_release_carry_all_fields() {
        let press = PointerEvent::Press {
            world_pos: Vec2::new(1.0, 2.0),
            button: PointerButton::Primary,
            shift: true,
        };
        let release = PointerEvent::Release {
            world_pos: Vec2::new(5.0, 6.0),
            button: PointerButton::Secondary,
            shift: false,
        };

        if let PointerEvent::Press {
            world_pos,
            button,
            shift,
        } = press
        {
            assert_eq!(world_pos, Vec2::new(1.0, 2.0));
            assert_eq!(button, PointerButton::Primary);
            assert!(shift);
        } else {
            panic!("wrong variant");
        }

        if let PointerEvent::Release {
            world_pos,
            button,
            shift,
        } = release
        {
            assert_eq!(world_pos, Vec2::new(5.0, 6.0));
            assert_eq!(button, PointerButton::Secondary);
            assert!(!shift);
        } else {
            panic!("wrong variant");
        }
    }

    /// PointerEvent is Copy — no clone needed.
    #[test]
    fn pointer_event_is_copy() {
        let ev = PointerEvent::Move {
            world_pos: Vec2::new(0.0, 0.0),
        };
        let _copy = ev; // copy, not move
        let _ = ev; // still usable
    }
}
